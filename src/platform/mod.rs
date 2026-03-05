#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::PlatformWindow;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::PlatformWindow;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!(
    "windowed only supports Linux (X11) and Windows. \
     macOS support is not yet implemented."
);
