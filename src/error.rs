//! Error types for `windowed`.

use std::fmt;

/// A specialized [`Result`](std::result::Result) type for windowing operations.
///
/// # Examples
///
/// ```no_run
/// use windowed::{Window, WindowConfig, Result};
///
/// fn create_window() -> Result<Window> {
///     Window::new(WindowConfig::new("Demo"))
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when creating or interacting with a window.
#[derive(Debug)]
pub enum Error {
    /// A platform-specific API error occurred (e.g. Win32, X11, Cocoa).
    Platform(String),
    /// The window system could not connect to a display server (e.g., missing `$DISPLAY` or Wayland session).
    NoDisplay,
    /// An operation was attempted on a window that has already been destroyed or closed.
    WindowDestroyed,
    /// The requested operation or feature is unsupported on the current platform.
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Platform(msg) => write!(f, "platform error: {msg}"),
            Error::NoDisplay => write!(f, "could not connect to display server"),
            Error::WindowDestroyed => write!(f, "window has already been destroyed"),
            Error::Unsupported(feat) => write!(f, "unsupported on this platform: {feat}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            Error::Platform("test".into()).to_string(),
            "platform error: test"
        );
        assert_eq!(
            Error::NoDisplay.to_string(),
            "could not connect to display server"
        );
        assert_eq!(
            Error::WindowDestroyed.to_string(),
            "window has already been destroyed"
        );
        assert_eq!(
            Error::Unsupported("wayland").to_string(),
            "unsupported on this platform: wayland"
        );
    }
}
