//! The Horizon: Bottom Dock, Bottom-Left Real-Time Clock & Power Controller for Kinetix OS.
//!
//! Renders the 44-pixel taskbar dock containing:
//! - Kinetix Spark launcher button
//! - Real-time date and clock chronometer positioned at the bottom-left
//! - Active / Minimized application pills
//! - Functional power popup menu (Shutdown, Restart, Sleep, Cancel)

use super::prism::PrismSurface;

pub const HORIZON_HEIGHT: usize = 44;

pub struct HorizonDock {
    pub uptime_ms: u64,
    pub start_menu_open: bool,
    pub screen_width: usize,
    pub screen_height: usize,
}

impl HorizonDock {
    pub const fn new() -> Self {
        Self {
            uptime_ms: 0,
            start_menu_open: false,
            screen_width: 1024,
            screen_height: 768,
        }
    }

    pub fn set_dimensions(&mut self, width: usize, height: usize) {
        self.screen_width = width;
        self.screen_height = height;
    }

    pub fn set_uptime(&mut self, ms: u64) {
        self.uptime_ms = ms;
    }

    pub fn toggle_start_menu(&mut self) {
        self.start_menu_open = !self.start_menu_open;
    }

    fn dock_y(&self) -> usize {
        self.screen_height.saturating_sub(HORIZON_HEIGHT)
    }

    pub fn contains_start_button(&self, px: usize, py: usize) -> bool {
        let dy = self.dock_y();
        px >= 10 && px < 46 && py >= (dy + 6) && py < (dy + 38)
    }

    pub fn contains_shutdown(&self, px: usize, py: usize) -> bool {
        let menu_y = self.dock_y().saturating_sub(148);
        self.start_menu_open && px >= 16 && px < 216 && py >= (menu_y + 28) && py < (menu_y + 54)
    }

    pub fn contains_restart(&self, px: usize, py: usize) -> bool {
        let menu_y = self.dock_y().saturating_sub(148);
        self.start_menu_open && px >= 16 && px < 216 && py >= (menu_y + 56) && py < (menu_y + 82)
    }

    pub fn contains_sleep(&self, px: usize, py: usize) -> bool {
        let menu_y = self.dock_y().saturating_sub(148);
        self.start_menu_open && px >= 16 && px < 216 && py >= (menu_y + 84) && py < (menu_y + 110)
    }

    pub fn contains_cancel(&self, px: usize, py: usize) -> bool {
        let menu_y = self.dock_y().saturating_sub(148);
        self.start_menu_open && px >= 16 && px < 216 && py >= (menu_y + 112) && py < (menu_y + 138)
    }

