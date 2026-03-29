//! macOS Cocoa/AppKit platform backend.
//!
//! Uses the Objective-C runtime and AppKit/CoreGraphics frameworks directly
//! (zero external Rust dependencies).
//!
//! **Main thread required.** `NSApplication` and `NSWindow` must be created
//! and driven from the main thread; this is not enforced at compile time.

#![allow(
    non_upper_case_globals,
    non_snake_case,
    dead_code,
    clippy::upper_case_acronyms,
    clippy::too_many_arguments
)]

use std::ffi::{c_char, c_double, c_int, c_void};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};

use crate::config::WindowConfig;
use crate::error::{Error, Result};
use crate::event::{ControlFlow, Event, Key, MouseButton};
use crate::rwd::{
    AppKitDisplayHandle, AppKitWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle,
};

// ─── Primitive aliases ────────────────────────────────────────────────────────
type id = *mut c_void;
type SEL = *mut c_void;
type Class = *mut c_void;
type BOOL = i8;
type NSUInteger = usize;
type NSInteger = isize;
pub type CGFloat = c_double;

const YES: BOOL = 1;
const NO: BOOL = 0;

// ─── Geometry structs ─────────────────────────────────────────────────────────
// Both `NSPoint`/`NSSize`/`NSRect` and `CGPoint`/`CGSize`/`CGRect` are the
// same struct layouts on macOS — we use one set for everything.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct NSPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct NSSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct NSRect {
    pub origin: NSPoint,
    pub size: NSSize,
}

// ─── NSEvent type constants ───────────────────────────────────────────────────
const NSEventTypeLeftMouseDown: NSUInteger = 1;
const NSEventTypeLeftMouseUp: NSUInteger = 2;
const NSEventTypeRightMouseDown: NSUInteger = 3;
const NSEventTypeRightMouseUp: NSUInteger = 4;
const NSEventTypeMouseMoved: NSUInteger = 5;
const NSEventTypeLeftMouseDragged: NSUInteger = 6;
const NSEventTypeRightMouseDragged: NSUInteger = 7;
const NSEventTypeKeyDown: NSUInteger = 10;
const NSEventTypeKeyUp: NSUInteger = 11;
const NSEventTypeScrollWheel: NSUInteger = 22;
const NSEventTypeOtherMouseDown: NSUInteger = 25;
const NSEventTypeOtherMouseUp: NSUInteger = 26;
const NSEventTypeOtherMouseDragged: NSUInteger = 27;
const NSEventMaskAny: NSUInteger = NSUInteger::MAX;

// ─── NSWindowStyleMask ────────────────────────────────────────────────────────
const NSWindowStyleMaskTitled: NSUInteger = 1 << 0;
const NSWindowStyleMaskClosable: NSUInteger = 1 << 1;
const NSWindowStyleMaskMiniaturizable: NSUInteger = 1 << 2;
const NSWindowStyleMaskResizable: NSUInteger = 1 << 3;
const NSWindowStyleMaskFullScreen: NSUInteger = 1 << 14;

const NSBackingStoreBuffered: NSUInteger = 2;
const NSApplicationActivationPolicyRegular: NSInteger = 0;
const NSUTF8StringEncoding: NSUInteger = 4;

// ─── ObjC runtime + frameworks ───────────────────────────────────────────────
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Class;
    fn sel_registerName(name: *const c_char) -> SEL;
    fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra: usize) -> Class;
    fn objc_registerClassPair(cls: Class);
    fn class_addMethod(cls: Class, name: SEL, imp: *mut c_void, types: *const c_char) -> BOOL;
    fn class_addIvar(
        cls: Class,
        name: *const c_char,
        size: usize,
        align: u8,
        types: *const c_char,
    ) -> BOOL;
    fn object_setInstanceVariable(
        obj: id,
        name: *const c_char,
        value: *mut c_void,
    ) -> *const c_void;
    fn object_getInstanceVariable(
        obj: id,
        name: *const c_char,
        out: *mut *mut c_void,
    ) -> *const c_void;
    fn objc_msgSend(recv: id, sel: SEL, ...) -> id;
}

