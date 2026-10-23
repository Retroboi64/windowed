# windowed

A minimal, zero-dependency, cross-platform windowing library for **Windows** (Win32), **Linux** (X11), and **macOS** (Cocoa).

No heavy runtime, no hidden dependencies, and no retained scene graph — just a native window, an event loop, and optional raw window handles you can hand off to renderers like `wgpu`, `ash`, or `softbuffer`.

---

## Platform Support Matrix

| Feature | Windows (Win32) | Linux (X11) | macOS (Cocoa) |
|---|:---:|:---:|:---:|
| **Zero Dependencies (Default)** | Yes | Yes | Yes |
| **Window Creation & Event Loop** | Yes | Yes | Yes |
| **Non-blocking Event Pump (`pump_events`)** | Yes | Yes | Yes |
| **Idle Wait Mode (`ControlFlow::Wait`)** | Yes | Yes | Yes |
| **Window Resizing & Positioning** | Yes | Yes | Yes |
| **Minimize & Maximize** | Yes | Yes | Yes |
| **Borderless Fullscreen** | Yes | Yes | Yes |
| **Window Visibility Control** | Yes | Yes | Yes |
| **Decorations / Borderless** | Yes | Yes | Yes |
| **Min / Max Size Constraints** | Yes | Yes | Yes |
| **Always On Top** | Yes | Yes | Yes |
| **Cursor Visibility & Grab / Confinement** | Yes | Yes | Yes |
| **Cursor Warping** | Yes | Yes | Yes |
| **Mouse Enter & Leave Events** | Yes | Yes | Yes |
| **Key Modifiers (Shift/Ctrl/Alt/Super)** | Yes | Yes | Yes |
| **Key Repeat Detection** | Yes | Yes | Yes |
| **File Drag & Drop (`DroppedFile`)** | Yes | Yes | Yes |
| **HiDPI / Dynamic Scale Factor** | Yes | Yes | Yes |
| **System Clipboard (Get & Set Text)** | Yes | Yes | Yes |
| **Raw Window Handle (`raw-window-handle` 0.6)** | Optional feature | Optional feature | Optional feature |
| **OpenGL Context & Buffer Swap** | Optional feature | Optional feature | Optional feature |

---

## Quick Start

Add `windowed` to your `Cargo.toml`:

```toml
[dependencies]
windowed = { path = "." }
```

### Standard Event Loop

```rust
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> windowed::Result<()> {
    let mut window = Window::new(
        WindowConfig::new("My Window")
            .size(800, 600)
            .resizable(true),
    )?;

    window.run(|event, window| match event {
        Event::CloseRequested => ControlFlow::Exit,
        Event::KeyDown {
            key: Key::Escape, ..
        } => ControlFlow::Exit,
        Event::RedrawRequested => {
            // Render frame here
            ControlFlow::Poll
        }
        _ => ControlFlow::Wait,
    })
}
```

Run the bundled example:

```sh
cargo run --example simple_window
```

### Custom Game / Tick Loop (`pump_events`)

If you want to drive your own game loop without yielding full control to `window.run`:

```rust
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> windowed::Result<()> {
    let mut window = Window::new(WindowConfig::new("Game Loop"))?;

    'game_loop: loop {
        let flow = window.pump_events(|event, _win| match event {
            Event::CloseRequested => ControlFlow::Exit,
            Event::KeyDown { key: Key::Escape, .. } => ControlFlow::Exit,
            _ => ControlFlow::Poll,
        })?;

        if matches!(flow, ControlFlow::Exit) {
            break 'game_loop;
        }

        // Run game simulation & physics tick
        // Render frame
    }

    Ok(())
}
```

---

## Configuration (`WindowConfig`)

`WindowConfig` provides a fluent builder to configure your window prior to creation:

```rust
let config = WindowConfig::new("Title")  // sets title, default 800x600, resizable
    .size(1280, 720)                     // inner (client area) size in physical pixels
    .position(100, 100)                  // initial top-left screen position (optional)
    .resizable(false)                    // lock window resizing
    .decorations(false)                  // borderless / chromeless window
    .always_on_top(true)                 // keep window above other windows
    .min_size(400, 300)                  // minimum allowed dimensions
    .max_size(1920, 1080)                // maximum allowed dimensions
    .transparent(true)                   // enable framebuffer transparency
    .visible(false);                     // create window initially hidden
```

---

## Runtime Window Controls

