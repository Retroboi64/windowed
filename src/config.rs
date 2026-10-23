//! Window configuration options.
//!
//! # Example
//!
//! ```
//! use windowed::WindowConfig;
//!
//! let config = WindowConfig::new("My App")
//!     .size(1280, 720)
//!     .resizable(false)
//!     .decorations(true)
//!     .always_on_top(false);
//! ```

/// Configuration parameters used to create a new [`Window`](crate::Window).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    /// Window title displayed in the titlebar.
    pub title: String,
    /// Desired width of the client area in logical/physical pixels.
    pub width: u32,
    /// Desired height of the client area in logical/physical pixels.
    pub height: u32,
    /// Optional initial screen position `(x, y)` of the top-left corner.
    pub position: Option<(i32, i32)>,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Whether the window should start in fullscreen mode.
    pub fullscreen: bool,
    /// Minimum allowed window size `(width, height)`.
    pub min_size: Option<(u32, u32)>,
    /// Maximum allowed window size `(width, height)`.
    pub max_size: Option<(u32, u32)>,
    /// Whether window decorations (title bar, borders) should be drawn.
    pub decorations: bool,
    /// Whether the window should stay above all other non-topmost windows.
    pub always_on_top: bool,
    /// Whether the window should support background transparency.
    pub transparent: bool,
    /// Whether the window should be immediately visible upon creation.
    pub visible: bool,
}

impl WindowConfig {
    /// Create a new configuration with default settings and the given title.
    ///
    /// # Example
    /// ```
    /// use windowed::WindowConfig;
    /// let config = WindowConfig::new("Demo");
    /// assert_eq!(config.width, 800);
    /// assert_eq!(config.height, 600);
    /// ```
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 800,
            height: 600,
            position: None,
            resizable: true,
            fullscreen: false,
            min_size: None,
            max_size: None,
            decorations: true,
            always_on_top: false,
            transparent: false,
            visible: true,
        }
    }

    /// Sets the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the initial client width and height in pixels.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the initial top-left position `(x, y)` in screen coordinates.
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.position = Some((x, y));
        self
    }

    /// Sets whether the window should start fullscreen.
    pub fn fullscreen(mut self, is_fullscreen: bool) -> Self {
        self.fullscreen = is_fullscreen;
        self
    }

    /// Sets whether the window can be resized by the user.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Sets the minimum allowed size in pixels.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    /// Sets the maximum allowed size in pixels.
    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some((width, height));
        self
    }

    /// Sets whether window decorations (title bar, borders, close buttons) are enabled.
    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Sets whether the window stays on top of other windows.
    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }

    /// Sets whether the window client area can be transparent.
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Sets whether the window is shown immediately upon creation.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

impl Default for WindowConfig {
    /// Returns default window configuration with title `"windowed"`, size 800x600, resizable.
    fn default() -> Self {
        Self::new("windowed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_builder() {
        let config = WindowConfig::new("Builder Test")
            .size(1920, 1080)
            .position(50, 100)
            .resizable(false)
            .fullscreen(true)
            .min_size(400, 300)
            .max_size(2560, 1440)
            .decorations(false)
            .always_on_top(true)
            .transparent(true)
            .visible(false);

        assert_eq!(config.title, "Builder Test");
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.position, Some((50, 100)));
        assert!(!config.resizable);
        assert!(config.fullscreen);
        assert_eq!(config.min_size, Some((400, 300)));
        assert_eq!(config.max_size, Some((2560, 1440)));
        assert!(!config.decorations);
        assert!(config.always_on_top);
        assert!(config.transparent);
        assert!(!config.visible);
    }

    #[test]
    fn test_window_config_default() {
        let def = WindowConfig::default();
        assert_eq!(def.title, "windowed");
        assert_eq!(def.width, 800);
        assert_eq!(def.height, 600);
        assert_eq!(def.position, None);
        assert!(def.resizable);
        assert!(!def.fullscreen);
        assert!(def.decorations);
        assert!(!def.always_on_top);
        assert!(!def.transparent);
        assert!(def.visible);
    }

    #[test]
    fn test_window_config_title_into() {
        let config = WindowConfig::new(String::from("Owned String"));
        assert_eq!(config.title, "Owned String");
        let modified = config.title("New &str");
        assert_eq!(modified.title, "New &str");
    }
}
