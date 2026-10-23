//! `rwd` — Raw Window Descriptors
//!
//! A zero-dependency replacement for the `raw-window-handle` crate,
//! providing the minimal handle types needed by `windowed`.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::num::NonZeroIsize;
use std::ptr::NonNull;

// ── Error ────────────────────────────────────────────────────────────────────
/// Error returned when acquiring a window or display handle fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    /// The handle is currently unavailable (e.g. window is closed or uninitialized).
    Unavailable,
}

impl std::fmt::Display for HandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "handle unavailable")
    }
}

impl std::error::Error for HandleError {}

// ── Platform-specific handle structs ─────────────────────────────────────────
/// Linux / X11 — Xlib window handle containing the XID and visual ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XlibWindowHandle {
    /// The Xlib `Window` XID.
    pub window: u64,
    /// The Xlib `VisualID`.
    pub visual_id: u64,
}

impl XlibWindowHandle {
    /// Creates a new `XlibWindowHandle` with the specified window XID.
    pub fn new(window: u64) -> Self {
        Self {
            window,
            visual_id: 0,
        }
    }
}

/// Linux / X11 — Xlib display handle containing the display pointer and screen number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XlibDisplayHandle {
    /// An optional pointer to the `Display`.
    pub display: Option<NonNull<c_void>>,
    /// The default screen number.
    pub screen: i32,
}

impl XlibDisplayHandle {
    /// Creates a new `XlibDisplayHandle`.
    pub fn new(display: Option<NonNull<c_void>>, screen: i32) -> Self {
        Self { display, screen }
    }
}

/// Windows — Win32 window handle containing `HWND` and optional `HINSTANCE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win32WindowHandle {
    /// The Win32 `HWND`.
    pub hwnd: NonZeroIsize,
    /// The optional Win32 instance handle `HINSTANCE`.
    pub hinstance: Option<NonZeroIsize>,
}

impl Win32WindowHandle {
    /// Creates a new `Win32WindowHandle` with the specified `HWND`.
    pub fn new(hwnd: NonZeroIsize) -> Self {
        Self {
            hwnd,
            hinstance: None,
        }
    }
}

/// Windows — display handle (no extra fields required).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowsDisplayHandle;

impl WindowsDisplayHandle {
    /// Creates a new `WindowsDisplayHandle`.
    pub fn new() -> Self {
        Self
    }
}

/// macOS — Cocoa window handle containing the `NSView` pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppKitWindowHandle {
    /// A non-null pointer to the window's `NSView`.
    pub ns_view: NonNull<c_void>,
}

impl AppKitWindowHandle {
    /// Creates a new `AppKitWindowHandle` with the given `NSView` pointer.
    pub fn new(ns_view: NonNull<c_void>) -> Self {
        Self { ns_view }
    }
}

/// macOS — Cocoa display handle (no extra fields required).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AppKitDisplayHandle;

impl AppKitDisplayHandle {
    /// Creates a new `AppKitDisplayHandle`.
    pub fn new() -> Self {
        Self
    }
}

// ── Enums ────────────────────────────────────────────────────────────────────
/// Enumeration of platform-specific raw window handles.
#[derive(Debug, Clone, Copy)]
pub enum RawWindowHandle {
    /// X11 / Xlib window handle.
    Xlib(XlibWindowHandle),
    /// Win32 window handle.
    Win32(Win32WindowHandle),
    /// Cocoa / AppKit window handle.
    AppKit(AppKitWindowHandle),
}

/// Enumeration of platform-specific raw display handles.
#[derive(Debug, Clone, Copy)]
pub enum RawDisplayHandle {
    /// X11 / Xlib display handle.
    Xlib(XlibDisplayHandle),
    /// Windows display handle.
    Windows(WindowsDisplayHandle),
    /// Cocoa / AppKit display handle.
    AppKit(AppKitDisplayHandle),
}

// ── Borrow wrappers ───────────────────────────────────────────────────────────
/// A borrowed wrapper around a [`RawWindowHandle`], tied to a lifetime `'a`.
pub struct WindowHandle<'a> {
    raw: RawWindowHandle,
    _marker: PhantomData<&'a ()>,
}

impl<'a> WindowHandle<'a> {
    /// Borrow a raw window handle.
    ///
    /// # Safety
    /// The caller must ensure that `raw` represents a valid window handle that
    /// remains valid for lifetime `'a'`.
    pub unsafe fn borrow_raw(raw: RawWindowHandle) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Returns the underlying [`RawWindowHandle`].
    pub fn as_raw(&self) -> RawWindowHandle {
        self.raw
    }
}

/// A borrowed wrapper around a [`RawDisplayHandle`], tied to a lifetime `'a`.
pub struct DisplayHandle<'a> {
    raw: RawDisplayHandle,
    _marker: PhantomData<&'a ()>,
}

impl<'a> DisplayHandle<'a> {
    /// Borrow a raw display handle.
    ///
    /// # Safety
    /// The caller must ensure that `raw` represents a valid display handle that
    /// remains valid for lifetime `'a'`.
    pub unsafe fn borrow_raw(raw: RawDisplayHandle) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Returns the underlying [`RawDisplayHandle`].
    pub fn as_raw(&self) -> RawDisplayHandle {
        self.raw
    }
}

// ── Traits ────────────────────────────────────────────────────────────────────
/// Trait for objects that can provide a borrowed [`WindowHandle`].
pub trait HasWindowHandle {
    /// Returns the window handle, or an error if unavailable.
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError>;
}

/// Trait for objects that can provide a borrowed [`DisplayHandle`].
pub trait HasDisplayHandle {
    /// Returns the display handle, or an error if unavailable.
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rwd_handles() {
        let hwnd = NonZeroIsize::new(12345).unwrap();
        let win = Win32WindowHandle::new(hwnd);
        assert_eq!(win.hwnd, hwnd);
        assert_eq!(win.hinstance, None);

        let raw = RawWindowHandle::Win32(win);
        let borrowed = unsafe { WindowHandle::borrow_raw(raw) };
        match borrowed.as_raw() {
            RawWindowHandle::Win32(w) => assert_eq!(w.hwnd, hwnd),
            _ => panic!("unexpected handle"),
        }

        let disp = WindowsDisplayHandle::new();
        let raw_d = RawDisplayHandle::Windows(disp);
        let borrowed_d = unsafe { DisplayHandle::borrow_raw(raw_d) };
        assert!(matches!(borrowed_d.as_raw(), RawDisplayHandle::Windows(_)));
    }

    #[test]
    fn test_handle_error_display() {
        assert_eq!(HandleError::Unavailable.to_string(), "handle unavailable");
    }
}
