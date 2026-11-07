//! Scribe Studio: Native Interactive Notes Chamber for Kinetix OS.
//!
//! Provides a dedicated windowed text editor with live typing, backspace deletion,
//! multi-line document buffering, word wrapping, and cursor rendering.

use alloc::string::String;
use alloc::vec::Vec;
use crate::canvas::chamber::FlowChamber;
use crate::canvas::prism::PrismSurface;
use spin::Mutex;

pub const SCRIBE_X: usize = 180;
pub const SCRIBE_Y: usize = 100;
pub const SCRIBE_WIDTH: usize = 640;
pub const SCRIBE_HEIGHT: usize = 480;

const MAX_COLS: usize = 72; // Maximum characters per line before wrap
const LINE_HEIGHT: usize = 18;

pub struct ScribeChamber {
    pub chamber: FlowChamber,
    pub buffer: Vec<char>,
    pub cursor_col: usize,
    pub cursor_row: usize,
}

impl ScribeChamber {
    pub const fn new() -> Self {
        Self {
            chamber: FlowChamber::new(
                1,
                "Scribe Studio — Untitled Note",
                SCRIBE_X,
                SCRIBE_Y,
                SCRIBE_WIDTH,
                SCRIBE_HEIGHT,
                0x000F_172A, // Dark slate canvas
            ),
            buffer: Vec::new(),
            cursor_col: 0,
            cursor_row: 0,
        }
    }

    /// Opens the Scribe window on the desktop.
    pub fn open(&mut self) {
        self.chamber.is_open = true;
        self.chamber.is_focused = true;
    }

    /// Closes the Scribe window.
    pub fn close(&mut self) {
        self.chamber.is_open = false;
        self.chamber.is_focused = false;
    }

    /// Returns whether the Scribe window is currently open.
    pub fn is_open(&self) -> bool {
        self.chamber.is_open
    }

    /// Clears the document buffer and resets cursor position.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_col = 0;
        self.cursor_row = 0;
    }

    /// Inserts a character into the document buffer at the cursor.
    pub fn insert_char(&mut self, ch: char) {
        if ch == '\n' {
            self.buffer.push('\n');
            self.cursor_col = 0;
            self.cursor_row += 1;
            return;
        }

        if ch == '\x08' {
            self.delete_backspace();
            return;
        }

        // Printable characters
        if !ch.is_control() {
            self.buffer.push(ch);
            self.cursor_col += 1;
            if self.cursor_col >= MAX_COLS {
                self.cursor_col = 0;
                self.cursor_row += 1;
            }
        }
    }

    /// Deletes the character preceding the cursor.
    pub fn delete_backspace(&mut self) {
        if let Some(removed) = self.buffer.pop() {
            if removed == '\n' {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    // Recalculate column from previous line
                    let mut col = 0;
                    for &c in self.buffer.iter().rev() {
                        if c == '\n' {
                            break;
                        }
                        col += 1;
                    }
                    self.cursor_col = col % MAX_COLS;
                }
            } else if self.cursor_col > 0 {
                self.cursor_col -= 1;
            } else if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = MAX_COLS - 1;
            }
        }
    }

    /// Appends an entire string slice to the note.
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.insert_char(ch);
        }
    }

    /// Converts the current buffer contents into a String.
    pub fn text(&self) -> String {
        self.buffer.iter().collect()
    }

    /// Renders the complete Scribe window and text buffer into the Prism backbuffer.
    pub fn render(&self, prism: &mut PrismSurface) {
        if !self.chamber.is_open {
            return;
        }

        // 1. Draw outer window frame, header, and close button
        self.chamber.render(prism);

        // 2. Draw Left Margin Gutter Guide Line (X = SCRIBE_X + 12)
        let gutter_x = self.chamber.x + 12;
        let gutter_top = self.chamber.y + 30;
        let gutter_h = self.chamber.height.saturating_sub(60);
        prism.fill_rect(gutter_x, gutter_top, 1, gutter_h, 0x001E_293B);

        // 3. Render Text Buffer
        let mut col = 0;
        let mut row = 0;
        let text_base_x = self.chamber.x + 20;
        let text_base_y = self.chamber.y + 36;

        for &ch in self.buffer.iter() {
            if ch == '\n' {
                col = 0;
                row += 1;
                continue;
            }

            if col >= MAX_COLS {
                col = 0;
                row += 1;
            }

            let px = text_base_x + col * 8;
            let py = text_base_y + row * LINE_HEIGHT;

            // Stop rendering if exceeded window bottom
            if py + 16 >= (self.chamber.y + self.chamber.height.saturating_sub(30)) {
                break;
            }

            prism.draw_char(px, py, ch, 0x00F8_FAFC, None);
            col += 1;
        }

        // 4. Render Active Typing Cursor at (cursor_col, cursor_row)
        let cursor_px = text_base_x + self.cursor_col * 8;
        let cursor_py = text_base_y + self.cursor_row * LINE_HEIGHT;
        if cursor_py + 16 < (self.chamber.y + self.chamber.height.saturating_sub(30)) {
            // Radiant cyan vertical cursor line (2px wide, 16px tall)
            prism.fill_rect(cursor_px, cursor_py, 2, 16, 0x0038_BDF8);
        }

        // 5. Draw Window Bottom Status Bar
        let status_y = self.chamber.y + self.chamber.height.saturating_sub(24);
        prism.fill_rect(self.chamber.x + 1, status_y, self.chamber.width.saturating_sub(2), 23, 0x000B_1120);
        prism.fill_rect(self.chamber.x + 1, status_y, self.chamber.width.saturating_sub(2), 1, 0x001E_293B);

        let char_count = self.buffer.len();
        let mut status_buf = [0u8; 48];
        let status_str = format_status(&mut status_buf, self.cursor_row + 1, self.cursor_col + 1, char_count);
        prism.draw_text(self.chamber.x + 14, status_y + 4, status_str, 0x0064_748B, None);
    }
}

