//! Windows Win32 platform backend.
#![allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::upper_case_acronyms
)]

use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use crate::config::WindowConfig;
use crate::error::{Error, Result};
use crate::event::{ControlFlow, Event, Key, MouseButton};
use crate::rwd::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

type HANDLE = isize;
type HWND = isize;
type HINSTANCE = isize;
type HBRUSH = isize;
type HCURSOR = isize;
type HMENU = isize;
type BOOL = i32;
type DWORD = u32;
type UINT = u32;
type INT = i32;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type LONG = i32;
type LONG_PTR = isize;

#[repr(C)]
struct POINT {
    x: LONG,
    y: LONG,
}
#[repr(C)]
struct RECT {
    left: LONG,
    top: LONG,
    right: LONG,
    bottom: LONG,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
struct WNDCLASSEXW {
    cbSize: UINT,
    style: UINT,
    lpfnWndProc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
    cbClsExtra: INT,
    cbWndExtra: INT,
    hInstance: HINSTANCE,
    hIcon: HANDLE,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: *const u16,
    lpszClassName: *const u16,
    hIconSm: HANDLE,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HANDLE,
    fErase: BOOL,
    rcPaint: RECT,
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [u8; 32],
}

const WM_DESTROY: UINT = 0x0002;
const WM_SIZE: UINT = 0x0005;
const WM_SETFOCUS: UINT = 0x0007;
const WM_KILLFOCUS: UINT = 0x0008;
const WM_CLOSE: UINT = 0x0010;
const WM_QUIT: UINT = 0x0012;
const WM_PAINT: UINT = 0x000F;
const WM_KEYDOWN: UINT = 0x0100;
const WM_KEYUP: UINT = 0x0101;
const WM_CHAR: UINT = 0x0102;
const WM_SYSKEYDOWN: UINT = 0x0104;
const WM_SYSKEYUP: UINT = 0x0105;
const WM_MOUSEMOVE: UINT = 0x0200;
const WM_LBUTTONDOWN: UINT = 0x0201;
const WM_LBUTTONUP: UINT = 0x0202;
const WM_RBUTTONDOWN: UINT = 0x0204;
const WM_RBUTTONUP: UINT = 0x0205;
const WM_MBUTTONDOWN: UINT = 0x0207;
const WM_MBUTTONUP: UINT = 0x0208;
const WM_MOUSEWHEEL: UINT = 0x020A;
const WM_MOUSELEAVE: UINT = 0x02A3;
const WM_MOUSEHOVER: UINT = 0x02A1;

const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;
const CS_OWNDC: UINT = 0x0020;

const WS_OVERLAPPED: DWORD = 0x0000_0000;
const WS_CAPTION: DWORD = 0x00C0_0000;
const WS_SYSMENU: DWORD = 0x0008_0000;
const WS_THICKFRAME: DWORD = 0x0004_0000;
const WS_MINIMIZEBOX: DWORD = 0x0002_0000;
const WS_MAXIMIZEBOX: DWORD = 0x0001_0000;
const WS_OVERLAPPEDWINDOW: DWORD =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

const CW_USEDEFAULT: INT = 0x8000_0000u32 as INT;
const SW_SHOW: INT = 5;
const PM_REMOVE: UINT = 0x0001;
const GWLP_USERDATA: INT = -21;
const GWL_STYLE: INT = -16;
const WHITE_BRUSH: INT = 0;
const WHEEL_DELTA: i32 = 120;
const WS_POPUP: DWORD = 0x8000_0000;
const WS_VISIBLE: DWORD = 0x1000_0000;
const MONITOR_DEFAULTTONEAREST: DWORD = 2;
const SWP_NOSIZE: UINT = 0x0001;
const SWP_NOMOVE: UINT = 0x0002;
const SWP_NOZORDER: UINT = 0x0004;
const SWP_NOOWNERZORDER: UINT = 0x0200;
const SWP_FRAMECHANGED: UINT = 0x0020;
const HWND_TOP: HWND = 0;

const IDC_ARROW: *const u16 = 32512usize as *const u16;

const VK_BACK: u16 = 0x08;
const VK_TAB: u16 = 0x09;
const VK_RETURN: u16 = 0x0D;
const VK_ESCAPE: u16 = 0x1B;
const VK_SPACE: u16 = 0x20;
const VK_PRIOR: u16 = 0x21;
const VK_NEXT: u16 = 0x22;
const VK_END: u16 = 0x23;
const VK_HOME: u16 = 0x24;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
const VK_INSERT: u16 = 0x2D;
const VK_DELETE: u16 = 0x2E;
const VK_LSHIFT: u16 = 0xA0;
const VK_RSHIFT: u16 = 0xA1;
const VK_LCONTROL: u16 = 0xA2;
const VK_RCONTROL: u16 = 0xA3;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;
const VK_LWIN: u16 = 0x5B;
const VK_RWIN: u16 = 0x5C;

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: DWORD,
        X: INT,
        Y: INT,
        nWidth: INT,
        nHeight: INT,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    fn DestroyWindow(hWnd: HWND) -> BOOL;
    fn ShowWindow(hWnd: HWND, nCmdShow: INT) -> BOOL;
    fn UpdateWindow(hWnd: HWND) -> BOOL;
    fn PeekMessageW(
        lpMsg: *mut MSG,
        hWnd: HWND,
        wMsgFilterMin: UINT,
        wMsgFilterMax: UINT,
        wRemoveMsg: UINT,
    ) -> BOOL;
    fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    fn PostQuitMessage(nExitCode: INT);
    fn GetWindowLongPtrW(hWnd: HWND, nIndex: INT) -> LONG_PTR;
    fn SetWindowLongPtrW(hWnd: HWND, nIndex: INT, dwNewLong: LONG_PTR) -> LONG_PTR;
    fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    fn AdjustWindowRect(lpRect: *mut RECT, dwStyle: DWORD, bMenu: BOOL) -> BOOL;
    fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    fn SetCursorPos(X: INT, Y: INT) -> BOOL;
    fn ShowCursor(bShow: BOOL) -> INT;
    fn SetCapture(hWnd: HWND) -> HWND;
    fn ReleaseCapture() -> BOOL;
    fn ClipCursor(lpRect: *const RECT) -> BOOL;
    fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HANDLE;
    fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    fn TrackMouseEvent(lpEventTrack: *mut TRACKMOUSEEVENT) -> BOOL;
    fn MonitorFromWindow(hwnd: HWND, dwFlags: DWORD) -> HMONITOR;
    fn GetMonitorInfoW(hMonitor: HMONITOR, lpmi: *mut MONITORINFO) -> BOOL;
    fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: INT,
        Y: INT,
        cx: INT,
        cy: INT,
        uFlags: UINT,
    ) -> BOOL;
    fn GetWindowPlacement(hWnd: HWND, lpwndpl: *mut WINDOWPLACEMENT) -> BOOL;
    fn SetWindowPlacement(hWnd: HWND, lpwndpl: *const WINDOWPLACEMENT) -> BOOL;
    fn GetWindowLongW(hWnd: HWND, nIndex: INT) -> LONG;
    fn SetWindowLongW(hWnd: HWND, nIndex: INT, dwNewLong: LONG) -> LONG;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetStockObject(fnObject: INT) -> HANDLE;
}

