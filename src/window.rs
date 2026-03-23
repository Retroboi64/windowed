use crate::config::WindowConfig;
use crate::error::Result;
use crate::event::{ControlFlow, Event};
use crate::platform::PlatformWindow;

#[allow(unused_imports)]
use crate::rwd::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
use std::ffi::{CString, c_void};

#[cfg(target_os = "windows")]
mod win_gl {
    use std::ffi::c_void;

    #[link(name = "opengl32")]
    unsafe extern "system" {
        pub fn wglGetProcAddress(name: *const i8) -> *const c_void;
        pub fn wglGetCurrentContext() -> *mut c_void;
        pub fn wglCreateContext(hdc: *mut c_void) -> *mut c_void;
        pub fn wglMakeCurrent(hdc: *mut c_void, hglrc: *mut c_void) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetModuleHandleA(lpModuleName: *const i8) -> *mut c_void;
        pub fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *const c_void;
    }

    pub unsafe fn get_proc_address_wgl(name: *const i8) -> *const c_void {
        let p = unsafe { wglGetProcAddress(name) };
        if !p.is_null() && p as isize != 1 && p as isize != 2 && p as isize != 3 && p as isize != -1
        {
            return p;
        }
        let module = unsafe { GetModuleHandleA(b"opengl32.dll\0".as_ptr() as *const i8) };
        if module.is_null() {
            return std::ptr::null();
        }
        unsafe { GetProcAddress(module, name) }
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        pub fn GetDC(hwnd: *mut c_void) -> *mut c_void;
        pub fn ChoosePixelFormat(hdc: *mut c_void, ppfd: *const PIXELFORMATDESCRIPTOR) -> i32;
        pub fn SetPixelFormat(
            hdc: *mut c_void,
            format: i32,
            ppfd: *const PIXELFORMATDESCRIPTOR,
        ) -> i32;
    }

    #[repr(C)]
    pub struct PIXELFORMATDESCRIPTOR {
        pub n_size: u16,
        pub n_version: u16,
        pub dw_flags: u32,
        pub i_pixel_type: u8,
        pub c_color_bits: u8,
        pub c_red_bits: u8,
        pub c_red_shift: u8,
        pub c_green_bits: u8,
        pub c_green_shift: u8,
        pub c_blue_bits: u8,
        pub c_blue_shift: u8,
        pub c_alpha_bits: u8,
        pub c_alpha_shift: u8,
        pub c_accum_bits: u8,
        pub c_accum_red_bits: u8,
        pub c_accum_green_bits: u8,
        pub c_accum_blue_bits: u8,
        pub c_accum_alpha_bits: u8,
        pub c_depth_bits: u8,
        pub c_stencil_bits: u8,
        pub c_aux_buffers: u8,
        pub i_layer_type: u8,
        pub b_reserved: u8,
        pub dw_layer_mask: u32,
        pub dw_visible_mask: u32,
        pub dw_damage_mask: u32,
    }

    pub const PFD_DRAW_TO_WINDOW: u32 = 0x00000004;
    pub const PFD_SUPPORT_OPENGL: u32 = 0x00000020;
    pub const PFD_DOUBLEBUFFER: u32 = 0x00000001;
    pub const PFD_TYPE_RGBA: u8 = 0;
    pub const PFD_MAIN_PLANE: u8 = 0;
}

#[cfg(target_os = "linux")]
mod glx {
    use std::ffi::c_void;

    #[link(name = "GL")]
    unsafe extern "C" {
        pub fn glXGetProcAddress(name: *const u8) -> *const c_void;
        pub fn glXGetCurrentContext() -> *mut c_void;
        pub fn glXChooseVisual(
            display: *mut c_void,
            screen: i32,
            attrib_list: *const i32,
        ) -> *mut c_void;
        pub fn glXCreateContext(
            display: *mut c_void,
            visual: *mut c_void,
            share_list: *mut c_void,
            direct: i32,
        ) -> *mut c_void;
        pub fn glXMakeCurrent(display: *mut c_void, drawable: u64, ctx: *mut c_void) -> i32;
    }

    #[link(name = "X11")]
    unsafe extern "C" {
        pub fn XOpenDisplay(display_name: *const i8) -> *mut c_void;
    }
}

pub struct Window {
    inner: PlatformWindow,
}

