use crate::config::WindowConfig;
use crate::error::Result;
use crate::event::{ControlFlow, Event};
use crate::platform::PlatformWindow;
use std::ffi::{CString, c_void};

#[allow(unused_imports)]
use crate::rwd::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

// ── Platform GL back-ends ─────────────────────────────────────────────────────
/// Win32 / WGL helpers (Windows only).
#[cfg(target_os = "windows")]
pub mod win_gl {
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

    #[link(name = "gdi32")]
    unsafe extern "system" {
        pub fn GetDC(hwnd: *mut c_void) -> *mut c_void;
        pub fn SwapBuffers(hdc: *mut c_void) -> i32;
        pub fn ChoosePixelFormat(hdc: *mut c_void, ppfd: *const PIXELFORMATDESCRIPTOR) -> i32;
        pub fn SetPixelFormat(
            hdc: *mut c_void,
            format: i32,
            ppfd: *const PIXELFORMATDESCRIPTOR,
        ) -> i32;
    }

    /// Try `wglGetProcAddress` first, then fall back to the static export from
    /// `opengl32.dll` for core functions that WGL doesn't expose.
    pub unsafe fn get_proc_address(name: *const i8) -> *const c_void {
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

    pub const PFD_DRAW_TO_WINDOW: u32 = 0x0000_0004;
    pub const PFD_SUPPORT_OPENGL: u32 = 0x0000_0020;
    pub const PFD_DOUBLEBUFFER: u32 = 0x0000_0001;
    pub const PFD_TYPE_RGBA: u8 = 0;
    pub const PFD_MAIN_PLANE: u8 = 0;
}

/// GLX helpers (Linux / X11 only).
#[cfg(target_os = "linux")]
pub mod glx {
    use std::ffi::c_void;