#[cfg(target_arch = "x86_64")]
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_msgSend_stret(out: *mut c_void, recv: id, sel: SEL, ...);
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWarpMouseCursorPosition(point: NSPoint) -> i32;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayHideCursor(display: u32) -> i32;
    fn CGDisplayShowCursor(display: u32) -> i32;
}

// ─── Selector / class helpers ────────────────────────────────────────────────
//
// `sel_registerName` is O(1) and internally caches results, so calling it on
// every frame is fine.

macro_rules! sel {
    ($lit:literal) => {{ unsafe { sel_registerName(concat!($lit, "\0").as_ptr() as *const c_char) } }};
}

macro_rules! cls {
    ($lit:literal) => {{ unsafe { objc_getClass(concat!($lit, "\0").as_ptr() as *const c_char) } }};
}

// ─── Typed message-send wrappers ─────────────────────────────────────────────
//
// We transmute `objc_msgSend` to typed function pointers so the Rust compiler
// emits the correct argument/return calling convention for each call pattern.
// This is the standard approach for calling ObjC from pure Rust FFI.

#[inline(always)]
unsafe fn msg0(recv: id, sel: SEL) -> id {
    let f: unsafe extern "C" fn(id, SEL) -> id =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel) }
}

#[inline(always)]
unsafe fn msg0_bool(recv: id, sel: SEL) -> BOOL {
    let f: unsafe extern "C" fn(id, SEL) -> BOOL =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel) }
}

#[inline(always)]
unsafe fn msg0_uint(recv: id, sel: SEL) -> NSUInteger {
    let f: unsafe extern "C" fn(id, SEL) -> NSUInteger =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel) }
}

#[inline(always)]
unsafe fn msg0_f64(recv: id, sel: SEL) -> CGFloat {
    let f: unsafe extern "C" fn(id, SEL) -> CGFloat =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel) }
}

/// Returns NSPoint (2×f64 = 16 bytes). Safe to return via `objc_msgSend` on
/// both x86_64 (fits in two XMM registers) and arm64.
#[inline(always)]
unsafe fn msg0_point(recv: id, sel: SEL) -> NSPoint {
    let f: unsafe extern "C" fn(id, SEL) -> NSPoint =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel) }
}

/// Returns NSRect (32 bytes). Needs `objc_msgSend_stret` on x86_64 (> 16 bytes)
/// but plain `objc_msgSend` on arm64.
#[inline(always)]
unsafe fn msg0_rect(recv: id, sel: SEL) -> NSRect {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut out: NSRect = std::mem::zeroed();
        let f: unsafe extern "C" fn(*mut NSRect, id, SEL) =
            std::mem::transmute(objc_msgSend_stret as usize);
        f(&mut out, recv, sel);
        out
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let f: unsafe extern "C" fn(id, SEL) -> NSRect = std::mem::transmute(objc_msgSend as usize);
        f(recv, sel)
    }
}

#[inline(always)]
unsafe fn msg1(recv: id, sel: SEL, a1: id) -> id {
    let f: unsafe extern "C" fn(id, SEL, id) -> id =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel, a1) }
}

#[inline(always)]
unsafe fn msg1_bool(recv: id, sel: SEL, a1: BOOL) {
    let f: unsafe extern "C" fn(id, SEL, BOOL) =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel, a1) }
}

#[inline(always)]
unsafe fn msg1_int(recv: id, sel: SEL, a1: NSInteger) -> id {
    let f: unsafe extern "C" fn(id, SEL, NSInteger) -> id =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel, a1) }
}

/// `initWithContentRect:styleMask:backing:defer:` — the NSRect is an argument
/// (not a return) so normal calling convention applies on both architectures.
#[inline(always)]
unsafe fn msg_init_window(
    recv: id,
    sel: SEL,
    rect: NSRect,
    style: NSUInteger,
    backing: NSUInteger,
    defer: BOOL,
) -> id {
    let f: unsafe extern "C" fn(id, SEL, NSRect, NSUInteger, NSUInteger, BOOL) -> id =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel, rect, style, backing, defer) }
}

/// `initWithFrame:` — NSRect argument.
#[inline(always)]
unsafe fn msg_init_frame(recv: id, sel: SEL, rect: NSRect) -> id {
    let f: unsafe extern "C" fn(id, SEL, NSRect) -> id =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel, rect) }
}

