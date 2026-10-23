#![warn(missing_docs)]
//! **windowed** — a minimal, zero-dependency, cross-platform windowing library.
//!
//! Supports:
//! - Windows (Win32)
//! - Linux   (X11 / Xlib)
//! - macOS   (Cocoa / AppKit)
//!
//! Optional OpenGL support is enabled with the `opengl` feature flag.
//!
//! # Quick start
//!
//! ```no_run
//! use windowed::{ControlFlow, Event, Key, Window, WindowConfig};
//!
//! fn main() -> windowed::Result<()> {
//!     let mut window = Window::new(WindowConfig::new("Hello").size(800, 600))?;
//!     window.run(|event, _window| match event {
//!         Event::CloseRequested => ControlFlow::Exit,
//!         Event::KeyDown { key: Key::Escape, .. } => ControlFlow::Exit,
//!         _ => ControlFlow::Continue,
//!     })
//! }
//! ```

mod platform;

pub mod buffer;
pub mod config;
pub mod error;
pub mod event;
pub mod rwd;
pub mod window;

pub use config::WindowConfig;
pub use error::{Error, Result};
pub use event::{ControlFlow, Event, Key, Modifiers, MouseButton};
pub use window::Window;

#[cfg(feature = "raw-window-handle")]
pub use raw_window_handle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_config_defaults() {
        let config = WindowConfig::new("test").size(800, 600);
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.resizable);
        assert!(config.decorations);
        assert!(!config.always_on_top);
        assert!(!config.transparent);
        assert!(config.visible);
    }

    #[test]
    fn control_flow_exit_variant() {
        let cf = ControlFlow::Exit;
        assert!(matches!(cf, ControlFlow::Exit));
    }
}
