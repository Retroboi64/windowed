mod platform;

pub mod config;
pub mod error;
pub mod event;
pub mod rwd;
pub mod window;

pub use config::WindowConfig;
pub use error::{Error, Result};
pub use event::{ControlFlow, Event, Key, MouseButton};
pub use window::Window;
