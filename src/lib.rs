use js_sys::Math::random;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture, console, HtmlCanvasElement, MouseEvent};
use console_error_panic_hook;
use std::panic;


thread_local! {
    static SIMULATION: RefCell<Option<Rc<RefCell<Simulation>>>> = RefCell::new(None);
}

#[derive(Debug)]
struct Simulation {
    context: WebGl2RenderingContext,
    program: WebGlProgram,
    canvas: HtmlCanvasElement,
    current_framebuffer: Rc<RefCell<WebGlFramebuffer>>,
    next_framebuffer: Rc<RefCell<WebGlFramebuffer>>,
    current_texture: Rc<RefCell<WebGlTexture>>,
    next_texture: Rc<RefCell<WebGlTexture>>,
    mouse_position: Rc<RefCell<(f64, f64)>>,
    scale: i32,
    states: i32,
    kernel_id: i32,
}

impl Simulation {
    fn new(canvas: HtmlCanvasElement, fragment_shader_file: &str, scale: i32, states: i32, kernel_id:i32) -> Result<Self, JsValue> {
        let context = canvas
            .get_context("webgl2")?
            .unwrap()
            .dyn_into::<WebGl2RenderingContext>()?;

        let vert_shader = compile_shader(
            &context,
            WebGl2RenderingContext::VERTEX_SHADER,
            include_str!("vertex_shader.glsl"),
        )?;

        let frag_shader = compile_shader(
            &context,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            fragment_shader_file,
        )?;

        let program = link_program(&context, &vert_shader, &frag_shader)?;
        context.use_program(Some(&program));

        let vertices: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        setup_vertex_buffer(&context, &program, &vertices)?;

        let resolution_location = context
            .get_uniform_location(&program, "u_resolution")
            .unwrap();
        context.uniform2f(
            Some(&resolution_location),
            canvas.width() as f32,
            canvas.height() as f32,
        );

        let (framebuffer1, texture1) = create_framebuffer(&context, canvas.width() as i32, canvas.height() as i32)?;
        let (framebuffer2, texture2) = create_framebuffer(&context, canvas.width() as i32, canvas.height() as i32)?;

        initialize_state(&context, canvas.width() as i32, canvas.height() as i32, &texture1, kernel_id)?;

        Ok(Self {
            context,
            program,
            canvas,
            current_framebuffer: Rc::new(RefCell::new(framebuffer1)),
            next_framebuffer: Rc::new(RefCell::new(framebuffer2)),
            current_texture: Rc::new(RefCell::new(texture1)),
            next_texture: Rc::new(RefCell::new(texture2)),
            mouse_position: Rc::new(RefCell::new((0.0, 0.0))),
            scale,
            states,
            kernel_id,
        })
    }

