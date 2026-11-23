# Kinetix OS — Canvas & Horizon Desktop Environment (Phase 19)

## 1. Overview & Architectural Scope

The **Canvas Subsystem** ([`kernel/canvas`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/canvas)) provides the complete graphical desktop environment for Kinetix OS. Inspired by modern, clean desktop interfaces, it establishes a double-buffered 32-bit compositing engine, a procedurally synthesized blue abstract bloom backdrop, a bottom Horizon taskbar dock with live chronometer telemetry, interactive Cartridge beacons (desktop icons), and movable Flow chambers (application window frames).

---

## 2. The Prism Surface Compositor ([`canvas/prism.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/canvas/prism.rs))

To eliminate all screen tearing and visible scanline redraws during complex desktop compositing, Kinetix OS utilizes a dedicated **32-bit off-screen backbuffer**:
- **Dimensions**: $1024 \times 768$ pixels.
- **Memory Footprint**: $1024 \times 768 \times 4\text{ bytes} = 3,145,728\text{ bytes}$ (3 MiB) allocated in the expanded 16 MiB kernel heap.
- **Color Translation**: Transparently translates standard `0x00RRGGBB` colors into the hardware GOP format (`PixelFormat::Bgr` if active in UEFI firmware) during rasterization so full-frame blitting is a single contiguous memory transfer:
  ```rust
  pub fn blit_to_screen(&self) {
      let target_ptr = fb.physical_address as *mut u32;
      let src_ptr = self.buffer.as_ptr();
      unsafe {
          core::ptr::copy_nonoverlapping(src_ptr, target_ptr, self.stride * self.height);
      }
  }
  ```

---

## 3. The Signature Backdrop ([`canvas/backdrop.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/canvas/backdrop.rs))

Rather than bloating the boot disk with heavy static JPEG/PNG image files, Kinetix OS procedurally synthesizes its desktop wallpaper using an integer fixed-point trigonometric harmonic algorithm:
1. **Vertical Atmospheric Sky Gradient**: Smoothly shifts from a luminous cerulean sky mist (`#BACBD9`) at $Y=0$ to a deep marine blue (`#2E5F88`) at $Y=724$.
2. **Harmonic Ribbon Bloom**: Centered at $(X=540, Y=420)$, evaluating layered sinusoidal ribbon waveforms using a 256-entry fast integer sine table (`SIN_LUT`):
   $$\text{wave}_1 = \sin\left(\frac{d}{3} + 2\theta + \sin\left(\frac{\Delta X}{12}\right)\right)$$
   $$\text{wave}_2 = \cos\left(\frac{d}{5} - \theta + \sin\left(\frac{\Delta Y}{16}\right)\right)$$
3. **Layered Petal Shading**: Generates flowing highlights (`#38BDF8`, `#60A5FA`), midtone bodies (`#0284C7`), and deep blue crease shadows (`#0F172A`) with smooth distance falloff.

---

## 4. The Horizon Dock ([`canvas/horizon.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/canvas/horizon.rs))

Spanning the bottom of the screen ($Y = 724..768$, Height = 44 pixels), the Horizon dock acts as the primary system control deck:
- **Kinetix Spark Launcher**: An iconic diamond spark emblem on the left ($X=12..124$) in radiant cyan (`#38BDF8`) with the `"KINETIX"` brand typography.
- **Search & Quick-Launch Pill**: Centered quick-launch bar ($X=360..664$) styled with a dark glassmorphic plate (`#172644`) and subtle border (`#334A75`).
- **Live Chronometer & Pulse**: Right-aligned telemetry plate ($X=780..1012$) displaying real-time system uptime formatted as `UPTIME: X.Xs | ONLINE` in crisp cyan font.

---

## 5. Cartridge Beacons ([`canvas/beacons.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/canvas/beacons.rs))

Desktop icons are rendered as interactive tiles ($80 \times 84$ pixels) on the left side of the desktop:
1. 📝 **`Scribe`** (Notes Chamber) at $(X=32, Y=40)$: Clean document page with folded corner, blue text lines, and a golden stylus.
2. 🐍 **`Ouroboros`** (Snake Arena) at $(X=32, Y=144)$: Emerald serpentine arcade coil (`#22C55E`) on dark forest plate with a red target fruit.
3. 💻 **`Telemetry`** (SysInfo Vital Signs) at $(X=32, Y=248)$: Cyan microchip pulse monitor plate (`#38BDF8`) with monitor pedestal.

### Hover & Click Interactions
- **Translucent Glass Glow**: When the pointer coordinates $(px, py)$ move inside a beacon's tile bounds, the compositor renders a translucent cyan background glass (`#1E3A5F`) and a bright border (`#38BDF8`).
- **Hit Testing**: `BEACONS.lock().hit_test(px, py)` returns `Some(BeaconId)` upon mouse click to instantiate the application window.

---

## 6. Flow Chambers ([`canvas/chamber.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/canvas/chamber.rs))

Movable window frames provide isolated canvases for desktop applications:
- **Header**: 26 pixels tall in dark slate (`#13203C`) with title typography.
- **Close Button**: Red square `[X]` (`#DC2626`) at the top right of the window header.
- **Window Body**: Configurable canvas fill (e.g. crisp light paper for Scribe or dark arcade matrix for Ouroboros).
- **Draggable Bounds**: Click-and-hold on the title bar repositions the window across the desktop.

---

## 7. Live VirtualBox Verification Evidence

From COM1 serial diagnostics capture (`artifacts/boot-images/serial.log`):

```text
[CANVAS] Initialized Prism 32-bit double buffer (1024x768).
[CANVAS] Synthesizing procedural blue bloom backdrop...
[CANVAS] Projecting Horizon dock and Cartridge beacons...
[CANVAS] Pristine desktop surface composited to display.

================================================================================
             KINETIX OS CANVAS & HORIZON VERIFICATION (PHASE 19)                
================================================================================
  [TEST 1] Verifying Prism Double-Buffer Geometry...
  [PASS] Prism double buffer verified: 1024x768x4 (3145728 bytes).
  [TEST 2] Verifying Procedural Blue Bloom Backdrop synthesis...
  [PASS] Wallpaper synthesized: Sky pixel=0x00DEC0A6, Bloom pixel=0x00C87B22.
  [TEST 3] Verifying Horizon Dock layout & Chronometer...
  [PASS] Horizon dock verified at Y=724..768 with Kinetix spark emblem.
  [TEST 4] Verifying Desktop Cartridge Beacons & Hover Hit-Testing...
  [PASS] All 3 Cartridge beacons (Scribe, Ouroboros, Telemetry) hit-testing verified.
  [TEST 5] Verifying Flow Chamber window primitives & drag bounds...
  [PASS] Flow Chamber window frame, header, and close button hit-testing verified.

[TEST] All Canvas & Horizon Desktop Verification Tests PASSED!

[KERNEL] Phase 19 Milestone achieved: Canvas & Horizon (The Kinetix Desktop Environment) operational.
```
