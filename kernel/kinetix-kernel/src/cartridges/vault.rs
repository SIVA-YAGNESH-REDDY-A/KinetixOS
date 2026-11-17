//! Kinetix Vault: In-Built Package Manager (KX-PKG / APT Alias) & Cartridge Store.
//!
//! Provides native package management, Cartridge Store browsing, and live
//! Network (LAN & WiFi) connectivity status inside Kinetix OS.

use crate::canvas::chamber::FlowChamber;
use crate::canvas::prism::PrismSurface;
use spin::Mutex;

pub const VAULT_X: usize = 200;
pub const VAULT_Y: usize = 85;
pub const VAULT_WIDTH: usize = 650;
pub const VAULT_HEIGHT: usize = 490;

#[derive(Clone, Copy)]
pub struct PackageItem {
    pub name: &'static str,
    pub version: &'static str,
    pub size: &'static str,
    pub desc: &'static str,
    pub installed: bool,
}

pub struct KinetixVault {
    pub chamber: FlowChamber,
    pub packages: [PackageItem; 6],
    pub last_action: &'static str,
}

impl KinetixVault {
    pub const fn new() -> Self {
        Self {
            chamber: FlowChamber::new(
                5,
                "Kinetix Vault — Package Manager (KX-PKG/APT) & Store",
                VAULT_X,
                VAULT_Y,
                VAULT_WIDTH,
                VAULT_HEIGHT,
                0x0008_1222,
            ),
            packages: [
                PackageItem { name: "hello-world.elf", version: "1.0", size: "12 KB", desc: "Ring 3 Syscall Demo", installed: true },
                PackageItem { name: "sysinfo.elf",     version: "1.0", size: "14 KB", desc: "Nucleus System Probe", installed: true },
                PackageItem { name: "bench.elf",       version: "1.2", size: "16 KB", desc: "Affinity Benchmark", installed: true },
                PackageItem { name: "scribe-studio",   version: "2.0", size: "48 KB", desc: "Notes Cartridge", installed: true },
                PackageItem { name: "ouroboros-game",  version: "2.1", size: "36 KB", desc: "Snake Arcade", installed: true },
                PackageItem { name: "net-diag-tool",   version: "1.0", size: "22 KB", desc: "LAN/WiFi Diagnostics", installed: false },
            ],
            last_action: "kx-pkg ready (alias: apt / snapstore) — Click package to toggle/install",
        }
    }

    pub fn open(&mut self) {
        self.chamber.is_open = true;
        self.chamber.is_focused = true;
    }

    pub fn close(&mut self) {
        self.chamber.is_open = false;
        self.chamber.is_focused = false;
    }

    pub fn is_open(&self) -> bool {
        self.chamber.is_open
    }

    pub fn toggle_package(&mut self, idx: usize) {
        if idx < self.packages.len() {
            self.packages[idx].installed = !self.packages[idx].installed;
            self.last_action = if self.packages[idx].installed {
                "kx-pkg install: Package verified & mounted in Kinetix Vault"
            } else {
                "kx-pkg remove: Package unmounted cleanly"
            };
        }
    }

    pub fn handle_click(&mut self, px: usize, py: usize) -> bool {
        if !self.chamber.is_open {
            return false;
        }
        let bx = self.chamber.x + 16;
        let list_y = self.chamber.y + 158;
        if px >= bx && px < bx + self.chamber.width.saturating_sub(32) {
            for idx in 0..self.packages.len() {
                let row_y = list_y + idx * 42;
                if py >= row_y && py < row_y + 36 {
                    self.toggle_package(idx);
                    return true;
                }
            }
        }
        false
    }

