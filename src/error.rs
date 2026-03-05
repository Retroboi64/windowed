use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Platform(String),
    NoDisplay,
    WindowDestroyed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Platform(msg) => write!(f, "Platform error: {msg}"),
            Error::NoDisplay => write!(f, "Could not connect to display server"),
            Error::WindowDestroyed => write!(f, "Window has already been destroyed"),
        }
    }
}

impl std::error::Error for Error {}
