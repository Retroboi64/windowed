# windowed

A minimal, zero-dependency cross-platform windowing library for **Windows** (Win32) and **Linux** (X11).

No GPU context, no retained scene graph — just a window, an event loop, and raw window handles you can hand off to a renderer like `wgpu`, `ash`, or `softbuffer`.

---

## Platform support

| OS | Backend |
|---|---|
| Windows 10 / 11 | Win32 |
| Linux | X11|
| macOS | ❌ Not yet implemented |

---

## Quick start

```toml
# Cargo.toml
[dependencies]
windowed = { path = "." }
```

```rust
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> windowed::Result<()> {
    let mut window = Window::new(
        WindowConfig::new("My Window")
            .size(800, 600)
            .resizable(true),
    )?;

    window.run(|event| {
        match event {
            Event::CloseRequested => return ControlFlow::Exit,
            Event::KeyDown(Key::Escape) => return ControlFlow::Exit,
            Event::RedrawRequested => { /* draw here */ }
            _ => {}
        }
        ControlFlow::Continue
    })
}
```

Run the bundled example:

```sh
cargo run --example simple
```

---

## WindowConfig

```rust
WindowConfig::new("Title")   // sets title, defaults: 800×600, resizable
    .size(1280, 720)          // inner (client area) size in physical pixels
    .position(100, 100)       // initial top-left screen position (optional)
    .resizable(false);        // lock window size
```

---

## Events

| Event | Description |
|---|---|
| `CloseRequested` | User clicked the × button or pressed Alt+F4 |
| `Resized { width, height }` | Window was resized |
| `FocusGained` / `FocusLost` | Keyboard focus changed |
| `KeyDown(Key)` / `KeyUp(Key)` | Physical key pressed / released |
| `CharInput(char)` | Translated character (use for text input) |
| `MouseMove { x, y }` | Cursor moved inside the window |
| `MouseDown { button, x, y }` | Mouse button pressed |
| `MouseUp { button, x, y }` | Mouse button released |
| `MouseWheel { delta }` | Scroll wheel — positive = up/forward |
| `RedrawRequested` | Window should be redrawn |

### KeyDown vs CharInput

- Use `KeyDown` / `KeyUp` for game controls and hotkeys (physical keys, layout-independent).
- Use `CharInput` for text fields (accounts for key-repeat, dead keys, and locale).

---

## ControlFlow

Returned from the event callback to drive the loop:

| Value | Behaviour |
|---|---|
| `Continue` | Wait for the next OS event (default, CPU-friendly) |
| `Poll` | Drain events then immediately emit `RedrawRequested` — use for real-time rendering |
| `WarpAndPoll(x, y)` | Warp cursor to `(x, y)` in window coordinates and switch to polling — use for FPS / captured mouse |
| `Exit` | Stop the event loop and return from `Window::run` |

---

## Window methods

```rust
window.request_redraw();            // queue a RedrawRequested event
window.set_title("new title");
window.inner_size();                // → (width, height) in physical pixels
window.set_cursor_visible(false);   // hide / show the OS cursor
window.warp_mouse(x, y);            // move cursor to window-local coordinates
```

---

## Raw window handles

`Window` implements [`HasWindowHandle`](https://docs.rs/raw-window-handle/latest/raw_window_handle/trait.HasWindowHandle.html) and [`HasDisplayHandle`](https://docs.rs/raw-window-handle/latest/raw_window_handle/trait.HasDisplayHandle.html) from the [`raw-window-handle`](https://crates.io/crates/raw-window-handle) crate, so you can pass it directly to `wgpu`, `softbuffer`, `ash`, or any other renderer that accepts those traits.

---

## Dependencies

| Crate | Used on |
|---|---|
| [`raw-window-handle`](https://crates.io/crates/raw-window-handle) | All platforms |

---

## License

MIT — see [LICENSE](LICENSE) for details.  
Copyright © 2025 Patrick Reese (Retroboi64)

Made with 🖤 by Retroboi64
