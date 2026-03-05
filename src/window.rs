use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

use crate::config::WindowConfig;
use crate::error::Result;
use crate::event::{ControlFlow, Event};
use crate::platform::PlatformWindow;

pub struct Window {
    inner: PlatformWindow,
}

impl Window {
    pub fn new(config: WindowConfig) -> Result<Self> {
        Ok(Self {
            inner: PlatformWindow::new(&config)?,
        })
    }

    pub fn run<F: FnMut(Event) -> ControlFlow>(&mut self, callback: F) -> Result<()> {
        self.inner.run(callback)
    }

    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    pub fn warp_mouse(&self, x: i32, y: i32) {
        self.inner.warp_mouse(x, y);
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        self.inner.set_cursor_visible(visible);
    }

    pub fn inner_size(&self) -> (u32, u32) {
        self.inner.inner_size()
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}
