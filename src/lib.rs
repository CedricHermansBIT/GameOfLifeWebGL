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
}

impl Simulation {
    fn new(canvas: HtmlCanvasElement, fragment_shader_file: &str, scale: i32, states: i32) -> Result<Self, JsValue> {
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

        initialize_state(&context, canvas.width() as i32, canvas.height() as i32, &texture1)?;

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

    let simulation = Simulation::new(canvas, include_str!("fragment_shader_gol.glsl"), 1, 0)?;
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
) -> Result<(), JsValue> {
    let mut initial_state = vec![0u8; (width * height * 4) as usize];
    for i in 0..(width * height) {
        let alive = if random() < 0.5 { 255 } else { 0 };  // Increased probability of alive cells
        initial_state[i as usize * 4] = alive;
        initial_state[i as usize * 4 + 1] = alive;
        initial_state[i as usize * 4 + 2] = alive;
        initial_state[i as usize * 4 + 3] = 255;
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
pub fn reset_simulation(shader_source: &str, scale: i32, states: i32) -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    // set canvas size based to scale
    let canvas: HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    canvas.set_width((document.body().unwrap().client_width() / scale) as u32);
    canvas.set_height((document.body().unwrap().client_height() /scale)as u32);

    let new_simulation = Simulation::new(canvas, shader_source, scale, states)?;
    new_simulation.setup_mouse_listener()?;

    SIMULATION.with(|simulation| {
        *simulation.borrow_mut() = Some(Rc::new(RefCell::new(new_simulation)));
    });

    Ok(())
}