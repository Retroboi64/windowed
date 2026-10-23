//! Event definitions and key codes.
//!
//! # Example
//!
//! ```
//! use windowed::{Event, Key, Modifiers};
//!
//! let event = Event::KeyDown {
//!     key: Key::A,
//!     repeat: false,
//!     modifiers: Modifiers::NONE,
//! };
//!
//! assert_eq!(event.key(), Some(Key::A));
//! assert!(!event.is_repeat());
//! ```

use std::path::PathBuf;

/// Application and window events delivered to the event loop callback.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Window close requested (e.g. user clicked close button or pressed Alt+F4).
    CloseRequested,
    /// Window client area resized to physical dimensions `(width, height)`.
    Resized {
        /// New width of the client area.
        width: u32,
        /// New height of the client area.
        height: u32,
    },
    /// Window was moved to screen position `(x, y)`.
    Moved {
        /// New X screen position.
        x: i32,
        /// New Y screen position.
        y: i32,
    },
    /// Window gained keyboard input focus.
    FocusGained,
    /// Window lost keyboard input focus.
    FocusLost,
    /// Mouse cursor entered the window client area.
    MouseEnter,
    /// Mouse cursor left the window client area.
    MouseLeave,
    #[doc(hidden)]
    CursorEntered,
    #[doc(hidden)]
    CursorLeft,
    /// A keyboard key was pressed.
    KeyDown {
        /// Physical key identifier.
        key: Key,
        /// `true` if this key event is an automatic key repeat.
        repeat: bool,
        /// Active keyboard modifier keys at the time of the event.
        modifiers: Modifiers,
    },
    /// A keyboard key was released.
    KeyUp {
        /// Physical key identifier.
        key: Key,
        /// Active keyboard modifier keys at the time of the event.
        modifiers: Modifiers,
    },
    /// Translated UTF-8 character input.
    CharInput(char),
    /// Mouse cursor moved within the window client area.
    MouseMove {
        /// X coordinate relative to the window's client top-left.
        x: i32,
        /// Y coordinate relative to the window's client top-left.
        y: i32,
    },
    /// Mouse button pressed.
    MouseDown {
        /// Mouse button pressed.
        button: MouseButton,
        /// X coordinate relative to the window's client top-left.
        x: i32,
        /// Y coordinate relative to the window's client top-left.
        y: i32,
    },
    /// Mouse button released.
    MouseUp {
        /// Mouse button released.
        button: MouseButton,
        /// X coordinate relative to the window's client top-left.
        x: i32,
        /// Y coordinate relative to the window's client top-left.
        y: i32,
    },
    /// Mouse scroll wheel moved.
    MouseWheel {
        /// Vertical scroll delta (positive = forward/up).
        delta: f32,
    },
    /// A file was dropped onto the window from the system file manager.
    DroppedFile(PathBuf),
    /// Display DPI scale factor changed (e.g. window moved between monitors).
    ScaleFactorChanged {
        /// New display scale factor (1.0 = standard 96 DPI, 2.0 = 200% scaling).
        scale_factor: f64,
    },
    /// Window was minimized.
    Minimized,
    /// Window was restored from minimized or maximized state.
    Restored,
    /// Window needs to be redrawn.
    RedrawRequested,
}

impl Event {
    /// Returns the physical key if this is a `KeyDown` or `KeyUp` event.
    ///
    /// # Example
    /// ```
    /// use windowed::{Event, Key, Modifiers};
    /// let ev = Event::KeyDown { key: Key::Escape, repeat: false, modifiers: Modifiers::NONE };
    /// assert_eq!(ev.key(), Some(Key::Escape));
    /// ```
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

    /// Returns `true` if this is an auto-repeated `KeyDown` event.
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
    /// Shift key is active.
    pub shift: bool,
    /// Control key is active.
    pub ctrl: bool,
    /// Alt / Option key is active.
    pub alt: bool,
    /// Super / Windows / Command key is active.
    pub super_key: bool,
}

impl Modifiers {
    /// Constant with no modifier keys active.
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        super_key: false,
    };

    /// Returns `true` if no modifier keys are pressed.
    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.super_key
    }
}

/// Control flow decision returned from event loop callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    /// Continue running the event loop.
    Continue,
    /// Wait for new OS events without consuming CPU cycles while idle.
    Wait,
    /// Poll for events continuously without blocking, redrawing every frame.
    Poll,
    /// Warps the mouse pointer to `(x, y)` and continues polling.
    WarpAndPoll(i32, i32),
    /// Exits the event loop immediately.
    Exit,
}

/// Physical mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary / Left mouse button.
    Left,
    /// Secondary / Right mouse button.
    Right,
    /// Auxiliary / Middle mouse button.
    Middle,
}

/// Platform-independent physical keyboard keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_helpers() {
        let kd = Event::KeyDown {
            key: Key::Space,
            repeat: true,
            modifiers: Modifiers {
                shift: true,
                ctrl: false,
                alt: false,
                super_key: false,
            },
        };
        assert_eq!(kd.key(), Some(Key::Space));
        assert!(kd.is_repeat());
        assert_eq!(
            kd.modifiers(),
            Some(Modifiers {
                shift: true,
                ctrl: false,
                alt: false,
                super_key: false
            })
        );

        let ku = Event::KeyUp {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
        };
        assert_eq!(ku.key(), Some(Key::Enter));
        assert!(!ku.is_repeat());

        let close = Event::CloseRequested;
        assert_eq!(close.key(), None);
        assert!(!close.is_repeat());
        assert_eq!(close.modifiers(), None);
    }

    #[test]
    fn test_modifiers() {
        assert!(Modifiers::NONE.is_empty());
        let mods = Modifiers {
            shift: false,
            ctrl: true,
            alt: false,
            super_key: false,
        };
        assert!(!mods.is_empty());
        assert_eq!(Modifiers::default(), Modifiers::NONE);
    }

    #[test]
    fn test_control_flow_variants() {
        assert_eq!(ControlFlow::Continue, ControlFlow::Continue);
        assert_ne!(ControlFlow::Wait, ControlFlow::Poll);
        assert_eq!(
            ControlFlow::WarpAndPoll(10, 20),
            ControlFlow::WarpAndPoll(10, 20)
        );
        assert_ne!(ControlFlow::WarpAndPoll(10, 20), ControlFlow::Exit);
    }

    #[test]
    fn test_mouse_button_and_keys() {
        assert_eq!(MouseButton::Left, MouseButton::Left);
        assert_ne!(MouseButton::Left, MouseButton::Right);
        assert_eq!(Key::A, Key::A);
        assert_ne!(Key::Escape, Key::Enter);
    }

    #[test]
    fn test_event_variants_equality() {
        let p1 = PathBuf::from("test.txt");
        let p2 = PathBuf::from("test.txt");
        assert_eq!(Event::DroppedFile(p1), Event::DroppedFile(p2));
        assert_eq!(
            Event::ScaleFactorChanged { scale_factor: 1.5 },
            Event::ScaleFactorChanged { scale_factor: 1.5 }
        );
        assert_eq!(Event::Moved { x: 10, y: 20 }, Event::Moved { x: 10, y: 20 });
        assert_eq!(Event::Minimized, Event::Minimized);
        assert_eq!(Event::Restored, Event::Restored);
        assert_eq!(Event::MouseEnter, Event::MouseEnter);
        assert_eq!(Event::MouseLeave, Event::MouseLeave);
    }
}