impl Window {
    pub fn new(config: WindowConfig) -> Result<Self> {
        Ok(Self {
            inner: PlatformWindow::new(&config)?,
        })
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, callback: F) -> Result<()> {
        self.inner.run(callback)
    }

    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        self.inner.warp_mouse(x, y);
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        self.inner.set_cursor_visible(visible);
    }

    pub fn inner_size(&self) -> (u32, u32) {
        self.inner.inner_size()
    }

    pub unsafe fn create_gl_context(&self) {
        #[cfg(target_os = "linux")]
        unsafe {
            self.create_gl_context_linux()
        };

        #[cfg(target_os = "windows")]
        self.create_gl_context_windows();
    }

    #[cfg(target_os = "linux")]
    unsafe fn create_gl_context_linux(&self) {
        use glx::*;

        let display: *mut c_void = {
            let raw = self
                .display_handle()
                .expect("failed to acquire display handle")
                .as_raw();

            match raw {
                RawDisplayHandle::Xlib(h) => match h.display {
                    Some(ptr) => ptr.as_ptr() as *mut c_void,
                    None => {
                        eprintln!(
                            "[windowed] XlibDisplayHandle.display is None — \
                             falling back to XOpenDisplay(NULL) ($DISPLAY)"
                        );
                        let d = unsafe { XOpenDisplay(std::ptr::null()) };
                        assert!(!d.is_null(), "XOpenDisplay(NULL) failed — is $DISPLAY set?");
                        d
                    }
                },
                other => panic!("create_gl_context requires an Xlib display handle, got {other:?}"),
            }
        };

        let xid: u64 = match self
            .window_handle()
            .expect("failed to acquire window handle")
            .as_raw()
        {
            RawWindowHandle::Xlib(h) => h.window,
            other => panic!("create_gl_context requires an Xlib window handle, got {other:?}"),
        };

        assert!(xid != 0, "XlibWindowHandle.window XID is 0");

        // GLX_RGBA=4, GLX_DEPTH_SIZE=12 with value 24, None terminator
        let attribs: [i32; 5] = [4, 5, 12, 24, 0];
        let visual = unsafe { glXChooseVisual(display, 0, attribs.as_ptr()) };
        assert!(
            !visual.is_null(),
            "glXChooseVisual failed — check GLX support"
        );

        let ctx = unsafe {
            glXCreateContext(display, visual, std::ptr::null_mut(), 1 /* GL_TRUE */)
        };
        assert!(!ctx.is_null(), "glXCreateContext failed");

        let ok = unsafe { glXMakeCurrent(display, xid, ctx) };
        assert!(ok != 0, "glXMakeCurrent failed");
    }

    #[cfg(target_os = "windows")]
    fn create_gl_context_windows(&self) {
        use win_gl::*;

        let hwnd = match self
            .window_handle()
            .expect("failed to acquire window handle")
            .as_raw()
        {
            RawWindowHandle::Win32(w) => w.hwnd.get() as *mut c_void,
            other => panic!("create_gl_context requires a Win32 window handle, got {other:?}"),
        };

        let hdc = unsafe { GetDC(hwnd) };
        assert!(!hdc.is_null(), "GetDC failed");

        let pfd = PIXELFORMATDESCRIPTOR {
            n_size: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            n_version: 1,
            dw_flags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            i_pixel_type: PFD_TYPE_RGBA,
            c_color_bits: 32,
            c_red_bits: 0,
            c_red_shift: 0,
            c_green_bits: 0,
            c_green_shift: 0,
            c_blue_bits: 0,
            c_blue_shift: 0,
            c_alpha_bits: 8,
            c_alpha_shift: 0,
            c_accum_bits: 0,
            c_accum_red_bits: 0,
            c_accum_green_bits: 0,
            c_accum_blue_bits: 0,
            c_accum_alpha_bits: 0,
            c_depth_bits: 24,
            c_stencil_bits: 8,
            c_aux_buffers: 0,
            i_layer_type: PFD_MAIN_PLANE,
            b_reserved: 0,
            dw_layer_mask: 0,
            dw_visible_mask: 0,
            dw_damage_mask: 0,
        };

        let fmt = unsafe { ChoosePixelFormat(hdc, &pfd) };
        assert!(fmt != 0, "ChoosePixelFormat failed");

        let ok = unsafe { SetPixelFormat(hdc, fmt, &pfd) };
        assert!(ok != 0, "SetPixelFormat failed");

        let hglrc = unsafe { wglCreateContext(hdc) };
        assert!(!hglrc.is_null(), "wglCreateContext failed");

        let ok = unsafe { wglMakeCurrent(hdc, hglrc) };
        assert!(ok != 0, "wglMakeCurrent failed");
    }

    pub fn get_proc_address(&self, name: &str) -> *const c_void {
        let cname = CString::new(name).expect("GL function name contained a null byte");

        #[cfg(target_os = "linux")]
        unsafe {
            glx::glXGetProcAddress(cname.as_ptr() as *const u8)
        }

        #[cfg(target_os = "windows")]
        unsafe {
            win_gl::get_proc_address_wgl(cname.as_ptr())
        }
    }

    pub fn test_gl(&self) {
        #[cfg(target_os = "linux")]
        {
            let ctx = unsafe { glx::glXGetCurrentContext() };
            if ctx.is_null() {
                println!("[windowed] No OpenGL context (glXGetCurrentContext returned null)");
            } else {
                println!("[windowed] OpenGL context active (GLX): {ctx:p}");
            }
        }

        #[cfg(target_os = "windows")]
        {
            let ctx = unsafe { win_gl::wglGetCurrentContext() };
            if ctx.is_null() {
                println!("[windowed] No OpenGL context (wglGetCurrentContext returned null)");
            } else {
                println!("[windowed] OpenGL context active (WGL): {ctx:p}");
            }
        }

        // Resolve glGetString without depending on the gl crate.
        type GlGetString = unsafe extern "C" fn(name: u32) -> *const u8;
        let get_string: GlGetString = unsafe {
            let ptr = self.get_proc_address("glGetString");
            if ptr.is_null() {
                println!("[windowed] glGetString not found — context may not be current");
                return;
            }
            std::mem::transmute(ptr)
        };

        // GL_VERSION = 0x1F02
        let version = unsafe { get_string(0x1F02) };
        if version.is_null() {
            println!(
                "[windowed] glGetString(GL_VERSION) returned null — context may not be current"
            );
        } else {
            let s = unsafe { std::ffi::CStr::from_ptr(version as *const i8) };
            println!("[windowed] OpenGL version: {}", s.to_string_lossy());
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}