fn format_status<'a>(buf: &'a mut [u8], line: usize, col: usize, chars: usize) -> &'a str {
    let mut idx = 0;

    let p1 = b"Ln ";
    for &b in p1 { buf[idx] = b; idx += 1; }
    idx = append_num(buf, idx, line);

    let p2 = b", Col ";
    for &b in p2 { buf[idx] = b; idx += 1; }
    idx = append_num(buf, idx, col);

    let p3 = b"  |  Chars: ";
    for &b in p3 { buf[idx] = b; idx += 1; }
    idx = append_num(buf, idx, chars);

    let p4 = b"  |  UTF-8";
    for &b in p4 { buf[idx] = b; idx += 1; }

    unsafe { core::str::from_utf8_unchecked(&buf[..idx]) }
}

fn append_num(buf: &mut [u8], mut idx: usize, num: usize) -> usize {
    if num == 0 {
        buf[idx] = b'0';
        return idx + 1;
    }
    let mut temp = [0u8; 10];
    let mut count = 0;
    let mut n = num;
    while n > 0 {
        temp[count] = b'0' + (n % 10) as u8;
        n /= 10;
        count += 1;
    }
    for i in (0..count).rev() {
        buf[idx] = temp[i];
        idx += 1;
    }
    idx
}

pub static SCRIBE: Mutex<ScribeChamber> = Mutex::new(ScribeChamber::new());

/// Executes Phase 20 verification tests for the Scribe text editor chamber.
pub fn run_scribe_tests() {
    crate::println!("\n================================================================================");
    crate::println!("               KINETIX OS SCRIBE (CARTRIDGE I) VERIFICATION (PHASE 20)          ");
    crate::println!("================================================================================");

    let mut scribe = SCRIBE.lock();

    crate::println!("  [TEST 1] Verifying Scribe Chamber Launch & Window Bounds...");
    scribe.open();
    assert!(scribe.is_open(), "Scribe window must report is_open() == true");
    assert_eq!(scribe.chamber.width, SCRIBE_WIDTH);
    assert_eq!(scribe.chamber.height, SCRIBE_HEIGHT);
    crate::println!("  [PASS] Scribe chamber opened at ({}, {}) [{}x{}].", SCRIBE_X, SCRIBE_Y, SCRIBE_WIDTH, SCRIBE_HEIGHT);

    crate::println!("  [TEST 2] Verifying Typing Ingestion & Multi-line Note Buffer...");
    scribe.clear();
    scribe.insert_text("Kinetix OS - Sovereign 64-bit Desktop\n");
    scribe.insert_text("Note 1: Scribe Cartridge operational.\n");
    scribe.insert_text("Note 2: Pure Rust without libc.");
    let content = scribe.text();
    assert!(content.contains("Kinetix OS - Sovereign 64-bit Desktop"));
    assert!(content.contains("Note 1: Scribe Cartridge operational."));
    assert_eq!(scribe.cursor_row, 2);
    crate::println!("  [PASS] Inserted 3 lines ({} characters). Content verified.", content.len());

    crate::println!("  [TEST 3] Verifying Backspace Deletion...");
    let pre_len = scribe.buffer.len();
    scribe.delete_backspace(); // delete '.'
    assert_eq!(scribe.buffer.len(), pre_len - 1);
    scribe.insert_char('!');
    assert_eq!(scribe.buffer.last(), Some(&'!'));
    crate::println!("  [PASS] Backspace and character replacement confirmed.");

    crate::println!("  [TEST 4] Verifying Window Header Hit-Testing & Close Action...");
    assert!(scribe.chamber.contains_header(SCRIBE_X + 100, SCRIBE_Y + 10), "Header must contain click");
    let close_btn_x = SCRIBE_X + SCRIBE_WIDTH - 15;
    let close_btn_y = SCRIBE_Y + 10;
    assert!(scribe.chamber.contains_close(close_btn_x, close_btn_y), "Close button hit test must match");

    scribe.close();
    assert!(!scribe.is_open(), "Scribe must report is_open() == false after close");
    crate::println!("  [PASS] Close button [X] hit-tested and window closed cleanly.");

    // Re-open for user display on desktop
    scribe.open();
    crate::println!("  [PASS] Scribe re-opened on desktop workspace.");

    crate::println!("\n[TEST] All Scribe (Cartridge I) Verification Tests PASSED!\n");
}
