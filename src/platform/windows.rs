use std::sync::{Arc, Mutex};

use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::*;

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

extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
                ($lp & 0xFFFF) as i16 as i32,
                (($lp >> 16) & 0xFFFF) as i16 as i32,
            )
        };
    }

    match msg {
        WM_CLOSE => {
            push!(Event::CloseRequested);
            0
        }
        WM_SIZE => {
            let w = (lparam & 0xFFFF) as u32;
            let h = ((lparam >> 16) & 0xFFFF) as u32;
            push!(Event::Resized {
                width: w,
                height: h
            });
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
            unsafe { BeginPaint(hwnd, &mut ps) };
            push!(Event::RedrawRequested);
            unsafe { EndPaint(hwnd, &ps) };
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            push!(Event::KeyDown(vk_to_key(wparam as u32)));
            0
        }
        WM_KEYUP | WM_SYSKEYUP => {
            push!(Event::KeyUp(vk_to_key(wparam as u32)));
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseMove { x, y });
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Left,
                x,
                y
            });
            0
        }
        WM_LBUTTONUP => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Left,
                x,
                y
            });
            0
        }
        WM_RBUTTONDOWN => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Right,
                x,
                y
            });
            0
        }
        WM_RBUTTONUP => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Right,
                x,
                y
            });
            0
        }
        WM_MBUTTONDOWN => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Middle,
                x,
                y
            });
            0
        }
        WM_MBUTTONUP => {
            let (x, y) = mouse_pos!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Middle,
                x,
                y
            });
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) as i16) as f32 / WHEEL_DELTA as f32;
            push!(Event::MouseWheel { delta });
            0
        }
        WM_SETFOCUS => {
            push!(Event::FocusGained);
            0
        }
        WM_KILLFOCUS => {
            push!(Event::FocusLost);
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            if hinstance.is_null() {
                return Err(Error::Platform("GetModuleHandleW failed".into()));
            }

            let class_name = wide("windowed_wc\0");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance,
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
                lpszClassName: class_name.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassExW(&wc);

            let style: DWORD = if config.resizable {
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
            AdjustWindowRect(&mut rect, style, FALSE);

            let title = wide(&format!("{}\0", config.title));
            let state = Arc::new(Mutex::new(WindowState { events: Vec::new() }));

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );

            if hwnd.is_null() {
                return Err(Error::Platform("CreateWindowExW failed".into()));
            }

            let state_ptr = Arc::as_ptr(&state) as isize;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr);

            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

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
            let mut msg: MSG = std::mem::zeroed();
            let mut polling = false;

            loop {
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        return Ok(());
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                let events: Vec<Event> = {
                    let mut s = self.state.lock().unwrap();
                    std::mem::take(&mut s.events)
                };

                let mut any_poll = false;
                for event in events {
                    if let Event::Resized { width, height } = event {
                        self.width = width;
                        self.height = height;
                    }
                    match callback(event) {
                        ControlFlow::Exit => return Ok(()),
                        ControlFlow::Poll => any_poll = true,
                        ControlFlow::WarpAndPoll(x, y) => {
                            any_poll = true;
                            self.warp_mouse(x, y);
                        }
                        ControlFlow::Continue => {}
                    }
                }
                if any_poll {
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
            InvalidateRect(self.hwnd, std::ptr::null(), FALSE);
        }
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        unsafe {
            if visible {
                while winapi::um::winuser::ShowCursor(1) < 0 {}
            } else {
                while winapi::um::winuser::ShowCursor(0) >= 0 {}
            }
        }
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        unsafe {
            let mut pt = winapi::shared::windef::POINT { x, y };
            winapi::um::winuser::ClientToScreen(self.hwnd, &mut pt);
            winapi::um::winuser::SetCursorPos(pt.x, pt.y);
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let t = wide(&format!("{title}\0"));
            SetWindowTextW(self.hwnd, t.as_ptr());
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
            DestroyWindow(self.hwnd);
        }
    }
}

impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        use std::num::NonZeroIsize;
        let hwnd_val = NonZeroIsize::new(self.hwnd as isize).ok_or(HandleError::Unavailable)?;
        let mut handle = Win32WindowHandle::new(hwnd_val);
        handle.hinstance = NonZeroIsize::new(self.hinstance as isize);
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

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn vk_to_key(vk: u32) -> Key {
    match vk {
        0x41 => Key::A,
        0x42 => Key::B,
        0x43 => Key::C,
        0x44 => Key::D,
        0x45 => Key::E,
        0x46 => Key::F,
        0x47 => Key::G,
        0x48 => Key::H,
        0x49 => Key::I,
        0x4A => Key::J,
        0x4B => Key::K,
        0x4C => Key::L,
        0x4D => Key::M,
        0x4E => Key::N,
        0x4F => Key::O,
        0x50 => Key::P,
        0x51 => Key::Q,
        0x52 => Key::R,
        0x53 => Key::S,
        0x54 => Key::T,
        0x55 => Key::U,
        0x56 => Key::V,
        0x57 => Key::W,
        0x58 => Key::X,
        0x59 => Key::Y,
        0x5A => Key::Z,

        0x30 => Key::Num0,
        0x31 => Key::Num1,
        0x32 => Key::Num2,
        0x33 => Key::Num3,
        0x34 => Key::Num4,
        0x35 => Key::Num5,
        0x36 => Key::Num6,
        0x37 => Key::Num7,
        0x38 => Key::Num8,
        0x39 => Key::Num9,

        0x70 => Key::F1,
        0x71 => Key::F2,
        0x72 => Key::F3,
        0x73 => Key::F4,
        0x74 => Key::F5,
        0x75 => Key::F6,
        0x76 => Key::F7,
        0x77 => Key::F8,
        0x78 => Key::F9,
        0x79 => Key::F10,
        0x7A => Key::F11,
        0x7B => Key::F12,

        _ => Key::Unknown,
    }
}