/// `setFrameOrigin:` — NSPoint argument.
#[inline(always)]
unsafe fn msg_set_origin(recv: id, sel: SEL, pt: NSPoint) {
    let f: unsafe extern "C" fn(id, SEL, NSPoint) =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe { f(recv, sel, pt) }
}

/// `nextEventMatchingMask:untilDate:inMode:dequeue:`
#[inline(always)]
unsafe fn next_event(app: id, mask: NSUInteger, until: id, mode: id, dequeue: BOOL) -> id {
    let f: unsafe extern "C" fn(id, SEL, NSUInteger, id, id, BOOL) -> id =
        unsafe { std::mem::transmute(objc_msgSend as usize) };
    unsafe {
        f(
            app,
            sel!("nextEventMatchingMask:untilDate:inMode:dequeue:"),
            mask,
            until,
            mode,
            dequeue,
        )
    }
}

// ─── NSString helper ──────────────────────────────────────────────────────────
/// Create an `NSString` from a Rust `&str` (UTF-8).
/// Returns an **autoreleased** object — safe inside an autorelease pool.
unsafe fn nsstring(s: &str) -> id {
    let f: unsafe extern "C" fn(id, SEL, *const c_void, NSUInteger, NSUInteger) -> id =
        std::mem::transmute(objc_msgSend as usize);
    let cls = cls!("NSString");
    let alloc = msg0(cls, sel!("alloc"));
    let ns = f(
        alloc,
        sel!("initWithBytes:length:encoding:"),
        s.as_ptr() as *const c_void,
        s.len(),
        NSUTF8StringEncoding,
    );
    // autorelease so it lives until the enclosing pool drains
    msg0(ns, sel!("autorelease"))
}

// ─── Autorelease pool helpers ─────────────────────────────────────────────────
#[inline]
unsafe fn pool_push() -> id {
    let p = msg0(cls!("NSAutoreleasePool"), sel!("alloc"));
    msg0(p, sel!("init"))
}

#[inline]
unsafe fn pool_pop(pool: id) {
    let f: unsafe extern "C" fn(id, SEL) = std::mem::transmute(objc_msgSend as usize);
    f(pool, sel!("drain"));
}

// ─── Shared window state (accessed from delegate callbacks) ──────────────────
struct PlatformState {
    events: Vec<Event>,
}

const IVAR_STATE: &[u8] = b"_state\0";

unsafe fn delegate_get_state(this: id) -> Option<Arc<Mutex<PlatformState>>> {
    let mut ptr: *mut c_void = std::ptr::null_mut();
    object_getInstanceVariable(this, IVAR_STATE.as_ptr() as *const c_char, &mut ptr);
    if ptr.is_null() {
        return None;
    }
    let arc = Arc::from_raw(ptr as *const Mutex<PlatformState>);
    let clone = Arc::clone(&arc);
    std::mem::forget(arc);
    Some(clone)
}

unsafe extern "C" fn delegate_window_should_close(this: id, _cmd: SEL, _sender: id) -> BOOL {
    if let Some(state) = unsafe { delegate_get_state(this) } {
        if let Ok(mut s) = state.lock() {
            s.events.push(Event::CloseRequested);
        }
    }
    NO
}

unsafe extern "C" fn delegate_window_did_become_key(this: id, _cmd: SEL, _notif: id) {
    if let Some(state) = unsafe { delegate_get_state(this) } {
        if let Ok(mut s) = state.lock() {
            s.events.push(Event::FocusGained);
        }
    }
}

unsafe extern "C" fn delegate_window_did_resign_key(this: id, _cmd: SEL, _notif: id) {
    if let Some(state) = unsafe { delegate_get_state(this) } {
        if let Ok(mut s) = state.lock() {
            s.events.push(Event::FocusLost);
        }
    }
}

static DELEGATE_CLASS: OnceLock<Class> = OnceLock::new();

