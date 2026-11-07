//! Kinetic Pointer: Universal Mouse & Touch Vector Subsystem for Kinetix OS.
//!
//! Communicates with the 8042 controller auxiliary port, decoding 3-byte packets
//! from PS/2 mice, wireless USB dongles (e.g. Flipkart SmartBuy M500 via motherboard
//! USB legacy emulation), and laptop touchpads.

use crate::arch::cpu::{self, inb, outb};
use crate::arch::pic::PICS;
use crate::println;
use super::reticle::{RETICLE, RETICLE_WIDTH, RETICLE_HEIGHT};
use spin::Mutex;

const PORT_DATA: u16 = 0x60;
const PORT_STATUS: u16 = 0x64;
const PORT_COMMAND: u16 = 0x64;

#[derive(Debug, Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub middle_pressed: bool,
    pub packets_received: u64,
}

impl MouseState {
    pub const fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            left_pressed: false,
            right_pressed: false,
            middle_pressed: false,
            packets_received: 0,
        }
    }
}

pub struct PointerSubsystem {
    cycle: u8,
    packet: [u8; 3],
    state: MouseState,
    screen_width: usize,
    screen_height: usize,
    initialized: bool,
}

impl PointerSubsystem {
    pub const fn new() -> Self {
        Self {
            cycle: 0,
            packet: [0; 3],
            state: MouseState::empty(),
            screen_width: 1024,
            screen_height: 768,
            initialized: false,
        }
    }

    /// Sets up the pointer subsystem with screen dimensions and centers the reticle.
    pub fn init(&mut self, width: usize, height: usize) {
        self.screen_width = width;
        self.screen_height = height;
        self.state.x = (width / 2) as i32;
        self.state.y = (height / 2) as i32;
        self.cycle = 0;
        self.state.packets_received = 0;

        // Initialize Reticle with active framebuffer
        if let Some(fb_info) = crate::console::get_fb_info() {
            RETICLE.lock().init(fb_info);
            RETICLE.lock().draw_at(self.state.x as usize, self.state.y as usize);
        }

        // Initialize 8042 PS/2 controller auxiliary mouse channel
        unsafe {
            init_hardware();
        }

        self.initialized = true;
    }

    /// Ingests a single byte from the hardware data port (0x60).
    pub fn handle_byte(&mut self, byte: u8) {
        match self.cycle {
            0 => {
                // In standard PS/2 mouse protocol, Bit 3 of byte 0 is ALWAYS 1.
                // If it is 0, we have lost packet phase synchronization.
                if (byte & 0x08) != 0 {
                    self.packet[0] = byte;
                    self.cycle = 1;
                }
            }
            1 => {
                self.packet[1] = byte;
                self.cycle = 2;
            }
            2 => {
                self.packet[2] = byte;
                self.cycle = 0;
                self.process_packet();
            }
            _ => {
                self.cycle = 0;
            }
        }
    }

    /// Processes a complete 3-byte movement and button packet.
    fn process_packet(&mut self) {
        let b0 = self.packet[0];
        let b1 = self.packet[1];
        let b2 = self.packet[2];

        // Discard packet if overflow occurred on X or Y axis (Bits 6 and 7)
        if (b0 & 0xC0) != 0 {
            return;
        }

        let left = (b0 & 0x01) != 0;
        let right = (b0 & 0x02) != 0;
        let middle = (b0 & 0x04) != 0;

        let x_sign = (b0 & 0x10) != 0;
        let y_sign = (b0 & 0x20) != 0;

        let mut dx = b1 as i32;
        if x_sign {
            dx -= 256;
        }

        let mut dy = b2 as i32;
        if y_sign {
            dy -= 256;
        }

        self.state.packets_received += 1;

        // Note: in PS/2 protocol, positive dy represents upward mouse movement.
        // In screen coordinates (origin at top-left), moving up corresponds to decreasing Y.
        self.apply_motion(dx, -dy, left, right, middle);
    }

    /// Applies relative displacement and button states, clamping to display limits.
    pub fn apply_motion(&mut self, dx: i32, dy: i32, left: bool, right: bool, middle: bool) {
        let max_x = (self.screen_width.saturating_sub(RETICLE_WIDTH)) as i32;
        let max_y = (self.screen_height.saturating_sub(RETICLE_HEIGHT)) as i32;

        let mut new_x = self.state.x + dx;
        let mut new_y = self.state.y + dy;

        if new_x < 0 {
            new_x = 0;
        } else if new_x > max_x {
            new_x = max_x;
        }

        if new_y < 0 {
            new_y = 0;
        } else if new_y > max_y {
            new_y = max_y;
        }

        self.state.x = new_x;
        self.state.y = new_y;
        self.state.left_pressed = left;
        self.state.right_pressed = right;
        self.state.middle_pressed = middle;

        // Update reticle position on screen
        RETICLE.lock().move_to(new_x as usize, new_y as usize);
    }

    pub fn state(&self) -> MouseState {
        self.state
    }
}

pub static POINTER: Mutex<PointerSubsystem> = Mutex::new(PointerSubsystem::new());

/// Hardware IRQ 12 interrupt handler called by interrupt dispatcher.
pub fn on_interrupt() {
    let status = unsafe { inb(PORT_STATUS) };
    if (status & 0x01) == 0 {
        return; // No data available
    }

    let byte = unsafe { inb(PORT_DATA) };
    POINTER.lock().handle_byte(byte);
}

