//! Simple example — opens a blank window.
//!
//! Press Escape or close the window to quit.
//!
//! ```
//! cargo run --example simple_window
//! ```

use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> windowed::Result<()> {
    let config = WindowConfig::new("A simple window")
        .fullscreen(true)
        .resizable(true);

    let mut window = Window::new(config)?;

    // _window is a refrance to the window
    window.run(|event, _window| match event {
        Event::CloseRequested => ControlFlow::Exit,
        Event::KeyDown(Key::Escape) => ControlFlow::Exit,
        _ => ControlFlow::Continue,
    })
}