type HMONITOR = isize;

#[repr(C)]
#[derive(Copy, Clone)]
struct WINDOWPLACEMENT {
    length: UINT,
    flags: UINT,
    showCmd: UINT,
    ptMinPosition: POINT,
    ptMaxPosition: POINT,
    rcNormalPosition: RECT,
}

#[repr(C)]
struct MONITORINFO {
    cbSize: DWORD,
    rcMonitor: RECT,
    rcWork: RECT,
    dwFlags: DWORD,
}

#[repr(C)]
struct TRACKMOUSEEVENT {
    cbSize: DWORD,
    dwFlags: DWORD,
    hwndTrack: HWND,
    dwHoverTime: DWORD,
}

const TME_LEAVE: DWORD = 0x0000_0002;
const TME_HOVER: DWORD = 0x0000_0001;

struct WindowState {
    events: Vec<Event>,
    tracking_mouse: bool,
}

pub struct PlatformWindow {
    hwnd: HWND,
    hinstance: HINSTANCE,
    state: Arc<Mutex<WindowState>>,
    width: u32,
    height: u32,
    fullscreen: bool,
    mouse_grabbed: bool,
    saved_style: DWORD,
    saved_placement: WINDOWPLACEMENT,
}

unsafe impl Send for PlatformWindow {}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<WindowState> };
    if state_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let state = unsafe { &*state_ptr };

    macro_rules! push {
        ($ev:expr) => {
            if let Ok(mut s) = state.lock() {
                s.events.push($ev);
            }
        };
    }
    macro_rules! mouse_xy {
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
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }

        WM_SIZE => {
            let w = (lparam & 0xFFFF) as u32;
            let h = ((lparam >> 16) & 0xFFFF) as u32;
            if w > 0 && h > 0 {
                push!(Event::Resized {
                    width: w,
                    height: h
                });
            }
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
            // Bit 30 of lParam: 1 if key was already down before this message (auto-repeat).
            let repeat = (lparam >> 30) & 1 != 0;
            push!(Event::KeyDown {
                key: vk_to_key(wparam as u16),
                repeat,
            });
            0
        }
        WM_KEYUP | WM_SYSKEYUP => {
            push!(Event::KeyUp(vk_to_key(wparam as u16)));
            0
        }

        WM_CHAR => {
            let cp = wparam as u32;
            if cp >= 0x20 {
                if let Some(ch) = char::from_u32(cp) {
                    push!(Event::CharInput(ch));
                }
            }
            0
        }

        WM_MOUSEMOVE => {
            if let Ok(mut s) = state.lock() {
                if !s.tracking_mouse {
                    s.tracking_mouse = true;
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as DWORD,
                        dwFlags: TME_LEAVE | TME_HOVER,
                        hwndTrack: hwnd,
                        dwHoverTime: 1,
                    };
                    unsafe { TrackMouseEvent(&mut tme) };
                    s.events.push(Event::CursorEntered);
                }
            }
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseMove { x, y });
            0
        }
        WM_MOUSELEAVE => {
            if let Ok(mut s) = state.lock() {
                s.tracking_mouse = false;
                s.events.push(Event::CursorLeft);
            }
            0
        }

        WM_LBUTTONDOWN => {
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Left,
                x,
                y
            });
            0
        }
        WM_LBUTTONUP => {
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Left,
                x,
                y
            });
            0
        }
        WM_RBUTTONDOWN => {
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Right,
                x,
                y
            });
            0
        }
        WM_RBUTTONUP => {
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Right,
                x,
                y
            });
            0
        }
        WM_MBUTTONDOWN => {
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseDown {
                button: MouseButton::Middle,
                x,
                y
            });
            0
        }
        WM_MBUTTONUP => {
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseUp {
                button: MouseButton::Middle,
                x,
                y
            });
            0
        }

        WM_MOUSEWHEEL => {
            let raw = ((wparam >> 16) & 0xFFFF) as i16;
            push!(Event::MouseWheel {
                delta: raw as f32 / WHEEL_DELTA as f32
            });
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

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        unsafe {
            let hinstance = GetModuleHandleW(std::ptr::null());
            if hinstance == 0 {
                return Err(Error::Platform("GetModuleHandleW failed".into()));
            }

            let class_name = to_wide("windowed_wc\0");
            let hcursor = LoadCursorW(0, IDC_ARROW);
            let hbrush = GetStockObject(WHITE_BRUSH);

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: 0,
                hCursor: hcursor,
                hbrBackground: hbrush,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: 0,
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
                right: config.width as LONG,
                bottom: config.height as LONG,
            };
            AdjustWindowRect(&mut rect, style, 0);

            let (cx, cy) = config
                .position
                .map(|(x, y)| (x, y))
                .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

            let title = to_wide(&format!("{}\0", config.title));
            let state = Arc::new(Mutex::new(WindowState {
                events: Vec::new(),
                tracking_mouse: false,
            }));

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                style,
                cx,
                cy,
                rect.right - rect.left,
                rect.bottom - rect.top,
                0,
                0,
                hinstance,
                std::ptr::null_mut(),
            );
            if hwnd == 0 {
                return Err(Error::Platform("CreateWindowExW failed".into()));
            }

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::as_ptr(&state) as LONG_PTR);
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            Ok(Self {
                hwnd,
                hinstance,
                state,
                width: config.width,
                height: config.height,
                fullscreen: false,
                mouse_grabbed: false,
                saved_style: 0,
                saved_placement: unsafe { std::mem::zeroed() },
            })
        }
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, mut callback: F) -> Result<()> {
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        let mut polling = false;

        loop {
            unsafe {
                while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        return Ok(());
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
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

    pub fn request_redraw(&self) {
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        unsafe {
            if visible {
                while ShowCursor(1) < 0 {}
            } else {
                while ShowCursor(0) >= 0 {}
            }
        }
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        unsafe {
            let mut wr: RECT = std::mem::zeroed();
            let mut cr: RECT = std::mem::zeroed();
            GetWindowRect(self.hwnd, &mut wr);
            GetClientRect(self.hwnd, &mut cr);
            let border_x = (wr.right - wr.left - cr.right) / 2;
            let top_inset = wr.bottom - wr.top - cr.bottom - border_x;
            SetCursorPos(wr.left + border_x + x, wr.top + top_inset + y);
        }
    }

    pub fn set_title(&self, title: &str) {
        let t = to_wide(&format!("{title}\0"));
        unsafe {
            SetWindowTextW(self.hwnd, t.as_ptr());
        }
    }

    pub fn set_fullscreen(&mut self, is_fullscreen: bool) {
        if self.fullscreen == is_fullscreen {
            return;
        }
        self.fullscreen = is_fullscreen;
        unsafe {
            if is_fullscreen {
                // Save current style and window placement so we can restore later.
                self.saved_style = GetWindowLongW(self.hwnd, GWL_STYLE) as DWORD;
                self.saved_placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as UINT;
                GetWindowPlacement(self.hwnd, &mut self.saved_placement);

                // Get the bounds of the monitor the window is currently on.
                let monitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi: MONITORINFO = std::mem::zeroed();
                mi.cbSize = std::mem::size_of::<MONITORINFO>() as DWORD;
                GetMonitorInfoW(monitor, &mut mi);
                let r = mi.rcMonitor;

                // Strip all chrome and cover the full monitor rectangle.
                SetWindowLongW(self.hwnd, GWL_STYLE, (WS_POPUP | WS_VISIBLE) as LONG);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOP,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                );
            } else {
                // Restore the previous style and placement.
                SetWindowLongW(self.hwnd, GWL_STYLE, self.saved_style as LONG);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                );
                SetWindowPlacement(self.hwnd, &self.saved_placement);
            }
        }
    }

    pub fn set_mouse_grabbed(&mut self, grabbed: bool) {
        if self.mouse_grabbed == grabbed {
            return;
        }
        self.mouse_grabbed = grabbed;
        unsafe {
            if grabbed {
                SetCapture(self.hwnd);
                // Confine the cursor to the client area.
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(self.hwnd, &mut rect);
                let mut tl = POINT {
                    x: rect.left,
                    y: rect.top,
                };
                let mut br = POINT {
                    x: rect.right,
                    y: rect.bottom,
                };
                ClientToScreen(self.hwnd, &mut tl);
                ClientToScreen(self.hwnd, &mut br);
                let screen_rect = RECT {
                    left: tl.x,
                    top: tl.y,
                    right: br.x,
                    bottom: br.y,
                };
                ClipCursor(&screen_rect);
            } else {
                ReleaseCapture();
                ClipCursor(std::ptr::null());
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
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            DestroyWindow(self.hwnd);
        }
    }
}

impl HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        use std::num::NonZeroIsize;
        let hwnd_val = NonZeroIsize::new(self.hwnd).ok_or(HandleError::Unavailable)?;
        let mut h = Win32WindowHandle::new(hwnd_val);
        h.hinstance = NonZeroIsize::new(self.hinstance);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(h)) })
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

fn vk_to_key(vk: u16) -> Key {
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
        0x60 => Key::Num0,
        0x61 => Key::Num1,
        0x62 => Key::Num2,
        0x63 => Key::Num3,
        0x64 => Key::Num4,
        0x65 => Key::Num5,
        0x66 => Key::Num6,
        0x67 => Key::Num7,
        0x68 => Key::Num8,
        0x69 => Key::Num9,

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
        VK_LWIN => Key::LeftSuper,
        VK_RWIN => Key::RightSuper,
        _ => Key::Unknown,
    }
}