fn get_delegate_class() -> Class {
    *DELEGATE_CLASS.get_or_init(|| unsafe {
        let cls = objc_allocateClassPair(
            cls!("NSObject"),
            b"WindowerDelegate\0".as_ptr() as *const c_char,
            0,
        );
        assert!(
            !cls.is_null(),
            "objc_allocateClassPair failed for WindowerDelegate"
        );

        class_addIvar(
            cls,
            IVAR_STATE.as_ptr() as *const c_char,
            std::mem::size_of::<usize>(),
            std::mem::align_of::<usize>() as u8,
            b"^v\0".as_ptr() as *const c_char,
        );

        class_addMethod(
            cls,
            sel!("windowShouldClose:"),
            delegate_window_should_close as usize as *mut c_void,
            b"c@:@\0".as_ptr() as *const c_char,
        );

        class_addMethod(
            cls,
            sel!("windowDidBecomeKey:"),
            delegate_window_did_become_key as usize as *mut c_void,
            b"v@:@\0".as_ptr() as *const c_char,
        );

        class_addMethod(
            cls,
            sel!("windowDidResignKey:"),
            delegate_window_did_resign_key as usize as *mut c_void,
            b"v@:@\0".as_ptr() as *const c_char,
        );

        objc_registerClassPair(cls);
        cls
    })
}

// ─── View subclass ────────────────────────────────────────────────────────────
// WindowerView — NSView subclass that:
//   • accepts first responder (so it receives key events), and
//   • silences the default NSBeep on unhandled `keyDown:`.

unsafe extern "C" fn view_accepts_first_responder(_this: id, _cmd: SEL) -> BOOL {
    YES
}
unsafe extern "C" fn view_key_down(_this: id, _cmd: SEL, _event: id) { /* swallow — no beep */
}

static VIEW_CLASS: OnceLock<Class> = OnceLock::new();

fn get_view_class() -> Class {
    *VIEW_CLASS.get_or_init(|| unsafe {
        let cls = objc_allocateClassPair(
            cls!("NSView"),
            b"WindowerView\0".as_ptr() as *const c_char,
            0,
        );
        assert!(
            !cls.is_null(),
            "objc_allocateClassPair failed for WindowerView"
        );

        class_addMethod(
            cls,
            sel!("acceptsFirstResponder"),
            view_accepts_first_responder as usize as *mut c_void,
            b"c@:\0".as_ptr() as *const c_char,
        );

        class_addMethod(
            cls,
            sel!("keyDown:"),
            view_key_down as usize as *mut c_void,
            b"v@:@\0".as_ptr() as *const c_char,
        );

        objc_registerClassPair(cls);
        cls
    })
}

// ─── PlatformWindow ───────────────────────────────────────────────────────────
pub struct PlatformWindow {
    app: id,
    window: id,
    view: id,
    delegate: id,
    run_loop_mode: id,
    state: Arc<Mutex<PlatformState>>,
    width: u32,
    height: u32,
    pub gl_ctx: *mut c_void,
}

unsafe impl Send for PlatformWindow {}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        unsafe {
            let pool = pool_push();

            let app = msg0(cls!("NSApplication"), sel!("sharedApplication"));
            if app.is_null() {
                pool_pop(pool);
                return Err(Error::Platform(
                    "[NSApplication sharedApplication] returned nil".into(),
                ));
            }
            msg1_int(
                app,
                sel!("setActivationPolicy:"),
                NSApplicationActivationPolicyRegular,
            );
            msg0(app, sel!("finishLaunching"));

            let style: NSUInteger = NSWindowStyleMaskTitled
                | NSWindowStyleMaskClosable
                | NSWindowStyleMaskMiniaturizable
                | if config.resizable {
                    NSWindowStyleMaskResizable
                } else {
                    0
                };

