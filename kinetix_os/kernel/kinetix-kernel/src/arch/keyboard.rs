//! PS/2 Keyboard Driver & Scancode Set 1 Decoder for Kinetix OS.
//!
//! Handles IRQ 1 (Vector 33) keyboard interrupts, decodes make/break scancodes,
//! tracks Shift/Caps Lock modifier states, and pushes ASCII characters into
//! a synchronized thread-safe ring buffer.

use spin::Mutex;

const BUFFER_CAPACITY: usize = 256;

/// Circular ring buffer for keyboard characters.
struct RingBuffer {
    data: [char; BUFFER_CAPACITY],
    read_pos: usize,
    write_pos: usize,
    count: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            data: ['\0'; BUFFER_CAPACITY],
            read_pos: 0,
            write_pos: 0,
            count: 0,
        }
    }

    fn push(&mut self, c: char) -> bool {
        if self.count >= BUFFER_CAPACITY {
            return false; // Buffer full
        }
        self.data[self.write_pos] = c;
        self.write_pos = (self.write_pos + 1) % BUFFER_CAPACITY;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<char> {
        if self.count == 0 {
            return None;
        }
        let c = self.data[self.read_pos];
        self.read_pos = (self.read_pos + 1) % BUFFER_CAPACITY;
        self.count -= 1;
        Some(c)
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

pub struct KeyboardState {
    lshift: bool,
    rshift: bool,
    caps_lock: bool,
    ctrl: bool,
    alt: bool,
    buffer: RingBuffer,
}

impl KeyboardState {
    const fn new() -> Self {
        Self {
            lshift: false,
            rshift: false,
            caps_lock: false,
            ctrl: false,
            alt: false,
            buffer: RingBuffer::new(),
        }
    }

    fn shift(&self) -> bool {
        self.lshift || self.rshift
    }

    /// Handles a raw 8-bit scancode from I/O port 0x60.
    pub fn handle_scancode(&mut self, scancode: u8) {
        let is_break = (scancode & 0x80) != 0;
        let code = scancode & 0x7F;

        if is_break {
            match code {
                0x2A => self.lshift = false,
                0x36 => self.rshift = false,
                0x1D => self.ctrl = false,
                0x38 => self.alt = false,
                _ => {}
            }
        } else {
            match code {
                0x2A => self.lshift = true,
                0x36 => self.rshift = true,
                0x3A => self.caps_lock = !self.caps_lock,
                0x1D => self.ctrl = true,
                0x38 => self.alt = true,
                _ => {
                    if let Some(ch) = self.decode_ascii(code) {
                        self.buffer.push(ch);
                    }
                }
            }
        }
    }

    fn decode_ascii(&self, code: u8) -> Option<char> {
        let shift = self.shift();
        let use_upper = if self.caps_lock { !shift } else { shift };

        match code {
            0x01 => Some('\x1B'), // Escape
            0x02 => Some(if shift { '!' } else { '1' }),
            0x03 => Some(if shift { '@' } else { '2' }),
            0x04 => Some(if shift { '#' } else { '3' }),
            0x05 => Some(if shift { '$' } else { '4' }),
            0x06 => Some(if shift { '%' } else { '5' }),
            0x07 => Some(if shift { '^' } else { '6' }),
            0x08 => Some(if shift { '&' } else { '7' }),
            0x09 => Some(if shift { '*' } else { '8' }),
            0x0A => Some(if shift { '(' } else { '9' }),
            0x0B => Some(if shift { ')' } else { '0' }),
            0x0C => Some(if shift { '_' } else { '-' }),
            0x0D => Some(if shift { '+' } else { '=' }),
            0x0E => Some('\x08'), // Backspace
            0x0F => Some('\t'),   // Tab
            0x10 => Some(if use_upper { 'Q' } else { 'q' }),
            0x11 => Some(if use_upper { 'W' } else { 'w' }),
            0x12 => Some(if use_upper { 'E' } else { 'e' }),
            0x13 => Some(if use_upper { 'R' } else { 'r' }),
            0x14 => Some(if use_upper { 'T' } else { 't' }),
            0x15 => Some(if use_upper { 'Y' } else { 'y' }),
            0x16 => Some(if use_upper { 'U' } else { 'u' }),
            0x17 => Some(if use_upper { 'I' } else { 'i' }),
            0x18 => Some(if use_upper { 'O' } else { 'o' }),
            0x19 => Some(if use_upper { 'P' } else { 'p' }),
            0x1A => Some(if shift { '{' } else { '[' }),
            0x1B => Some(if shift { '}' } else { ']' }),
            0x1C => Some('\n'), // Enter
            0x1E => Some(if use_upper { 'A' } else { 'a' }),
            0x1F => Some(if use_upper { 'S' } else { 's' }),
            0x20 => Some(if use_upper { 'D' } else { 'd' }),
            0x21 => Some(if use_upper { 'F' } else { 'f' }),
            0x22 => Some(if use_upper { 'G' } else { 'g' }),
            0x23 => Some(if use_upper { 'H' } else { 'h' }),
            0x24 => Some(if use_upper { 'J' } else { 'j' }),
            0x25 => Some(if use_upper { 'K' } else { 'k' }),
            0x26 => Some(if use_upper { 'L' } else { 'l' }),
            0x27 => Some(if shift { ':' } else { ';' }),
            0x28 => Some(if shift { '"' } else { '\'' }),
            0x29 => Some(if shift { '~' } else { '`' }),
            0x2B => Some(if shift { '|' } else { '\\' }),
            0x2C => Some(if use_upper { 'Z' } else { 'z' }),
            0x2D => Some(if use_upper { 'X' } else { 'x' }),
            0x2E => Some(if use_upper { 'C' } else { 'c' }),
            0x2F => Some(if use_upper { 'V' } else { 'v' }),
            0x30 => Some(if use_upper { 'B' } else { 'b' }),
            0x31 => Some(if use_upper { 'N' } else { 'n' }),
            0x32 => Some(if use_upper { 'M' } else { 'm' }),
            0x33 => Some(if shift { '<' } else { ',' }),
            0x34 => Some(if shift { '>' } else { '.' }),
            0x35 => Some(if shift { '?' } else { '/' }),
            0x39 => Some(' '), // Spacebar
            0x48 => Some('\x11'), // Up Arrow
            0x50 => Some('\x12'), // Down Arrow
            0x4B => Some('\x13'), // Left Arrow
            0x4D => Some('\x14'), // Right Arrow
            _ => None,
        }
    }
}

pub static KEYBOARD: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());

/// Called by the IRQ 1 interrupt handler when a keyboard event occurs.
pub fn on_interrupt(scancode: u8) {
    KEYBOARD.lock().handle_scancode(scancode);
}

/// Retrieves the next available character from the keyboard buffer.
#[allow(dead_code)]
pub fn pop_char() -> Option<char> {
    KEYBOARD.lock().buffer.pop()
}

/// Checks whether any character is available in the keyboard buffer.
#[allow(dead_code)]
pub fn has_chars() -> bool {
    !KEYBOARD.lock().buffer.is_empty()
}

/// Runs verification tests for PS/2 Scancode Set 1 decoding and buffering.
pub fn run_keyboard_tests() {
    crate::println!("\n[TEST] Running PS/2 Keyboard Driver & Scancode Decoder Tests...");
    {
        let mut kb = KEYBOARD.lock();

        // Test 1: Simple letter make code ('a' = 0x1E)
        kb.handle_scancode(0x1E);
        assert_eq!(kb.buffer.pop(), Some('a'), "Scancode 0x1E must decode to 'a'");

        // Test 2: Shifted letter make code ('A' = Shift + 0x1E)
        kb.handle_scancode(0x2A); // Left Shift make
        kb.handle_scancode(0x1E); // 'a' make -> 'A'
        assert_eq!(kb.buffer.pop(), Some('A'), "Shift + 0x1E must decode to 'A'");
        kb.handle_scancode(0xAA); // Left Shift break

        // Test 3: Enter key ('\n' = 0x1C)
        kb.handle_scancode(0x1C);
        assert_eq!(kb.buffer.pop(), Some('\n'), "Scancode 0x1C must decode to '\\n'");

        // Test 4: Backspace key ('\x08' = 0x0E)
        kb.handle_scancode(0x0E);
        assert_eq!(kb.buffer.pop(), Some('\x08'), "Scancode 0x0E must decode to backspace");

        crate::println!("  [PASS] PS/2 Scancode Decoder (letters, shift modifiers, enter, backspace)");
    }
}
