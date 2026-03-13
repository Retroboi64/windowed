//! Simple example: Opens a simple blank window
//!   For Version "0.2.1"
//!
//! Press Escape or close the window to quit.
//!
//! Run with:
//!   cargo run --example simple_window

use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> windowed::Result<()> {
    // We set a config which you can customize
    let config: WindowConfig = WindowConfig::new("A simple window")
        .size(800, 300)
        .resizable(true);

    // Then we create the simple window in order to be used
    let mut window = Window::new(config)?;

    // This is windows runtime
    window.run(move |event| {
        match &event {
            Event::CloseRequested => {
                return ControlFlow::Exit;
            }
            Event::KeyDown(Key::Escape) => {
                return ControlFlow::Exit;
            }
            Event::RedrawRequested => {}
            _ => {}
        }
        ControlFlow::Continue
    })
}
