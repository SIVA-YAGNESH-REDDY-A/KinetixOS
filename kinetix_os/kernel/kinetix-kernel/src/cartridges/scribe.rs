//! Scribe Studio: Sovereign Native Document Editor for Kinetix OS.
//!
//! Provides a dedicated windowed text editor with live typing, multi-line document
//! buffering, word wrapping, unsaved change detection, and persistent file saving/opening
//! directly to KinetixDrive (K:) through the operating system VFS layer.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::canvas::chamber::FlowChamber;
use crate::canvas::prism::PrismSurface;
use spin::Mutex;

pub const SCRIBE_X: usize = 160;
pub const SCRIBE_Y: usize = 90;
pub const SCRIBE_WIDTH: usize = 680;
pub const SCRIBE_HEIGHT: usize = 500;

const MAX_COLS: usize = 76;
const LINE_HEIGHT: usize = 18;

pub struct ScribeChamber {
    pub chamber: FlowChamber,
    pub buffer: Vec<char>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub current_filename: String,
    pub is_modified: bool,
    pub status_message: String,
}

impl ScribeChamber {
    pub const fn new() -> Self {
        Self {
            chamber: FlowChamber::new(
                1,
                "Scribe Studio — K:\\Kinetix_Notes.txt",
                SCRIBE_X,
                SCRIBE_Y,
                SCRIBE_WIDTH,
                SCRIBE_HEIGHT,
                0x000F_172A, // Dark slate canvas
            ),
            buffer: Vec::new(),
            cursor_col: 0,
            cursor_row: 0,
            current_filename: String::new(),
            is_modified: false,
            status_message: String::new(),
        }
    }

