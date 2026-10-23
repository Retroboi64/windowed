//! Windows Win32 platform backend.
#![allow(
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::upper_case_acronyms
)]

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::WindowConfig;
use crate::error::{Error, Result};
use crate::event::{ControlFlow, Event, Key, Modifiers, MouseButton};
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
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
struct POINT {
    x: LONG,
    y: LONG,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

#[repr(C)]
struct MINMAXINFO {
    ptReserved: POINT,
    ptMaxSize: POINT,
    ptMaxPosition: POINT,
    ptMinTrackSize: POINT,
    ptMaxTrackSize: POINT,
}

const WM_DESTROY: UINT = 0x0002;
const WM_MOVE: UINT = 0x0003;
const WM_SIZE: UINT = 0x0005;
const WM_SETFOCUS: UINT = 0x0007;
const WM_KILLFOCUS: UINT = 0x0008;
const WM_CLOSE: UINT = 0x0010;
const WM_QUIT: UINT = 0x0012;
const WM_PAINT: UINT = 0x000F;
const WM_GETMINMAXINFO: UINT = 0x0024;
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
const WM_DROPFILES: UINT = 0x0233;
const WM_DPICHANGED: UINT = 0x02E0;
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

const WS_EX_TOPMOST: DWORD = 0x0000_0008;
const WS_EX_LAYERED: DWORD = 0x0008_0000;

const CW_USEDEFAULT: INT = 0x8000_0000u32 as INT;
const SW_HIDE: INT = 0;
const SW_MAXIMIZE: INT = 3;
const SW_SHOW: INT = 5;
const SW_MINIMIZE: INT = 6;
const SW_RESTORE: INT = 9;

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
const SWP_NOACTIVATE: UINT = 0x0010;
const SWP_FRAMECHANGED: UINT = 0x0020;
const SWP_NOOWNERZORDER: UINT = 0x0200;
const HWND_TOP: HWND = 0;

const SIZE_RESTORED: WPARAM = 0;
const SIZE_MINIMIZED: WPARAM = 1;
const SIZE_MAXIMIZED: WPARAM = 2;

const IDC_ARROW: *const u16 = 32512usize as *const u16;

const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
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
    fn GetKeyState(nVirtKey: INT) -> i16;
    fn GetDpiForWindow(hwnd: HWND) -> UINT;
    fn SetProcessDpiAwarenessContext(value: isize) -> BOOL;
    fn WaitMessage() -> BOOL;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn DragAcceptFiles(hWnd: HWND, fAccept: BOOL);
    fn DragQueryFileW(hDrop: HANDLE, iFile: UINT, lpszFile: *mut u16, cch: UINT) -> UINT;
    fn DragFinish(hDrop: HANDLE);
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    fn GetLastError() -> DWORD;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetStockObject(fnObject: INT) -> HANDLE;
}

type HMONITOR = isize;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
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
    is_minimized: bool,
    min_size: Option<(u32, u32)>,
    max_size: Option<(u32, u32)>,
}

#[derive(Default)]
struct FullscreenState {
    fullscreen: bool,
    saved_style: DWORD,
    saved_placement: WINDOWPLACEMENT,
}

pub struct PlatformWindow {
    hwnd: HWND,
    hinstance: HINSTANCE,
    state: Arc<Mutex<WindowState>>,
    fullscreen_state: Mutex<FullscreenState>,
    mouse_grabbed: AtomicBool,
    width: AtomicU32,
    height: AtomicU32,
}

unsafe impl Send for PlatformWindow {}
unsafe impl Sync for PlatformWindow {}

