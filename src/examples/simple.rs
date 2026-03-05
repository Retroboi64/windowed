//! Simple example: open a window and print events.
//!
//! Press Escape or close the window to exit.
//!
//! Run with:
//!   cargo run --example simple

use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> windowed::Result<()> {
    let mut window = Window::new(
        WindowConfig::new("windowed — simple example")
            .size(800, 600)
            .resizable(true),
    )?;

    println!("Window created. Press Escape or close the window to exit.");

    window.run(|event| {
        match &event {
            Event::CloseRequested => {
                println!("Close requested — goodbye!");
                return ControlFlow::Exit;
            }
            Event::KeyDown(Key::Escape) => {
                println!("Escape pressed — goodbye!");
                return ControlFlow::Exit;
            }
            Event::Resized { width, height } => {
                println!("Resized to {width}x{height}");
            }
            Event::KeyDown(key) => println!("KeyDown:  {key:?}"),
            Event::KeyUp(key)   => println!("KeyUp:    {key:?}"),
            Event::MouseDown { button, x, y } => {
                println!("MouseDown {button:?} at ({x}, {y})");
            }
            Event::MouseUp { button, x, y } => {
                println!("MouseUp   {button:?} at ({x}, {y})");
            }
            Event::MouseWheel { delta } => println!("Wheel delta: {delta}"),
            Event::FocusGained => println!("Focus gained"),
            Event::FocusLost   => println!("Focus lost"),
            Event::RedrawRequested | Event::MouseMove { .. } => {}
        }
        ControlFlow::Continue
    })
}
