use std::sync::{Arc, Mutex};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, EndPaint, GetStockObject, HBRUSH, InvalidateRect, PAINTSTRUCT,
            UpdateWindow, WHITE_BRUSH,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{
                VIRTUAL_KEY, VK_A, VK_B, VK_BACK, VK_C, VK_D, VK_DELETE, VK_DOWN, VK_E, VK_END,
                VK_ESCAPE, VK_F, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9,
                VK_F10, VK_F11, VK_F12, VK_G, VK_H, VK_HOME, VK_I, VK_INSERT, VK_J, VK_K, VK_L,
                VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_M, VK_N, VK_NEXT, VK_NUMPAD0,
                VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7,
                VK_NUMPAD8, VK_NUMPAD9, VK_O, VK_P, VK_PRIOR, VK_Q, VK_R, VK_RCONTROL, VK_RETURN,
                VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_S, VK_SPACE, VK_T, VK_TAB, VK_U, VK_UP, VK_V,
                VK_W, VK_X, VK_Y, VK_Z,
            },
            WindowsAndMessaging::{
                AdjustWindowRect, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW,
                DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetClientRect,
                GetWindowLongPtrW, GetWindowRect, IDC_ARROW, LoadCursorW, MSG, PM_REMOVE,
                PeekMessageW, PostQuitMessage, RegisterClassExW, SW_SHOW, SetCursorPos,
                SetWindowLongPtrW, SetWindowTextW, ShowCursor, ShowWindow, TranslateMessage,
                WHEEL_DELTA, WM_CHAR, WM_CLOSE, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
                WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
                WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETFOCUS, WM_SIZE,
                WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSEXW, WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED,
                WS_OVERLAPPEDWINDOW, WS_SYSMENU,
            },
        },
    },
    core::PCWSTR,
};

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

use crate::config::WindowConfig;
use crate::error::{Error, Result};
use crate::event::{ControlFlow, Event, Key, MouseButton};

struct WindowState {
    events: Vec<Event>,
}

pub struct PlatformWindow {
    hwnd: HWND,
    hinstance: HINSTANCE,
    state: Arc<Mutex<WindowState>>,
    width: u32,
    height: u32,
}

