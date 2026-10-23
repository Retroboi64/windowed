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
        .size(800, 600)
        .resizable(true);

    let mut window = Window::new(config)?;

    window.run(|event, _window| match event {
        Event::CloseRequested => ControlFlow::Exit,
        Event::KeyDown {
            key: Key::Escape, ..
        } => ControlFlow::Exit,
        _ => ControlFlow::Continue,
    })
}
