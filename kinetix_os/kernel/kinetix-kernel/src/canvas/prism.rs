//! The Prism: Double-Buffered 32-bit Surface Compositor for Kinetix OS.
//!
//! Provides a 3 MiB off-screen rendering backbuffer with 2D drawing primitives:
//! filled rectangles, outlines, text rendering, icon blitting, and hardware blit.

use alloc::vec::Vec;
use crate::console::font::{self, FONT_WIDTH, FONT_HEIGHT};
use kinetix_boot_protocol::FramebufferInfo;
use spin::Mutex;

pub struct PrismSurface {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub buffer: Vec<u32>,
    pub fb_target: Option<FramebufferInfo>,
    pub is_bgr: bool,
}

impl PrismSurface {
    pub const fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            stride: 0,
            buffer: Vec::new(),
            fb_target: None,
            is_bgr: false,
        }
    }

    /// Initializes the off-screen double buffer with linear framebuffer geometry.
    pub fn init(&mut self, fb: FramebufferInfo) {
        let width = fb.width as usize;
        let height = fb.height as usize;
        let stride = fb.stride as usize;
        let total_pixels = stride * height;

        self.width = width;
        self.height = height;
        self.stride = stride;
        self.is_bgr = false; // Modern UEFI 32-bit GOP linear framebuffers are 0x00RRGGBB
        self.fb_target = Some(fb);

        // Allocate 1024x768 u32 words (~3 MiB) in the kernel heap
        self.buffer = alloc::vec![0x000F_172A; total_pixels];
    }

    /// Translates standard 0x00RRGGBB color into native hardware format (BGR if necessary).
    #[inline(always)]
    pub fn to_native(&self, color: u32) -> u32 {
        if self.is_bgr {
            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;
            (b << 16) | (g << 8) | r
        } else {
            color
        }
    }

    /// Fills the entire backbuffer with a single color.
    #[allow(dead_code)]
    pub fn clear(&mut self, color: u32) {
        let native = self.to_native(color);
        for pixel in self.buffer.iter_mut() {
            *pixel = native;
        }
    }

    /// Writes a single pixel directly to the backbuffer with bounds checking.
    #[inline]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let offset = y * self.stride + x;
            if offset < self.buffer.len() {
                self.buffer[offset] = self.to_native(color);
            }
        }
    }

    /// Reads a pixel value from the backbuffer.
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            let offset = y * self.stride + x;
            if offset < self.buffer.len() {
                return self.buffer[offset];
            }
        }
        0
    }

    /// Draws a solid filled rectangle.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let native = self.to_native(color);
        let max_x = core::cmp::min(x + w, self.width);
        let max_y = core::cmp::min(y + h, self.height);

        for row in y..max_y {
            let row_start = row * self.stride;
            for col in x..max_x {
                let offset = row_start + col;
                if offset < self.buffer.len() {
                    self.buffer[offset] = native;
                }
            }
        }
    }

    /// Draws a 1-pixel hollow rectangle outline.
    pub fn draw_rect_outline(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        let native = self.to_native(color);
        let max_x = core::cmp::min(x + w, self.width);
        let max_y = core::cmp::min(y + h, self.height);

        // Top and bottom horizontal lines
        for col in x..max_x {
            if y < self.height {
                self.buffer[y * self.stride + col] = native;
            }
            if max_y > 0 && (max_y - 1) < self.height {
                self.buffer[(max_y - 1) * self.stride + col] = native;
            }
        }

        // Left and right vertical lines
        for row in y..max_y {
            if x < self.width {
                self.buffer[row * self.stride + x] = native;
            }
            if max_x > 0 && (max_x - 1) < self.width {
                self.buffer[row * self.stride + (max_x - 1)] = native;
            }
        }
    }

    /// Draws an ASCII character at pixel coordinates (px, py).
    pub fn draw_char(&mut self, px: usize, py: usize, ch: char, fg: u32, bg: Option<u32>) {
        let glyph = font::get_glyph(ch);
        let native_fg = self.to_native(fg);
        let native_bg = bg.map(|c| self.to_native(c));

        for (row_idx, &row_byte) in glyph.iter().enumerate() {
            let y = py + row_idx;
            if y >= self.height {
                break;
            }

            let row_offset = y * self.stride;
            for col_idx in 0..FONT_WIDTH {
                let x = px + col_idx;
                if x >= self.width {
                    break;
                }

                let is_fg = (row_byte & (0x80 >> col_idx)) != 0;
                let offset = row_offset + x;
                if is_fg {
                    self.buffer[offset] = native_fg;
                } else if let Some(bg_color) = native_bg {
                    self.buffer[offset] = bg_color;
                }
            }
        }
    }

    /// Draws a text string starting at (px, py).
    pub fn draw_text(&mut self, px: usize, py: usize, text: &str, fg: u32, bg: Option<u32>) {
        let mut curr_x = px;
        let mut curr_y = py;

        for ch in text.chars() {
            if ch == '\n' {
                curr_x = px;
                curr_y += FONT_HEIGHT;
                if curr_y >= self.height {
                    break;
                }
                continue;
            }

            if curr_x + FONT_WIDTH > self.width {
                curr_x = px;
                curr_y += FONT_HEIGHT;
                if curr_y >= self.height {
                    break;
                }
            }

            self.draw_char(curr_x, curr_y, ch, fg, bg);
            curr_x += FONT_WIDTH;
        }
    }

    /// High-speed copy of the entire off-screen backbuffer to the physical linear framebuffer.
    pub fn blit_to_screen(&self) {
        let fb = match self.fb_target {
            Some(ref info) => info,
            None => return,
        };

        let target_ptr = fb.physical_address as *mut u32;
        let src_ptr = self.buffer.as_ptr();
        let total_pixels = self.stride * self.height;

        unsafe {
            core::ptr::copy_nonoverlapping(src_ptr, target_ptr, total_pixels);
        }
    }

    /// Blits a dirty bounding box (x, y, w, h) to screen for incremental updates.
    #[allow(dead_code)]
    pub fn blit_rect(&self, x: usize, y: usize, w: usize, h: usize) {
        let fb = match self.fb_target {
            Some(ref info) => info,
            None => return,
        };

        let target_ptr = fb.physical_address as *mut u32;
        let src_ptr = self.buffer.as_ptr();
        let max_x = core::cmp::min(x + w, self.width);
        let max_y = core::cmp::min(y + h, self.height);
        let copy_width = max_x.saturating_sub(x);

        for row in y..max_y {
            let offset = row * self.stride + x;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    src_ptr.add(offset),
                    target_ptr.add(offset),
                    copy_width,
                );
            }
        }
    }
}

pub static PRISM: Mutex<PrismSurface> = Mutex::new(PrismSurface::new());
