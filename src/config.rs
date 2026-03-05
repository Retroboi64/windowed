#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
}

impl WindowConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 800,
            height: 600,
            resizable: true,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new("windowed")
    }
}
