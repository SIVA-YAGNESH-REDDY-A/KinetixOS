//! Flow Chambers: Draggable Window Frames & Workspaces for Kinetix OS.
//!
//! Provides window frame primitives: dark frosted-glass title bars,
//! close buttons, canvas areas, and mouse drag-and-move mechanics.

use super::prism::PrismSurface;

#[allow(dead_code)]
pub struct FlowChamber {
    pub id: u8,
    pub title: &'static str,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub is_open: bool,
    pub is_focused: bool,
    pub is_dragging: bool,
    pub drag_offset_x: usize,
    pub drag_offset_y: usize,
    pub bg_color: u32,
}

impl FlowChamber {
    pub const fn new(id: u8, title: &'static str, x: usize, y: usize, width: usize, height: usize, bg_color: u32) -> Self {
        Self {
            id,
            title,
            x,
            y,
            width,
            height,
            is_open: false,
            is_focused: false,
            is_dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
            bg_color,
        }
    }

    pub fn contains(&self, px: usize, py: usize) -> bool {
        self.is_open && px >= self.x && px < (self.x + self.width) && py >= self.y && py < (self.y + self.height)
    }

    pub fn contains_header(&self, px: usize, py: usize) -> bool {
        self.is_open && px >= self.x && px < (self.x + self.width) && py >= self.y && py < (self.y + 26)
    }

    pub fn contains_close(&self, px: usize, py: usize) -> bool {
        let close_x = self.x + self.width.saturating_sub(24);
        let close_y = self.y + 3;
        self.is_open && px >= close_x && px < (close_x + 20) && py >= close_y && py < (close_y + 20)
    }

    /// Renders the complete window frame and canvas into the Prism surface.
    #[allow(dead_code)]
    pub fn render(&self, prism: &mut PrismSurface) {
        if !self.is_open {
            return;
        }

        let border_color = if self.is_focused { 0x0038_BDF8 } else { 0x002A_3F68 };

        // 1. Window Body Canvas
        prism.fill_rect(self.x, self.y + 26, self.width, self.height.saturating_sub(26), self.bg_color);

        // 2. Title Bar Header (26 pixels tall)
        prism.fill_rect(self.x, self.y, self.width, 26, 0x0013_203C);
        prism.fill_rect(self.x, self.y, self.width, 1, 0x002A_3F68);

        // Title text
        prism.draw_text(self.x + 10, self.y + 5, self.title, 0x00F8_FAFC, None);

        // 3. Close Button [X] on the top-right
        let close_x = self.x + self.width.saturating_sub(24);
        let close_y = self.y + 4;
        prism.fill_rect(close_x, close_y, 18, 18, 0x00DC_2626); // Crimson red
        prism.draw_text(close_x + 5, close_y + 1, "X", 0x00FF_FFFF, None);

        // 4. Outer 1-pixel Frame Border
        prism.draw_rect_outline(self.x, self.y, self.width, self.height, border_color);
    }
}