            let content_rect = NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: config.width as CGFloat,
                    height: config.height as CGFloat,
                },
            };

            let window_alloc = msg0(cls!("NSWindow"), sel!("alloc"));
            let window = msg_init_window(
                window_alloc,
                sel!("initWithContentRect:styleMask:backing:defer:"),
                content_rect,
                style,
                NSBackingStoreBuffered,
                NO,
            );
            if window.is_null() {
                pool_pop(pool);
                return Err(Error::Platform(
                    "NSWindow initWithContentRect failed".into(),
                ));
            }

            msg1(window, sel!("setTitle:"), nsstring(config.title.as_str()));

            if let Some((x, y)) = config.position {
                msg_set_origin(
                    window,
                    sel!("setFrameOrigin:"),
                    NSPoint {
                        x: x as CGFloat,
                        y: y as CGFloat,
                    },
                );
            } else {
                msg0(window, sel!("center"));
            }

            if let Some((w, h)) = config.min_size {
                let f: unsafe extern "C" fn(id, SEL, NSSize) =
                    std::mem::transmute(objc_msgSend as usize);
                f(
                    window,
                    sel!("setMinSize:"),
                    NSSize {
                        width: w as CGFloat,
                        height: h as CGFloat,
                    },
                );
            }
            if let Some((w, h)) = config.max_size {
                let f: unsafe extern "C" fn(id, SEL, NSSize) =
                    std::mem::transmute(objc_msgSend as usize);
                f(
                    window,
                    sel!("setMaxSize:"),
                    NSSize {
                        width: w as CGFloat,
                        height: h as CGFloat,
                    },
                );
            }

            let view_cls = get_view_class();
            let view_alloc = msg0(view_cls, sel!("alloc"));
            let view = msg_init_frame(view_alloc, sel!("initWithFrame:"), content_rect);
            if view.is_null() {
                pool_pop(pool);
                return Err(Error::Platform("WindowerView initWithFrame failed".into()));
            }
            msg1(window, sel!("setContentView:"), view);
            msg1(window, sel!("makeFirstResponder:"), view);

            msg1_bool(window, sel!("setacceptsmousemovedevents:"), yes);

            let delegate_cls = get_delegate_class();
            let delegate = msg0(msg0(delegate_cls, sel!("alloc")), sel!("init"));

            let state = Arc::new(Mutex::new(PlatformState { events: Vec::new() }));
            let state_ptr = Arc::as_ptr(&state) as *mut c_void;
            object_setInstanceVariable(delegate, IVAR_STATE.as_ptr() as *const c_char, state_ptr);

            msg1(window, sel!("setDelegate:"), delegate);

            let run_loop_mode = {
                let ns = nsstring("kCFRunLoopDefaultMode");
                msg0(ns, sel!("retain"))
            };

            msg1(window, sel!("makeKeyAndOrderFront:"), std::ptr::null_mut());
            {
                let f: unsafe extern "C" fn(id, SEL, BOOL) =
                    std::mem::transmute(objc_msgSend as usize);
                f(app, sel!("activateIgnoringOtherApps:"), YES);
            }

            let bounds = view_bounds(view);

            pool_pop(pool);

            Ok(Self {
                app,
                window,
                view,
                delegate,
                run_loop_mode,
                state,
                width: bounds.size.width as u32,
                height: bounds.size.height as u32,
                gl_ctx: std::ptr::null_mut(),
            })
        }
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, mut callback: F) -> Result<()> {
        loop {
            let pool = unsafe { pool_push() };

            loop {
                let event = unsafe {
                    next_event(
                        self.app,
                        NSEventMaskAny,
                        std::ptr::null_mut(),
                        self.run_loop_mode,
                        YES,
                    )
                };
                if event.is_null() {
                    break;
                }

                if let Some(ev) = self.translate_event(event) {
                    let cf = callback(ev);
                    if let Some(r) = self.handle_cf(cf) {
                        unsafe { pool_pop(pool) };
                        return r;
                    }
                }

                unsafe { msg1(self.app, sel!("sendEvent:"), event) };
            }

            let delegate_events: Vec<Event> = {
                let mut s = self.state.lock().unwrap();
                std::mem::take(&mut s.events)
            };
            for ev in delegate_events {
                let cf = callback(ev);
                if let Some(r) = self.handle_cf(cf) {
                    unsafe { pool_pop(pool) };
                    return r;
                }
            }

            let bounds = unsafe { view_bounds(self.view) };
            let w = bounds.size.width as u32;
            let h = bounds.size.height as u32;
            if (w != self.width || h != self.height) && w > 0 && h > 0 {
                self.width = w;
                self.height = h;
                let cf = callback(Event::Resized {
                    width: w,
                    height: h,
                });
                if let Some(r) = self.handle_cf(cf) {
                    unsafe { pool_pop(pool) };
                    return r;
                }
            }

            let cf = callback(Event::RedrawRequested);
            if let Some(r) = self.handle_cf(cf) {
                unsafe { pool_pop(pool) };
                return r;
            }

            unsafe { msg0(self.app, sel!("updateWindows")) };
            unsafe { pool_pop(pool) };

            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn handle_cf(&mut self, cf: ControlFlow) -> Option<Result<()>> {
        match cf {
            ControlFlow::Exit => Some(Ok(())),
            ControlFlow::WarpAndPoll(x, y) => {
                self.warp_mouse(x, y);
                None
            }
            ControlFlow::Poll => None,
            ControlFlow::Continue => None,
        }
    }

    fn translate_event(&self, event: id) -> Option<Event> {
        let ty = unsafe { msg0_uint(event, sel!("type")) };

        match ty {
            NSEventTypeKeyDown => {
                let code = unsafe { msg0_uint(event, sel!("keyCode")) } as u16;
                Some(Event::KeyDown(keycode_to_key(code)))
            }
            NSEventTypeKeyUp => {
                let code = unsafe { msg0_uint(event, sel!("keyCode")) } as u16;
                Some(Event::KeyUp(keycode_to_key(code)))
            }
            NSEventTypeLeftMouseDown => {
                let (x, y) = self.event_pos(event);
                Some(Event::MouseDown {
                    button: MouseButton::Left,
                    x,
                    y,
                })
            }
            NSEventTypeLeftMouseUp => {
                let (x, y) = self.event_pos(event);
                Some(Event::MouseUp {
                    button: MouseButton::Left,
                    x,
                    y,
                })
            }
            NSEventTypeRightMouseDown => {
                let (x, y) = self.event_pos(event);
                Some(Event::MouseDown {
                    button: MouseButton::Right,
                    x,
                    y,
                })
            }
            NSEventTypeRightMouseUp => {
                let (x, y) = self.event_pos(event);
                Some(Event::MouseUp {
                    button: MouseButton::Right,
                    x,
                    y,
                })
            }
            NSEventTypeOtherMouseDown => {
                let btn_num = unsafe { msg0_uint(event, sel!("buttonNumber")) };
                if btn_num == 2 {
                    let (x, y) = self.event_pos(event);
                    Some(Event::MouseDown {
                        button: MouseButton::Middle,
                        x,
                        y,
                    })
                } else {
                    None
                }
            }
            NSEventTypeOtherMouseUp => {
                let btn_num = unsafe { msg0_uint(event, sel!("buttonNumber")) };
                if btn_num == 2 {
                    let (x, y) = self.event_pos(event);
                    Some(Event::MouseUp {
                        button: MouseButton::Middle,
                        x,
                        y,
                    })
                } else {
                    None
                }
            }
            NSEventTypeMouseMoved
            | NSEventTypeLeftMouseDragged
            | NSEventTypeRightMouseDragged
            | NSEventTypeOtherMouseDragged => {
                let (x, y) = self.event_pos(event);
                Some(Event::MouseMove { x, y })
            }
            NSEventTypeScrollWheel => {
                let delta = unsafe { msg0_f64(event, sel!("scrollingDeltaY")) } as f32;
                if delta != 0.0 {
                    Some(Event::MouseWheel { delta })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert `locationInWindow` (Cocoa bottom-left origin) to our
    /// top-left origin within the content view.
    fn event_pos(&self, event: id) -> (i32, i32) {
        let pt = unsafe { msg0_point(event, sel!("locationInWindow")) };
        (pt.x as i32, self.height as i32 - pt.y as i32)
    }

    pub fn request_redraw(&self) {}

    pub fn set_title(&self, title: &str) {
        let pool = unsafe { pool_push() };
        unsafe {
            msg1(self.window, sel!("setTitle:"), nsstring(title));
            pool_pop(pool);
        }
    }

    pub fn set_fullscreen(&self, is_fullscreen: bool) {
        unsafe {
            let style = msg0_uint(self.window, sel!("styleMask"));
            let currently_fullscreen = (style & NSWindowStyleMaskFullScreen) != 0;
            if is_fullscreen != currently_fullscreen {
                msg1(self.window, sel!("toggleFullScreen:"), std::ptr::null_mut());
            }
        }
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        unsafe {
            let cocoa_pt = NSPoint {
                x: x as CGFloat,
                y: self.height as CGFloat - y as CGFloat,
            };

            let f: unsafe extern "C" fn(id, SEL, NSPoint) -> NSPoint =
                std::mem::transmute(objc_msgSend as usize);
            let screen_cocoa = f(self.window, sel!("convertPointToScreen:"), cocoa_pt);

            let screen_frame = msg0_rect(msg0(cls!("NSScreen"), sel!("mainScreen")), sel!("frame"));
            let cg_y = screen_frame.size.height - screen_cocoa.y;
            CGWarpMouseCursorPosition(NSPoint {
                x: screen_cocoa.x,
                y: cg_y,
            });
        }
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        unsafe {
            let display = CGMainDisplayID();
            if visible {
                CGDisplayShowCursor(display);
            } else {
                CGDisplayHideCursor(display);
            }
        }
    }

    pub fn inner_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for PlatformWindow {
    fn drop(&mut self) {
        unsafe {
            object_setInstanceVariable(
                self.delegate,
                IVAR_STATE.as_ptr() as *const c_char,
                std::ptr::null_mut(),
            );
            msg0(self.window, sel!("close"));
            msg0(self.run_loop_mode, sel!("release"));
        }
    }
}

impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let ptr = NonNull::new(self.view).ok_or(HandleError::Unavailable)?;
        let h = AppKitWindowHandle::new(ptr);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(h)) })
    }
}

impl HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::AppKit(AppKitDisplayHandle::new()))
        })
    }
}

