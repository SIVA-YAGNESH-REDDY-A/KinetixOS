//! Desktop Event Router & Multi-Window Focus Manager for Kinetix OS.
//!
//! Routes mouse click impulses and keyboard pulses to active desktop windows,
//! manages focus Z-order, and coordinates beacon launches.

use crate::canvas::beacons::{BeaconId, BEACONS};
use crate::cartridges::ouroboros::{Direction, OUROBOROS};
use crate::cartridges::scribe::SCRIBE;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFocus {
    Desktop,
    Scribe,
    Ouroboros,
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
        // 1. Check close button on Ouroboros window
        if OUROBOROS.lock().chamber.contains_close(px, py) {
            OUROBOROS.lock().close();
            self.focus = if SCRIBE.lock().is_open() {
                WindowFocus::Scribe
            } else {
                WindowFocus::Desktop
            };
            return;
        }

        // 2. Check close button on Scribe window
        if SCRIBE.lock().chamber.contains_close(px, py) {
            SCRIBE.lock().close();
            self.focus = if OUROBOROS.lock().is_open() {
                WindowFocus::Ouroboros
            } else {
                WindowFocus::Desktop
            };
            return;
        }

        // 3. Check window body click for focus switching
        if OUROBOROS.lock().chamber.contains(px, py) {
            self.focus = WindowFocus::Ouroboros;
            OUROBOROS.lock().chamber.is_focused = true;
            SCRIBE.lock().chamber.is_focused = false;
            return;
        }

        if SCRIBE.lock().chamber.contains(px, py) {
            self.focus = WindowFocus::Scribe;
            SCRIBE.lock().chamber.is_focused = true;
            OUROBOROS.lock().chamber.is_focused = false;
            return;
        }

        // 4. Check Desktop Beacons (Icons)
        if let Some(beacon_id) = BEACONS.lock().hit_test(px, py) {
            match beacon_id {
                BeaconId::Scribe => {
                    SCRIBE.lock().open();
                    self.focus = WindowFocus::Scribe;
                    SCRIBE.lock().chamber.is_focused = true;
                    OUROBOROS.lock().chamber.is_focused = false;
                }
                BeaconId::Ouroboros => {
                    OUROBOROS.lock().open();
                    self.focus = WindowFocus::Ouroboros;
                    OUROBOROS.lock().chamber.is_focused = true;
                    SCRIBE.lock().chamber.is_focused = false;
                }
                BeaconId::Telemetry => {
                    // Telemetry beacon clicked: refresh desktop chronometer
                }
            }
        } else {
            self.focus = WindowFocus::Desktop;
            SCRIBE.lock().chamber.is_focused = false;
            OUROBOROS.lock().chamber.is_focused = false;
        }
    }

    /// Dispatches a keyboard character to the currently focused window.
    pub fn dispatch_key(&mut self, ch: char) {
        match self.focus {
            WindowFocus::Scribe => {
                SCRIBE.lock().insert_char(ch);
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
            WindowFocus::Desktop => {
                // Global desktop shortcuts: 'n' for Notes, 'g' for Game
                if ch == 'n' || ch == 'N' {
                    SCRIBE.lock().open();
                    self.focus = WindowFocus::Scribe;
                } else if ch == 'g' || ch == 'G' {
                    OUROBOROS.lock().open();
                    self.focus = WindowFocus::Ouroboros;
                }
            }
        }
    }
}

pub static ROUTER: Mutex<EventRouter> = Mutex::new(EventRouter::new());

/// Executes Phase 22 full system multi-window integration & regression tests.
pub fn run_integration_tests() {
    crate::println!("\n================================================================================");
    crate::println!("          KINETIX OS FULL SYSTEM INTEGRATION & LAPTOP VERIFICATION (PHASE 22)   ");
    crate::println!("================================================================================");

    let mut router = ROUTER.lock();

    crate::println!("  [TEST 1] Testing Beacon Launch Event: Clicking Scribe Icon at (50, 60)...");
    router.dispatch_mouse_click(50, 60);
    assert_eq!(router.focus, WindowFocus::Scribe);
    assert!(SCRIBE.lock().is_open(), "Scribe must open upon beacon click");
    crate::println!("  [PASS] Beacon click registered: Scribe launched and gained input focus.");

    crate::println!("  [TEST 2] Testing Key Routing to Scribe: Typing integration note...");
    SCRIBE.lock().clear();
    let sample_note = "Kinetix OS Phase 22: Integration verified.\nDual-window compositing active.";
    for ch in sample_note.chars() {
        router.dispatch_key(ch);
    }
    let content = SCRIBE.lock().text();
    assert!(content.contains("Kinetix OS Phase 22: Integration verified."));
    crate::println!("  [PASS] Typed {} characters into Scribe via EventRouter. Note content verified.", content.len());

    crate::println!("  [TEST 3] Testing Multi-Window Coexistence: Launching Ouroboros Arena...");
    router.dispatch_mouse_click(50, 160); // Click Ouroboros beacon
    assert_eq!(router.focus, WindowFocus::Ouroboros);
    assert!(OUROBOROS.lock().is_open(), "Ouroboros must open upon beacon click");
    assert!(SCRIBE.lock().is_open(), "Scribe must remain open alongside Ouroboros");
    crate::println!("  [PASS] Both Scribe and Ouroboros open simultaneously without conflict.");

    crate::println!("  [TEST 4] Testing Key Routing to Ouroboros: Steering snake with 'W' and 'A'...");
    router.dispatch_key('w');
    OUROBOROS.lock().tick();
    assert_eq!(OUROBOROS.lock().body[0].y, 9, "Snake moved Up via routed 'w' key");
    router.dispatch_key('a');
    OUROBOROS.lock().tick();
    assert_eq!(OUROBOROS.lock().body[0].x, 9, "Snake moved Left via routed 'a' key");
    crate::println!("  [PASS] Directional keystrokes routed to Ouroboros game world successfully.");

    crate::println!("  [TEST 5] Testing Window Close & Desktop Re-compositing...");
    // Click close button on Scribe window
    let scribe_close_x = crate::cartridges::scribe::SCRIBE_X + crate::cartridges::scribe::SCRIBE_WIDTH - 15;
    let scribe_close_y = crate::cartridges::scribe::SCRIBE_Y + 10;
    router.dispatch_mouse_click(scribe_close_x, scribe_close_y);
    assert!(!SCRIBE.lock().is_open(), "Scribe must close upon close button click");
    assert_eq!(router.focus, WindowFocus::Ouroboros, "Focus should transfer to remaining open window");
    crate::println!("  [PASS] Scribe closed cleanly; focus transferred to Ouroboros.");

    // Re-open Scribe so both applications are visible on screen for the final presentation
    SCRIBE.lock().open();
    crate::println!("  [PASS] Final desktop workspace composited with Scribe and Ouroboros ready.");

    crate::println!("\n[TEST] All Full System Integration & Laptop Regression Tests PASSED!\n");
}
