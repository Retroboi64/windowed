#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::PlatformWindow;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::PlatformWindow;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::PlatformWindow;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
compile_error!(
    "windowed supports Linux (X11), Windows, and macOS. \
     Your target OS is not yet implemented."
);