All runtime controls are methods on [`Window`](file:///c:/Users/reese/Desktop/windowed/src/window.rs) taking `&self`, allowing you to call them directly from event callbacks:

```rust
// Window sizing and geometry
window.set_size(1024, 768);           // resize inner client area
window.set_position(150, 150);        // move top-left corner on screen
let (width, height) = window.inner_size();

// Window state & presentation
window.set_title("New Title");        // dynamically update title
window.set_visible(true);             // show or hide window
window.minimize();                    // minimize to taskbar / dock
window.maximize();                    // maximize to fill screen work area
window.set_fullscreen(true);          // toggle borderless fullscreen

// Mouse cursor controls
window.set_cursor_visible(false);     // hide OS mouse cursor
window.set_cursor_grab(true);         // confine mouse cursor inside client area
window.warp_mouse(100, 100);          // move cursor to window client coordinates

// HiDPI & Clipboard
let scale = window.scale_factor();    // e.g. 1.0, 1.25, 1.5, 2.0
window.set_clipboard_text("Hello!");  // copy text to system clipboard
let text = window.get_clipboard_text()?; // read text from system clipboard

// Repaint requests
window.request_redraw();              // queue Event::RedrawRequested
```

---

## Events

The [`Event`](file:///c:/Users/reese/Desktop/windowed/src/event.rs) enum delivers user interactions and system notifications:

| Event | Description |
|---|---|
| `CloseRequested` | User clicked the close button or pressed Alt+F4 / Cmd+Q |
| `Resized { width, height }` | Window client area was resized |
| `Moved(x, y)` | Window position changed on screen |
| `FocusGained` / `FocusLost` | Window gained or lost keyboard focus |
| `KeyDown { key, repeat, modifiers }` | Physical key pressed (with repeat flag and modifier state) |
| `KeyUp { key, modifiers }` | Physical key released |
| `CharInput(char)` | Translated unicode text character (for text entry) |
| `MouseMove { x, y }` | Mouse cursor moved inside the window client area |
| `MouseDown { button, x, y }` | Mouse button pressed (`Left`, `Right`, `Middle`) |
| `MouseUp { button, x, y }` | Mouse button released |
| `MouseWheel { delta }` | Scroll wheel movement (positive = forward/up) |
| `MouseEnter` / `MouseLeave` | Cursor entered or left the window client area |
| `DroppedFile(PathBuf)` | File dragged and dropped onto the window |
| `ScaleFactorChanged(scale)` | Display DPI / scale factor changed |
| `Minimized` / `Restored` | Window minimized to taskbar or restored |
| `RedrawRequested` | Window needs to be repainted |

### Modifiers

The `Modifiers` struct tracks active modifier keys:
```rust
if event_modifiers.ctrl && event_modifiers.shift {
    // Ctrl+Shift combo
}
```
Available fields: `shift`, `ctrl`, `alt`, and `super_key` (Windows key or Command key).

---

## ControlFlow

Returned from event callbacks to drive event processing:

| Value | Behaviour |
|---|---|
| `Continue` | Continue running the event loop |
| `Wait` | Idle wait for incoming OS events (0% idle CPU usage) |
| `Poll` | Non-blocking drain that emits `RedrawRequested` every cycle (for games and real-time graphics) |
| `WarpAndPoll(x, y)` | Warps mouse to `(x, y)` and continues polling |
| `Exit` | Terminate the event loop immediately |

---

## Raw Window Handles (`wgpu`, `softbuffer`, etc.)

`windowed` provides two ways to access native OS handles:

1. **Zero-dependency descriptors (`windowed::rwd`):** Built-in handle types available by default without pulling in any external crates.
2. **`raw-window-handle` crate (v0.6):** Enable via the `raw-window-handle` feature flag in `Cargo.toml`:

```toml
[dependencies]
windowed = { path = ".", features = ["raw-window-handle"] }
```

When enabled, `Window` implements `raw_window_handle::HasWindowHandle` and `raw_window_handle::HasDisplayHandle`, allowing direct plug-and-play with crates like `wgpu`, `vulkano`, `ash`, or `softbuffer`.

---

## Optional OpenGL Support

Enable the `opengl` feature to create an OpenGL context and present frames cross-platform:

```toml
[dependencies]
windowed = { path = ".", features = ["opengl"] }
```

Run the bundled OpenGL triangle example:

```sh
cargo run --example gl_window --features opengl
```

---

## License

MIT — see [LICENSE](LICENSE) for details.  
Copyright (c) 2025-2026 Patrick Reese (Retroboi64)