    /// Renders the dock bar across the bottom of the Prism surface.
    pub fn render(&self, prism: &mut PrismSurface) {
        let width = prism.width;
        let height = prism.height;
        let dock_y = height.saturating_sub(HORIZON_HEIGHT);

        // 1. Dark glassmorphic background (#0A1124)
        prism.fill_rect(0, dock_y, width, HORIZON_HEIGHT, 0x000A_1124);

        // 2. Top subtle highlight separator line (#2A3D66)
        prism.fill_rect(0, dock_y, width, 1, 0x002A_3D66);

        // 3. Kinetix Spark Launcher Button on the far left
        let btn_bg = if self.start_menu_open { 0x001E_3A5F } else { 0x0013_203C };
        prism.fill_rect(10, dock_y + 6, 36, 32, btn_bg);
        prism.draw_rect_outline(10, dock_y + 6, 36, 32, 0x0038_BDF8);
        draw_spark_glyph(prism, 22, dock_y + 16, 0x0038_BDF8);

        // 4. REAL-TIME CLOCK AND DATE POSITIONED AT THE BOTTOM-LEFT
        let dt = crate::arch::rtc::read_datetime();
        let mut time_buf = [0u8; 32];
        let time_str = format_datetime(&mut time_buf, dt);

        let clock_x = 52;
        let clock_w = 210;
        prism.fill_rect(clock_x, dock_y + 6, clock_w, 32, 0x0010_1D36);
        prism.draw_rect_outline(clock_x, dock_y + 6, clock_w, 32, 0x002A_426B);

        // Small chronometer icon
        prism.fill_rect(clock_x + 8, dock_y + 14, 14, 14, 0x001E_2E48);
        prism.draw_rect_outline(clock_x + 8, dock_y + 14, 14, 14, 0x0038_BDF8);
        prism.put_pixel(clock_x + 15, dock_y + 18, 0x0038_BDF8);
        prism.put_pixel(clock_x + 15, dock_y + 21, 0x0038_BDF8);
        prism.put_pixel(clock_x + 18, dock_y + 21, 0x0038_BDF8);

        prism.draw_text(clock_x + 28, dock_y + 14, time_str, 0x0038_BDF8, None);

        // 5. Center Active Window Taskbar Buttons
        let center_x = 276;
        let mut app_x = center_x;

        let (kpc_open, kpc_min) = {
            let g = crate::cartridges::kinetix_pc::KINETIX_PC.lock();
            (g.chamber.is_open, g.chamber.is_minimized)
        };
        let (term_open, term_min) = {
            let g = crate::cartridges::terminal::TERMINAL.lock();
            (g.chamber.is_open, g.chamber.is_minimized)
        };
        let (scribe_open, scribe_min) = {
            let g = crate::cartridges::scribe::SCRIBE.lock();
            (g.chamber.is_open, g.chamber.is_minimized)
        };
        let (telem_open, telem_min) = {
            let g = crate::cartridges::telemetry::TELEMETRY.lock();
            (g.chamber.is_open, g.chamber.is_minimized)
        };
        let (speed_open, speed_min) = {
            let g = crate::cartridges::speed_test::SPEED_TEST.lock();
            (g.chamber.is_open, g.chamber.is_minimized)
        };

        let apps = [
            ("KinetixPC", kpc_open, kpc_min),
            ("Terminal", term_open, term_min),
            ("Scribe", scribe_open, scribe_min),
            ("Telemetry", telem_open, telem_min),
            ("SpeedTest", speed_open, speed_min),
        ];

        for (name, is_open, is_minimized) in apps {
            if is_open {
                let pill_w = 96;
                let bg = if is_minimized { 0x0011_1B2A } else { 0x001C_3558 };
                let border = if is_minimized { 0x0033_4155 } else { 0x0038_BDF8 };
                prism.fill_rect(app_x, dock_y + 8, pill_w, 28, bg);
                prism.draw_rect_outline(app_x, dock_y + 8, pill_w, 28, border);

                // Indicator pip
                prism.fill_rect(app_x + 6, dock_y + 20, 4, 4, if is_minimized { 0x0094_A3B8 } else { 0x0022_C55E });
                prism.draw_text(app_x + 14, dock_y + 14, name, 0x00F8_FAFC, None);
                app_x += pill_w + 8;
            }
        }

        // 6. Right Network & Uptime Status Pill
        let net = crate::arch::net::NET_STATE.lock();
        let right_w = 210usize;
        let right_x = width.saturating_sub(right_w + 12);
        prism.fill_rect(right_x, dock_y + 8, right_w, 28, 0x0013_203C);
        prism.draw_rect_outline(right_x, dock_y + 8, right_w, 28, 0x002E_4770);

        let net_icon_color = if net.usb_device_connected {
            0x0038_BDF8 // Cyan for Mobile Phone
        } else if net.link_up {
            0x0022_C55E // Green for Ethernet
        } else {
            0x00F8_7171 // Red for Offline
        };
        prism.fill_rect(right_x + 10, dock_y + 18, 8, 8, net_icon_color);

        let (net_status, text_color) = if net.usb_device_connected {
            ("USB: MotoEdge 60", 0x0038_BDF8)
        } else if net.link_up {
            ("ETH: 10.0.2.15", 0x00E2_E8F0)
        } else {
            ("Offline", 0x0094_A3B8)
        };
        prism.draw_text(right_x + 24, dock_y + 14, net_status, text_color, None);

        // 7. Draw Kinetix Power Popup Menu if toggled open
        if self.start_menu_open {
            let menu_y = dock_y.saturating_sub(148);
            prism.fill_rect(10, menu_y, 210, 142, 0x000F_172A);
            prism.draw_rect_outline(10, menu_y, 210, 142, 0x0038_BDF8);

            // Header
            prism.draw_text(18, menu_y + 8, "Kinetix Power Management", 0x0094_A3B8, None);
            prism.fill_rect(14, menu_y + 22, 202, 1, 0x001E_293B);

            // Option 1: Shutdown
            prism.fill_rect(16, menu_y + 28, 198, 24, 0x007F_1D1D);
            prism.draw_rect_outline(16, menu_y + 28, 198, 24, 0x00EF_4444);
            prism.draw_text(24, menu_y + 33, "[*] Shutdown System", 0x00FE_F2F2, None);

            // Option 2: Restart
            prism.fill_rect(16, menu_y + 56, 198, 24, 0x0071_3F12);
            prism.draw_rect_outline(16, menu_y + 56, 198, 24, 0x00FA_CC15);
            prism.draw_text(24, menu_y + 61, "[>] Restart System", 0x00FE_FCE8, None);

            // Option 3: Sleep
            prism.fill_rect(16, menu_y + 84, 198, 24, 0x0014_304A);
            prism.draw_rect_outline(16, menu_y + 84, 198, 24, 0x0038_BDF8);
            prism.draw_text(24, menu_y + 89, "[z] Sleep (Low Power)", 0x00E0_F2FE, None);

            // Option 4: Cancel
            prism.fill_rect(16, menu_y + 112, 198, 24, 0x001E_293B);
            prism.draw_rect_outline(16, menu_y + 112, 198, 24, 0x0064_748B);
            prism.draw_text(24, menu_y + 117, "[x] Cancel Menu", 0x00CBD5E1, None);
        }
    }
}