unsafe fn current_modifiers() -> Modifiers {
    // SAFETY: GetKeyState queries current key states synchronously.
    unsafe {
        Modifiers {
            shift: (GetKeyState(VK_SHIFT) as u16 & 0x8000) != 0,
            ctrl: (GetKeyState(VK_CONTROL) as u16 & 0x8000) != 0,
            alt: (GetKeyState(VK_MENU) as u16 & 0x8000) != 0,
            super_key: ((GetKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0)
                || ((GetKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0),
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: GetWindowLongPtrW retrieves the pointer to Arc<Mutex<WindowState>> stored
    // during window creation. If null or during teardown, DefWindowProcW is called safely.
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<WindowState> };
    if state_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: state_ptr points to the Arc<Mutex<WindowState>> which outlives the HWND.
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
            // SAFETY: PostQuitMessage instructs the message loop to quit.
            unsafe { PostQuitMessage(0) };
            0
        }

        WM_MOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            push!(Event::Moved { x, y });
            0
        }

        WM_SIZE => {
            let w = (lparam & 0xFFFF) as u32;
            let h = ((lparam >> 16) & 0xFFFF) as u32;

            if let Ok(mut s) = state.lock() {
                if wparam == SIZE_MINIMIZED {
                    if !s.is_minimized {
                        s.is_minimized = true;
                        s.events.push(Event::Minimized);
                    }
                } else if (wparam == SIZE_RESTORED || wparam == SIZE_MAXIMIZED) && s.is_minimized {
                    s.is_minimized = false;
                    s.events.push(Event::Restored);
                }
                if w > 0 && h > 0 {
                    s.events.push(Event::Resized {
                        width: w,
                        height: h,
                    });
                }
            }
            0
        }

        WM_GETMINMAXINFO => {
            if let Ok(s) = state.lock() {
                let mmi = lparam as *mut MINMAXINFO;
                if !mmi.is_null() {
                    // SAFETY: Pointer is valid as provided by Windows message.
                    unsafe {
                        if let Some((min_w, min_h)) = s.min_size {
                            (*mmi).ptMinTrackSize.x = min_w as LONG;
                            (*mmi).ptMinTrackSize.y = min_h as LONG;
                        }
                        if let Some((max_w, max_h)) = s.max_size {
                            (*mmi).ptMaxTrackSize.x = max_w as LONG;
                            (*mmi).ptMaxTrackSize.y = max_h as LONG;
                        }
                    }
                }
            }
            0
        }

        WM_PAINT => {
            // SAFETY: Standard Win32 BeginPaint/EndPaint pair.
            unsafe {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                BeginPaint(hwnd, &mut ps);
                push!(Event::RedrawRequested);
                EndPaint(hwnd, &ps);
            }
            0
        }

        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let repeat = (lparam >> 30) & 1 != 0;
            let modifiers = unsafe { current_modifiers() };
            push!(Event::KeyDown {
                key: vk_to_key(wparam as u16),
                repeat,
                modifiers,
            });
            0
        }
        WM_KEYUP | WM_SYSKEYUP => {
            let modifiers = unsafe { current_modifiers() };
            push!(Event::KeyUp {
                key: vk_to_key(wparam as u16),
                modifiers,
            });
            0
        }

        WM_CHAR => {
            let cp = wparam as u32;
            if cp >= 0x20
                && let Some(ch) = char::from_u32(cp) {
                push!(Event::CharInput(ch));
            }
            0
        }

        WM_MOUSEMOVE => {
            if let Ok(mut s) = state.lock()
                && !s.tracking_mouse {
                s.tracking_mouse = true;
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as DWORD,
                    dwFlags: TME_LEAVE | TME_HOVER,
                    hwndTrack: hwnd,
                    dwHoverTime: 1,
                };
                // SAFETY: TrackMouseEvent request is passed a valid pointer to TRACKMOUSEEVENT.
                unsafe { TrackMouseEvent(&mut tme) };
                s.events.push(Event::MouseEnter);
                #[allow(deprecated)]
                s.events.push(Event::CursorEntered);
            }
            let (x, y) = mouse_xy!(lparam);
            push!(Event::MouseMove { x, y });
            0
        }
        WM_MOUSELEAVE => {
            if let Ok(mut s) = state.lock() {
                s.tracking_mouse = false;
                s.events.push(Event::MouseLeave);
                #[allow(deprecated)]
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

        WM_DROPFILES => {
            let hdrop = wparam as HANDLE;
            // SAFETY: DragQueryFileW retrieves files dropped onto the window.
            unsafe {
                let count = DragQueryFileW(hdrop, 0xFFFFFFFF, std::ptr::null_mut(), 0);
                for i in 0..count {
                    let len = DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0);
                    if len > 0 {
                        let mut buf = vec![0u16; len as usize + 1];
                        DragQueryFileW(hdrop, i, buf.as_mut_ptr(), buf.len() as UINT);
                        let path_str = String::from_utf16_lossy(&buf[..len as usize]);
                        push!(Event::DroppedFile(PathBuf::from(path_str)));
                    }
                }
                DragFinish(hdrop);
            }
            0
        }

        WM_DPICHANGED => {
            let dpi = (wparam & 0xFFFF) as u32;
            let scale = dpi as f64 / 96.0;
            push!(Event::ScaleFactorChanged {
                scale_factor: scale
            });
            // Update window position and size suggested by Windows in lParam RECT.
            let rect_ptr = lparam as *const RECT;
            if !rect_ptr.is_null() {
                // SAFETY: rect_ptr is valid pointer to suggested window RECT.
                unsafe {
                    let r = &*rect_ptr;
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
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

        // SAFETY: DefWindowProcW handles all unhandled window messages.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

impl PlatformWindow {
    pub fn new(config: &WindowConfig) -> Result<Self> {
        // Initialize DPI awareness if possible (Per-Monitor V2 awareness = -4).
        // SAFETY: SetProcessDpiAwarenessContext is supported on Windows 10+.
        unsafe {
            let _ = SetProcessDpiAwarenessContext(-4);
        }

        // SAFETY: Retrieving module handle for current process.
        let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };
        if hinstance == 0 {
            return Err(Error::Platform("GetModuleHandleW failed".into()));
        }

        let class_name = to_wide("windowed_wc");
        // SAFETY: Standard stock objects and cursor loading.
        let hcursor = unsafe { LoadCursorW(0, IDC_ARROW) };
        let hbrush = unsafe { GetStockObject(WHITE_BRUSH) };

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
        // SAFETY: RegisterClassExW registers the window class for the process.
        let atom = unsafe { RegisterClassExW(&wc) };
        if atom == 0 {
            const ERROR_CLASS_ALREADY_EXISTS: DWORD = 1410;
            // SAFETY: Calling GetLastError to verify if registration failed due to duplicate class name.
            let err = unsafe { GetLastError() };
            if err != ERROR_CLASS_ALREADY_EXISTS {
                return Err(Error::Platform(format!("RegisterClassExW failed: {err}")));
            }
        }

        let mut style: DWORD = if !config.decorations {
            WS_POPUP
        } else if config.resizable {
            WS_OVERLAPPEDWINDOW
        } else {
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX
        };

        if config.visible {
            style |= WS_VISIBLE;
        }

        let mut ex_style: DWORD = 0;
        if config.always_on_top {
            ex_style |= WS_EX_TOPMOST;
        }
        if config.transparent {
            ex_style |= WS_EX_LAYERED;
        }

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: config.width as LONG,
            bottom: config.height as LONG,
        };
        // SAFETY: AdjustWindowRect calculates window bounds required for desired client size.
        unsafe { AdjustWindowRect(&mut rect, style, 0) };

        let (cx, cy) = config
            .position
            .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

        let title = to_wide(&config.title);
        let state = Arc::new(Mutex::new(WindowState {
            events: Vec::new(),
            tracking_mouse: false,
            is_minimized: false,
            min_size: config.min_size,
            max_size: config.max_size,
        }));

        // SAFETY: CreateWindowExW creates the top-level window.
        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
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
            )
        };
        if hwnd == 0 {
            return Err(Error::Platform("CreateWindowExW failed".into()));
        }

        // SAFETY: We attach the state Arc pointer to GWLP_USERDATA so wnd_proc can access it.
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Arc::as_ptr(&state) as LONG_PTR);
            DragAcceptFiles(hwnd, 1);
            if config.visible {
                ShowWindow(hwnd, SW_SHOW);
                UpdateWindow(hwnd);
            }
        }

        let initial_fullscreen = config.fullscreen;
        let pw = Self {
            hwnd,
            hinstance,
            state,
            fullscreen_state: Mutex::new(FullscreenState::default()),
            mouse_grabbed: AtomicBool::new(false),
            width: AtomicU32::new(config.width),
            height: AtomicU32::new(config.height),
        };

        if initial_fullscreen {
            pw.set_fullscreen(true);
        }

        Ok(pw)
    }

    pub fn pump_events<F: FnMut(Event) -> ControlFlow>(
        &mut self,
        mut callback: F,
    ) -> Result<ControlFlow> {
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        // SAFETY: PeekMessageW pumps pending messages from the Win32 message queue.
        unsafe {
            while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return Ok(ControlFlow::Exit);
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        let events: Vec<Event> = {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut s.events)
        };

        let mut cf = ControlFlow::Continue;
        for event in events {
            if let Event::Resized { width, height } = event {
                self.width.store(width, Ordering::Relaxed);
                self.height.store(height, Ordering::Relaxed);
            }
            match callback(event) {
                ControlFlow::Exit => return Ok(ControlFlow::Exit),
                ControlFlow::WarpAndPoll(x, y) => {
                    self.warp_mouse(x, y);
                    cf = ControlFlow::Poll;
                }
                ControlFlow::Poll => cf = ControlFlow::Poll,
                ControlFlow::Wait => cf = ControlFlow::Wait,
                ControlFlow::Continue => {}
            }
        }

        Ok(cf)
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, mut callback: F) -> Result<()> {
        let mut flow = ControlFlow::Continue;

        loop {
            let res = self.pump_events(&mut callback)?;
            match res {
                ControlFlow::Exit => return Ok(()),
                ControlFlow::Poll | ControlFlow::WarpAndPoll(_, _) => flow = ControlFlow::Poll,
                ControlFlow::Wait => flow = ControlFlow::Wait,
                ControlFlow::Continue => {}
            }

            if flow == ControlFlow::Poll {
                self.request_redraw();
                std::thread::yield_now();
            } else {
                // In Wait/Continue mode, idle until a message arrives.
                // SAFETY: WaitMessage blocks until an input or window message arrives.
                unsafe {
                    WaitMessage();
                }
            }
        }
    }

    pub fn request_redraw(&self) {
        // SAFETY: InvalidateRect marks client rectangle as needing redraw.
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        // SAFETY: ShowCursor increments/decrements cursor display counter.
        unsafe {
            if visible {
                while ShowCursor(1) < 0 {}
            } else {
                while ShowCursor(0) >= 0 {}
            }
        }
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        // SAFETY: ClientToScreen converts window-relative coordinates to screen coordinates,
        // and SetCursorPos moves the cursor.
        unsafe {
            let mut pt = POINT { x, y };
            ClientToScreen(self.hwnd, &mut pt);
            SetCursorPos(pt.x, pt.y);
        }
    }

    pub fn set_title(&self, title: &str) {
        let t = to_wide(title);
        // SAFETY: SetWindowTextW sets the window title string.
        unsafe {
            SetWindowTextW(self.hwnd, t.as_ptr());
        }
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.width.store(width, Ordering::Relaxed);
        self.height.store(height, Ordering::Relaxed);

        // SAFETY: AdjustWindowRect and SetWindowPos resize window while preserving client area.
        unsafe {
            let style = GetWindowLongW(self.hwnd, GWL_STYLE) as DWORD;
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as LONG,
                bottom: height as LONG,
            };
            AdjustWindowRect(&mut rect, style, 0);
            SetWindowPos(
                self.hwnd,
                HWND_TOP,
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn set_position(&self, x: i32, y: i32) {
        // SAFETY: SetWindowPos changes window screen position.
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOP,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn set_visible(&self, visible: bool) {
        // SAFETY: ShowWindow toggles visibility.
        unsafe {
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    pub fn minimize(&self) {
        // SAFETY: ShowWindow minimizes the window.
        unsafe {
            ShowWindow(self.hwnd, SW_MINIMIZE);
        }
    }

    pub fn maximize(&self) {
        // SAFETY: ShowWindow maximizes the window.
        unsafe {
            ShowWindow(self.hwnd, SW_MAXIMIZE);
        }
    }

    pub fn set_fullscreen(&self, is_fullscreen: bool) {
        let mut fs = self.fullscreen_state.lock().unwrap_or_else(|e| e.into_inner());
        if fs.fullscreen == is_fullscreen {
            return;
        }
        fs.fullscreen = is_fullscreen;

        // SAFETY: Standard Win32 fullscreen transition via style change and window placement.
        unsafe {
            if is_fullscreen {
                fs.saved_style = GetWindowLongW(self.hwnd, GWL_STYLE) as DWORD;
                fs.saved_placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as UINT;
                GetWindowPlacement(self.hwnd, &mut fs.saved_placement);

                let monitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi: MONITORINFO = std::mem::zeroed();
                mi.cbSize = std::mem::size_of::<MONITORINFO>() as DWORD;
                GetMonitorInfoW(monitor, &mut mi);
                let r = mi.rcMonitor;

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
                SetWindowLongW(self.hwnd, GWL_STYLE, fs.saved_style as LONG);
                SetWindowPos(
                    self.hwnd,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                );
                SetWindowPlacement(self.hwnd, &fs.saved_placement);
            }
        }
    }

    pub fn set_cursor_grab(&self, grab: bool) {
        self.set_mouse_grabbed(grab);
    }

    pub fn set_mouse_grabbed(&self, grabbed: bool) {
        if self.mouse_grabbed.swap(grabbed, Ordering::SeqCst) == grabbed {
            return;
        }
        // SAFETY: Win32 SetCapture and ClipCursor confine or release the cursor.
        unsafe {
            if grabbed {
                SetCapture(self.hwnd);
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

    pub fn scale_factor(&self) -> f64 {
        // SAFETY: GetDpiForWindow retrieves DPI for HWND, fallback to standard 96 DPI.
        unsafe {
            let dpi = GetDpiForWindow(self.hwnd);
            if dpi > 0 {
                dpi as f64 / 96.0
            } else {
                1.0
            }
        }
    }

    pub fn inner_size(&self) -> (u32, u32) {
        (
            self.width.load(Ordering::Relaxed),
            self.height.load(Ordering::Relaxed),
        )
    }
}

impl Drop for PlatformWindow {
    fn drop(&mut self) {
        // SAFETY: Clear GWLP_USERDATA so subsequent message callbacks cannot access
        // freed state, then destroy the window.
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
        // SAFETY: The HWND and HINSTANCE are valid for the lifetime of self.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(h)) })
    }
}

impl HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Windows display handle requires no OS-specific pointer.
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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

#[cfg(feature = "raw-window-handle")]
impl PlatformWindow {
    pub fn window_handle_rwh(&self) -> std::result::Result<::raw_window_handle::WindowHandle<'_>, ::raw_window_handle::HandleError> {
        use std::num::NonZeroIsize;
        let hwnd_val = NonZeroIsize::new(self.hwnd).ok_or(::raw_window_handle::HandleError::Unavailable)?;
        let mut h = ::raw_window_handle::Win32WindowHandle::new(hwnd_val);
        h.hinstance = NonZeroIsize::new(self.hinstance);
        // SAFETY: HWND and HINSTANCE remain valid for lifetime of self.
        Ok(unsafe { ::raw_window_handle::WindowHandle::borrow_raw(::raw_window_handle::RawWindowHandle::Win32(h)) })
    }

    pub fn display_handle_rwh(&self) -> std::result::Result<::raw_window_handle::DisplayHandle<'_>, ::raw_window_handle::HandleError> {
        // SAFETY: Windows display handle is a unit struct with no state.
        Ok(unsafe {
            ::raw_window_handle::DisplayHandle::borrow_raw(::raw_window_handle::RawDisplayHandle::Windows(
                ::raw_window_handle::WindowsDisplayHandle::new(),
            ))
        })
    }
}

#[cfg(feature = "raw-window-handle")]
impl ::raw_window_handle::HasWindowHandle for PlatformWindow {
    fn window_handle(&self) -> std::result::Result<::raw_window_handle::WindowHandle<'_>, ::raw_window_handle::HandleError> {
        self.window_handle_rwh()
    }
}

#[cfg(feature = "raw-window-handle")]
impl ::raw_window_handle::HasDisplayHandle for PlatformWindow {
    fn display_handle(&self) -> std::result::Result<::raw_window_handle::DisplayHandle<'_>, ::raw_window_handle::HandleError> {
        self.display_handle_rwh()
    }
}