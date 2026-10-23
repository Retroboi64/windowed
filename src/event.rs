//! Event definitions and key codes.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    CloseRequested,
    Resized {
        width: u32,
        height: u32,
    },
    Moved {
        x: i32,
        y: i32,
    },
    FocusGained,
    FocusLost,
    MouseEnter,
    MouseLeave,
    #[doc(hidden)]
    CursorEntered,
    #[doc(hidden)]
    CursorLeft,
    KeyDown {
        key: Key,
        repeat: bool,
        modifiers: Modifiers,
    },
    KeyUp {
        key: Key,
        modifiers: Modifiers,
    },
    CharInput(char),
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseDown {
        button: MouseButton,
        x: i32,
        y: i32,
    },
    MouseUp {
        button: MouseButton,
        x: i32,
        y: i32,
    },
    MouseWheel {
        delta: f32,
    },
    DroppedFile(PathBuf),
    ScaleFactorChanged {
        scale_factor: f64,
    },
    Minimized,
    Restored,
    RedrawRequested,
}

impl Event {
    /// Returns the physical key if this is a `KeyDown` or `KeyUp` event.
    pub fn key(&self) -> Option<Key> {
        match self {
            Event::KeyDown { key, .. } | Event::KeyUp { key, .. } => Some(*key),
            _ => None,
        }
    }

    /// Returns the active modifiers if this is a `KeyDown` or `KeyUp` event.
    pub fn modifiers(&self) -> Option<Modifiers> {
        match self {
            Event::KeyDown { modifiers, .. } | Event::KeyUp { modifiers, .. } => Some(*modifiers),
            _ => None,
        }
    }

    /// Returns `true` if this is a repeated `KeyDown` event.
    pub fn is_repeat(&self) -> bool {
        match self {
            Event::KeyDown { repeat, .. } => *repeat,
            _ => false,
        }
    }
}

/// Keyboard modifier keys state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        super_key: false,
    };

    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.super_key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    Continue,
    Poll,
    WarpAndPoll(i32, i32),
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    Enter,
    Escape,
    Space,
    Backspace,
    Tab,
    Delete,
    Insert,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,

    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftSuper,
    RightSuper,

    Unknown,
}
