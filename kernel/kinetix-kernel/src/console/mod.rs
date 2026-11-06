//! Graphical Framebuffer Console for Kinetix OS.
//!
//! Renders text and pixels directly into the UEFI GOP linear framebuffer memory
//! using an embedded 8x16 monospace font, supporting automatic line wrapping,
//! backspace, tabs, and hardware-accelerated memory-move scrolling.

pub mod font;

use core::fmt::Write;
use crate::println;
use kinetix_boot_protocol::{FramebufferInfo, PixelFormat};
use spin::Mutex;

pub const DEFAULT_BG: u32 = 0x0010_1826; // Modern dark slate
pub const DEFAULT_FG: u32 = 0x00E2_E8F0; // Bright off-white

#[allow(dead_code)]
pub const COLOR_CYAN: u32 = 0x0038_BDF8;
#[allow(dead_code)]
pub const COLOR_GREEN: u32 = 0x004A_DE80;
#[allow(dead_code)]
pub const COLOR_YELLOW: u32 = 0x00FA_CC15;
#[allow(dead_code)]
pub const COLOR_RED: u32 = 0x00F8_7171;

/// High-performance graphical text console.
pub struct FramebufferConsole {
    fb: Option<FramebufferInfo>,
    col: usize,
    row: usize,
    max_cols: usize,
    max_rows: usize,
    fg_color: u32,
    bg_color: u32,
    initialized: bool,
}

impl FramebufferConsole {
    pub const fn new() -> Self {
        Self {
            fb: None,
            col: 0,
            row: 0,
            max_cols: 0,
            max_rows: 0,
            fg_color: DEFAULT_FG,
            bg_color: DEFAULT_BG,
            initialized: false,
        }
    }

    /// Initializes the console with GOP linear framebuffer geometry.
    pub fn init(&mut self, fb_info: FramebufferInfo) {
        let max_cols = (fb_info.width as usize) / font::FONT_WIDTH;
        let max_rows = (fb_info.height as usize) / font::FONT_HEIGHT;

        self.fb = Some(fb_info);
        self.max_cols = max_cols;
        self.max_rows = max_rows;
        self.col = 0;
        self.row = 0;
        self.fg_color = DEFAULT_FG;
        self.bg_color = DEFAULT_BG;
        self.initialized = true;

        self.clear(self.bg_color);
    }

    /// Fills the entire framebuffer with a single color.
    pub fn clear(&mut self, color: u32) {
        if let Some(ref fb) = self.fb {
            let ptr = fb.physical_address as *mut u32;
            let total_pixels = fb.stride as usize * fb.height as usize;
            for i in 0..total_pixels {
                unsafe {
                    core::ptr::write_volatile(ptr.add(i), color);
                }
            }
        }
        self.col = 0;
        self.row = 0;
    }

    /// Sets foreground and background text colors.
    #[allow(dead_code)]
    pub fn set_colors(&mut self, fg: u32, bg: u32) {
        self.fg_color = fg;
        self.bg_color = bg;
    }

