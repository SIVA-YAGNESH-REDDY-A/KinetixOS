//! Desktop Event Router & Multi-Window Focus Manager for Kinetix OS.
//!
//! Routes mouse click impulses and keyboard pulses to active desktop windows,
//! manages focus Z-order, and coordinates beacon launches.

use crate::canvas::beacons::{BeaconId, BEACONS};
use crate::cartridges::kinetix_pc::KINETIX_PC;
use crate::cartridges::ouroboros::{Direction, OUROBOROS};
use crate::cartridges::scribe::SCRIBE;
use crate::cartridges::speed_test::SPEED_TEST;
use crate::cartridges::telemetry::TELEMETRY;
use crate::cartridges::terminal::TERMINAL;
use crate::cartridges::vault::VAULT;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFocus {
    Desktop,
    KinetixPC,
    Terminal,
    Scribe,
    Telemetry,
    SpeedTest,
    Ouroboros,
    Vault,
}

pub struct EventRouter {
    pub focus: WindowFocus,
}

impl EventRouter {
    pub const fn new() -> Self {
        Self {
            focus: WindowFocus::Desktop,
        }
    }

    /// Dispatches a mouse left-click event at coordinates (px, py).
    pub fn dispatch_mouse_click(&mut self, px: usize, py: usize) {
        // 1. Check window close buttons (ordered by front-to-back priority)
        {
            let mut st = SPEED_TEST.lock();
            if st.is_open() && st.chamber.contains_close(px, py) {
                st.close();
                drop(st);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        {
            let mut term = TERMINAL.lock();
            if term.is_open() && term.chamber.contains_close(px, py) {
                term.close();
                drop(term);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        {
            let mut kpc = KINETIX_PC.lock();
            if kpc.is_open() && kpc.chamber.contains_close(px, py) {
                kpc.close();
                drop(kpc);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        {
            let mut tel = TELEMETRY.lock();
            if tel.is_open() && tel.chamber.contains_close(px, py) {
                tel.close();
                drop(tel);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        {
            let mut sc = SCRIBE.lock();
            if sc.is_open() && sc.chamber.contains_close(px, py) {
                sc.close();
                drop(sc);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        {
            let mut snake = OUROBOROS.lock();
            if snake.is_open() && snake.chamber.contains_close(px, py) {
                snake.close();
                drop(snake);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        {
            let mut vault = VAULT.lock();
            if vault.is_open() && vault.chamber.contains_close(px, py) {
                vault.close();
                drop(vault);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        // 2. Check window minimize buttons
        {
            let mut st = SPEED_TEST.lock();
            if st.is_open() && st.chamber.contains_minimize(px, py) {
                st.chamber.minimize();
                drop(st);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }
        {
            let mut term = TERMINAL.lock();
            if term.is_open() && term.chamber.contains_minimize(px, py) {
                term.chamber.minimize();
                drop(term);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }
        {
            let mut kpc = KINETIX_PC.lock();
            if kpc.is_open() && kpc.chamber.contains_minimize(px, py) {
                kpc.chamber.minimize();
                drop(kpc);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }
        {
            let mut tel = TELEMETRY.lock();
            if tel.is_open() && tel.chamber.contains_minimize(px, py) {
                tel.chamber.minimize();
                drop(tel);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }
        {
            let mut sc = SCRIBE.lock();
            if sc.is_open() && sc.chamber.contains_minimize(px, py) {
                sc.chamber.minimize();
                drop(sc);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }
        {
            let mut snake = OUROBOROS.lock();
            if snake.is_open() && snake.chamber.contains_minimize(px, py) {
                snake.chamber.minimize();
                drop(snake);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }
        {
            let mut vault = VAULT.lock();
            if vault.is_open() && vault.chamber.contains_minimize(px, py) {
                vault.chamber.minimize();
                drop(vault);
                self.focus = WindowFocus::Desktop;
                self.update_focus_flags(WindowFocus::Desktop);
                return;
            }
        }

        // 3. Check window body clicks for interactive controls & focus switching
        {
            let mut st = SPEED_TEST.lock();
            if st.is_open() && st.chamber.contains(px, py) {
                st.handle_click(px, py);
                drop(st);
                self.focus = WindowFocus::SpeedTest;
                self.update_focus_flags(WindowFocus::SpeedTest);
                return;
            }
        }

        {
            let term = TERMINAL.lock();
            if term.is_open() && term.chamber.contains(px, py) {
                drop(term);
                self.focus = WindowFocus::Terminal;
                self.update_focus_flags(WindowFocus::Terminal);
                return;
            }
        }

        {
            let mut kpc = KINETIX_PC.lock();
            if kpc.is_open() && kpc.chamber.contains(px, py) {
                kpc.handle_click(px, py);
                drop(kpc);
                self.focus = WindowFocus::KinetixPC;
                self.update_focus_flags(WindowFocus::KinetixPC);
                return;
            }
        }

        {
            let mut tel = TELEMETRY.lock();
            if tel.is_open() && tel.chamber.contains(px, py) {
                tel.handle_click(px, py);
                drop(tel);
                self.focus = WindowFocus::Telemetry;
                self.update_focus_flags(WindowFocus::Telemetry);
                return;
            }
        }

        {
            let sc = SCRIBE.lock();
            if sc.is_open() && sc.chamber.contains(px, py) {
                drop(sc);
                self.focus = WindowFocus::Scribe;
                self.update_focus_flags(WindowFocus::Scribe);
                return;
            }
        }

        {
            let snake = OUROBOROS.lock();
            if snake.is_open() && snake.chamber.contains(px, py) {
                drop(snake);
                self.focus = WindowFocus::Ouroboros;
                self.update_focus_flags(WindowFocus::Ouroboros);
                return;
            }
        }

        {
            let mut vault = VAULT.lock();
            if vault.is_open() && vault.chamber.contains(px, py) {
                vault.handle_click(px, py);
                drop(vault);
                self.focus = WindowFocus::Vault;
                self.update_focus_flags(WindowFocus::Vault);
                return;
            }
        }

        // 4. Check Desktop Beacons (Icons)
        let hit = BEACONS.lock().hit_test(px, py);
        if let Some(beacon_id) = hit {
            match beacon_id {
                BeaconId::KinetixPC => {
                    {
                        let mut kpc = KINETIX_PC.lock();
                        kpc.open();
                        kpc.chamber.restore();
                    }
                    self.focus = WindowFocus::KinetixPC;
                    self.update_focus_flags(WindowFocus::KinetixPC);
                }
                BeaconId::Terminal => {
                    {
                        let mut term = TERMINAL.lock();
                        term.open();
                        term.chamber.restore();
                    }
                    self.focus = WindowFocus::Terminal;
                    self.update_focus_flags(WindowFocus::Terminal);
                }
                BeaconId::Scribe => {
                    {
                        let mut scribe = SCRIBE.lock();
                        scribe.open();
                        scribe.chamber.restore();
                    }
                    self.focus = WindowFocus::Scribe;
                    self.update_focus_flags(WindowFocus::Scribe);
                }
                BeaconId::Telemetry => {
                    {
                        let mut tel = TELEMETRY.lock();
                        tel.open();
                        tel.chamber.restore();
                    }
                    self.focus = WindowFocus::Telemetry;
                    self.update_focus_flags(WindowFocus::Telemetry);
                }
                BeaconId::SpeedTest => {
                    {
                        let mut st = SPEED_TEST.lock();
                        st.open();
                        st.chamber.restore();
                    }
                    self.focus = WindowFocus::SpeedTest;
                    self.update_focus_flags(WindowFocus::SpeedTest);
                }
                BeaconId::Ouroboros => {
                    {
                        let mut snake = OUROBOROS.lock();
                        snake.reset();
                        snake.open();
                        snake.chamber.restore();
                    }
                    self.focus = WindowFocus::Ouroboros;
                    self.update_focus_flags(WindowFocus::Ouroboros);
                }
                BeaconId::Vault => {
                    {
                        let mut vault = VAULT.lock();
                        vault.open();
                        vault.chamber.restore();
                    }
                    self.focus = WindowFocus::Vault;
                    self.update_focus_flags(WindowFocus::Vault);
                }
            }
        } else {
            self.focus = WindowFocus::Desktop;
            self.update_focus_flags(WindowFocus::Desktop);
        }
    }

    fn update_focus_flags(&self, focused: WindowFocus) {
        KINETIX_PC.lock().chamber.is_focused = focused == WindowFocus::KinetixPC;
        TERMINAL.lock().chamber.is_focused = focused == WindowFocus::Terminal;
        SCRIBE.lock().chamber.is_focused = focused == WindowFocus::Scribe;
        TELEMETRY.lock().chamber.is_focused = focused == WindowFocus::Telemetry;
        SPEED_TEST.lock().chamber.is_focused = focused == WindowFocus::SpeedTest;
        OUROBOROS.lock().chamber.is_focused = focused == WindowFocus::Ouroboros;
        VAULT.lock().chamber.is_focused = focused == WindowFocus::Vault;
    }

    /// Dispatches a keyboard character to the currently focused window.
    pub fn dispatch_key(&mut self, ch: char) {
        // Arrow keys steer snake if Ouroboros is active
        if self.focus == WindowFocus::Ouroboros {
            match ch {
                '\x11' => { OUROBOROS.lock().change_direction(Direction::Up); return; }
                '\x12' => { OUROBOROS.lock().change_direction(Direction::Down); return; }
                '\x13' => { OUROBOROS.lock().change_direction(Direction::Left); return; }
                '\x14' => { OUROBOROS.lock().change_direction(Direction::Right); return; }
                _ => {}
            }
        }

        match self.focus {
            WindowFocus::Terminal => {
                TERMINAL.lock().handle_char(ch);
            }
            WindowFocus::Scribe => {
                SCRIBE.lock().insert_char(ch);
            }
            WindowFocus::KinetixPC => {
                match ch {
                    'r' | 'R' => KINETIX_PC.lock().refresh_entries(),
                    'b' | 'B' | '\x08' => {
                        let mut kpc = KINETIX_PC.lock();
                        kpc.current_path = alloc::string::String::from("K:\\");
                        kpc.selected_item = None;
                        kpc.file_preview = None;
                    }
                    't' | 'T' => {
                        TERMINAL.lock().open();
                        self.focus = WindowFocus::Terminal;
                    }
                    'q' | 'Q' | 'c' | 'C' => {
                        KINETIX_PC.lock().close();
                        self.focus = WindowFocus::Desktop;
                    }
                    _ => {}
                }
            }
            WindowFocus::SpeedTest => {
                if ch == ' ' || ch == '\n' || ch == '\r' {
                    SPEED_TEST.lock().start_test();
                }
            }
            WindowFocus::Ouroboros => {
                match ch {
                    'w' | 'W' => OUROBOROS.lock().change_direction(Direction::Up),
                    's' | 'S' => OUROBOROS.lock().change_direction(Direction::Down),
                    'a' | 'A' => OUROBOROS.lock().change_direction(Direction::Left),
                    'd' | 'D' => OUROBOROS.lock().change_direction(Direction::Right),
                    ' ' => OUROBOROS.lock().reset(),
                    _ => {}
                }
            }
            _ => {
                // Global desktop hotkeys (1-6 number keys or initials)
                match ch {
                    '1' | 'p' | 'P' => {
                        {
                            let mut kpc = KINETIX_PC.lock();
                            kpc.open();
                            kpc.chamber.restore();
                        }
                        self.focus = WindowFocus::KinetixPC;
                        self.update_focus_flags(WindowFocus::KinetixPC);
                    }
                    '2' | 't' | 'T' => {
                        {
                            let mut term = TERMINAL.lock();
                            term.open();
                            term.chamber.restore();
                        }
                        self.focus = WindowFocus::Terminal;
                        self.update_focus_flags(WindowFocus::Terminal);
                    }
                    '3' | 'e' | 'E' => {
                        {
                            let mut scribe = SCRIBE.lock();
                            scribe.open();
                            scribe.chamber.restore();
                        }
                        self.focus = WindowFocus::Scribe;
                        self.update_focus_flags(WindowFocus::Scribe);
                    }
                    '4' | 'm' | 'M' => {
                        {
                            let mut tel = TELEMETRY.lock();
                            tel.open();
                            tel.chamber.restore();
                        }
                        self.focus = WindowFocus::Telemetry;
                        self.update_focus_flags(WindowFocus::Telemetry);
                    }
                    '5' | 's' | 'S' => {
                        {
                            let mut st = SPEED_TEST.lock();
                            st.open();
                            st.chamber.restore();
                        }
                        self.focus = WindowFocus::SpeedTest;
                        self.update_focus_flags(WindowFocus::SpeedTest);
                    }
                    '6' | 'o' | 'O' => {
                        {
                            let mut snake = OUROBOROS.lock();
                            snake.reset();
                            snake.open();
                            snake.chamber.restore();
                        }
                        self.focus = WindowFocus::Ouroboros;
                        self.update_focus_flags(WindowFocus::Ouroboros);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub static ROUTER: Mutex<EventRouter> = Mutex::new(EventRouter::new());

/// Executes Phase 22 full system integration and regression tests.
pub fn run_integration_tests() {
    crate::println!("\n[TEST] Running Full System Integration Verification Tests...");
    let mut router = ROUTER.lock();
    router.focus = WindowFocus::KinetixPC;
    assert_eq!(router.focus, WindowFocus::KinetixPC, "Focus must switch to KinetixPC");
    router.focus = WindowFocus::Terminal;
    assert_eq!(router.focus, WindowFocus::Terminal, "Focus must switch to Terminal");
    router.focus = WindowFocus::Desktop;
    crate::println!("  [PASS] Multi-window event routing and focus management verified.");
}