    fn setup_mouse_listener(&self) -> Result<(), JsValue> {
        let mouse_position = self.mouse_position.clone();
        let canvas_clone = self.canvas.clone();
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            let rect = canvas_clone.get_bounding_client_rect();
            let x = event.client_x() as f64 - rect.left();
            let y = event.client_y() as f64 - rect.top();
            *mouse_position.borrow_mut() = (x, y);
        }) as Box<dyn FnMut(_)>);
        self.canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    }

    fn update(&self) {
        let scale = self.scale;
        // Calculate the next state
        self.context.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&self.next_framebuffer.borrow()));
        self.context.viewport(0, 0, self.canvas.width() as i32, self.canvas.height() as i32);
        
        self.context.use_program(Some(&self.program));
        
        self.context.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.current_texture.borrow()));
        
        let u_current_state_location = self.context.get_uniform_location(&self.program, "u_current_state");
        self.context.uniform1i(u_current_state_location.as_ref(), 0);

        let (mouse_x, mouse_y) = *self.mouse_position.borrow();
        let u_mouse_location = self.context.get_uniform_location(&self.program, "u_mouse");
        self.context.uniform2f(u_mouse_location.as_ref(), (mouse_x / scale as f64) as f32, (self.canvas.height() as f64 - (mouse_y/scale as f64)) as f32);

        let u_states_location = self.context.get_uniform_location(&self.program, "u_states");
        self.context.uniform1f(u_states_location.as_ref(), self.states as f32);

        let u_kernel_location = self.context.get_uniform_location(&self.program, "u_kernel_id");
        self.context.uniform1i(u_kernel_location.as_ref(), self.kernel_id);

        self.context.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);

        // Render the current state to the canvas
        self.context.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        self.context.viewport(0, 0, self.canvas.width() as i32, self.canvas.height() as i32);
        
        self.context.active_texture(WebGl2RenderingContext::TEXTURE0);
        self.context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&self.current_texture.borrow()));
        
        self.context.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);

        // Swap framebuffers and textures
        std::mem::swap(&mut *self.current_framebuffer.borrow_mut(), &mut *self.next_framebuffer.borrow_mut());
        std::mem::swap(&mut *self.current_texture.borrow_mut(), &mut *self.next_texture.borrow_mut());

        check_gl_error(&self.context, "After render loop");
    }
}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console::log_1(&"Starting WebAssembly...".into());

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    // set the canvas size to the window size
    let scale = 1;
    canvas.set_width((document.body().unwrap().client_width() / scale) as u32);
    canvas.set_height((document.body().unwrap().client_height() /scale)as u32);

    let simulation = Simulation::new(canvas, include_str!("../shaders/fragment_shader_gol.glsl"), 1, 0, 0)?;
    simulation.setup_mouse_listener()?;
    let simulation = Rc::new(RefCell::new(simulation));
    SIMULATION.with(|sim| {
        *sim.borrow_mut() = Some(simulation.clone());
    });

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        SIMULATION.with(|sim| {
            if let Some(simulation) = sim.borrow().as_ref() {
                simulation.borrow_mut().update();
            }
        });
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));
    
    request_animation_frame(g.borrow().as_ref().unwrap());


    console::log_1(&"WebAssembly started successfully.".into());

    Ok(())
}

