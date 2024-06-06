use js_sys::{Float32Array, Uint16Array};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlShader};

use crate::{log, map_parsers::MapAssets};

const A_POSITION: &str = "a_position";
const U_BASE_MAP: &str = "u_base_map";
const U_PROVINCE_COLORS: &str = "u_province_colors";

pub fn webgl_draw_map(
    canvas: HtmlCanvasElement,
    assets: MapAssets,
) -> Result<impl Fn(&Vec<image::Rgb<u8>>) -> (), JsValue> {
    let gl = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    let vertex_shader_code = r#"#version 300 es
        in vec2 a_position;
        out vec2 v_tex_position;
        
        void main() {
            gl_Position = vec4(a_position, 0, 1);
            v_tex_position = (a_position + vec2(1, -1)) / vec2(2, -2);
        }"#;

    let fragment_shader_code = r#"#version 300 es
        precision mediump float;
        in vec2 v_tex_position;
        out vec4 out_color;
        uniform mediump usampler2D u_base_map;
        uniform mediump sampler2D u_province_colors;
        
        void main() {
            mediump uvec4 province_id = texture(u_base_map, v_tex_position);
            if (province_id == uvec4(0, 0, 0, 0)) {
                out_color = vec4(0, 1, 0, 1);
            } else {
                out_color = texelFetch(u_province_colors, ivec2(int(province_id.x), 0), 0);
                //out_color = vec4(float(province_id.x % 256u) / 255.0, 0, 0, 1);
            }
        }"#;

    log!("Got context");
    let vertex_shader = compile_shader(
        &gl,
        WebGl2RenderingContext::VERTEX_SHADER,
        vertex_shader_code,
    )
    .inspect_err(|err| log!("ERROR COMPILING VERTEX SHADER: {err}"))
    .expect("Failed to compile vertex shader");
    log!("compiled vertex");
    let fragment_shader = compile_shader(
        &gl,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        &fragment_shader_code,
    )
    .inspect_err(|err| log!("ERROR COMPILING FRAGMENT SHADER: {err}"))
    .expect("Failed to compile fragment shader");
    log!("compiled fragment");
    let program = link_program(&gl, &vertex_shader, &fragment_shader)?;
    log!("linked program");
    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
    gl.use_program(Some(&program));
    log!("using program");

    // ==== SETUP VERTEX BUFFER ====
    let js_vertex_array = Float32Array::new_with_length(8);
    js_vertex_array.copy_from(&[-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);

    // Create a buffer and fill it with our values
    let vertex_buffer = gl.create_buffer().expect("Failed to create vertex buffer");
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&vertex_buffer));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &js_vertex_array,
        WebGl2RenderingContext::STATIC_DRAW,
    );

    // Setup vertex array object
    let vao = gl
        .create_vertex_array()
        .expect("Failed to create vertex array object");
    gl.bind_vertex_array(Some(&vao));
    let a_position_location = gl.get_attrib_location(&program, A_POSITION);
    gl.enable_vertex_attrib_array(a_position_location as u32);

    // Assign `a_position` to read from the vertex buffer
    gl.vertex_attrib_pointer_with_i32(
        a_position_location as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );
    log!("setup vertex buffer");

    // ==== SETUP BASE MAP TEXTURE ====
    let js_texture_array = Uint16Array::new_with_length(5632 * 2048);
    js_texture_array.copy_from(&assets.base_map);

    let base_map_texture = gl.create_texture().expect("Failed to create texture");
    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&base_map_texture));

    // upload the texture image to the texture handle
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
        WebGl2RenderingContext::TEXTURE_2D,
        0,
        WebGl2RenderingContext::R16UI as i32,
        5632,
        2048,
        0,
        WebGl2RenderingContext::RED_INTEGER,
        WebGl2RenderingContext::UNSIGNED_SHORT,
        Some(&js_texture_array),
    )
    .unwrap();

    // this stuff is apparently necessary because the canvas might not be the right size
    // and otherwise it just doesn't seem to work at all
    // the alternative is mipmapping, but we don't want that here
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_S,
        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_T,
        WebGl2RenderingContext::CLAMP_TO_EDGE as i32,
    );

    // Assign TEXTURE0 to U_BASE_MAP
    let u_base_map_position = gl
        .get_uniform_location(&program, U_BASE_MAP)
        .expect("Couldn't find u_base_map");
    gl.uniform1i(Some(&u_base_map_position), 0); // 0 as in TEXTURE0
    log!("setup texture");

    // ==== SETUP COLOR MAPPING ====

    let color_map_palette_texture = gl.create_texture().expect("Failed to create texture");
    gl.active_texture(WebGl2RenderingContext::TEXTURE1);
    gl.bind_texture(
        WebGl2RenderingContext::TEXTURE_2D,
        Some(&color_map_palette_texture),
    );

    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
        WebGl2RenderingContext::NEAREST as i32,
    );

    let u_province_colors = gl
        .get_uniform_location(&program, U_PROVINCE_COLORS)
        .expect("Couldn't find u_province_colors");
    gl.uniform1i(Some(&u_province_colors), 1);

    return Ok(move |color_map: &Vec<image::Rgb<u8>>| {
        let rs_color_map_array: Vec<u8> = color_map.iter().flat_map(|image::Rgb(x)| *x).collect();
        gl.bind_texture(
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&color_map_palette_texture),
        );

        // upload the texture image to the texture handle
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGB as i32,
            assets.provinces_len as i32,
            1,
            0,
            WebGl2RenderingContext::RGB,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            Some(&rs_color_map_array),
        )
        .unwrap();

        gl.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);
    });
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
