# Kinetix OS — Ouroboros Arena: Interactive Snake Arcade Cartridge (Phase 21)

## 1. Overview & Architectural Scope

**Ouroboros Arena** ([`kernel/cartridges/ouroboros.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/cartridges/ouroboros.rs)) is the native graphical arcade cartridge in Kinetix OS. Built in pure Rust without external game engines or libc, it demonstrates real-time 2D grid rendering, directional vector physics, collision detection, food spawning via pseudo-random number generation, and state management inside a windowed desktop environment.

---

## 2. Window Chamber Geometry & Playing Grid

- **Window Dimensions**: $X = 300$, $Y = 120$, Width = $424$, Height = $460$.
- **Header**: 26 pixels tall styled in dark arcade slate (`#13203C`) displaying live score: `"Ouroboros Arena — Score: X"` and a crimson red close button `[X]` (`#DC2626`).
- **Playing Grid**: $20 \times 20$ cells.
  - Cell Dimensions: $19 \times 19$ pixels + 1-pixel cell margin ($20 \times 20 = 400 \times 400$ pixels).
  - Grid Offset: $X_{\text{origin}} = 312$, $Y_{\text{origin}} = 156$.
  - Grid Plate: Deep forest black (`#06140D`) framed by a 1-pixel emerald border (`#22C55E`).

---

## 3. Physics & Mechanics

### 3.1 Serpentine Body Representation
The snake body is represented as an ordered vector of grid coordinates (`Vec<GridPoint>`).
- **Head**: Lime green cell (`#4ADE80`) with dark 1-pixel eyes.
- **Body Segments**: Emerald green cells (`#22C55E`) with darker 1-pixel borders (`#15803D`).
- **Initial State**: 3 segments centered at $(10, 10)$, $(9, 10)$, and $(8, 10)$ moving `Right`.

### 3.2 Directional Control & 180° Reversal Prevention
- Movement directions: `Up`, `Down`, `Left`, `Right` (controlled via Arrow keys or `W`, `A`, `S`, `D`).
- **Self-Reversal Guard**: Immediate 180-degree turn requests (e.g. `Right` while moving `Left`) are discarded to prevent self-collision errors.

### 3.3 Collision Engine & Food Spawning
1. **Wall Collisions**: Moving past grid boundaries ($X < 0 \lor X \ge 20 \lor Y < 0 \lor Y \ge 20$) triggers `GameState::GameOver`.
2. **Self Collisions**: Running into any existing body segment triggers `GameState::GameOver`.
3. **Food Consumption**:
   - Spawns at unoccupied grid cells using an integer LCG pseudo-random generator ($R_{n+1} = (1664525 \cdot R_n + 1013904223) \pmod{2^{32}}$).
   - Rendered as a ruby red orb (`#EF4444`) with white specular glimmer.
   - Upon eating food: score increments by 10 points and snake length increases by 1 segment.

---

## 4. Game Over Overlay & Live Status

- **Game Over Modal**: Displays an illuminated dark glass plate across the center grid with bright crimson warning `"GAME OVER!"` and `"Press SPACE to Restart"` in radiant yellow (`#FACC15`).
- **Bottom Status Bar**: Displays live match telemetry: `Score: X  |  Length: Y  |  Speed: 10 ticks/s` in light emerald typography (`#86EFAC`).

---

## 5. Live VirtualBox Verification Evidence

From COM1 serial diagnostics capture (`artifacts/boot-images/serial.log`):

```text
================================================================================
             KINETIX OS OUROBOROS (CARTRIDGE II) VERIFICATION (PHASE 21)        
================================================================================
  [TEST 1] Verifying Ouroboros Arena Window Launch & Bounds...
  [PASS] Ouroboros arena launched at (300, 120) [424x460].
  [TEST 2] Verifying Initial Snake State & Food Placement...
  [PASS] Snake initialized: 3 segments, head at (10, 10), food at (19, 8).
  [TEST 3] Verifying Movement Dynamics & Directional Physics...
  [PASS] Movement steps and directional turns verified.
  [TEST 4] Verifying 180-Degree Reversal Prevention...
  [PASS] Instant self-reversal prevented.
  [TEST 5] Verifying Food Consumption & Growth...
  [PASS] Food consumed: score incremented to 10, length grew to 4.
  [TEST 6] Verifying Wall Collision & Game Over State...
  [PASS] Wall collision detected; Game Over state triggered.
  [TEST 7] Verifying Match Restart...
  [PASS] Game reset verified: state restored to Running with clean score.

[TEST] All Ouroboros (Cartridge II) Verification Tests PASSED!

[KERNEL] Phase 21 Milestone achieved: Cartridge II (Ouroboros Snake Arena) operational.
```
