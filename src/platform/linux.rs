//! Linux / X11 platform backend.
#![allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::upper_case_acronyms
)]

use std::ffi::{CString, c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::ptr::NonNull;

use crate::config::WindowConfig;
use crate::error::{Error, Result};
use crate::event::{ControlFlow, Event, Key, MouseButton};
use crate::rwd::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle, XlibDisplayHandle, XlibWindowHandle,
};

// ── C type aliases ────────────────────────────────────────────────────────────
type XDisplay = c_void;
type XWindow = c_ulong;
type Atom = c_ulong;
type Bool = c_int;
type Pixmap = c_ulong;
type Cursor = c_ulong;
type GC = *mut c_void;

// ── X11 event type constants ──────────────────────────────────────────────────
const KEY_PRESS: c_int = 2;
const KEY_RELEASE: c_int = 3;
const BUTTON_PRESS: c_int = 4;
const BUTTON_RELEASE: c_int = 5;
const MOTION_NOTIFY: c_int = 6;
const ENTER_NOTIFY: c_int = 7;
const LEAVE_NOTIFY: c_int = 8;
const FOCUS_IN: c_int = 9;
const FOCUS_OUT: c_int = 10;
const CONFIGURE_NOTIFY: c_int = 22;
const CLIENT_MESSAGE: c_int = 33;

// ── X11 event mask constants ──────────────────────────────────────────────────
const KEY_PRESS_MASK: c_long = 1 << 0;
const KEY_RELEASE_MASK: c_long = 1 << 1;
const BUTTON_PRESS_MASK: c_long = 1 << 2;
const BUTTON_RELEASE_MASK: c_long = 1 << 3;
const POINTER_MOTION_MASK: c_long = 1 << 6;
const ENTER_WINDOW_MASK: c_long = 1 << 4;
const LEAVE_WINDOW_MASK: c_long = 1 << 5;
const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;
const FOCUS_CHANGE_MASK: c_long = 1 << 21;

// ── XCreateWindow valuemask flags ────────────────────────────────────────────
const CW_BACK_PIXEL: c_ulong = 1 << 1;
const CW_EVENT_MASK: c_ulong = 1 << 11;
const CW_CURSOR: c_ulong = 1 << 14;

const INPUT_OUTPUT: c_uint = 1;
const COPY_FROM_PARENT: c_long = 0;
const PROP_MODE_REPLACE: c_int = 0;
const GC_FOREGROUND: c_ulong = 1 << 2;
const P_MIN_SIZE: c_long = 1 << 4;
const P_MAX_SIZE: c_long = 1 << 5;