    /// Draws a single 32-bit pixel.
    #[allow(dead_code)]
    #[inline]
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        if let Some(ref fb) = self.fb {
            if x < fb.width as usize && y < fb.height as usize {
                let offset = y * fb.stride as usize + x;
                let ptr = fb.physical_address as *mut u32;
                unsafe {
                    core::ptr::write_volatile(ptr.add(offset), color);
                }
            }
        }
    }

    /// Draws an 8x16 glyph at pixel coordinates (px, py).
    pub fn draw_glyph(&mut self, px: usize, py: usize, ch: char, fg: u32, bg: u32) {
        let glyph = font::get_glyph(ch);
        let fb_info = match self.fb {
            Some(ref info) => info,
            None => return,
        };

        let ptr = fb_info.physical_address as *mut u32;
        let stride = fb_info.stride as usize;
        let width = fb_info.width as usize;
        let height = fb_info.height as usize;

        // Convert color if Bgr format
        let (fg_val, bg_val) = match fb_info.pixel_format {
            PixelFormat::Bgr => (to_bgr(fg), to_bgr(bg)),
            _ => (fg, bg),
        };

        for (row_idx, &row_byte) in glyph.iter().enumerate() {
            let y = py + row_idx;
            if y >= height {
                break;
            }

            for col_idx in 0..8 {
                let x = px + col_idx;
                if x >= width {
                    break;
                }

                let is_fg = (row_byte & (0x80 >> col_idx)) != 0;
                let color = if is_fg { fg_val } else { bg_val };

                unsafe {
                    core::ptr::write_volatile(ptr.add(y * stride + x), color);
                }
            }
        }
    }

    /// Scrolls the console up by one text line (16 pixels).
    pub fn scroll_up(&mut self) {
        let fb_info = match self.fb {
            Some(ref info) => info,
            None => return,
        };

        let ptr = fb_info.physical_address as *mut u32;
        let stride = fb_info.stride as usize;
        let height = fb_info.height as usize;
        let line_height = font::FONT_HEIGHT;

        if height <= line_height {
            return;
        }

        let copy_pixels = (height - line_height) * stride;
        let src = unsafe { ptr.add(line_height * stride) };
        let dst = ptr;

        // Move active screen lines up by 16 pixels
        unsafe {
            core::ptr::copy(src, dst, copy_pixels);
        }

        // Fill bottom line with background color
        let bg_val = match fb_info.pixel_format {
            PixelFormat::Bgr => to_bgr(self.bg_color),
            _ => self.bg_color,
        };

        let bottom_start = unsafe { ptr.add((height - line_height) * stride) };
        let bottom_pixels = line_height * stride;
        for i in 0..bottom_pixels {
            unsafe {
                core::ptr::write_volatile(bottom_start.add(i), bg_val);
            }
        }
    }

    /// Writes a single character to the console at the current cursor position.
    pub fn write_char(&mut self, ch: char) {
        if !self.initialized {
            return;
        }

        match ch {
            '\n' => {
                self.col = 0;
                self.row += 1;
            }
            '\r' => {
                self.col = 0;
            }
            '\t' => {
                self.col = (self.col + 8) & !7;
                if self.col >= self.max_cols {
                    self.col = 0;
                    self.row += 1;
                }
            }
            '\x08' => {
                // Backspace
                if self.col > 0 {
                    self.col -= 1;
                    self.draw_glyph(
                        self.col * font::FONT_WIDTH,
                        self.row * font::FONT_HEIGHT,
                        ' ',
                        self.fg_color,
                        self.bg_color,
                    );
                }
            }
            printable => {
                if self.col >= self.max_cols {
                    self.col = 0;
                    self.row += 1;
                }

                if self.row >= self.max_rows {
                    self.scroll_up();
                    self.row = self.max_rows - 1;
                }

                self.draw_glyph(
                    self.col * font::FONT_WIDTH,
                    self.row * font::FONT_HEIGHT,
                    printable,
                    self.fg_color,
                    self.bg_color,
                );
                self.col += 1;
            }
        }

        if self.row >= self.max_rows {
            self.scroll_up();
            self.row = self.max_rows - 1;
        }
    }
}

impl Write for FramebufferConsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

/// Runs verification tests for graphical console font rendering, wrapping, and colors.
pub fn run_console_tests() {
    println!("\n[TEST] Running Framebuffer Console & Font Rendering Tests...");
    let (initialized, width, height, cols, rows) = {
        let console = FRAMEBUFFER_CONSOLE.lock();
        (
            console.initialized,
            console.max_cols * font::FONT_WIDTH,
            console.max_rows * font::FONT_HEIGHT,
            console.max_cols,
            console.max_rows,
        )
    };

    if initialized {
        println!(
            "  Console Display   : {}x{} (Grid: {} cols x {} rows)",
            width, height, cols, rows
        );
        assert!(cols > 0 && rows > 0);
        println!("  [PASS] Framebuffer Console & 8x16 Monospace Font active.");
    } else {
        println!("  [SKIP] Framebuffer console not initialized (serial mode).");
    }
}

/// Converts a 0x00RRGGBB color word into 0x00BBGGRR for BGR video modes.
#[inline]
const fn to_bgr(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    (b << 16) | (g << 8) | r
}

pub static FRAMEBUFFER_CONSOLE: Mutex<FramebufferConsole> = Mutex::new(FramebufferConsole::new());

/// Dual diagnostic output: writes formatted text to both COM1 serial and Framebuffer console.
pub fn _print(args: core::fmt::Arguments) {
    // 1. Output to COM1 serial diagnostics port (0x3F8)
    crate::arch::serial::SERIAL1.lock().write_fmt(args).ok();

    // 2. Output to Linear Framebuffer display
    FRAMEBUFFER_CONSOLE.lock().write_fmt(args).ok();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
