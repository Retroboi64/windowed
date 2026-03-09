use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::connection::Connection;
use x11rb::protocol::Event as XEvent;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle, XlibDisplayHandle, XlibWindowHandle,
};

use crate::config::WindowConfig;
use crate::error::{Error, Result};
use crate::event::{ControlFlow, Event, Key, MouseButton};

pub struct PlatformWindow {
    conn: RustConnection,
    window: u32,
    screen_num: usize,
    wm_delete_window: u32,
    blank_cursor: u32,
    width: u32,
    height: u32,
}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| Error::Platform(e.to_string()))?;

        let screen = &conn.setup().roots[screen_num].clone();

        let window = conn
            .generate_id()
            .map_err(|e| Error::Platform(e.to_string()))?;

        let event_mask = EventMask::EXPOSURE
            | EventMask::KEY_PRESS
            | EventMask::KEY_RELEASE
            | EventMask::BUTTON_PRESS
            | EventMask::BUTTON_RELEASE
            | EventMask::POINTER_MOTION
            | EventMask::STRUCTURE_NOTIFY
            | EventMask::FOCUS_CHANGE;

        let (x, y) = config.position.unwrap_or((0, 0));

        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            screen.root,
            x as i16,
            y as i16,
            config.width as u16,
            config.height as u16,
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
            &CreateWindowAux::new()
                .background_pixel(screen.white_pixel)
                .event_mask(event_mask),
        )
        .map_err(|e| Error::Platform(e.to_string()))?;

        conn.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            config.title.as_bytes(),
        )
        .map_err(|e| Error::Platform(e.to_string()))?;

        let utf8_string = intern_atom(&conn, b"UTF8_STRING")?;
        let net_wm_name = intern_atom(&conn, b"_NET_WM_NAME")?;
        conn.change_property8(
            PropMode::REPLACE,
            window,
            net_wm_name,
            utf8_string,
            config.title.as_bytes(),
        )
        .map_err(|e| Error::Platform(e.to_string()))?;

        let wm_protocols = intern_atom(&conn, b"WM_PROTOCOLS")?;
        let wm_delete_window = intern_atom(&conn, b"WM_DELETE_WINDOW")?;
        conn.change_property32(
            PropMode::REPLACE,
            window,
            wm_protocols,
            AtomEnum::ATOM,
            &[wm_delete_window],
        )
        .map_err(|e| Error::Platform(e.to_string()))?;

        if !config.resizable {
            set_fixed_size(&conn, window, config.width, config.height)?;
        }

        let blank_cursor = create_blank_cursor(&conn, &screen)?;

        conn.map_window(window)
            .map_err(|e| Error::Platform(e.to_string()))?;
        conn.flush().map_err(|e| Error::Platform(e.to_string()))?;

        Ok(Self {
            conn,
            window,
            screen_num,
            wm_delete_window,
            blank_cursor,
            width: config.width,
            height: config.height,
        })
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, mut callback: F) -> Result<()> {
        let mut polling = false;

        loop {
            let x_event_opt = if polling {
                self.conn
                    .poll_for_event()
                    .map_err(|e| Error::Platform(e.to_string()))?
            } else {
                Some(
                    self.conn
                        .wait_for_event()
                        .map_err(|e| Error::Platform(e.to_string()))?,
                )
            };

            let Some(x_event) = x_event_opt else {
                self.request_redraw();
                self.conn
                    .flush()
                    .map_err(|e| Error::Platform(e.to_string()))?;
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            };

            if let Some(event) = self.translate_event(x_event) {
                match callback(event) {
                    ControlFlow::Exit => return Ok(()),
                    ControlFlow::Poll => polling = true,
                    ControlFlow::WarpAndPoll(x, y) => {
                        polling = true;
                        self.warp_mouse(x, y);
                    }
                    ControlFlow::Continue => {}
                }
            }

            self.conn
                .flush()
                .map_err(|e| Error::Platform(e.to_string()))?;
        }
    }

    fn translate_event(&mut self, x_event: XEvent) -> Option<Event> {
        match x_event {
            XEvent::ClientMessage(e) => {
                if e.data.as_data32()[0] == self.wm_delete_window {
                    Some(Event::CloseRequested)
                } else {
                    None
                }
            }

            XEvent::Expose(e) if e.count == 0 => Some(Event::RedrawRequested),
            XEvent::Expose(_) => None,

            XEvent::ConfigureNotify(e) => {
                let w = e.width as u32;
                let h = e.height as u32;
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

            XEvent::KeyPress(e) => Some(Event::KeyDown(x11_keycode_to_key(e.detail))),
            XEvent::KeyRelease(e) => Some(Event::KeyUp(x11_keycode_to_key(e.detail))),

            XEvent::ButtonPress(e) => match e.detail {
                4 => Some(Event::MouseWheel { delta: 1.0 }),
                5 => Some(Event::MouseWheel { delta: -1.0 }),
                btn => x11_button(btn).map(|button| Event::MouseDown {
                    button,
                    x: e.event_x as i32,
                    y: e.event_y as i32,
                }),
            },

            XEvent::ButtonRelease(e) => x11_button(e.detail).map(|button| Event::MouseUp {
                button,
                x: e.event_x as i32,
                y: e.event_y as i32,
            }),

            XEvent::MotionNotify(e) => Some(Event::MouseMove {
                x: e.event_x as i32,
                y: e.event_y as i32,
            }),

            XEvent::FocusIn(_) => Some(Event::FocusGained),
            XEvent::FocusOut(_) => Some(Event::FocusLost),

            _ => None,
        }
    }

    pub fn request_redraw(&self) {
        let event = ExposeEvent {
            response_type: EXPOSE_EVENT,
            window: self.window,
            x: 0,
            y: 0,
            width: self.width as u16,
            height: self.height as u16,
            count: 0,
            sequence: 0,
        };
        let _ = self
            .conn
            .send_event(false, self.window, EventMask::EXPOSURE, event);
        let _ = self.conn.flush();
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        let cursor = if visible { 0u32 } else { self.blank_cursor };
        let aux = ChangeWindowAttributesAux::new().cursor(cursor);
        let _ = self.conn.change_window_attributes(self.window, &aux);
        let _ = self.conn.flush();
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        use x11rb::protocol::xproto::ConnectionExt as _;
        let _ = self
            .conn
            .warp_pointer(x11rb::NONE, self.window, 0, 0, 0, 0, x as i16, y as i16);
        let _ = self.conn.flush();
    }

    pub fn set_title(&self, title: &str) {
        let _ = self.conn.change_property8(
            PropMode::REPLACE,
            self.window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        );
        let _ = self.conn.flush();
    }

    pub fn inner_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for PlatformWindow {
    fn drop(&mut self) {
        let _ = self.conn.free_cursor(self.blank_cursor);
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}

unsafe impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let mut handle = XlibWindowHandle::new(self.window as u64);
        handle.visual_id = 0;
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Xlib(handle)) })
    }
}

