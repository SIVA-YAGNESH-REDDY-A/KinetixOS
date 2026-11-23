//! The Beacons: Interactive Desktop Icons & Application Launchers for Kinetix OS.
//!
//! Renders desktop icon tiles with custom visual emblems, labels,
//! hover illumination, and mouse click detection.

use super::prism::PrismSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeaconId {
    KinetixPC,
    Terminal,
    Scribe,
    Telemetry,
    SpeedTest,
    Ouroboros,
    Vault,
}

pub struct CartridgeBeacon {
    pub id: BeaconId,
    pub label: &'static str,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl CartridgeBeacon {
    pub const fn new(id: BeaconId, label: &'static str, x: usize, y: usize) -> Self {
        Self {
            id,
            label,
            x,
            y,
            width: 80,
            height: 80,
        }
    }

    pub fn contains(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < (self.x + self.width) && py >= self.y && py < (self.y + self.height)
    }

    /// Renders the beacon tile, including hover highlight if hovered.
    pub fn render(&self, prism: &mut PrismSurface, is_hovered: bool) {
        // 1. Draw hover background glass if pointer is over the beacon
        if is_hovered {
            prism.fill_rect(self.x, self.y, self.width, self.height, 0x001E_3A5F);
            prism.draw_rect_outline(self.x, self.y, self.width, self.height, 0x0038_BDF8);
        }

        // 2. Draw Application Emblem (32x32) at (self.x + 24, self.y + 10)
        let icon_x = self.x + 24;
        let icon_y = self.y + 10;

        match self.id {
            BeaconId::KinetixPC => {
                // KinetixPC Drive & Folder Icon: Amber/Cyan storage chassis
                prism.fill_rect(icon_x, icon_y + 4, 30, 24, 0x001E_293B);
                prism.draw_rect_outline(icon_x, icon_y + 4, 30, 24, 0x00FA_CC15);
                prism.fill_rect(icon_x, icon_y, 14, 6, 0x00FA_CC15); // Folder tab
                prism.fill_rect(icon_x + 4, icon_y + 11, 22, 4, 0x0038_BDF8); // Drive bay 1
                prism.fill_rect(icon_x + 4, icon_y + 18, 22, 4, 0x004A_DE80); // Drive bay 2
            }
            BeaconId::Terminal => {
                // Kinetix Terminal Icon: Slate box with green prompt >_
                prism.fill_rect(icon_x, icon_y + 2, 30, 26, 0x0008_0E18);
                prism.draw_rect_outline(icon_x, icon_y + 2, 30, 26, 0x0038_BDF8);
                prism.fill_rect(icon_x + 2, icon_y + 2, 26, 5, 0x001E_3A5F); // Window header
                // Prompt >_
                prism.put_pixel(icon_x + 6, icon_y + 14, 0x004A_DE80);
                prism.put_pixel(icon_x + 8, icon_y + 15, 0x004A_DE80);
                prism.put_pixel(icon_x + 6, icon_y + 16, 0x004A_DE80);
                prism.fill_rect(icon_x + 12, icon_y + 17, 6, 2, 0x004A_DE80);
            }
            BeaconId::Scribe => {
                // Scribe Document Icon: Clean document page with lines
                prism.fill_rect(icon_x, icon_y, 24, 30, 0x00F8_FAFC); // White paper
                prism.fill_rect(icon_x + 16, icon_y, 8, 8, 0x0094_A3B8); // Folded corner
                prism.fill_rect(icon_x + 4, icon_y + 12, 14, 2, 0x0002_84C7);
                prism.fill_rect(icon_x + 4, icon_y + 17, 16, 2, 0x0002_84C7);
                prism.fill_rect(icon_x + 4, icon_y + 22, 10, 2, 0x0002_84C7);
                prism.fill_rect(icon_x + 24, icon_y + 18, 6, 12, 0x00FA_CC15);
            }
            BeaconId::Telemetry => {
                // Telemetry CPU / Monitor Icon: Cyan pulse plate
                prism.fill_rect(icon_x, icon_y, 30, 26, 0x000C_2038);
                prism.draw_rect_outline(icon_x, icon_y, 30, 26, 0x0038_BDF8);
                prism.put_pixel(icon_x + 4, icon_y + 13, 0x0038_BDF8);
                prism.put_pixel(icon_x + 8, icon_y + 13, 0x0038_BDF8);
                prism.put_pixel(icon_x + 12, icon_y + 7, 0x0038_BDF8);
                prism.put_pixel(icon_x + 16, icon_y + 19, 0x0038_BDF8);
                prism.put_pixel(icon_x + 20, icon_y + 13, 0x0038_BDF8);
                prism.put_pixel(icon_x + 24, icon_y + 13, 0x0038_BDF8);
                prism.fill_rect(icon_x + 12, icon_y + 26, 6, 4, 0x0064_748B);
                prism.fill_rect(icon_x + 8, icon_y + 29, 14, 2, 0x0064_748B);
            }
            BeaconId::SpeedTest => {
                // Speed Test Speedometer Icon
                prism.fill_rect(icon_x, icon_y + 2, 30, 26, 0x0008_1626);
                prism.draw_rect_outline(icon_x, icon_y + 2, 30, 26, 0x0022_C55E);
                // Gauge arc ticks
                prism.fill_rect(icon_x + 4, icon_y + 8, 4, 4, 0x0038_BDF8);
                prism.fill_rect(icon_x + 13, icon_y + 5, 4, 4, 0x0022_C55E);
                prism.fill_rect(icon_x + 22, icon_y + 8, 4, 4, 0x00FA_CC15);
                // Needle
                prism.fill_rect(icon_x + 14, icon_y + 12, 3, 10, 0x00F8_7171);
            }
            BeaconId::Ouroboros => {
                // Ouroboros Arcade Snake Icon: Emerald serpentine loop
                prism.fill_rect(icon_x, icon_y, 30, 30, 0x000F_2A1D);
                prism.draw_rect_outline(icon_x, icon_y, 30, 30, 0x0022_C55E);
                prism.fill_rect(icon_x + 4, icon_y + 6, 18, 4, 0x004A_DE80);
                prism.fill_rect(icon_x + 18, icon_y + 6, 4, 14, 0x004A_DE80);
                prism.fill_rect(icon_x + 8, icon_y + 16, 14, 4, 0x004A_DE80);
                prism.fill_rect(icon_x + 8, icon_y + 10, 4, 8, 0x004A_DE80);
                prism.fill_rect(icon_x + 18, icon_y + 24, 4, 4, 0x00F8_7171);
            }
            BeaconId::Vault => {
                // Kinetix Vault Icon
                prism.fill_rect(icon_x, icon_y + 2, 30, 26, 0x0013_1C38);
                prism.draw_rect_outline(icon_x, icon_y + 2, 30, 26, 0x0038_BDF8);
                prism.fill_rect(icon_x + 4, icon_y + 8, 22, 6, 0x00C0_84FC);
                prism.fill_rect(icon_x + 11, icon_y + 16, 8, 8, 0x0038_BDF8);
            }
        }

        // 3. Label Text Centered underneath
        let label_len = self.label.len() * 8;
        let text_x = if self.width > label_len {
            self.x + (self.width - label_len) / 2
        } else {
            self.x + 2
        };
        let text_y = self.y + 54;
        let text_color = if is_hovered { 0x0038_BDF8 } else { 0x00F8_FAFC };

        prism.draw_text(text_x, text_y, self.label, text_color, None);
    }
}

