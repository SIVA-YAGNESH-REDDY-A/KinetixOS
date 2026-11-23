//! Flow Chambers: Draggable Window Frames & Workspaces for Kinetix OS.
//!
//! Provides window frame primitives: dark frosted-glass title bars,
//! minimize [-], maximize [+], close [X] buttons, canvas areas, and mouse drag-and-move mechanics.

use super::prism::PrismSurface;

pub struct FlowChamber {
    pub id: u8,
    pub title: &'static str,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub is_open: bool,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_dragging: bool,
    pub drag_offset_x: usize,
    pub drag_offset_y: usize,
    pub saved_x: usize,
    pub saved_y: usize,
    pub saved_width: usize,
    pub saved_height: usize,
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
            is_minimized: false,
            is_maximized: false,
            is_dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
            saved_x: x,
            saved_y: y,
            saved_width: width,
            saved_height: height,
            bg_color,
        }
    }

    pub fn contains(&self, px: usize, py: usize) -> bool {
        self.is_open && !self.is_minimized && px >= self.x && px < (self.x + self.width) && py >= self.y && py < (self.y + self.height)
    }

    pub fn contains_header(&self, px: usize, py: usize) -> bool {
        let btn_start = self.x + self.width.saturating_sub(74);
        self.is_open && !self.is_minimized && !self.is_maximized && px >= self.x && px < btn_start && py >= self.y && py < (self.y + 26)
    }

    pub fn contains_close(&self, px: usize, py: usize) -> bool {
        let close_x = self.x + self.width.saturating_sub(26);
        self.is_open && !self.is_minimized && px >= close_x && px <= (self.x + self.width) && py >= self.y && py < (self.y + 26)
    }

    pub fn contains_maximize(&self, px: usize, py: usize) -> bool {
        let max_x = self.x + self.width.saturating_sub(50);
        let close_x = self.x + self.width.saturating_sub(26);
        self.is_open && !self.is_minimized && px >= max_x && px < close_x && py >= self.y && py < (self.y + 26)
    }

    pub fn contains_minimize(&self, px: usize, py: usize) -> bool {
        let min_x = self.x + self.width.saturating_sub(74);
        let max_x = self.x + self.width.saturating_sub(50);
        self.is_open && !self.is_minimized && px >= min_x && px < max_x && py >= self.y && py < (self.y + 26)
    }

    pub fn minimize(&mut self) {
        self.is_minimized = true;
        self.is_focused = false;
    }

    pub fn restore(&mut self) {
        self.is_minimized = false;
        self.is_focused = true;
    }

    pub fn toggle_maximize(&mut self, screen_w: usize, screen_h: usize) {
        if self.is_maximized {
            self.x = self.saved_x;
            self.y = self.saved_y;
            self.width = self.saved_width;
            self.height = self.saved_height;
            self.is_maximized = false;
        } else {
            self.saved_x = self.x;
            self.saved_y = self.y;
            self.saved_width = self.width;
            self.saved_height = self.height;
            self.x = 8;
            self.y = 8;
            self.width = screen_w.saturating_sub(16);
            self.height = (screen_h.saturating_sub(52)).saturating_sub(8);
            self.is_maximized = true;
        }
    }

    pub fn start_drag(&mut self, px: usize, py: usize) {
        if self.contains_header(px, py) {
            self.is_dragging = true;
            self.drag_offset_x = px.saturating_sub(self.x);
            self.drag_offset_y = py.saturating_sub(self.y);
        }
    }

    pub fn update_drag(&mut self, px: usize, py: usize, screen_w: usize, screen_h: usize) {
        if self.is_dragging {
            let mut new_x = px.saturating_sub(self.drag_offset_x);
            let mut new_y = py.saturating_sub(self.drag_offset_y);

            let max_x = screen_w.saturating_sub(self.width);
            let max_y = (screen_h.saturating_sub(44)).saturating_sub(self.height);

            if new_x > max_x { new_x = max_x; }
            if new_y > max_y { new_y = max_y; }
            if new_y < 10 { new_y = 10; }

            self.x = new_x;
            self.y = new_y;
        }
    }

    pub fn stop_drag(&mut self) {
        self.is_dragging = false;
    }

    /// Renders the complete window frame and canvas into the Prism surface.
    pub fn render(&self, prism: &mut PrismSurface) {
        if !self.is_open || self.is_minimized {
            return;
        }

        let border_color = if self.is_focused { 0x0038_BDF8 } else { 0x002A_3F68 };

        // 1. Window Body Canvas
        prism.fill_rect(self.x, self.y + 26, self.width, self.height.saturating_sub(26), self.bg_color);

        // 2. Title Bar Header (26 pixels tall)
        let header_bg = if self.is_focused { 0x0017_2B4D } else { 0x0010_1D36 };
        prism.fill_rect(self.x, self.y, self.width, 26, header_bg);
        prism.fill_rect(self.x, self.y, self.width, 1, 0x002A_3F68);

        // Title text
        prism.draw_text(self.x + 10, self.y + 5, self.title, 0x00F8_FAFC, None);

        // 3. Minimize Button [-]
        let min_x = self.x + self.width.saturating_sub(74);
        let max_x = self.x + self.width.saturating_sub(50);
        let close_x = self.x + self.width.saturating_sub(26);
        let btn_y = self.y + 4;
        prism.fill_rect(min_x + 1, btn_y, 21, 18, 0x001E_2E48);
        prism.draw_rect_outline(min_x + 1, btn_y, 21, 18, 0x0033_4A6A);
        prism.fill_rect(min_x + 6, btn_y + 11, 10, 2, 0x00CBD5E1);

        // 4. Maximize Button [+]
        prism.fill_rect(max_x + 1, btn_y, 21, 18, 0x001E_2E48);
        prism.draw_rect_outline(max_x + 1, btn_y, 21, 18, 0x0033_4A6A);
        prism.draw_rect_outline(max_x + 6, btn_y + 4, 10, 10, 0x00CBD5E1);

        // 5. Close Button [X] on the top-right
        prism.fill_rect(close_x + 1, btn_y, 23, 18, 0x00DC_2626); // Crimson red
        prism.draw_text(close_x + 8, btn_y + 1, "X", 0x00FF_FFFF, None);

        // 6. Outer 1-pixel Frame Border
        prism.draw_rect_outline(self.x, self.y, self.width, self.height, border_color);
    }
}
