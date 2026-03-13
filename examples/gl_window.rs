//! Simple example: Opens a window with opengl and than renders an orange triangle.
//!
//! Press Escape or close the window to quit.
//!
//! Run with:
//!   cargo run --example gl_window

use gl::{FRAGMENT_SHADER, VERTEX_SHADER};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

const VERTEX_SHADER_SOURCE: &[u8] = b"#version 330 core
layout (location = 0) in vec3 aPos;
void main()
{
    gl_Position = vec4(aPos.x, aPos.y, aPos.z, 1.0);
}
\0";

const FRAGMENT_SHADER_SOURCE: &[u8] = b"#version 330 core
out vec4 FragColor;
void main()
{
    FragColor = vec4(1.0, 0.5, 0.2, 1.0);
}
\0";

#[rustfmt::skip]
const VERTICES: [f32; 9] = [
    -0.5, -0.5, 0.0,   // bottom-left
     0.5, -0.5, 0.0,   // bottom-right
     0.0,  0.5, 0.0,   // top-centre
];

#[cfg(target_os = "linux")]
#[link(name = "GL")]
unsafe extern "C" {
    fn glXSwapBuffers(display: *mut std::ffi::c_void, drawable: u64);
}

#[cfg(target_os = "windows")]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn SwapBuffers(hdc: *mut std::ffi::c_void) -> i32;
    fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

fn compile_shader(src: &[u8], kind: gl::types::GLenum) -> u32 {
    let shader = unsafe { gl::CreateShader(kind) };
    let ptr = src.as_ptr().cast::<i8>();
    unsafe { gl::ShaderSource(shader, 1, &ptr, std::ptr::null()) };
    unsafe { gl::CompileShader(shader) };
    let mut ok = 0i32;
    unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut ok) };
    if ok == 0 {
        let mut len = 0i32;
        unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len) };
        let mut buf = vec![0u8; len as usize];
        unsafe { gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buf.as_mut_ptr().cast()) };
        panic!("shader compile error: {}", String::from_utf8_lossy(&buf));
    }
    shader
}

fn link_program(vert: u32, frag: u32) -> u32 {
    let program = unsafe { gl::CreateProgram() };
    unsafe {
        gl::AttachShader(program, vert);
        gl::AttachShader(program, frag);
        gl::LinkProgram(program);
    }
    let mut ok = 0i32;
    unsafe { gl::GetProgramiv(program, gl::LINK_STATUS, &mut ok) };
    if ok == 0 {
        let mut len = 0i32;
        unsafe { gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len) };
        let mut buf = vec![0u8; len as usize];
        unsafe {
            gl::GetProgramInfoLog(program, len, std::ptr::null_mut(), buf.as_mut_ptr().cast())
        };
        panic!("program link error: {}", String::from_utf8_lossy(&buf));
    }
    program
}

fn main() -> windowed::Result<()> {
    let config = WindowConfig::new("windowed — triangle")
        .size(800, 600)
        .resizable(true);

    let mut window = Window::new(config)?;
    unsafe { window.create_gl_context() };
    gl::load_with(|s| window.get_proc_address(s) as *const _);
    window.test_gl();

    // ── Shaders + geometry ─────────────────────────────────────────────
    let (program, vao) = unsafe {
        let vert = compile_shader(VERTEX_SHADER_SOURCE, VERTEX_SHADER);
        let frag = compile_shader(FRAGMENT_SHADER_SOURCE, FRAGMENT_SHADER);
        let program = link_program(vert, frag);
        gl::DeleteShader(vert);
        gl::DeleteShader(frag);

        let mut vao = 0u32;
        let mut vbo = 0u32;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (VERTICES.len() * std::mem::size_of::<f32>()) as isize,
            VERTICES.as_ptr().cast(),
            gl::STATIC_DRAW,
        );
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            (3 * std::mem::size_of::<f32>()) as i32,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
        (program, vao)
    };

    // ── Grab display/window handles once for swap ──────────────────────
    #[cfg(target_os = "linux")]
    let (x_display, x_window) = {
        use raw_window_handle::{
            HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
        };
        #[link(name = "X11")]
        unsafe extern "C" {
            fn XOpenDisplay(name: *const i8) -> *mut std::ffi::c_void;
        }
        let display = match window.display_handle().unwrap().as_raw() {
            RawDisplayHandle::Xlib(h) => match h.display {
                Some(ptr) => ptr.as_ptr() as *mut std::ffi::c_void,
                None => unsafe { XOpenDisplay(std::ptr::null()) },
            },
            _ => panic!("expected Xlib display"),
        };
        let xid = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Xlib(h) => h.window,
            _ => panic!("expected Xlib window"),
        };
        (display, xid)
    };

    #[cfg(target_os = "windows")]
    let hdc = {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let hwnd = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(h) => h.hwnd.get() as *mut std::ffi::c_void,
            _ => panic!("expected Win32 window"),
        };
        unsafe { GetDC(hwnd) }
    };

    println!("Window created. Press Escape or close the window to quit.");

    window.run(move |event| {
        match &event {
            Event::CloseRequested => {
                println!("Close requested — goodbye!");
                return ControlFlow::Exit;
            }
            Event::KeyDown(Key::Escape) => {
                println!("Escape pressed — goodbye!");
                return ControlFlow::Exit;
            }
            Event::Resized { width, height } => unsafe {
                gl::Viewport(0, 0, *width as i32, *height as i32);
            },
            Event::RedrawRequested => unsafe {
                gl::ClearColor(0.15, 0.15, 0.2, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);

                gl::UseProgram(program);
                gl::BindVertexArray(vao);
                gl::DrawArrays(gl::TRIANGLES, 0, 3);

                // Present the frame — this is what was missing.
                #[cfg(target_os = "linux")]
                glXSwapBuffers(x_display, x_window);

                #[cfg(target_os = "windows")]
                SwapBuffers(hdc);
            },
            _ => {}
        }

        // Keep requesting redraws so the event loop doesn't stall.
        ControlFlow::Continue
    })
}
