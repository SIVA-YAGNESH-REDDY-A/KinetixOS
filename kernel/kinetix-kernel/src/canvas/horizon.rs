//! The Horizon: Bottom Dock & System Chronometer for Kinetix OS.
//!
//! Renders the modern 44-pixel dock bar containing the Kinetix Spark launcher emblem,
//! active app indicators, and live system chronometer telemetry.

use super::prism::PrismSurface;

pub const HORIZON_Y: usize = 724;
pub const HORIZON_HEIGHT: usize = 44;

pub struct HorizonDock {
    uptime_ms: u64,
}

impl HorizonDock {
    pub const fn new() -> Self {
        Self { uptime_ms: 0 }
    }

    #[allow(dead_code)]
    pub fn set_uptime(&mut self, ms: u64) {
        self.uptime_ms = ms;
    }

    /// Renders the dock bar across the bottom of the Prism surface.
    pub fn render(&self, prism: &mut PrismSurface) {
        let width = prism.width;

        // 1. Dark glassmorphic background (#0A1124)
        prism.fill_rect(0, HORIZON_Y, width, HORIZON_HEIGHT, 0x000A_1124);

        // 2. Top subtle highlight separator line (#2A3D66)
        prism.fill_rect(0, HORIZON_Y, width, 1, 0x002A_3D66);

        // 3. Kinetix Spark Launcher Button on the left
        // Launcher pill: (12, 730, 110, 32)
        prism.fill_rect(12, 730, 112, 32, 0x0013_203C);
        prism.draw_rect_outline(12, 730, 112, 32, 0x0038_BDF8);

        // Draw diamond spark glyph (16x16) at (X=20, Y=738)
        draw_spark_glyph(prism, 20, 738, 0x0038_BDF8);

        // Label: "KINETIX"
        prism.draw_text(42, 738, "KINETIX", 0x00F8_FAFC, None);

        // 4. Center Search / Quick-Launch Pill (Inspired by reference desktop)
        // Position: (360, 732, 304, 28)
        prism.fill_rect(360, 732, 304, 28, 0x0017_2644);
        prism.draw_rect_outline(360, 732, 304, 28, 0x0033_4A75);
        prism.draw_text(376, 738, "Search Kinetix Cartridges...", 0x0094_A3B8, None);

        // 5. Right Chronometer & System Pulse
        // Formatted uptime: e.g. "UPTIME: 1.2s | READY"
        let secs = self.uptime_ms / 1000;
        let tenths = (self.uptime_ms % 1000) / 100;

        let mut time_buf = [0u8; 32];
        let time_str = format_time(&mut time_buf, secs, tenths);

        // Right status pill
        prism.fill_rect(780, 732, 232, 28, 0x0013_203C);
        prism.draw_rect_outline(780, 732, 232, 28, 0x002E_4770);
        prism.draw_text(792, 738, time_str, 0x0038_BDF8, None);
    }
}

/// Draws an iconic 12x12 diamond spark emblem.
fn draw_spark_glyph(prism: &mut PrismSurface, px: usize, py: usize, color: u32) {
    // Top tip
    prism.put_pixel(px + 6, py + 1, color);
    prism.put_pixel(px + 6, py + 2, color);
    // Upper diamond
    prism.fill_rect(px + 5, py + 3, 3, 2, color);
    prism.fill_rect(px + 4, py + 5, 5, 2, color);
    // Center horizontal bar
    prism.fill_rect(px + 1, py + 6, 11, 2, color);
    // Lower diamond
    prism.fill_rect(px + 4, py + 8, 5, 2, color);
    prism.fill_rect(px + 5, py + 10, 3, 2, color);
    // Bottom tip
    prism.put_pixel(px + 6, py + 12, color);
    prism.put_pixel(px + 6, py + 13, color);
}

fn format_time<'a>(buf: &'a mut [u8], secs: u64, tenths: u64) -> &'a str {
    let prefix = b"UPTIME: ";
    let mut idx = 0;
    for &b in prefix {
        buf[idx] = b;
        idx += 1;
    }

    // Number of seconds
    let mut num = secs;
    let mut digits = [0u8; 10];
    let mut d_count = 0;
    if num == 0 {
        digits[0] = b'0';
        d_count = 1;
    } else {
        while num > 0 {
            digits[d_count] = b'0' + (num % 10) as u8;
            num /= 10;
            d_count += 1;
        }
    }
    for i in (0..d_count).rev() {
        buf[idx] = digits[i];
        idx += 1;
    }

    buf[idx] = b'.';
    idx += 1;
    buf[idx] = b'0' + (tenths as u8);
    idx += 1;

    let suffix = b"s | ONLINE";
    for &b in suffix {
        buf[idx] = b;
        idx += 1;
    }

    unsafe { core::str::from_utf8_unchecked(&buf[..idx]) }
}

pub static HORIZON: spin::Mutex<HorizonDock> = spin::Mutex::new(HorizonDock::new());
