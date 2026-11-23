//! The Reticle: Zero-Flicker Software Cursor Engine for Kinetix OS.
//!
//! Blits a crisp arrow cursor sprite onto the linear framebuffer
//! while backing up and restoring underlying pixels, guaranteeing
//! zero screen tearing and zero flicker.

use kinetix_boot_protocol::FramebufferInfo;
use spin::Mutex;

pub const RETICLE_WIDTH: usize = 12;
pub const RETICLE_HEIGHT: usize = 19;

// 0 = Transparent, 1 = Outline (Dark #1A1A1A), 2 = Core (Radiant White #FFFFFF)
pub const RETICLE_SPRITE: [[u8; RETICLE_WIDTH]; RETICLE_HEIGHT] = [
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0],
    [1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0],
    [1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 0],
    [1, 2, 2, 1, 2, 2, 1, 0, 0, 0, 0, 0],
    [1, 2, 1, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 1, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0],
];

const COLOR_OUTLINE: u32 = 0x001A_1A1A;
const COLOR_CORE: u32 = 0x00FF_FFFF;

pub struct Reticle {
    fb_info: Option<FramebufferInfo>,
    current_x: usize,
    current_y: usize,
    saved_pixels: [u32; RETICLE_WIDTH * RETICLE_HEIGHT],
    is_visible: bool,
}

impl Reticle {
    pub const fn new() -> Self {
        Self {
            fb_info: None,
            current_x: 0,
            current_y: 0,
            saved_pixels: [0; RETICLE_WIDTH * RETICLE_HEIGHT],
            is_visible: false,
        }
    }

    pub fn init(&mut self, fb_info: FramebufferInfo) {
        self.fb_info = Some(fb_info);
        self.is_visible = false;
    }

    /// Restores the previously saved background pixels at (current_x, current_y).
    pub fn restore_background(&mut self) {
        if !self.is_visible {
            return;
        }

        let fb = match self.fb_info {
            Some(ref info) => info,
            None => return,
        };

        let ptr = fb.physical_address as *mut u32;
        let stride = fb.stride as usize;
        let width = fb.width as usize;
        let height = fb.height as usize;

        for row in 0..RETICLE_HEIGHT {
            let py = self.current_y + row;
            if py >= height {
                break;
            }

            for col in 0..RETICLE_WIDTH {
                let px = self.current_x + col;
                if px >= width {
                    break;
                }

                if RETICLE_SPRITE[row][col] != 0 {
                    let saved_color = self.saved_pixels[row * RETICLE_WIDTH + col];
                    unsafe {
                        core::ptr::write_volatile(ptr.add(py * stride + px), saved_color);
                    }
                }
            }
        }

        self.is_visible = false;
    }

    /// Saves the background at (new_x, new_y) and renders the reticle sprite.
    pub fn draw_at(&mut self, new_x: usize, new_y: usize) {
        let fb = match self.fb_info {
            Some(ref info) => info,
            None => return,
        };

        let ptr = fb.physical_address as *mut u32;
        let stride = fb.stride as usize;
        let width = fb.width as usize;
        let height = fb.height as usize;

        self.current_x = new_x;
        self.current_y = new_y;

        // 1. Save underlying pixels and draw reticle in a single pass
        for row in 0..RETICLE_HEIGHT {
            let py = new_y + row;
            if py >= height {
                break;
            }

            for col in 0..RETICLE_WIDTH {
                let px = new_x + col;
                if px >= width {
                    break;
                }

                let pixel_offset = py * stride + px;
                let original_pixel = unsafe { core::ptr::read_volatile(ptr.add(pixel_offset)) };
                self.saved_pixels[row * RETICLE_WIDTH + col] = original_pixel;

                let sprite_val = RETICLE_SPRITE[row][col];
                if sprite_val != 0 {
                    let color = if sprite_val == 1 {
                        COLOR_OUTLINE
                    } else {
                        COLOR_CORE
                    };
                    unsafe {
                        core::ptr::write_volatile(ptr.add(pixel_offset), color);
                    }
                }
            }
        }

        self.is_visible = true;
    }

    /// Seamlessly moves the reticle to a new coordinate without flicker.
    pub fn move_to(&mut self, new_x: usize, new_y: usize) {
        self.restore_background();
        self.draw_at(new_x, new_y);
    }

    #[allow(dead_code)]
    pub fn position(&self) -> (usize, usize) {
        (self.current_x, self.current_y)
    }

    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
}

pub static RETICLE: Mutex<Reticle> = Mutex::new(Reticle::new());