fn setup_vertex_buffer(
    context: &WebGl2RenderingContext,
    program: &WebGlProgram,
    vertices: &[f32],
) -> Result<(), JsValue> {
    let position_attribute_location = context.get_attrib_location(program, "position");
    let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

    unsafe {
        let positions_array_buf_view = js_sys::Float32Array::view(vertices);

        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf_view,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }

    let vao = context
        .create_vertex_array()
        .ok_or("Could not create vertex array object")?;
    context.bind_vertex_array(Some(&vao));

    context.vertex_attrib_pointer_with_i32(
        position_attribute_location as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    context.bind_vertex_array(Some(&vao));

    Ok(())
}

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

fn initialize_state(
    context: &WebGl2RenderingContext,
    width: i32,
    height: i32,
    texture: &WebGlTexture,
    kernel_id: i32,
) -> Result<(), JsValue> {
    let mut initial_state = vec![0u8; (width * height * 4) as usize];
    // orbium start
    if kernel_id >= 3 {
        let pattern: Vec<Vec<f64>>= match kernel_id {
            3 => {
                vec![vec![0.0,0.0,0.0,0.0,0.0,0.0,0.1,0.14,0.1,0.0,0.0,0.03,0.03,0.0,0.0,0.3,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.08,0.24,0.3,0.3,0.18,0.14,0.15,0.16,0.15,0.09,0.2,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.15,0.34,0.44,0.46,0.38,0.18,0.14,0.11,0.13,0.19,0.18,0.45,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.06,0.13,0.39,0.5,0.5,0.37,0.06,0.0,0.0,0.0,0.02,0.16,0.68,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.11,0.17,0.17,0.33,0.4,0.38,0.28,0.14,0.0,0.0,0.0,0.0,0.0,0.18,0.42,0.0,0.0], vec![0.0,0.0,0.09,0.18,0.13,0.06,0.08,0.26,0.32,0.32,0.27,0.0,0.0,0.0,0.0,0.0,0.0,0.82,0.0,0.0], vec![0.27,0.0,0.16,0.12,0.0,0.0,0.0,0.25,0.38,0.44,0.45,0.34,0.0,0.0,0.0,0.0,0.0,0.22,0.17,0.0], vec![0.0,0.07,0.2,0.02,0.0,0.0,0.0,0.31,0.48,0.57,0.6,0.57,0.0,0.0,0.0,0.0,0.0,0.0,0.49,0.0], vec![0.0,0.59,0.19,0.0,0.0,0.0,0.0,0.2,0.57,0.69,0.76,0.76,0.49,0.0,0.0,0.0,0.0,0.0,0.36,0.0], vec![0.0,0.58,0.19,0.0,0.0,0.0,0.0,0.0,0.67,0.83,0.9,0.92,0.87,0.12,0.0,0.0,0.0,0.0,0.22,0.07], vec![0.0,0.0,0.46,0.0,0.0,0.0,0.0,0.0,0.7,0.93,1.0,1.0,1.0,0.61,0.0,0.0,0.0,0.0,0.18,0.11], vec![0.0,0.0,0.82,0.0,0.0,0.0,0.0,0.0,0.47,1.0,1.0,0.98,1.0,0.96,0.27,0.0,0.0,0.0,0.19,0.1], vec![0.0,0.0,0.46,0.0,0.0,0.0,0.0,0.0,0.25,1.0,1.0,0.84,0.92,0.97,0.54,0.14,0.04,0.1,0.21,0.05], vec![0.0,0.0,0.0,0.4,0.0,0.0,0.0,0.0,0.09,0.8,1.0,0.82,0.8,0.85,0.63,0.31,0.18,0.19,0.2,0.01], vec![0.0,0.0,0.0,0.36,0.1,0.0,0.0,0.0,0.05,0.54,0.86,0.79,0.74,0.72,0.6,0.39,0.28,0.24,0.13,0.0], vec![0.0,0.0,0.0,0.01,0.3,0.07,0.0,0.0,0.08,0.36,0.64,0.7,0.64,0.6,0.51,0.39,0.29,0.19,0.04,0.0], vec![0.0,0.0,0.0,0.0,0.1,0.24,0.14,0.1,0.15,0.29,0.45,0.53,0.52,0.46,0.4,0.31,0.21,0.08,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.08,0.21,0.21,0.22,0.29,0.36,0.39,0.37,0.33,0.26,0.18,0.09,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.03,0.13,0.19,0.22,0.24,0.24,0.23,0.18,0.13,0.05,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.02,0.06,0.08,0.09,0.07,0.05,0.01,0.0,0.0,0.0,0.0,0.0]]
            }
            4 => {
                vec![vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.01,0.02,0.03,0.04,0.04,0.04,0.03,0.02,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.04,0.1,0.16,0.2,0.23,0.25,0.24,0.21,0.18,0.14,0.1,0.07,0.03,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.01,0.09,0.2,0.33,0.44,0.52,0.56,0.58,0.55,0.51,0.44,0.37,0.3,0.23,0.16,0.08,0.01,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.13,0.29,0.45,0.6,0.75,0.85,0.9,0.91,0.88,0.82,0.74,0.64,0.55,0.46,0.36,0.25,0.12,0.03,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.14,0.38,0.6,0.78,0.93,1.0,1.0,1.0,1.0,1.0,1.0,0.99,0.89,0.78,0.67,0.56,0.44,0.3,0.15,0.04,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.08,0.39,0.74,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.98,0.85,0.74,0.62,0.47,0.3,0.14,0.03,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.32,0.76,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.88,0.75,0.61,0.45,0.27,0.11,0.01,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.35,0.83,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.88,0.73,0.57,0.38,0.19,0.05,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.5,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.99,1.0,1.0,1.0,1.0,0.99,1.0,1.0,1.0,1.0,1.0,1.0,0.85,0.67,0.47,0.27,0.11,0.01], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.55,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.93,0.83,0.79,0.84,0.88,0.89,0.9,0.93,0.98,1.0,1.0,1.0,1.0,0.98,0.79,0.57,0.34,0.15,0.03], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.47,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.9,0.72,0.54,0.44,0.48,0.6,0.7,0.76,0.82,0.91,0.99,1.0,1.0,1.0,1.0,0.91,0.67,0.41,0.19,0.05], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.27,0.99,1.0,1.0,1.0,1.0,0.9,0.71,0.65,0.55,0.38,0.2,0.14,0.21,0.36,0.52,0.64,0.73,0.84,0.95,1.0,1.0,1.0,1.0,1.0,0.78,0.49,0.24,0.07], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.14,0.63,0.96,1.0,1.0,1.0,0.84,0.17,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.13,0.35,0.51,0.64,0.77,0.91,0.99,1.0,1.0,1.0,1.0,0.88,0.58,0.29,0.09], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.07,0.38,0.72,0.95,1.0,1.0,1.0,0.22,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.11,0.33,0.5,0.67,0.86,0.99,1.0,1.0,1.0,1.0,0.95,0.64,0.33,0.1], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.32,0.49,0.71,0.93,1.0,1.0,1.0,0.56,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.1,0.31,0.52,0.79,0.98,1.0,1.0,1.0,1.0,0.98,0.67,0.35,0.11], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.01,0.6,0.83,0.98,1.0,1.0,0.68,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.15,0.38,0.71,0.97,1.0,1.0,1.0,1.0,0.97,0.67,0.35,0.11], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.51,0.96,1.0,1.0,0.18,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.09,0.34,0.68,0.95,1.0,1.0,1.0,1.0,0.91,0.61,0.32,0.1], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.13,0.56,0.99,1.0,1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.17,0.45,0.76,0.96,1.0,1.0,1.0,1.0,0.82,0.52,0.26,0.07], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.33,0.7,0.94,1.0,1.0,0.44,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.33,0.68,0.91,0.99,1.0,1.0,1.0,1.0,0.71,0.42,0.19,0.03], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.53,0.89,1.0,1.0,1.0,0.8,0.43,0.04,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.47,0.86,1.0,1.0,1.0,1.0,1.0,0.95,0.58,0.32,0.12,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.77,0.99,1.0,0.97,0.58,0.41,0.33,0.18,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.54,0.95,1.0,1.0,1.0,1.0,1.0,0.8,0.44,0.21,0.06,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.39,0.83,1.0,1.0,0.55,0.11,0.05,0.15,0.22,0.06,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.58,0.99,1.0,1.0,1.0,1.0,1.0,0.59,0.29,0.11,0.01,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.04,0.55,0.81,0.86,0.97,1.0,1.0,0.5,0.0,0.0,0.01,0.09,0.03,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.26,0.78,1.0,1.0,1.0,1.0,1.0,0.66,0.35,0.13,0.03,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.33,1.0,1.0,1.0,1.0,1.0,1.0,0.93,0.11,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.23,0.73,0.95,1.0,1.0,1.0,1.0,1.0,0.62,0.35,0.12,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.51,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.72,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.56,0.25,0.09,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.12,0.38,1.0,1.0,1.0,0.66,0.08,0.55,1.0,1.0,1.0,0.03,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.35,1.0,1.0,1.0,1.0,1.0,1.0,0.67,0.12,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.6,1.0,1.0,1.0,1.0,1.0,1.0,0.49,0.0,0.0,0.87,1.0,0.88,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,1.0,1.0,1.0,1.0,1.0,1.0,0.7,0.07,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.04,0.21,0.48,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.0,0.0,0.04,0.42,0.26,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.12,0.21,0.34,0.58,1.0,1.0,1.0,0.99,0.97,0.99,0.46,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.5,1.0,1.0,1.0,1.0,0.96,0.0,0.31,1.0,1.0,1.0,0.53,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.2,0.21,0.0,0.0,0.0,0.27,1.0,1.0,1.0,1.0,1.0,1.0,0.87,0.52,0.01,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.84,1.0,1.0,1.0,1.0,1.0,0.0,0.0,0.0,0.83,1.0,1.0,0.52,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.26,0.82,0.59,0.02,0.0,0.0,0.46,1.0,1.0,1.0,1.0,1.0,0.9,0.55,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.39,0.99,1.0,1.0,1.0,1.0,0.78,0.04,0.0,0.0,0.0,0.93,0.92,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.69,1.0,1.0,0.36,0.0,0.0,1.0,1.0,0.65,0.66,0.97,0.87,0.54,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.55,0.75,0.59,0.74,1.0,1.0,0.0,0.0,0.75,0.71,0.18,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.29,0.0,0.0,0.45,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.47,0.39,0.71,0.25,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.69,0.81,0.8,0.92,1.0,0.13,0.0,0.0,0.13,0.94,0.58,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,1.0,1.0,0.34,0.0,0.04,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.24,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.63,0.85,0.9,0.98,1.0,0.09,0.0,0.0,0.02,1.0,0.64,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.59,1.0,1.0,0.84,0.0,0.0,1.0,1.0,1.0,1.0,1.0,1.0,0.64,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.64,0.65,0.67,1.0,1.0,0.21,0.01,0.0,0.04,0.02,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.69,1.0,1.0,1.0,0.29,0.37,1.0,1.0,0.6,0.63,1.0,0.84,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.44,0.73,0.73,0.85,1.0,0.97,0.23,0.05,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.06,0.0,0.0,0.0,0.97,1.0,1.0,1.0,1.0,1.0,1.0,0.33,0.24,0.67,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.12,0.55,0.9,0.9,1.0,1.0,1.0,0.43,0.04,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.31,0.54,0.0,0.0,0.0,0.88,1.0,1.0,1.0,1.0,1.0,1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.29,0.71,1.0,1.0,1.0,1.0,0.79,0.28,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.4,0.77,0.54,0.0,0.0,0.87,1.0,1.0,1.0,1.0,1.0,0.31,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.16,0.27,0.41,0.72,0.99,1.0,1.0,0.82,0.42,0.09,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.1,0.55,0.58,0.58,0.77,0.99,1.0,1.0,1.0,1.0,0.63,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.31,0.48,0.45,0.46,0.63,0.88,1.0,0.83,0.59,0.28,0.06,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.32,0.7,0.95,1.0,1.0,1.0,1.0,0.7,0.58,0.12,0.04,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.23,0.54,0.53,0.48,0.57,0.59,0.65,0.63,0.55,0.35,0.13,0.03,0.02,0.09,0.74,1.0,0.09,0.0,0.0,0.0,0.32,0.86,1.0,1.0,1.0,1.0,0.57,0.44,0.31,0.16,0.01,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.31,0.45,0.31,0.18,0.28,0.39,0.47,0.54,0.5,0.35,0.2,0.16,0.28,0.75,1.0,0.42,0.01,0.0,0.0,0.6,1.0,1.0,1.0,1.0,0.51,0.29,0.09,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.14,0.3,0.4,0.54,0.71,0.74,0.65,0.49,0.35,0.27,0.47,0.6,0.6,0.72,0.98,1.0,1.0,1.0,1.0,0.65,0.33,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.06,0.33,0.53,0.69,0.94,0.99,1.0,0.84,0.41,0.16,0.15,0.96,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.73,0.13,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.42,0.86,0.98,0.98,0.99,1.0,0.94,0.63,0.32,0.62,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.65,0.23,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.07,0.62,0.95,1.0,1.0,0.99,0.98,0.99,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0.98,0.14,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.03,0.46,0.89,1.0,1.0,0.97,0.83,0.75,0.81,0.94,1.0,1.0,1.0,1.0,0.99,0.03,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.14,0.57,0.88,0.93,0.81,0.58,0.45,0.48,0.64,0.86,0.97,0.99,0.99,0.42,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.23,0.45,0.47,0.39,0.29,0.19,0.2,0.46,0.28,0.03,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.08,0.22,0.24,0.15,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0], vec![0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.07,0.22,0.14,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]]
                }
            _ => {
                vec![vec![0.0]]
            }
        };
        let pattern_height = pattern.len() as i32;
        let pattern_width = pattern[0].len() as i32;
        let center_x = (width / 2) - pattern_width / 2;
        let center_y = (height / 2) - pattern_height / 2;
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) as usize * 4;
                let mut value = 0.0;
                if x>=center_x && y>=center_y && x<(center_x+pattern_width) && y<(center_y+pattern_height) {
                    value = pattern[(pattern_height-(y-center_y)-1) as usize ][(x-center_x) as usize];
                }
                initial_state[index] = (value * 255.0) as u8;
                initial_state[index + 1] = (value * 255.0) as u8;
                initial_state[index + 2] = (value * 255.0) as u8;
                initial_state[index + 3] = 255;
            }
        }
    } 
    else {
        for i in 0..(width * height) {
            let alive = if random() < 0.5 { 255 } else { 0 };  // Increased probability of alive cells
            initial_state[i as usize * 4] = alive;
            initial_state[i as usize * 4 + 1] = alive;
            initial_state[i as usize * 4 + 2] = alive;
            initial_state[i as usize * 4 + 3] = 255;
        }
    }

    context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
    context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        WebGl2RenderingContext::TEXTURE_2D,
        0,
        WebGl2RenderingContext::RGBA as i32,
        width,
        height,
        0,
        WebGl2RenderingContext::RGBA,
        WebGl2RenderingContext::UNSIGNED_BYTE,
        Some(&initial_state),
    )?;

    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_S,
        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_T,
        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
    );

    Ok(())
}

