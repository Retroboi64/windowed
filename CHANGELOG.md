# Changelog

All notable changes to `windowed` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-10-22

### Breaking Changes
- **`Event::KeyDown` and `Event::KeyUp` structure:**
  - Changed `Event::KeyDown(Key)` to `Event::KeyDown { key: Key, repeat: bool, modifiers: Modifiers }`.
  - Changed `Event::KeyUp(Key)` to `Event::KeyUp { key: Key, modifiers: Modifiers }`.
  - Added convenience helper methods `event.key() -> Option<Key>` and `event.is_repeat() -> bool` to simplify pattern matching.
- **`Window::run` callback parameter:**
  - Changed callback signature from `FnMut(Event, &mut Window) -> ControlFlow` to `FnMut(Event, &Window) -> ControlFlow`. Window controls now take `&self` through internal platform synchronization.

### Added
- **Runtime Window Controls (`Window`):**
  - `set_title(&self, title: &str)`: Change window title dynamically.
  - `set_size(&self, width: u32, height: u32)`: Resize window client area.
  - `set_position(&self, x: i32, y: i32)`: Set window screen position.
  - `set_visible(&self, visible: bool)`: Show or hide window.
  - `minimize(&self)`: Minimize window to taskbar/dock.
  - `maximize(&self)`: Maximize window to fill screen work area.
  - `set_fullscreen(&self, is_fullscreen: bool)`: Toggle borderless fullscreen.
  - `set_cursor_visible(&self, visible: bool)`: Hide or show mouse cursor.
  - `set_cursor_grab(&self, grab: bool)`: Confine mouse cursor to window area.
  - `scale_factor(&self) -> f64`: Query HiDPI scale factor.
  - `set_clipboard_text(&self, text: &str) -> Result<()>`: Write text to system clipboard.
  - `get_clipboard_text(&self) -> Result<String>`: Read text from system clipboard.
- **Configuration Options (`WindowConfig`):**
  - `decorations(bool)`: Enable or disable titlebar and window borders.
  - `always_on_top(bool)`: Keep window on top of other windows.
  - `min_size(width, height)`: Minimum window dimensions constraint.
  - `max_size(width, height)`: Maximum window dimensions constraint.
  - `transparent(bool)`: Request transparent window background.
  - `visible(bool)`: Control initial window visibility.
- **New Events (`Event`):**
  - `Event::Moved(i32, i32)`: Window repositioned on screen.
  - `Event::DroppedFile(PathBuf)`: Drag-and-drop file delivery.
  - `Event::ScaleFactorChanged(f64)`: HiDPI display scale changed.
  - `Event::MouseEnter` / `Event::MouseLeave`: Cursor entered/left window client area.
  - `Event::Minimized` / `Event::Restored`: Window minimize/restore state transitions.
  - `Modifiers` struct tracking Shift, Ctrl, Alt, and Super (Windows/Command) keys.
- **Event Loop & Control Flow:**
  - Added `ControlFlow::Wait` mode that sleeps the thread when no OS events are pending (0% idle CPU).
  - Added non-blocking `Window::pump_events` method to allow user-driven game loops and tick routines.
- **Raw Window Handle Interoperability:**
  - Added optional `raw-window-handle` (v0.6) feature integration via `rwh_06` / `raw-window-handle` flags.
  - Implemented `HasWindowHandle` and `HasDisplayHandle` for `Window` and `PlatformWindow` on Windows, Linux, and macOS.
  - Default build remains 100% dependency-free with internal `rwd` descriptors.
- **Platform Implementations:**
  - **Windows (Win32):** Full HiDPI awareness (`WM_DPICHANGED`), cursor confinement via `ClipCursor`, mouse hover tracking via `TrackMouseEvent`, clipboard access, and correct window placement restoration.
  - **Linux (X11):** Session Wayland detection with descriptive error messages when `$DISPLAY` is unset, X11 clipboard selection transfer, and window manager hints (`_NET_WM_STATE`, `_MOTIF_WM_HINTS`).
  - **macOS (Cocoa):** Working zero-dependency Cocoa / AppKit backend via raw `objc` runtime FFI, supporting event pumping, HiDPI (`backingScaleFactor`), clipboard, and window state controls.
- **Quality & Documentation:**
  - Comprehensive unit test suite covering event handling, modifier states, and config builders.
  - GitHub Actions CI workflow covering Windows, Ubuntu Linux, and macOS.
  - Complete doc comments with working examples across all public structs, enums, and functions (`cargo doc` produces 0 warnings).
  - Configured `rustfmt.toml` and `.clippy.toml`.

### Fixed
- Fixed circular feature dependency in `Cargo.toml`.
- Added safety comments to all `unsafe` blocks and functions per Rust 2024 edition guidelines (`unsafe_op_in_unsafe_fn`).
- Fixed derive attributes and struct layout issues on Win32 `POINT` / `RECT` / `WINDOWPLACEMENT`.
- Replaced mouse warp client-space math with proper `ClientToScreen` conversion.
- Replaced unhandled panics and unwraps with structured `Result` error returns.

[Unreleased]: https://github.com/Retroboi64/windowed/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/Retroboi64/windowed/compare/v0.3.1...v0.4.0