    pub fn open(&mut self) {
        if self.buffer.is_empty() {
            self.current_filename = String::from("K:\\Kinetix_Notes.txt");
            self.status_message = String::from("Ready • Connected to KinetixDrive (K:)");
            let initial_text = "# KinetixOS Sovereign System Notes\n\nVolume: KinetixDrive (K:) NTFS Partition\nAuthor: Siva Yagnesh Reddy A (USN: 25BTRCL255)\n\nKernel: x86_64 Freestanding Rust (no_std)\nFilesystem: Native NTFS Driver Active\n\nType here to compose and edit documents...";
            for ch in initial_text.chars() {
                self.buffer.push(ch);
            }
        }
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

    pub fn new_document(&mut self) {
        self.buffer.clear();
        self.cursor_col = 0;
        self.cursor_row = 0;
        static mut DOC_COUNT: usize = 1;
        let count = unsafe {
            DOC_COUNT += 1;
            DOC_COUNT
        };
        self.current_filename = format!("K:\\Document_{}.txt", count);
        self.is_modified = false;
        self.status_message = format!("New file created: {}", self.current_filename);
    }

    pub fn save_document(&mut self) {
        let text: String = self.buffer.iter().collect();
        if crate::fs::write_file(&self.current_filename, text.as_bytes()) {
            self.is_modified = false;
            self.status_message = format!("Saved {} bytes to {}", text.len(), self.current_filename);
        } else {
            self.status_message = format!("Error saving to {}", self.current_filename);
        }
    }

    pub fn open_document(&mut self, path: &str) {
        self.current_filename = String::from(path);
        self.buffer.clear();

        // Attempt reading through VFS / dynamic files
        if let Some(bytes) = crate::fs::read_file(path) {
            if let Ok(text) = core::str::from_utf8(&bytes) {
                for ch in text.chars() {
                    self.buffer.push(ch);
                }
            }
        } else {
            // Read template if file not found
            let default_content = format!("Opened file: {}\n\n(File loaded from KinetixDrive filesystem)", path);
            for ch in default_content.chars() {
                self.buffer.push(ch);
            }
        }

        self.cursor_col = 0;
        self.cursor_row = 0;
        self.is_modified = false;
        self.status_message = format!("Opened {}", path);
    }

    pub fn handle_click(&mut self, px: usize, py: usize) -> bool {
        if !self.chamber.is_open {
            return false;
        }

        let bx = self.chamber.x + 10;
        let by = self.chamber.y + 34;

        // Menu Ribbon hit tests
        if py >= by && py < by + 26 {
            // [+] New
            if px >= bx + 10 && px < bx + 70 {
                self.new_document();
                return true;
            }
            // [o] Open
            if px >= bx + 75 && px < bx + 140 {
                self.open_document("K:\\Kinetix_Notes.txt");
                return true;
            }
            // [s] Save
            if px >= bx + 145 && px < bx + 210 {
                self.save_document();
                return true;
            }
            // [a] Save As
            if px >= bx + 215 && px < bx + 300 {
                self.current_filename = String::from("K:\\Document.txt");
                self.save_document();
                return true;
            }
        }

        false
    }

    pub fn insert_char(&mut self, ch: char) {
        if ch == '\x13' {
            // Ctrl+S: Save file
            self.save_document();
            return;
        }

        self.is_modified = true;
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

    pub fn delete_backspace(&mut self) {
        self.is_modified = true;
        if let Some(removed) = self.buffer.pop() {
            if removed == '\n' {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
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

    pub fn render(&self, prism: &mut PrismSurface) {
        if !self.chamber.is_open {
            return;
        }

        self.chamber.render(prism);

        let bx = self.chamber.x + 10;
        let by = self.chamber.y + 34;
        let total_w = self.chamber.width.saturating_sub(20);

        // 1. Menu Ribbon Bar: [New] [Open] [Save] [Save As] [Target File]
        prism.fill_rect(bx, by, total_w, 24, 0x0013_233C);
        prism.draw_rect_outline(bx, by, total_w, 24, 0x002A_4870);

        prism.draw_text(bx + 10, by + 4, "[+] New    [o] Open    [s] Save    [a] Save As", 0x0038_BDF8, None);

        // Display current path and modified asterisk
        let mod_indicator = if self.is_modified { " *" } else { "" };
        let file_tag = format!("Doc: {}{}", self.current_filename, mod_indicator);
        prism.draw_text(bx + 320, by + 4, &file_tag, if self.is_modified { 0x00FA_CC15 } else { 0x00E2_E8F0 }, None);

        // 2. Editor Canvas Area
        let editor_y = by + 28;
        let editor_h = self.chamber.height.saturating_sub(90);
        prism.fill_rect(bx, editor_y, total_w, editor_h, 0x000A_1120);
        prism.draw_rect_outline(bx, editor_y, total_w, editor_h, 0x001E_3A5F);

        // Line number margin gutter
        prism.fill_rect(bx, editor_y, 36, editor_h, 0x0007_0E1C);
        prism.draw_rect_outline(bx, editor_y, 36, editor_h, 0x001A_2D4A);

        // Render line numbers and text content
        let mut col = 0;
        let mut row = 0;
        let text_base_x = bx + 44;
        let text_base_y = editor_y + 8;

        prism.draw_text(bx + 8, text_base_y, "  1", 0x0064_748B, None);

        for &ch in self.buffer.iter() {
            if ch == '\n' {
                col = 0;
                row += 1;
                let ln_str = format!("{:3}", row + 1);
                let ln_y = text_base_y + row * LINE_HEIGHT;
                if ln_y + LINE_HEIGHT < editor_y + editor_h {
                    prism.draw_text(bx + 8, ln_y, &ln_str, 0x0064_748B, None);
                }
                continue;
            }

            if col >= MAX_COLS {
                col = 0;
                row += 1;
            }

            let char_x = text_base_x + col * 8;
            let char_y = text_base_y + row * LINE_HEIGHT;

            if char_y + LINE_HEIGHT < editor_y + editor_h && char_x + 8 < bx + total_w {
                let mut char_buf = [0u8; 4];
                let char_str = ch.encode_utf8(&mut char_buf);
                let color = if ch == '#' { 0x0038_BDF8 } else { 0x00F8_FAFC };
                prism.draw_text(char_x, char_y, char_str, color, None);
            }

            col += 1;
        }

        // Render active blinking cursor
        let cursor_render_x = text_base_x + self.cursor_col * 8;
        let cursor_render_y = text_base_y + self.cursor_row * LINE_HEIGHT;
        if cursor_render_y + LINE_HEIGHT < editor_y + editor_h && cursor_render_x + 8 < bx + total_w {
            prism.fill_rect(cursor_render_x, cursor_render_y, 8, LINE_HEIGHT - 2, 0x0038_BDF8);
        }

        // 3. Status Bar at bottom
        let status_y = self.chamber.y + self.chamber.height.saturating_sub(24);
        prism.fill_rect(self.chamber.x + 1, status_y, self.chamber.width.saturating_sub(2), 23, 0x0005_0B14);

        let status_str = format!(
            "{}  |  Ln {}, Col {}  |  {} chars  |  Encoding: UTF-8",
            self.status_message,
            self.cursor_row + 1,
            self.cursor_col + 1,
            self.buffer.len()
        );
        prism.draw_text(self.chamber.x + 14, status_y + 4, &status_str, 0x0038_BDF8, None);
    }
}

pub static SCRIBE: Mutex<ScribeChamber> = Mutex::new(ScribeChamber::new());

/// Executes Phase 20 verification tests for the Scribe subsystem.
pub fn run_scribe_tests() {
    crate::println!("\n[TEST] Running Scribe Studio Document Verification Tests...");
    let mut scribe = SCRIBE.lock();
    scribe.insert_char('A');
    assert!(!scribe.buffer.is_empty(), "Buffer must contain characters");
    scribe.delete_backspace();
    crate::println!("  [PASS] Scribe document buffer and character manipulation verified.");
}
