//! Low-level buffer-swap helpers for OpenGL.
//!
//! These are `pub(crate)` or `pub` building blocks consumed by [`crate::window::Window::swap_buffers`].
//! Most users should call that method rather than using this module directly.

// ── Linux / X11 ──────────────────────────────────────────────────────────────
#[cfg(all(feature = "opengl", target_os = "linux"))]
#[link(name = "GL")]
unsafe extern "C" {
    /// Exchanges the front and back buffers on the specified display and drawable.
    pub fn glXSwapBuffers(display: *mut std::ffi::c_void, drawable: u64);
}

// ── Windows ───────────────────────────────────────────────────────────────────
#[cfg(all(feature = "opengl", target_os = "windows"))]
#[link(name = "gdi32")]
unsafe extern "system" {
    /// Exchanges the front and back buffers of the current device context.
    pub fn SwapBuffers(hdc: *mut std::ffi::c_void) -> i32;
    /// Retrieves a device context handle for the specified window handle.
    pub fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

// ── macOS ─────────────────────────────────────────────────────────────────────

/// Flushes the specified Cocoa OpenGL context surface.
///
/// # Safety
/// `ctx` must be a valid, retained `NSOpenGLContext*`.
#[cfg(target_os = "macos")]
pub unsafe fn macos_flush_buffer(ctx: *mut std::ffi::c_void) {
    use std::ffi::c_char;
    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const c_char) -> *mut std::ffi::c_void;
        fn objc_msgSend(
            recv: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            ...
        ) -> *mut std::ffi::c_void;
    }
    // SAFETY: Registering selector and sending flushBuffer message to context.
    let sel = unsafe { sel_registerName(b"flushBuffer\0".as_ptr() as *const c_char) };
    let f: unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) =
        unsafe { std::mem::transmute(objc_msgSend as *const () as usize) };
    unsafe { f(ctx, sel) };
}

/// Resolve an OpenGL symbol from the system OpenGL framework on macOS.
///
/// # Safety
/// `name` must be a null-terminated C string.
#[cfg(target_os = "macos")]
pub unsafe fn macos_get_proc_address(name: *const std::ffi::c_char) -> *const std::ffi::c_void {
    use std::ffi::c_int;
    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(filename: *const std::ffi::c_char, flags: c_int) -> *mut std::ffi::c_void;
        fn dlsym(
            handle: *mut std::ffi::c_void,
            symbol: *const std::ffi::c_char,
        ) -> *const std::ffi::c_void;
    }
    static GL_HANDLE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let handle = GL_HANDLE.get_or_init(|| {
        // SAFETY: dlopen system OpenGL framework on macOS.
        let ptr = unsafe {
            dlopen(
                b"/System/Library/Frameworks/OpenGL.framework/OpenGL\0".as_ptr()
                    as *const std::ffi::c_char,
                1,
            )
        };
        ptr as usize
    });
    // SAFETY: Resolving symbol via dlsym.
    unsafe { dlsym(*handle as *mut std::ffi::c_void, name) }
}