unsafe fn view_bounds(view: id) -> NSRect {
    msg0_rect(view, sel!("bounds"))
}

fn keycode_to_key(code: u16) -> Key {
    match code {
        0x00 => Key::A,
        0x0B => Key::B,
        0x08 => Key::C,
        0x02 => Key::D,
        0x0E => Key::E,
        0x03 => Key::F,
        0x05 => Key::G,
        0x04 => Key::H,
        0x22 => Key::I,
        0x26 => Key::J,
        0x28 => Key::K,
        0x25 => Key::L,
        0x2E => Key::M,
        0x2D => Key::N,
        0x1F => Key::O,
        0x23 => Key::P,
        0x0C => Key::Q,
        0x0F => Key::R,
        0x01 => Key::S,
        0x11 => Key::T,
        0x20 => Key::U,
        0x09 => Key::V,
        0x0D => Key::W,
        0x07 => Key::X,
        0x10 => Key::Y,
        0x06 => Key::Z,

        0x1D => Key::Num0,
        0x12 => Key::Num1,
        0x13 => Key::Num2,
        0x14 => Key::Num3,
        0x15 => Key::Num4,
        0x17 => Key::Num5,
        0x16 => Key::Num6,
        0x1A => Key::Num7,
        0x1C => Key::Num8,
        0x19 => Key::Num9,

        0x7A => Key::F1,
        0x78 => Key::F2,
        0x63 => Key::F3,
        0x76 => Key::F4,
        0x60 => Key::F5,
        0x61 => Key::F6,
        0x62 => Key::F7,
        0x64 => Key::F8,
        0x65 => Key::F9,
        0x6D => Key::F10,
        0x67 => Key::F11,
        0x6F => Key::F12,

        0x24 => Key::Enter,
        0x35 => Key::Escape,
        0x31 => Key::Space,
        0x33 => Key::Backspace,
        0x30 => Key::Tab,
        0x75 => Key::Delete,

        0x7B => Key::ArrowLeft,
        0x7C => Key::ArrowRight,
        0x7E => Key::ArrowUp,
        0x7D => Key::ArrowDown,

        0x73 => Key::Home,
        0x77 => Key::End,
        0x74 => Key::PageUp,
        0x79 => Key::PageDown,

        0x38 => Key::LeftShift,
        0x3C => Key::RightShift,
        0x3B => Key::LeftCtrl,
        0x3E => Key::RightCtrl,
        0x3A => Key::LeftAlt,
        0x3D => Key::RightAlt,
        0x37 => Key::LeftSuper,
        0x36 => Key::RightSuper,

        _ => Key::Unknown,
    }
}