fn create_framebuffer(
    context: &WebGl2RenderingContext,
    width: i32,
    height: i32,
) -> Result<(WebGlFramebuffer, WebGlTexture), JsValue> {
    let framebuffer = context
        .create_framebuffer()
        .ok_or("Failed to create framebuffer")?;
    context.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&framebuffer));

    let texture = context.create_texture().ok_or("Failed to create texture")?;
    context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
    context.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        WebGl2RenderingContext::TEXTURE_2D,
        0,
        WebGl2RenderingContext::RGBA as i32,
        width,
        height,
        0,
        WebGl2RenderingContext::RGBA,
        WebGl2RenderingContext::UNSIGNED_BYTE,
        None,
    )?;
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    context.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );

    context.framebuffer_texture_2d(
        WebGl2RenderingContext::FRAMEBUFFER,
        WebGl2RenderingContext::COLOR_ATTACHMENT0,
        WebGl2RenderingContext::TEXTURE_2D,
        Some(&texture),
        0,
    );

    let status = context.check_framebuffer_status(WebGl2RenderingContext::FRAMEBUFFER);
    if status != WebGl2RenderingContext::FRAMEBUFFER_COMPLETE {
        return Err(JsValue::from_str(&format!("Framebuffer is not complete: {}", status)));
    }

    Ok((framebuffer, texture))
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn check_gl_error(context: &WebGl2RenderingContext, location: &str) {
    let error = context.get_error();
    if error != WebGl2RenderingContext::NO_ERROR {
        console::log_1(&format!("WebGL error at {}: 0x{:X}", location, error).into());
    }
}

#[wasm_bindgen]
pub fn reset_simulation(shader_source: &str, scale: i32, states: i32, kernel: i32) -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    // set canvas size based to scale
    let canvas: HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    canvas.set_width((document.body().unwrap().client_width() / scale) as u32);
    canvas.set_height((document.body().unwrap().client_height() /scale)as u32);

    let new_simulation = Simulation::new(canvas, shader_source, scale, states, kernel)?;
    new_simulation.setup_mouse_listener()?;

    SIMULATION.with(|simulation| {
        *simulation.borrow_mut() = Some(Rc::new(RefCell::new(new_simulation)));
    });

    Ok(())
}