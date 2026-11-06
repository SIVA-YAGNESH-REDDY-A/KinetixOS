# Kinetix OS — Hardware Interaction Layer: Framebuffer Console & PS/2 Keyboard

**Document ID:** SPEC-KERNEL-HW-003  
**Status:** Implemented & Verified in VirtualBox  
**Target Hardware:** x86_64 UEFI Linear GOP Framebuffer & PS/2 Keyboard Controller (8042)  
**Implementation Language:** Pure Rust (`core`, `alloc`, `no_std`)  

---

## 1. Overview and Architecture

Phase 9 implements the hardware interaction and user interface layer of Kinetix OS, enabling graphical on-screen text rendering and interactive keyboard input:

```
+-------------------------------------------------------------------------+
|                  Dual Console Output Dispatcher                         |
|      - Formatted print! and println! output simultaneously sent to:     |
|         1. COM1 16550 UART Serial Port (0x3F8) for persistent logging   |
|         2. 1024x768 Linear Framebuffer Display for visual interaction   |
+-------------------------------------------------------------------------+
                                    |
           +------------------------+------------------------+
           |                                                 |
           v                                                 v
+--------------------------------------+   +--------------------------------------+
|    Linear Framebuffer Console        |   |         PS/2 Keyboard Driver         |
|  - 1024x768 @ 32 bpp (BGR/RGB)       |   |  - I/O Port 0x60 Data, IRQ 1 (vec 33)|
|  - Embedded 8x16 Monospace Font      |   |  - Scancode Set 1 State Machine      |
|  - 128 Columns x 48 Rows Text Grid   |   |  - Shift, Caps Lock, Ctrl, Alt       |
|  - \n, \r, \t, \x08 (Backspace)      |   |  - 256-entry Circular Ring Buffer    |
|  - Memory-Move Row Scrolling         |   |  - Non-blocking pop_char() API       |
+--------------------------------------+   +--------------------------------------+
```

---

## 2. Linear Framebuffer Console (`console/mod.rs` & `font.rs`)

### 2.1 Display Geometry & Color Encoding
- **Physical Address:** Dynamically discovered from UEFI GOP in `BootInfo` (`0x8000_0000`).
- **Resolution:** 1024 pixels wide $\times$ 768 pixels high.
- **Pixel Stride:** 1024 pixels per scanline (4096 bytes per row).
- **Pixel Format:** 32-bit BGR/RGB (`to_bgr` color conversion ensures consistent color appearance).
- **Default Theme:** Modern Dark Slate background (`0x0010_1826`) with bright off-white text (`0x00E2_E8F0`).

### 2.2 Embedded 8x16 Monospace Font
- Complete bitmap font table (`FONT_DATA`) embedded in `.rodata` for ASCII 32 (`' '`) through 126 (`'~'`).
- Each glyph is 16 rows of 8 bits ($8 \times 16 = 128$ bits = 16 bytes).
- Text Grid Dimensions:
  $$\text{Columns} = \frac{1024}{8} = 128 \quad\quad \text{Rows} = \frac{768}{16} = 48$$

### 2.3 Terminal Control Features
- **Newline (`\n`):** Resets column to 0, advances row by 1.
- **Carriage Return (`\r`):** Resets column to 0.
- **Tab (`\t`):** Advances column to the next multiple of 8.
- **Backspace (`\x08`):** Decrements column and clears character cell with background color.
- **Hardware-Accelerated Scrolling:** When the row reaches 48, scanlines $16 \dots 768$ are copied upward by 16 pixels via `core::ptr::copy`, and the bottom 16 scanlines are cleared with background color.

---

## 3. PS/2 Keyboard Driver (`arch/keyboard.rs`)

### 3.1 Scancode Set 1 Decoder
- Handles make (press, bit 7 = 0) and break (release, bit 7 = 1) scancodes from port `0x60`.
- **Modifier Tracking:**
  - Left Shift (`0x2A` press, `0xAA` release) & Right Shift (`0x36` press, `0xB6` release).
  - Caps Lock (`0x3A` press toggles uppercase state).
  - Ctrl (`0x1D` / `0x9D`) and Alt (`0x38` / `0xB8`).
- **Key Translation:** Maps alphanumeric keys, symbols, enter (`\n`), backspace (`\x08`), and space (`' '`) to ASCII characters according to current Shift/Caps Lock state.

### 3.2 Thread-Safe Input Buffering
- Typed characters are pushed into a 256-character circular ring buffer (`RingBuffer`) protected by `spin::Mutex`.
- Provides non-blocking `pop_char() -> Option<char>` and `has_chars() -> bool` for shell and application input.

---

## 4. Live VirtualBox Verification Evidence

Booting in Oracle VirtualBox 7.2 produced the following serial log:

```text
[TEST] Running Framebuffer Console & Font Rendering Tests...
  Console Display   : 1024x768 (Grid: 128 cols x 48 rows)
  [PASS] Framebuffer Console & 8x16 Monospace Font active.

[TEST] Running PS/2 Keyboard Driver & Scancode Decoder Tests...
  [PASS] PS/2 Scancode Decoder (letters, shift modifiers, enter, backspace)

[KERNEL] Phase 9 Milestone achieved: Hardware Interaction Layer operational.
```