unsafe impl Send for PlatformWindow {}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<WindowState> };

    if state_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let state = unsafe { &*state_ptr };

    macro_rules! push {
        ($event:expr) => {
            if let Ok(mut s) = state.lock() {
                s.events.push($event);
            }
        };
    }

    macro_rules! mouse_pos {
        ($lp:expr) => {
            (
                ($lp.0 & 0xFFFF) as i16 as i32,
                (($lp.0 >> 16) & 0xFFFF) as i16 as i32,
            )
        };
    }

    match msg {
        WM_CLOSE => {
            push!(Event::CloseRequested);
            LRESULT(0)
        }

        WM_SIZE => {
            let w = (lparam.0 & 0xFFFF) as u32;
            let h = ((lparam.0 >> 16) & 0xFFFF) as u32;
            if w > 0 && h > 0 {
                push!(Event::Resized {
                    width: w,
                    height: h
                });
            }
            LRESULT(0)
        }

        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            unsafe { BeginPaint(hwnd, &mut ps) };
            push!(Event::RedrawRequested);
            unsafe { EndPaint(hwnd, &ps) };
            LRESULT(0)
        }

        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = VIRTUAL_KEY(wparam.0 as u16);
            push!(Event::KeyDown(vk_to_key(vk)));
            LRESULT(0)
        }

        WM_KEYUP | WM_SYSKEYUP => {
            let vk = VIRTUAL_KEY(wparam.0 as u16);
            push!(Event::KeyUp(vk_to_key(vk)));
            LRESULT(0)
        }

        WM_CHAR => {
            let codepoint = wparam.0 as u32;
            if codepoint >= 0x20 {
                if let Some(ch) = char::from_u32(codepoint) {
                    push!(Event::CharInput(ch));
                }
            }
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseMove { x, y });
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Left,
                x,
                y
            });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Left,
                x,
                y
            });
            LRESULT(0)
        }

        WM_RBUTTONDOWN => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Right,
                x,
                y
            });
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Right,
                x,
                y
            });
            LRESULT(0)
        }

        WM_MBUTTONDOWN => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Middle,
                x,
                y
            });
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Middle,
                x,
                y
            });
            LRESULT(0)
        }

        WM_MOUSEWHEEL => {
            let raw = ((wparam.0 >> 16) & 0xFFFF) as i16;
            let delta = raw as f32 / WHEEL_DELTA as f32;
            push!(Event::MouseWheel { delta });
            LRESULT(0)
        }

        WM_SETFOCUS => {
            push!(Event::FocusGained);
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            push!(Event::FocusLost);
            LRESULT(0)
        }

        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        unsafe {
            let hinstance: HINSTANCE = GetModuleHandleW(None)
                .map_err(|e| Error::Platform(e.to_string()))?
                .into();

            let class_name = to_wide("windowed_wc\0");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance,
                hCursor: LoadCursorW(None, IDC_ARROW)
                    .map_err(|e| Error::Platform(e.to_string()))?,
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&wc);

            let style = if config.resizable {
                WS_OVERLAPPEDWINDOW
            } else {
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX
            };

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: config.width as i32,
                bottom: config.height as i32,
            };
            AdjustWindowRect(&mut rect, style, false)
                .map_err(|e| Error::Platform(e.to_string()))?;

            let (cx, cy) = config
                .position
                .map(|(x, y)| (x, y))
                .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

            let title = to_wide(&format!("{}\0", config.title));
            let state = Arc::new(Mutex::new(WindowState { events: Vec::new() }));

            let hwnd = CreateWindowExW(
                Default::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                cx,
                cy,
                rect.right - rect.left,
                rect.bottom - rect.top,
                None,
                None,
                hinstance,
                None,
            )
            .map_err(|e| Error::Platform(e.to_string()))?;

            let state_ptr = Arc::as_ptr(&state) as isize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr);

            ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            Ok(Self {
                hwnd,
                hinstance,
                state,
                width: config.width,
                height: config.height,
            })
        }
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, mut callback: F) -> Result<()> {
        unsafe {
            let mut msg = MSG::default();
            let mut polling = false;

            loop {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                        return Ok(());
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let events: Vec<Event> = {
                    let mut s = self.state.lock().unwrap();
                    std::mem::take(&mut s.events)
                };

                let mut want_poll = false;
                for event in events {
                    if let Event::Resized { width, height } = event {
                        self.width = width;
                        self.height = height;
                    }
                    match callback(event) {
                        ControlFlow::Exit => return Ok(()),
                        ControlFlow::Poll => want_poll = true,
                        ControlFlow::WarpAndPoll(x, y) => {
                            want_poll = true;
                            self.warp_mouse(x, y);
                        }
                        ControlFlow::Continue => {}
                    }
                }
                if want_poll {
                    polling = true;
                }

                if polling {
                    self.request_redraw();
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    pub fn request_redraw(&self) {
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        unsafe {
            if visible {
                while ShowCursor(true) < 0 {}
            } else {
                while ShowCursor(false) >= 0 {}
            }
        }
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        unsafe {
            let mut wr = RECT::default();
            let mut cr = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut wr);
            let _ = GetClientRect(self.hwnd, &mut cr);
            let border_x = (wr.right - wr.left - cr.right) / 2;
            let top_inset = wr.bottom - wr.top - cr.bottom - border_x;
            let _ = SetCursorPos(wr.left + border_x + x, wr.top + top_inset + y);
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let t = to_wide(&format!("{title}\0"));
            let _ = SetWindowTextW(self.hwnd, PCWSTR(t.as_ptr()));
        }
    }

    pub fn inner_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for PlatformWindow {
    fn drop(&mut self) {
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        use std::num::NonZeroIsize;
        let hwnd_val = NonZeroIsize::new(self.hwnd.0 as isize).ok_or(HandleError::Unavailable)?;
        let mut handle = Win32WindowHandle::new(hwnd_val);
        handle.hinstance = NonZeroIsize::new(self.hinstance.0 as isize);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

impl HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn vk_to_key(vk: VIRTUAL_KEY) -> Key {
    match vk {
        VK_A => Key::A,
        VK_B => Key::B,
        VK_C => Key::C,
        VK_D => Key::D,
        VK_E => Key::E,
        VK_F => Key::F,
        VK_G => Key::G,
        VK_H => Key::H,
        VK_I => Key::I,
        VK_J => Key::J,
        VK_K => Key::K,
        VK_L => Key::L,
        VK_M => Key::M,
        VK_N => Key::N,
        VK_O => Key::O,
        VK_P => Key::P,
        VK_Q => Key::Q,
        VK_R => Key::R,
        VK_S => Key::S,
        VK_T => Key::T,
        VK_U => Key::U,
        VK_V => Key::V,
        VK_W => Key::W,
        VK_X => Key::X,
        VK_Y => Key::Y,
        VK_Z => Key::Z,

        VIRTUAL_KEY(0x30) => Key::Num0,
        VIRTUAL_KEY(0x31) => Key::Num1,
        VIRTUAL_KEY(0x32) => Key::Num2,
        VIRTUAL_KEY(0x33) => Key::Num3,
        VIRTUAL_KEY(0x34) => Key::Num4,
        VIRTUAL_KEY(0x35) => Key::Num5,
        VIRTUAL_KEY(0x36) => Key::Num6,
        VIRTUAL_KEY(0x37) => Key::Num7,
        VIRTUAL_KEY(0x38) => Key::Num8,
        VIRTUAL_KEY(0x39) => Key::Num9,

        VK_NUMPAD0 => Key::Num0,
        VK_NUMPAD1 => Key::Num1,
        VK_NUMPAD2 => Key::Num2,
        VK_NUMPAD3 => Key::Num3,
        VK_NUMPAD4 => Key::Num4,
        VK_NUMPAD5 => Key::Num5,
        VK_NUMPAD6 => Key::Num6,
        VK_NUMPAD7 => Key::Num7,
        VK_NUMPAD8 => Key::Num8,
        VK_NUMPAD9 => Key::Num9,

        VK_F1 => Key::F1,
        VK_F2 => Key::F2,
        VK_F3 => Key::F3,
        VK_F4 => Key::F4,
        VK_F5 => Key::F5,
        VK_F6 => Key::F6,
        VK_F7 => Key::F7,
        VK_F8 => Key::F8,
        VK_F9 => Key::F9,
        VK_F10 => Key::F10,
        VK_F11 => Key::F11,
        VK_F12 => Key::F12,

        VK_RETURN => Key::Enter,
        VK_ESCAPE => Key::Escape,
        VK_SPACE => Key::Space,
        VK_BACK => Key::Backspace,
        VK_TAB => Key::Tab,
        VK_DELETE => Key::Delete,
        VK_INSERT => Key::Insert,

        VK_LEFT => Key::ArrowLeft,
        VK_RIGHT => Key::ArrowRight,
        VK_UP => Key::ArrowUp,
        VK_DOWN => Key::ArrowDown,

        VK_HOME => Key::Home,
        VK_END => Key::End,
        VK_PRIOR => Key::PageUp,
        VK_NEXT => Key::PageDown,

        VK_LSHIFT => Key::LeftShift,
        VK_RSHIFT => Key::RightShift,
        VK_LCONTROL => Key::LeftCtrl,
        VK_RCONTROL => Key::RightCtrl,
        VK_LMENU => Key::LeftAlt,
        VK_RMENU => Key::RightAlt,

        _ => Key::Unknown,
    }
}