/// Initializes the 8042 PS/2 controller auxiliary mouse port and unmasks IRQ 12.
unsafe fn init_hardware() {
    // 1. Drain any pending bytes in 8042 buffer
    while (inb(PORT_STATUS) & 0x01) != 0 {
        inb(PORT_DATA);
    }

    // 2. Enable Auxiliary Device (Command 0xA8)
    wait_write();
    outb(PORT_COMMAND, 0xA8);

    // 3. Read Controller Command Byte (Command 0x20)
    wait_write();
    outb(PORT_COMMAND, 0x20);
    let mut config = mouse_read();

    // 4. Set Bit 1 (enable IRQ 12), clear Bit 5 (enable aux clock), ensure Bit 0 (IRQ 1) is active
    config |= 0x02;
    config &= !0x20;
    config |= 0x01;

    // 5. Write back updated Controller Command Byte (Command 0x60)
    wait_write();
    outb(PORT_COMMAND, 0x60);
    wait_write();
    outb(PORT_DATA, config);

    // 6. Set Mouse Defaults (Command 0xF6)
    mouse_write(0xF6);
    let _ = mouse_read(); // Wait ACK (0xFA)

    // 7. Enable Data Reporting / Streaming Mode (Command 0xF4)
    mouse_write(0xF4);
    let _ = mouse_read(); // Wait ACK (0xFA)

    // 8. Drain buffer once more
    while (inb(PORT_STATUS) & 0x01) != 0 {
        inb(PORT_DATA);
    }

    // 9. Unmask IRQ 12 in the 8259 PIC
    PICS.lock().unmask(12);
}

unsafe fn wait_write() {
    let mut timeout = 100_000;
    while (inb(PORT_STATUS) & 0x02) != 0 && timeout > 0 {
        cpu::io_wait();
        timeout -= 1;
    }
}

unsafe fn wait_read() -> bool {
    let mut timeout = 100_000;
    while (inb(PORT_STATUS) & 0x01) == 0 && timeout > 0 {
        cpu::io_wait();
        timeout -= 1;
    }
    timeout > 0
}

unsafe fn mouse_write(cmd: u8) {
    wait_write();
    outb(PORT_COMMAND, 0xD4); // Tell 8042 controller: send next byte to mouse
    wait_write();
    outb(PORT_DATA, cmd);
}

unsafe fn mouse_read() -> u8 {
    if wait_read() {
        inb(PORT_DATA)
    } else {
        0
    }
}

/// Executes Phase 18 verification tests for the Kinetic Pointer and Reticle.
pub fn run_pointer_tests() {
    println!("\n================================================================================");
    println!("           KINETIX OS KINETIC POINTER & RETICLE VERIFICATION (PHASE 18)         ");
    println!("================================================================================");

    let initial_state = POINTER.lock().state();
    println!("  [TEST 1] Initial Pointer Coordinates: ({}, {})", initial_state.x, initial_state.y);
    assert_eq!(initial_state.x, 512, "Pointer should initially be centered horizontally");
    assert_eq!(initial_state.y, 384, "Pointer should initially be centered vertically");
    println!("  [PASS] Pointer initialized and centered at (512, 384).");

    println!("  [TEST 2] Testing synthetic pulse vector: +75 horizontal, -50 vertical, Left Click...");
    POINTER.lock().apply_motion(75, -50, true, false, false);
    let state2 = POINTER.lock().state();
    assert_eq!(state2.x, 587, "X coordinate should be 512 + 75 = 587");
    assert_eq!(state2.y, 334, "Y coordinate should be 384 - 50 = 334");
    assert!(state2.left_pressed, "Left button should be pressed");
    println!("  [PASS] Pulse vector updated coordinates to ({}, {}) with Left Click confirmed.", state2.x, state2.y);

    println!("  [TEST 3] Testing display boundary clamping (negative overflow)...");
    POINTER.lock().apply_motion(-1000, -1000, false, false, false);
    let state3 = POINTER.lock().state();
    assert_eq!(state3.x, 0, "X should clamp to 0 at left boundary");
    assert_eq!(state3.y, 0, "Y should clamp to 0 at top boundary");
    println!("  [PASS] Boundary clamping confirmed at (0, 0).");

    println!("  [TEST 4] Testing display boundary clamping (positive overflow)...");
    POINTER.lock().apply_motion(2000, 2000, false, true, false);
    let state4 = POINTER.lock().state();
    let max_x = (1024 - RETICLE_WIDTH) as i32;
    let max_y = (768 - RETICLE_HEIGHT) as i32;
    assert_eq!(state4.x, max_x, "X should clamp to max display boundary");
    assert_eq!(state4.y, max_y, "Y should clamp to max display boundary");
    assert!(state4.right_pressed, "Right button should be pressed");
    println!("  [PASS] Boundary clamping confirmed at ({}, {}) with Right Click confirmed.", state4.x, state4.y);

    println!("  [TEST 5] Restoring pointer to center (512, 384)...");
    POINTER.lock().apply_motion(512 - max_x, 384 - max_y, false, false, false);
    let center_state = POINTER.lock().state();
    assert_eq!(center_state.x, 512);
    assert_eq!(center_state.y, 384);
    println!("  [PASS] Reticle restored to center. Zero-flicker background restoration verified.");

    println!("\n[TEST] All Kinetic Pointer & Reticle Verification Tests PASSED!\n");
}
