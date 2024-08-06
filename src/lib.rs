use js_sys::Math::random;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlFramebuffer, WebGlProgram, WebGlShader, WebGlTexture, console, HtmlCanvasElement, MouseEvent};
use console_error_panic_hook;
use std::panic;

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    console::log_1(&"Starting WebAssembly...".into());

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    // set the canvas size to the window size
    canvas.set_width(document.body().unwrap().client_width() as u32);
    canvas.set_height(document.body().unwrap().client_height() as u32);

    let context = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    let vert_shader = compile_shader(
        &context,
        WebGl2RenderingContext::VERTEX_SHADER,
        include_str!("vertex_shader.glsl"),
    )?;

        // Initialize mouse position
        let mouse_position = Rc::new(RefCell::new((0.0, 0.0)));

        // Add mouse move event listener
        {
            let mouse_position = mouse_position.clone();
            let canvas_clone = canvas.clone();
            let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
                let rect = canvas_clone.get_bounding_client_rect();
                let x = event.client_x() as f64 - rect.left();
                let y = event.client_y() as f64 - rect.top();
                *mouse_position.borrow_mut() = (x, y);
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

    let frag_shader = compile_shader(
        &context,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        include_str!("fragment_shader.glsl"),
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

    let current_framebuffer = Rc::new(RefCell::new(framebuffer1));
    let next_framebuffer = Rc::new(RefCell::new(framebuffer2));
    let current_texture = Rc::new(RefCell::new(texture1));
    let next_texture = Rc::new(RefCell::new(texture2));

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let kernel: [i32; 9] = [
        1 , 1 , 1,
        1 , 0 , 1,
        1 , 1 , 1,
    ];

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        // Calculate the next state
        context.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, Some(&next_framebuffer.borrow()));
        context.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
        
        context.use_program(Some(&program));
        
        context.active_texture(WebGl2RenderingContext::TEXTURE0);
        context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&current_texture.borrow()));
        
        let u_current_state_location = context.get_uniform_location(&program, "u_current_state");
        context.uniform1i(u_current_state_location.as_ref(), 0);

        let (mouse_x, mouse_y) = *mouse_position.borrow();
        let u_mouse_location = context.get_uniform_location(&program, "u_mouse");
        context.uniform2f(u_mouse_location.as_ref(), mouse_x as f32, (canvas.height() as f64 - mouse_y) as f32);

        let u_kernel_location = context.get_uniform_location(&program, "u_kernel");
        context.uniform1iv_with_i32_array(u_kernel_location.as_ref(), &kernel);

        
        context.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);

        // Render the current state to the canvas
        context.bind_framebuffer(WebGl2RenderingContext::FRAMEBUFFER, None);
        context.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
        
        context.active_texture(WebGl2RenderingContext::TEXTURE0);
        context.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&current_texture.borrow()));
        
        context.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);

        // Swap framebuffers and textures
        std::mem::swap(&mut *current_framebuffer.borrow_mut(), &mut *next_framebuffer.borrow_mut());
        std::mem::swap(&mut *current_texture.borrow_mut(), &mut *next_texture.borrow_mut());

        check_gl_error(&context, "After render loop");

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