/// Draws an iconic 12x12 diamond spark emblem.
fn draw_spark_glyph(prism: &mut PrismSurface, px: usize, py: usize, color: u32) {
    prism.put_pixel(px + 6, py + 1, color);
    prism.put_pixel(px + 6, py + 2, color);
    prism.fill_rect(px + 5, py + 3, 3, 2, color);
    prism.fill_rect(px + 4, py + 5, 5, 2, color);
    prism.fill_rect(px + 1, py + 6, 11, 2, color);
    prism.fill_rect(px + 4, py + 8, 5, 2, color);
    prism.fill_rect(px + 5, py + 10, 3, 2, color);
    prism.put_pixel(px + 6, py + 12, color);
    prism.put_pixel(px + 6, py + 13, color);
}

fn format_datetime<'a>(buf: &'a mut [u8], dt: crate::arch::rtc::RtcDateTime) -> &'a str {
    // Format: "YYYY-MM-DD  HH:MM:SS" (20 chars)
    let y = dt.year;
    buf[0] = b'0' + ((y / 1000) % 10) as u8;
    buf[1] = b'0' + ((y / 100) % 10) as u8;
    buf[2] = b'0' + ((y / 10) % 10) as u8;
    buf[3] = b'0' + (y % 10) as u8;
    buf[4] = b'-';
    buf[5] = b'0' + (dt.month / 10);
    buf[6] = b'0' + (dt.month % 10);
    buf[7] = b'-';
    buf[8] = b'0' + (dt.day / 10);
    buf[9] = b'0' + (dt.day % 10);
    buf[10] = b' ';
    buf[11] = b' ';
    buf[12] = b'0' + (dt.hour / 10);
    buf[13] = b'0' + (dt.hour % 10);
    buf[14] = b':';
    buf[15] = b'0' + (dt.minute / 10);
    buf[16] = b'0' + (dt.minute % 10);
    buf[17] = b':';
    buf[18] = b'0' + (dt.second / 10);
    buf[19] = b'0' + (dt.second % 10);

    unsafe { core::str::from_utf8_unchecked(&buf[..20]) }
}

pub static HORIZON: spin::Mutex<HorizonDock> = spin::Mutex::new(HorizonDock::new());