// ── XEvent sub-structs (64-bit Linux LP64 ABI) ────────────────────────────────
#[repr(C)]
#[derive(Copy, Clone)]
struct XKeyEvent {
    type_: c_int,
    _p0: c_int,
    serial: c_ulong,
    send_event: Bool,
    _p1: c_int,
    display: *mut XDisplay,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    keycode: c_uint,
    same_screen: Bool,
    _p2: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct XButtonEvent {
    type_: c_int,
    _p0: c_int,
    serial: c_ulong,
    send_event: Bool,
    _p1: c_int,
    display: *mut XDisplay,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    button: c_uint,
    same_screen: Bool,
    _p2: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct XMotionEvent {
    type_: c_int,
    _p0: c_int,
    serial: c_ulong,
    send_event: Bool,
    _p1: c_int,
    display: *mut XDisplay,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    is_hint: std::ffi::c_char,
    _p2: [u8; 3],
    same_screen: Bool,
    _p3: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct XCrossingEvent {
    type_: c_int,
    _p0: c_int,
    serial: c_ulong,
    send_event: Bool,
    _p1: c_int,
    display: *mut XDisplay,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    mode: c_int,
    detail: c_int,
    same_screen: Bool,
    focus: Bool,
    _p2: c_int,
    state: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct XConfigureEvent {
    type_: c_int,
    _p0: c_int,
    serial: c_ulong,
    send_event: Bool,
    _p1: c_int,
    display: *mut XDisplay,
    event: XWindow,
    window: XWindow,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    _p2: c_int,
    above: XWindow,
    override_redirect: Bool,
    _p3: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct XClientMessageEvent {
    type_: c_int,
    _p0: c_int,
    serial: c_ulong,
    send_event: Bool,
    _p1: c_int,
    display: *mut XDisplay,
    window: XWindow,
    message_type: Atom,
    format: c_int,
    _p2: c_int,
    data: [c_long; 5],
}

#[repr(C)]
union XEvent {
    type_: c_int,
    key: XKeyEvent,
    button: XButtonEvent,
    motion: XMotionEvent,
    crossing: XCrossingEvent,
    configure: XConfigureEvent,
    client_message: XClientMessageEvent,
    _pad: [c_long; 24],
}

// ── Misc C structs ────────────────────────────────────────────────────────────
#[repr(C)]
struct XSetWindowAttributes {
    background_pixmap: Pixmap,
    background_pixel: c_ulong,
    border_pixmap: Pixmap,
    border_pixel: c_ulong,
    bit_gravity: c_int,
    win_gravity: c_int,
    backing_store: c_int,
    _p0: c_int,
    backing_planes: c_ulong,
    backing_pixel: c_ulong,
    save_under: Bool,
    _p1: c_int,
    event_mask: c_long,
    do_not_propagate_mask: c_long,
    override_redirect: Bool,
    _p2: c_int,
    colormap: c_ulong,
    cursor: Cursor,
}

#[repr(C)]
struct XSizeHints {
    flags: c_long,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    min_width: c_int,
    min_height: c_int,
    max_width: c_int,
    max_height: c_int,
    width_inc: c_int,
    height_inc: c_int,
    min_aspect_x: c_int,
    min_aspect_y: c_int,
    max_aspect_x: c_int,
    max_aspect_y: c_int,
    base_width: c_int,
    base_height: c_int,
    win_gravity: c_int,
    _pad: c_int,
}

#[repr(C)]
struct XColor {
    pixel: c_ulong,
    red: u16,
    green: u16,
    blue: u16,
    flags: std::ffi::c_char,
    pad: std::ffi::c_char,
}

#[repr(C)]
struct XGCValues {
    function: c_int,
    _p0: c_int,
    plane_mask: c_ulong,
    foreground: c_ulong,
    background: c_ulong,
    line_width: c_int,
    line_style: c_int,
    cap_style: c_int,
    join_style: c_int,
    fill_style: c_int,
    fill_rule: c_int,
    arc_mode: c_int,
    _p1: c_int,
    tile: c_ulong,
    stipple: c_ulong,
    ts_x_origin: c_int,
    ts_y_origin: c_int,
    font: c_ulong,
    subwindow_mode: c_int,
    graphics_exposures: c_int,
    clip_x_origin: c_int,
    clip_y_origin: c_int,
    clip_mask: c_ulong,
    dash_offset: c_int,
    dashes: std::ffi::c_char,
    _p2: [u8; 3],
}

// ── X11 FFI ───────────────────────────────────────────────────────────────────
#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(name: *const c_char) -> *mut XDisplay;
    fn XCloseDisplay(display: *mut XDisplay);
    fn XDefaultScreen(display: *mut XDisplay) -> c_int;
    fn XRootWindow(display: *mut XDisplay, screen: c_int) -> XWindow;
    fn XWhitePixel(display: *mut XDisplay, screen: c_int) -> c_ulong;
    fn XCreateWindow(
        display: *mut XDisplay,
        parent: XWindow,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        border_width: c_uint,
        depth: c_int,
        class: c_uint,
        visual: *mut c_void,
        valuemask: c_ulong,
        attributes: *mut XSetWindowAttributes,
    ) -> XWindow;
    fn XMapWindow(display: *mut XDisplay, w: XWindow) -> c_int;
    fn XFlush(display: *mut XDisplay) -> c_int;
    fn XStoreName(display: *mut XDisplay, w: XWindow, name: *const c_char) -> c_int;
    fn XInternAtom(display: *mut XDisplay, name: *const c_char, only_if_exists: Bool) -> Atom;
    fn XSetWMProtocols(
        display: *mut XDisplay,
        w: XWindow,
        protocols: *mut Atom,
        count: c_int,
    ) -> c_int;
    fn XSetWMNormalHints(display: *mut XDisplay, w: XWindow, hints: *mut XSizeHints);
    fn XPending(display: *mut XDisplay) -> c_int;
    fn XNextEvent(display: *mut XDisplay, event_return: *mut XEvent) -> c_int;
    fn XWarpPointer(
        display: *mut XDisplay,
        src_w: XWindow,
        dest_w: XWindow,
        src_x: c_int,
        src_y: c_int,
        src_width: c_uint,
        src_height: c_uint,
        dest_x: c_int,
        dest_y: c_int,
    ) -> c_int;
    fn XChangeWindowAttributes(
        display: *mut XDisplay,
        w: XWindow,
        valuemask: c_ulong,
        attributes: *mut XSetWindowAttributes,
    ) -> c_int;
    fn XDestroyWindow(display: *mut XDisplay, w: XWindow) -> c_int;
    fn XCreatePixmap(
        display: *mut XDisplay,
        d: XWindow,
        width: c_uint,
        height: c_uint,
        depth: c_uint,
    ) -> Pixmap;
    fn XCreateGC(
        display: *mut XDisplay,
        d: XWindow,
        valuemask: c_ulong,
        values: *mut XGCValues,
    ) -> GC;
    fn XFillRectangle(
        display: *mut XDisplay,
        d: XWindow,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    fn XFreeGC(display: *mut XDisplay, gc: GC) -> c_int;
    fn XCreatePixmapCursor(
        display: *mut XDisplay,
        source: Pixmap,
        mask: Pixmap,
        fg: *mut XColor,
        bg: *mut XColor,
        x: c_uint,
        y: c_uint,
    ) -> Cursor;
    fn XFreePixmap(display: *mut XDisplay, pixmap: Pixmap) -> c_int;
    fn XFreeCursor(display: *mut XDisplay, cursor: Cursor) -> c_int;
    fn XChangeProperty(
        display: *mut XDisplay,
        w: XWindow,
        property: Atom,
        type_: Atom,
        format: c_int,
        mode: c_int,
        data: *const c_void,
        nelements: c_int,
    ) -> c_int;
}

// ── PlatformWindow ────────────────────────────────────────────────────────────
pub struct PlatformWindow {
    display: *mut XDisplay,
    window: XWindow,
    screen: c_int,
    wm_delete_window: Atom,
    blank_cursor: Cursor,
    width: u32,
    height: u32,
}

unsafe impl Send for PlatformWindow {}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        let display = unsafe { XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            return Err(Error::NoDisplay);
        }

        let screen = unsafe { XDefaultScreen(display) };
        let root = unsafe { XRootWindow(display, screen) };
        let white = unsafe { XWhitePixel(display, screen) };

        let event_mask = KEY_PRESS_MASK
            | KEY_RELEASE_MASK
            | BUTTON_PRESS_MASK
            | BUTTON_RELEASE_MASK
            | POINTER_MOTION_MASK
            | ENTER_WINDOW_MASK
            | LEAVE_WINDOW_MASK
            | STRUCTURE_NOTIFY_MASK
            | FOCUS_CHANGE_MASK;

        let (x, y) = config.position.unwrap_or((0, 0));
        let mut attrs: XSetWindowAttributes = unsafe { std::mem::zeroed() };
        attrs.background_pixel = white;
        attrs.event_mask = event_mask;

        let window = unsafe {
            XCreateWindow(
                display,
                root,
                x,
                y,
                config.width,
                config.height,
                0,
                COPY_FROM_PARENT as c_int,
                INPUT_OUTPUT,
                std::ptr::null_mut(),
                CW_BACK_PIXEL | CW_EVENT_MASK,
                &mut attrs,
            )
        };
        if window == 0 {
            return Err(Error::Platform("XCreateWindow failed".into()));
        }

        // ── Window title ──────────────────────────────────────────────────────
        let title_c = CString::new(config.title.as_str())
            .unwrap_or_else(|_| CString::new("windowed").unwrap());
        unsafe { XStoreName(display, window, title_c.as_ptr()) };

        let utf8_atom = intern_atom(display, b"UTF8_STRING\0");
        let net_wm_name = intern_atom(display, b"_NET_WM_NAME\0");
        let tb = config.title.as_bytes();
        unsafe {
            XChangeProperty(
                display,
                window,
                net_wm_name,
                utf8_atom,
                8,
                PROP_MODE_REPLACE,
                tb.as_ptr() as *const c_void,
                tb.len() as c_int,
            );
        }

        // ── WM_DELETE_WINDOW protocol ─────────────────────────────────────────
        let mut wm_delete_window = intern_atom(display, b"WM_DELETE_WINDOW\0");
        unsafe { XSetWMProtocols(display, window, &mut wm_delete_window, 1) };

        // ── Size hints (non-resizable) ────────────────────────────────────────
        if !config.resizable {
            let mut hints: XSizeHints = unsafe { std::mem::zeroed() };
            hints.flags = P_MIN_SIZE | P_MAX_SIZE;
            hints.min_width = config.width as c_int;
            hints.min_height = config.height as c_int;
            hints.max_width = config.width as c_int;
            hints.max_height = config.height as c_int;
            unsafe { XSetWMNormalHints(display, window, &mut hints) };
        } else {
            let need_hints = config.min_size.is_some() || config.max_size.is_some();
            if need_hints {
                let mut hints: XSizeHints = unsafe { std::mem::zeroed() };
                if let Some((w, h)) = config.min_size {
                    hints.flags |= P_MIN_SIZE;
                    hints.min_width = w as c_int;
                    hints.min_height = h as c_int;
                }
                if let Some((w, h)) = config.max_size {
                    hints.flags |= P_MAX_SIZE;
                    hints.max_width = w as c_int;
                    hints.max_height = h as c_int;
                }
                unsafe { XSetWMNormalHints(display, window, &mut hints) };
            }
        }

        let blank_cursor = create_blank_cursor(display, root)?;

        unsafe {
            XMapWindow(display, window);
            XFlush(display);
        }

        Ok(Self {
            display,
            window,
            screen,
            wm_delete_window,
            blank_cursor,
            width: config.width,
            height: config.height,
        })
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, mut callback: F) -> Result<()> {
        loop {
            while unsafe { XPending(self.display) } > 0 {
                let mut ev: XEvent = unsafe { std::mem::zeroed() };
                unsafe { XNextEvent(self.display, &mut ev) };
                if let Some(event) = self.translate_event(&ev) {
                    match callback(event) {
                        ControlFlow::Exit => return Ok(()),
                        ControlFlow::WarpAndPoll(x, y) => self.warp_mouse(x, y),
                        ControlFlow::Poll | ControlFlow::Continue => {}
                    }
                }
            }

            match callback(Event::RedrawRequested) {
                ControlFlow::Exit => return Ok(()),
                ControlFlow::WarpAndPoll(x, y) => self.warp_mouse(x, y),
                ControlFlow::Poll | ControlFlow::Continue => {}
            }

            unsafe { XFlush(self.display) };
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn translate_event(&mut self, ev: &XEvent) -> Option<Event> {
        match unsafe { ev.type_ } {
            CLIENT_MESSAGE => {
                let atom = unsafe { ev.client_message.data[0] as Atom };
                if atom == self.wm_delete_window {
                    Some(Event::CloseRequested)
                } else {
                    None
                }
            }
            CONFIGURE_NOTIFY => {
                let w = unsafe { ev.configure.width as u32 };
                let h = unsafe { ev.configure.height as u32 };
                if w != self.width || h != self.height {
                    self.width = w;
                    self.height = h;
                    Some(Event::Resized {
                        width: w,
                        height: h,
                    })
                } else {
                    None
                }
            }
            KEY_PRESS => Some(Event::KeyDown(x11_keycode_to_key(
                unsafe { ev.key.keycode } as u8,
            ))),
            KEY_RELEASE => Some(Event::KeyUp(x11_keycode_to_key(
                unsafe { ev.key.keycode } as u8
            ))),
            BUTTON_PRESS => {
                let (btn, x, y) = unsafe { (ev.button.button, ev.button.x, ev.button.y) };
                match btn {
                    4 => Some(Event::MouseWheel { delta: 1.0 }),
                    5 => Some(Event::MouseWheel { delta: -1.0 }),
                    b => x11_button(b).map(|button| Event::MouseDown { button, x, y }),
                }
            }
            BUTTON_RELEASE => {
                let (btn, x, y) = unsafe { (ev.button.button, ev.button.x, ev.button.y) };
                x11_button(btn).map(|button| Event::MouseUp { button, x, y })
            }
            MOTION_NOTIFY => Some(Event::MouseMove {
                x: unsafe { ev.motion.x },
                y: unsafe { ev.motion.y },
            }),
            ENTER_NOTIFY => Some(Event::CursorEntered),
            LEAVE_NOTIFY => Some(Event::CursorLeft),
            FOCUS_IN => Some(Event::FocusGained),
            FOCUS_OUT => Some(Event::FocusLost),
            _ => None,
        }
    }

    pub fn request_redraw(&self) {}

    pub fn set_cursor_visible(&self, visible: bool) {
        let cursor = if visible { 0u64 } else { self.blank_cursor };
        let mut attrs: XSetWindowAttributes = unsafe { std::mem::zeroed() };
        attrs.cursor = cursor;
        unsafe {
            XChangeWindowAttributes(self.display, self.window, CW_CURSOR, &mut attrs);
            XFlush(self.display);
        }
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        unsafe {
            XWarpPointer(self.display, 0, self.window, 0, 0, 0, 0, x, y);
            XFlush(self.display);
        }
    }

    pub fn set_title(&self, title: &str) {
        if let Ok(c) = CString::new(title) {
            unsafe {
                XStoreName(self.display, self.window, c.as_ptr());
            }
        }
        let utf8_atom = intern_atom(self.display, b"UTF8_STRING\0");
        let net_wm_name = intern_atom(self.display, b"_NET_WM_NAME\0");
        let bytes = title.as_bytes();
        unsafe {
            XChangeProperty(
                self.display,
                self.window,
                net_wm_name,
                utf8_atom,
                8,
                PROP_MODE_REPLACE,
                bytes.as_ptr() as *const c_void,
                bytes.len() as c_int,
            );
            XFlush(self.display);
        }
    }

    pub fn inner_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for PlatformWindow {
    fn drop(&mut self) {
        unsafe {
            XFreeCursor(self.display, self.blank_cursor);
            XDestroyWindow(self.display, self.window);
            XFlush(self.display);
            XCloseDisplay(self.display);
        }
    }
}

impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let mut h = XlibWindowHandle::new(self.window);
        h.visual_id = 0;
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Xlib(h)) })
    }
}

impl HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        let ptr = NonNull::new(self.display).expect("display is null");
        let h = XlibDisplayHandle::new(Some(ptr), self.screen);
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Xlib(h)) })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn intern_atom(display: *mut XDisplay, name: &[u8]) -> Atom {
    unsafe { XInternAtom(display, name.as_ptr() as *const c_char, 0) }
}