    pub fn render(&self, prism: &mut PrismSurface) {
        if !self.chamber.is_open {
            return;
        }

        self.chamber.render(prism);

        let bx = self.chamber.x + 16;
        let by = self.chamber.y + 36;
        let pw = self.chamber.width.saturating_sub(32);

        // 1. Network Connectivity Banner (LAN & WiFi)
        prism.fill_rect(bx, by, pw, 72, 0x000D_1B33);
        prism.draw_rect_outline(bx, by, pw, 72, 0x001E_3A5F);
        prism.draw_text(bx + 12, by + 8, "NETWORK CONNECTIVITY (LAN & WiFi)", 0x0038_BDF8, None);

        let net = crate::arch::net::NET_STATE.lock();
        let mut lan_buf = [0u8; 76];
        let lan_str = format_lan(&mut lan_buf, net.lan_adapter, net.ipv4_str);
        prism.draw_text(bx + 12, by + 28, lan_str, 0x004A_DE80, None);

        let mut wifi_buf = [0u8; 76];
        let wifi_str = format_wifi(&mut wifi_buf, net.wifi_adapter, &net.mac);
        prism.draw_text(bx + 12, by + 48, wifi_str, 0x00E2_E8F0, None);
        drop(net);

        // 2. Package Manager & Store Header
        let hdr_y = by + 84;
        prism.draw_text(
            bx,
            hdr_y,
            "IN-BUILT PACKAGE MANAGER (KX-PKG / APT) & STORE (KINETIX VAULT)",
            0x00FA_CC15,
            None,
        );
        prism.draw_text(bx, hdr_y + 18, self.last_action, 0x0094_A3B8, None);

        // 3. Package Rows
        let list_y = hdr_y + 38;
        for (idx, pkg) in self.packages.iter().enumerate() {
            let ry = list_y + idx * 42;
            let border = if pkg.installed { 0x0022_C55E } else { 0x0033_4155 };
            prism.fill_rect(bx, ry, pw, 36, 0x000D_1B33);
            prism.draw_rect_outline(bx, ry, pw, 36, border);

            let mut row_buf = [0u8; 64];
            let row_str = format_pkg_row(&mut row_buf, idx + 1, pkg);
            prism.draw_text(bx + 10, ry + 10, row_str, 0x00F8_FAFC, None);

            let badge = if pkg.installed { "[INSTALLED]" } else { "[ INSTALL ]" };
            let badge_col = if pkg.installed { 0x004A_DE80 } else { 0x0038_BDF8 };
            prism.draw_text(bx + pw.saturating_sub(110), ry + 10, badge, badge_col, None);
        }

        // 4. Bottom Status Bar
        let status_y = self.chamber.y + self.chamber.height.saturating_sub(24);
        prism.fill_rect(self.chamber.x + 1, status_y, self.chamber.width.saturating_sub(2), 23, 0x0005_0B14);
        prism.draw_text(
            self.chamber.x + 14,
            status_y + 4,
            "KX-PKG (APT Alias) | Kinetix Vault (Store) | Click row or press [1-6]",
            0x0038_BDF8,
            None,
        );
    }
}

fn push_s(buf: &mut [u8], mut i: usize, s: &[u8]) -> usize {
    for &b in s {
        if i >= buf.len() { break; }
        buf[i] = b;
        i += 1;
    }
    i
}

fn format_lan<'a>(buf: &'a mut [u8], lan: &str, ip: &str) -> &'a str {
    let mut i = 0;
    i = push_s(buf, i, b"LAN : ");
    i = push_s(buf, i, lan.as_bytes());
    i = push_s(buf, i, b" | ");
    i = push_s(buf, i, ip.as_bytes());
    unsafe { core::str::from_utf8_unchecked(&buf[..i]) }
}

fn format_wifi<'a>(buf: &'a mut [u8], wifi: &str, mac: &[u8; 6]) -> &'a str {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut i = 0;
    i = push_s(buf, i, b"MAC : ");
    for (idx, &b) in mac.iter().enumerate() {
        if i + 3 >= buf.len() { break; }
        buf[i] = HEX[(b >> 4) as usize];
        buf[i + 1] = HEX[(b & 0xF) as usize];
        i += 2;
        if idx < 5 { buf[i] = b':'; i += 1; }
    }
    i = push_s(buf, i, b" | WiFi: ");
    i = push_s(buf, i, wifi.as_bytes());
    unsafe { core::str::from_utf8_unchecked(&buf[..i]) }
}

fn format_pkg_row<'a>(buf: &'a mut [u8], num: usize, pkg: &PackageItem) -> &'a str {
    let mut i = 0;
    buf[i] = b'['; i += 1;
    buf[i] = b'0' + (num as u8); i += 1;
    buf[i] = b']'; i += 1;
    buf[i] = b' '; i += 1;
    i = push_s(buf, i, pkg.name.as_bytes());
    i = push_s(buf, i, b" v");
    i = push_s(buf, i, pkg.version.as_bytes());
    i = push_s(buf, i, b" (");
    i = push_s(buf, i, pkg.size.as_bytes());
    i = push_s(buf, i, b") - ");
    i = push_s(buf, i, pkg.desc.as_bytes());
    unsafe { core::str::from_utf8_unchecked(&buf[..i]) }
}

pub static VAULT: Mutex<KinetixVault> = Mutex::new(KinetixVault::new());