pub struct BeaconManager {
    pub beacons: [CartridgeBeacon; 6],
    pub active_hover: Option<BeaconId>,
}

impl BeaconManager {
    pub const fn new() -> Self {
        Self {
            beacons: [
                CartridgeBeacon::new(BeaconId::KinetixPC,  "KinetixPC",  28, 30),
                CartridgeBeacon::new(BeaconId::Terminal,   "Terminal",   28, 120),
                CartridgeBeacon::new(BeaconId::Scribe,     "Scribe",     28, 210),
                CartridgeBeacon::new(BeaconId::Telemetry,  "Telemetry",  28, 300),
                CartridgeBeacon::new(BeaconId::SpeedTest,  "SpeedTest",  28, 390),
                CartridgeBeacon::new(BeaconId::Ouroboros,  "Ouroboros",  28, 480),
            ],
            active_hover: None,
        }
    }

    pub fn render(&mut self, prism: &mut PrismSurface, pointer_x: usize, pointer_y: usize) {
        self.active_hover = None;

        for beacon in self.beacons.iter() {
            let is_hovered = beacon.contains(pointer_x, pointer_y);
            if is_hovered {
                self.active_hover = Some(beacon.id);
            }
            beacon.render(prism, is_hovered);
        }
    }

    pub fn hit_test(&self, pointer_x: usize, pointer_y: usize) -> Option<BeaconId> {
        for beacon in self.beacons.iter() {
            if beacon.contains(pointer_x, pointer_y) {
                return Some(beacon.id);
            }
        }
        None
    }
}

pub static BEACONS: spin::Mutex<BeaconManager> = spin::Mutex::new(BeaconManager::new());