fn create_blank_cursor(display: *mut XDisplay, root: XWindow) -> Result<Cursor> {
    let pixmap = unsafe { XCreatePixmap(display, root, 1, 1, 1) };
    if pixmap == 0 {
        return Err(Error::Platform("XCreatePixmap failed".into()));
    }
    let mut gcv: XGCValues = unsafe { std::mem::zeroed() };
    gcv.foreground = 0;
    let gc = unsafe { XCreateGC(display, pixmap, GC_FOREGROUND, &mut gcv) };
    unsafe { XFillRectangle(display, pixmap, gc, 0, 0, 1, 1) };
    unsafe { XFreeGC(display, gc) };
    let mut black: XColor = unsafe { std::mem::zeroed() };
    let cursor =
        unsafe { XCreatePixmapCursor(display, pixmap, pixmap, &mut black, &mut black, 0, 0) };
    unsafe { XFreePixmap(display, pixmap) };
    Ok(cursor)
}

fn x11_button(btn: c_uint) -> Option<MouseButton> {
    match btn {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        _ => None,
    }
}

fn x11_keycode_to_key(code: u8) -> Key {
    match code {
        24 => Key::Q,
        25 => Key::W,
        26 => Key::E,
        27 => Key::R,
        28 => Key::T,
        29 => Key::Y,
        30 => Key::U,
        31 => Key::I,
        32 => Key::O,
        33 => Key::P,
        38 => Key::A,
        39 => Key::S,
        40 => Key::D,
        41 => Key::F,
        42 => Key::G,
        43 => Key::H,
        44 => Key::J,
        45 => Key::K,
        46 => Key::L,
        52 => Key::Z,
        53 => Key::X,
        54 => Key::C,
        55 => Key::V,
        56 => Key::B,
        57 => Key::N,
        58 => Key::M,
        10 => Key::Num1,
        11 => Key::Num2,
        12 => Key::Num3,
        13 => Key::Num4,
        14 => Key::Num5,
        15 => Key::Num6,
        16 => Key::Num7,
        17 => Key::Num8,
        18 => Key::Num9,
        19 => Key::Num0,
        67 => Key::F1,
        68 => Key::F2,
        69 => Key::F3,
        70 => Key::F4,
        71 => Key::F5,
        72 => Key::F6,
        73 => Key::F7,
        74 => Key::F8,
        75 => Key::F9,
        76 => Key::F10,
        95 => Key::F11,
        96 => Key::F12,
        36 => Key::Enter,
        9 => Key::Escape,
        65 => Key::Space,
        22 => Key::Backspace,
        23 => Key::Tab,
        119 => Key::Delete,
        118 => Key::Insert,
        113 => Key::ArrowLeft,
        114 => Key::ArrowRight,
        111 => Key::ArrowUp,
        116 => Key::ArrowDown,
        110 => Key::Home,
        115 => Key::End,
        112 => Key::PageUp,
        117 => Key::PageDown,
        50 => Key::LeftShift,
        62 => Key::RightShift,
        37 => Key::LeftCtrl,
        105 => Key::RightCtrl,
        64 => Key::LeftAlt,
        108 => Key::RightAlt,
        133 => Key::LeftSuper,
        134 => Key::RightSuper,
        _ => Key::Unknown,
    }
}
