//! `rwd` — Raw Window Descriptors
//!
//! A zero-dependency replacement for the `raw-window-handle` crate,
//! providing the minimal handle types needed by `windowed`.

use std::ffi::c_void;
use std::marker::PhantomData;
use std::num::NonZeroIsize;
use std::ptr::NonNull;

// ── Error ────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleError {
    Unavailable,
}

impl std::fmt::Display for HandleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "handle unavailable")
    }
}

impl std::error::Error for HandleError {}

// ── Platform-specific handle structs ─────────────────────────────────────────
/// Linux / X11 — Xlib window XID.
#[derive(Debug, Clone, Copy)]
pub struct XlibWindowHandle {
    pub window: u64,
    pub visual_id: u64,
}

impl XlibWindowHandle {
    pub fn new(window: u64) -> Self {
        Self {
            window,
            visual_id: 0,
        }
    }
}

/// Linux / X11 — Xlib display connection.
#[derive(Debug, Clone, Copy)]
pub struct XlibDisplayHandle {
    pub display: Option<NonNull<c_void>>,
    pub screen: i32,
}

impl XlibDisplayHandle {
    pub fn new(display: Option<NonNull<c_void>>, screen: i32) -> Self {
        Self { display, screen }
    }
}

/// Windows — Win32 HWND.
#[derive(Debug, Clone, Copy)]
pub struct Win32WindowHandle {
    pub hwnd: NonZeroIsize,
    pub hinstance: Option<NonZeroIsize>,
}

impl Win32WindowHandle {
    pub fn new(hwnd: NonZeroIsize) -> Self {
        Self {
            hwnd,
            hinstance: None,
        }
    }
}

/// Windows — display handle (no extra fields required).
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsDisplayHandle;

impl WindowsDisplayHandle {
    pub fn new() -> Self {
        Self
    }
}

/// macOS — Cocoa NSView pointer.
#[derive(Debug, Clone, Copy)]
pub struct AppKitWindowHandle {
    /// A non-null pointer to the window's `NSView`.
    pub ns_view: NonNull<c_void>,
}

impl AppKitWindowHandle {
    pub fn new(ns_view: NonNull<c_void>) -> Self {
        Self { ns_view }
    }
}

/// macOS — Cocoa display handle (no extra fields required).
#[derive(Debug, Clone, Copy, Default)]
pub struct AppKitDisplayHandle;

impl AppKitDisplayHandle {
    pub fn new() -> Self {
        Self
    }
}

// ── Enums ────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy)]
pub enum RawWindowHandle {
    Xlib(XlibWindowHandle),
    Win32(Win32WindowHandle),
    AppKit(AppKitWindowHandle),
}

#[derive(Debug, Clone, Copy)]
pub enum RawDisplayHandle {
    Xlib(XlibDisplayHandle),
    Windows(WindowsDisplayHandle),
    AppKit(AppKitDisplayHandle),
}

// ── Borrow wrappers ───────────────────────────────────────────────────────────
pub struct WindowHandle<'a> {
    raw: RawWindowHandle,
    _marker: PhantomData<&'a ()>,
}

impl<'a> WindowHandle<'a> {
    pub unsafe fn borrow_raw(raw: RawWindowHandle) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub fn as_raw(&self) -> RawWindowHandle {
        self.raw
    }
}

pub struct DisplayHandle<'a> {
    raw: RawDisplayHandle,
    _marker: PhantomData<&'a ()>,
}

impl<'a> DisplayHandle<'a> {
    pub unsafe fn borrow_raw(raw: RawDisplayHandle) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub fn as_raw(&self) -> RawDisplayHandle {
        self.raw
    }
}

// ── Traits ────────────────────────────────────────────────────────────────────
pub trait HasWindowHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError>;
}

pub trait HasDisplayHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError>;
}
