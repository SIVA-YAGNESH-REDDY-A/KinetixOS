# Kinetix OS — Scribe Studio: Interactive Notes Chamber (Phase 20)

## 1. Overview & Architectural Scope

**Scribe Studio** ([`kernel/cartridges/scribe.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/cartridges/scribe.rs)) is the first native GUI productivity cartridge in Kinetix OS. It provides an interactive windowed text editor for capturing notes, ideas, and system scratchpad data directly on the desktop without host OS dependencies, libc, or POSIX runtimes.

---

## 2. Window Chamber Geometry & Aesthetics

Scribe executes inside a dedicated `FlowChamber` framed by modern glassmorphic styling:
- **Geometry**: $X = 180$, $Y = 100$, Width = $640$, Height = $480$.
- **Window Header**: 26 pixels tall in dark slate (`#13203C`) displaying title typography: `"Scribe Studio — Untitled Note"`.
- **Close Button**: Red square `[X]` (`#DC2626`) at the top-right corner ($X+616, Y+4, 18 \times 18$).
- **Canvas Body**: Deep dark slate plate (`#0F172A`) providing high contrast for comfortable long-form text editing.
- **Left Margin Gutter**: Vertical separator line at $X = 192$ (`#1E293B`) delineating line structure.

---

## 3. The Document Buffer & Editing Engine

The text editor maintains a dynamic `Vec<char>` document buffer with dual-axis cursor coordinates:
- **Maximum Columns per Line**: 72 characters ($72 \times 8 = 576$ pixels, fitting inside the 640-pixel window).
- **Line Height**: 18 pixels (16-pixel font + 2-pixel vertical leading).
- **Cursor Tracking**:
  - `cursor_col`: horizontal character column index.
  - `cursor_row`: vertical line index.

### 3.1 Editing Operations
1. **Character Insertion (`insert_char`)**:
   - Printable ASCII (`' '..='~'`) and non-control UTF-8 characters are appended to the buffer and `cursor_col` advances.
   - When `cursor_col >= MAX_COLS`, it wraps to the next line (`cursor_row += 1`, `cursor_col = 0`).
2. **Newline (`\n`)**:
   - Appends a newline to the buffer, resets `cursor_col = 0`, and increments `cursor_row`.
3. **Backspace Deletion (`delete_backspace`)**:
   - Deletes the character preceding the cursor.
   - If deleting across a newline, decrements `cursor_row` and recalculates `cursor_col` from the remaining line length.
4. **Typing Cursor**:
   - Rendered as a radiant electric cyan vertical line (`#38BDF8`, 2px wide, 16px tall) at $(X_{\text{base}} + \text{cursor\_col} \times 8, Y_{\text{base}} + \text{cursor\_row} \times 18)$.

---

## 4. Live Bottom Status Bar

The bottom 23 pixels of the window ($Y = 557..580$) display live document telemetry:
- Background: `#0B1120` with top separator line `#1E293B`.
- Formatted status: `Ln X, Col Y  |  Chars: Z  |  UTF-8` in muted slate `#64748B`.

---

## 5. Live VirtualBox Verification Evidence

From COM1 serial diagnostics capture (`artifacts/boot-images/serial.log`):

```text
================================================================================
               KINETIX OS SCRIBE (CARTRIDGE I) VERIFICATION (PHASE 20)          
================================================================================
  [TEST 1] Verifying Scribe Chamber Launch & Window Bounds...
  [PASS] Scribe chamber opened at (180, 100) [640x480].
  [TEST 2] Verifying Typing Ingestion & Multi-line Note Buffer...
  [PASS] Inserted 3 lines (107 characters). Content verified.
  [TEST 3] Verifying Backspace Deletion...
  [PASS] Backspace and character replacement confirmed.
  [TEST 4] Verifying Window Header Hit-Testing & Close Action...
  [PASS] Close button [X] hit-tested and window closed cleanly.
  [PASS] Scribe re-opened on desktop workspace.

[TEST] All Scribe (Cartridge I) Verification Tests PASSED!

[KERNEL] Phase 20 Milestone achieved: Cartridge I (Scribe Notes Chamber) operational.
```
