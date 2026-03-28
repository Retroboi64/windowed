#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub position: Option<(i32, i32)>,
    pub resizable: bool,

    pub min_size: Option<(u32, u32)>,
    pub max_size: Option<(u32, u32)>,
}

impl WindowConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 800,
            height: 600,
            position: None,
            resizable: true,
            min_size: None,
            max_size: None,
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

    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.position = Some((x, y));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some((width, height));
        self
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new("windowed")
    }
}
