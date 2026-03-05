#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    CloseRequested,
    Resized { width: u32, height: u32 },
    FocusGained,
    FocusLost,
    KeyDown(Key),
    KeyUp(Key),
    MouseMove { x: i32, y: i32 },
    MouseDown { button: MouseButton, x: i32, y: i32 },
    MouseUp { button: MouseButton, x: i32, y: i32 },
    MouseWheel { delta: f32 },
    RedrawRequested,
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

    // --- Digits (top row) ---
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

    Unknown,
}
