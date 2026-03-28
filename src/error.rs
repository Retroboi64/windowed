use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Platform(String),
    NoDisplay,
    WindowDestroyed,
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