    #[link(name = "GL")]
    unsafe extern "C" {
        pub fn glXGetProcAddress(name: *const u8) -> *const c_void;
        pub fn glXGetCurrentContext() -> *mut c_void;
        pub fn glXSwapBuffers(display: *mut c_void, drawable: u64);
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

/// NSOpenGLContext helpers (macOS only).
#[cfg(target_os = "macos")]
pub mod cgl {
    use std::ffi::{c_char, c_void};

    pub unsafe fn get_current_context() -> *mut c_void {
        #[link(name = "OpenGL", kind = "framework")]
        unsafe extern "C" {
            fn CGLGetCurrentContext() -> *mut c_void;
        }
        unsafe { CGLGetCurrentContext() }
    }

    pub unsafe fn get_proc_address(name: *const c_char) -> *const c_void {
        crate::buffer::macos_get_proc_address(name)
    }

    /// Flush the current `NSOpenGLContext` by messaging it via the ObjC runtime.
    pub unsafe fn flush_buffer(ctx: *mut c_void) {
        crate::buffer::macos_flush_buffer(ctx)
    }
}

// ── Window ────────────────────────────────────────────────────────────────────
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

    // ── OpenGL ─────────────────────────────────────────────────────────────────

    /// Create and make current a legacy OpenGL context for this window.
    ///
    /// # Safety
    /// Must be called from the same thread that will issue GL commands.
    pub unsafe fn create_gl_context(&mut self) {
        #[cfg(target_os = "linux")]
        unsafe {
            self.create_gl_context_linux()
        }

        #[cfg(target_os = "windows")]
        unsafe {
            self.create_gl_context_windows()
        }

        #[cfg(target_os = "macos")]
        unsafe {
            self.create_gl_context_macos()
        }
    }

    /// Swap the front and back buffers (present the frame).
    ///
    /// Call this at the end of your `RedrawRequested` handler.
    ///
    /// # Safety
    /// An OpenGL context must have been created via [`Self::create_gl_context`]
    /// and must still be current on this thread.
    pub unsafe fn swap_buffers(&self) {
        #[cfg(target_os = "linux")]
        unsafe {
            use glx::glXSwapBuffers;
            let (display, xid) = self.linux_display_xid();
            glXSwapBuffers(display, xid);
        }

        #[cfg(target_os = "windows")]
        unsafe {
            use win_gl::{GetDC, SwapBuffers};
            let hwnd = self.win32_hwnd();
            let hdc = GetDC(hwnd);
            SwapBuffers(hdc);
        }

        #[cfg(target_os = "macos")]
        unsafe {
            let ctx = self.inner.gl_ctx;
            if !ctx.is_null() {
                cgl::flush_buffer(ctx);
            }
        }
    }

    pub fn get_proc_address(&self, name: &str) -> *const c_void {
        let cname = CString::new(name).expect("GL function name contained a null byte");

        #[cfg(target_os = "linux")]
        return unsafe { glx::glXGetProcAddress(cname.as_ptr() as *const u8) };

        #[cfg(target_os = "windows")]
        return unsafe { win_gl::get_proc_address(cname.as_ptr()) };

        #[cfg(target_os = "macos")]
        return unsafe { cgl::get_proc_address(cname.as_ptr()) };
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

        #[cfg(target_os = "macos")]
        {
            let ctx = unsafe { cgl::get_current_context() };
            if ctx.is_null() {
                println!("[windowed] No OpenGL context (CGLGetCurrentContext returned null)");
            } else {
                println!("[windowed] OpenGL context active (CGL): {ctx:p}");
            }
        }

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

    // ── Private per-platform GL helpers ─────────────────────────────────────────
    #[cfg(target_os = "linux")]
    unsafe fn create_gl_context_linux(&self) {
        use glx::*;

        let (display, xid) = self.linux_display_xid();
        assert!(xid != 0, "XlibWindowHandle.window XID is 0");

        // GLX_RGBA=4, GLX_DOUBLEBUFFER=5, GLX_DEPTH_SIZE=12 → 24, terminator
        let attribs: [i32; 5] = [4, 5, 12, 24, 0];
        let visual = unsafe { glXChooseVisual(display, 0, attribs.as_ptr()) };
        assert!(
            !visual.is_null(),
            "glXChooseVisual failed — check GLX support"
        );

        let ctx = unsafe { glXCreateContext(display, visual, std::ptr::null_mut(), 1) };
        assert!(!ctx.is_null(), "glXCreateContext failed");

        let ok = unsafe { glXMakeCurrent(display, xid, ctx) };
        assert!(ok != 0, "glXMakeCurrent failed");
    }

    #[cfg(target_os = "linux")]
    fn linux_display_xid(&self) -> (*mut c_void, u64) {
        let raw_display = self
            .display_handle()
            .expect("failed to acquire display handle")
            .as_raw();

        let display = match raw_display {
            RawDisplayHandle::Xlib(h) => match h.display {
                Some(ptr) => ptr.as_ptr() as *mut c_void,
                None => {
                    eprintln!(
                        "[windowed] XlibDisplayHandle.display is None — \
                         falling back to XOpenDisplay(NULL) ($DISPLAY)"
                    );
                    let d = unsafe { glx::XOpenDisplay(std::ptr::null()) };
                    assert!(!d.is_null(), "XOpenDisplay(NULL) failed — is $DISPLAY set?");
                    d
                }
            },
            other => panic!("expected Xlib display handle, got {other:?}"),
        };

        let xid = match self
            .window_handle()
            .expect("failed to acquire window handle")
            .as_raw()
        {
            RawWindowHandle::Xlib(h) => h.window,
            other => panic!("expected Xlib window handle, got {other:?}"),
        };

        (display, xid)
    }

    #[cfg(target_os = "windows")]
    unsafe fn create_gl_context_windows(&self) {
        use win_gl::*;

        let hwnd = self.win32_hwnd();
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

    #[cfg(target_os = "windows")]
    fn win32_hwnd(&self) -> *mut c_void {
        match self
            .window_handle()
            .expect("failed to acquire window handle")
            .as_raw()
        {
            RawWindowHandle::Win32(w) => w.hwnd.get() as *mut c_void,
            other => panic!("expected Win32 window handle, got {other:?}"),
        }
    }

    #[cfg(target_os = "macos")]
    unsafe fn create_gl_context_macos(&mut self) {
        use std::ffi::c_void;

        type id = *mut c_void;
        type SEL = *mut c_void;
        type BOOL = i8;
        const YES: BOOL = 1;

        #[link(name = "objc")]
        unsafe extern "C" {
            fn objc_getClass(name: *const std::ffi::c_char) -> id;
            fn sel_registerName(name: *const std::ffi::c_char) -> SEL;
            fn objc_msgSend(recv: id, sel: SEL, ...) -> id;
        }

        macro_rules! sel {
            ($l:literal) => {
                unsafe { sel_registerName(concat!($l, "\0").as_ptr() as *const std::ffi::c_char) }
            };
        }
        macro_rules! cls {
            ($l:literal) => {
                unsafe { objc_getClass(concat!($l, "\0").as_ptr() as *const std::ffi::c_char) }
            };
        }

        let msg0 = |recv: id, sel: SEL| -> id {
            let f: unsafe extern "C" fn(id, SEL) -> id = std::mem::transmute(objc_msgSend as usize);
            unsafe { f(recv, sel) }
        };
        let msg1 = |recv: id, sel: SEL, a: id| -> id {
            let f: unsafe extern "C" fn(id, SEL, id) -> id =
                std::mem::transmute(objc_msgSend as usize);
            unsafe { f(recv, sel, a) }
        };
        let msg1_bool = |recv: id, sel: SEL, a: BOOL| {
            let f: unsafe extern "C" fn(id, SEL, BOOL) = std::mem::transmute(objc_msgSend as usize);
            unsafe { f(recv, sel, a) }
        };
        let msg_init_pf = |recv: id, sel: SEL, attrs: id| -> id {
            let f: unsafe extern "C" fn(id, SEL, id) -> id =
                std::mem::transmute(objc_msgSend as usize);
            unsafe { f(recv, sel, attrs) }
        };

        let attrs: Vec<u32> = vec![73, 5, 12, 24, 99, 0x3200, 0];
        let ns_number_cls = cls!("NSNumber");
        let ns_array_cls = cls!("NSMutableArray");
        let array = msg0(msg0(ns_array_cls, sel!("alloc")), sel!("init"));
        for &a in &attrs {
            let f: unsafe extern "C" fn(id, SEL, u32) -> id =
                std::mem::transmute(objc_msgSend as usize);
            let num = unsafe { f(cls!("NSNumber"), sel!("numberWithUnsignedInt:"), a) };
            let _ = ns_number_cls;
            msg1(array, sel!("addObject:"), num);
        }

        let pf_alloc = msg0(cls!("NSOpenGLPixelFormat"), sel!("alloc"));
        let pf = msg_init_pf(pf_alloc, sel!("initWithAttributes:"), attrs.as_ptr() as id);
        assert!(
            !pf.is_null(),
            "NSOpenGLPixelFormat initWithAttributes failed — no suitable pixel format"
        );

        let ctx_alloc = msg0(cls!("NSOpenGLContext"), sel!("alloc"));
        let ctx = {
            let f: unsafe extern "C" fn(id, SEL, id, id) -> id =
                std::mem::transmute(objc_msgSend as usize);
            unsafe {
                f(
                    ctx_alloc,
                    sel!("initWithFormat:shareContext:"),
                    pf,
                    std::ptr::null_mut(),
                )
            }
        };
        assert!(!ctx.is_null(), "NSOpenGLContext initWithFormat failed");

        let view = match self
            .window_handle()
            .expect("failed to acquire window handle")
            .as_raw()
        {
            crate::rwd::RawWindowHandle::AppKit(h) => h.ns_view.as_ptr(),
            other => panic!("expected AppKit window handle, got {other:?}"),
        };
        msg1(ctx, sel!("setView:"), view);
        msg1_bool(
            view as id,
            sel!("setWantsBestResolutionOpenGLSurface:"),
            YES,
        );
        msg0(ctx, sel!("makeCurrentContext"));

        self.inner.gl_ctx = ctx;
    }
}

// ── rwd trait forwarding ──────────────────────────────────────────────────────
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
