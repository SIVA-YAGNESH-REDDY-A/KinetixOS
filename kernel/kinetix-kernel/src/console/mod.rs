#![allow(dead_code)]

//! Linear Framebuffer Console for Kinetix Kernel
//!
//! Provides direct pixel rendering into the GOP video memory without BIOS calls.

use kinetix_boot_protocol::FramebufferInfo;
use spin::Mutex;

pub struct FramebufferConsole {
    fb: Option<FramebufferInfo>,
    cursor_x: usize,
    cursor_y: usize,
}

impl FramebufferConsole {
    pub const fn new() -> Self {
        Self {
            fb: None,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn init(&mut self, fb_info: FramebufferInfo) {
        self.fb = Some(fb_info);
        self.clear(0x0010_1020); // Dark navy blue background
    }

    pub fn clear(&mut self, color_rgb: u32) {
        if let Some(ref fb) = self.fb {
            let ptr = fb.physical_address as *mut u32;
            let total_pixels = fb.stride as usize * fb.height as usize;
            for i in 0..total_pixels {
                unsafe {
                    core::ptr::write_volatile(ptr.add(i), color_rgb);
                }
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, color_rgb: u32) {
        if let Some(ref fb) = self.fb {
            if x < fb.width as usize && y < fb.height as usize {
                let offset = y * fb.stride as usize + x;
                let ptr = fb.physical_address as *mut u32;
                unsafe {
                    core::ptr::write_volatile(ptr.add(offset), color_rgb);
                }
            }
        }
    }
}

pub static FRAMEBUFFER_CONSOLE: Mutex<FramebufferConsole> = Mutex::new(FramebufferConsole::new());