unsafe impl HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        let handle = XlibDisplayHandle::new(None, self.screen_num as i32);
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Xlib(handle)) })
    }
}

fn intern_atom(conn: &RustConnection, name: &[u8]) -> Result<u32> {
    conn.intern_atom(false, name)
        .map_err(|e| Error::Platform(e.to_string()))?
        .reply()
        .map(|r| r.atom)
        .map_err(|e| Error::Platform(e.to_string()))
}

fn create_blank_cursor(conn: &RustConnection, screen: &Screen) -> Result<u32> {
    let pixmap = conn
        .generate_id()
        .map_err(|e| Error::Platform(e.to_string()))?;
    conn.create_pixmap(1, pixmap, screen.root, 1, 1)
        .map_err(|e| Error::Platform(e.to_string()))?;

    let gc = conn
        .generate_id()
        .map_err(|e| Error::Platform(e.to_string()))?;
    conn.create_gc(gc, pixmap, &CreateGCAux::new().foreground(0))
        .map_err(|e| Error::Platform(e.to_string()))?;
    conn.poly_fill_rectangle(
        pixmap,
        gc,
        &[Rectangle {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        }],
    )
    .map_err(|e| Error::Platform(e.to_string()))?;
    conn.free_gc(gc)
        .map_err(|e| Error::Platform(e.to_string()))?;

    let cursor = conn
        .generate_id()
        .map_err(|e| Error::Platform(e.to_string()))?;
    conn.create_cursor(cursor, pixmap, pixmap, 0, 0, 0, 0, 0, 0, 0, 0)
        .map_err(|e| Error::Platform(e.to_string()))?;

    conn.free_pixmap(pixmap)
        .map_err(|e| Error::Platform(e.to_string()))?;

    Ok(cursor)
}

fn set_fixed_size(conn: &RustConnection, window: u32, width: u32, height: u32) -> Result<()> {
    let flags: u32 = 0x30;
    let hints: [u32; 18] = [
        flags, 0, 0, 0, 0, 0, width, height, // min size
        width, height, // max size
        0, 0, 0, 0, 0, 0, 0, 0,
    ];
    conn.change_property32(
        PropMode::REPLACE,
        window,
        AtomEnum::WM_NORMAL_HINTS,
        AtomEnum::WM_SIZE_HINTS,
        &hints,
    )
    .map_err(|e| Error::Platform(e.to_string()))?;
    Ok(())
}

fn x11_button(btn: u8) -> Option<MouseButton> {
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

        _ => Key::Unknown,
    }
}
