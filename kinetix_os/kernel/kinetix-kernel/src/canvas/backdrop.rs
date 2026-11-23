//! The Signature Backdrop: Procedural Sovereign Sapphire Bloom Synthesizer.
//!
//! Generates a smooth, elegant, high-definition wallpaper inspired by modern dark-mode
//! operating system aesthetics (deep sapphire night gradient with a central azure harmonic bloom).
//! Engineered with exact fixed-point Euclidean geometry to eliminate color cracking and banding.

use super::prism::PrismSurface;

// Fast integer sine lookup table: 256 entries representing 0..2*pi with amplitude 127
#[rustfmt::skip]
static SIN_LUT: [i32; 256] = [
      0,   3,   6,   9,  12,  15,  18,  21,  24,  28,  31,  34,  37,  40,  43,  46,
     48,  51,  54,  57,  60,  63,  65,  68,  71,  73,  76,  78,  81,  83,  85,  88,
     90,  92,  94,  96,  98, 100, 102, 104, 106, 108, 109, 111, 112, 114, 115, 117,
    118, 119, 120, 121, 122, 123, 124, 124, 125, 126, 126, 126, 127, 127, 127, 127,
    127, 127, 127, 127, 126, 126, 126, 125, 124, 124, 123, 122, 121, 120, 119, 118,
    117, 115, 114, 112, 111, 109, 108, 106, 104, 102, 100,  98,  96,  94,  92,  90,
     88,  85,  83,  81,  78,  76,  73,  71,  68,  65,  63,  60,  57,  54,  51,  48,
     46,  43,  40,  37,  34,  31,  28,  24,  21,  18,  15,  12,   9,   6,   3,   0,
     -3,  -6,  -9, -12, -15, -18, -21, -24, -28, -31, -34, -37, -40, -43, -46, -48,
    -51, -54, -57, -60, -63, -65, -68, -71, -73, -76, -78, -81, -83, -85, -88, -90,
    -92, -94, -96, -98,-100,-102,-104,-106,-108,-109,-111,-112,-114,-115,-117,-118,
   -119,-120,-121,-122,-123,-124,-124,-125,-126,-126,-126,-127,-127,-127,-127,-127,
   -127,-127,-127,-126,-126,-126,-125,-124,-124,-123,-122,-121,-120,-119,-118,-117,
   -115,-114,-112,-111,-109,-108,-106,-104,-102,-100, -98, -96, -94, -92, -90, -88,
    -85, -83, -81, -78, -76, -73, -71, -68, -65, -63, -60, -57, -54, -51, -48, -46,
    -43, -40, -37, -34, -31, -28, -24, -21, -18, -15, -12,  -9,  -6,  -3,   0,   0
];

#[inline(always)]
fn fast_sin(angle: i32) -> i32 {
    let a = (angle & 0xFF) as usize;
    SIN_LUT[a]
}

/// Computes the exact integer square root using binary digit restoration.
#[inline(always)]
fn isqrt(n: u64) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut op = n;
    let mut res = 0u64;
    let mut one = 1u64 << 62;
    while one > op {
        one >>= 2;
    }
    while one != 0 {
        if op >= res + one {
            op -= res + one;
            res = (res >> 1) + one;
        } else {
            res >>= 1;
        }
        one >>= 2;
    }
    res as u32
}

/// Generates the complete Kinetix OS desktop backdrop into the Prism backbuffer.
pub fn render_backdrop(prism: &mut PrismSurface) {
    let width = prism.width;
    let height = prism.height;
    let dock_y = height.saturating_sub(44).max(1);

    let center_x: i32 = (width / 2) as i32;
    let center_y: i32 = (dock_y / 2) as i32;
    let max_radius: i32 = ((core::cmp::min(width, dock_y) * 48) / 100).max(1) as i32;

    // Palette Colors:
    // 1. Midnight Sapphire Base: #0B132B -> #1A2644 -> #0E1626
    // 2. Radiant Azure Bloom:    #0369A1 -> #0284C7 -> #38BDF8

    for y in 0..height {
        if y >= dock_y {
            break;
        }

        let y_i32 = y as i32;
        let dy = y_i32 - center_y;
        let dy_sq = (dy as i64) * (dy as i64);

        // Smooth vertical ambient gradient (Dark Top -> Deep Navy Mid -> Obsidian Horizon)
        let t_norm = (y * 512) / dock_y; // 0..512
        let (base_r, base_g, base_b) = if t_norm < 256 {
            // Top half: 0x08, 0x10, 0x24 -> 0x14, 0x22, 0x42
            let u = t_norm as u32;
            let r = 8 + (12 * u) / 256;
            let g = 16 + (18 * u) / 256;
            let b = 36 + (30 * u) / 256;
            (r, g, b)
        } else {
            // Bottom half: 0x14, 0x22, 0x42 -> 0x0A, 0x12, 0x22
            let u = (t_norm - 256) as u32;
            let r = 20 - (10 * u) / 256;
            let g = 34 - (16 * u) / 256;
            let b = 66 - (32 * u) / 256;
            (r, g, b)
        };

        for x in 0..width {
            let x_i32 = x as i32;
            let dx = x_i32 - center_x;
            let dx_sq = (dx as i64) * (dx as i64);

            let dist_sq = dx_sq + dy_sq;
            let dist = isqrt(dist_sq as u64) as i32;

            if dist < max_radius {
                // Guaranteed strictly 0..255 linear falloff
                let linear_factor = (((max_radius - dist) as u32) * 255) / (max_radius as u32);
                // Quadratic smoothstep curve for feathering the outer boundary seamlessly
                let smooth_factor = (linear_factor * linear_factor) / 255;

                // Subtle harmonic ripple wave (amplitude -127..127)
                let wave_val = fast_sin((dist / 6) + (dx / 32));
                let wave_mod = ((wave_val + 128) as u32 * smooth_factor) / 768;

                // Center Bloom Core Color (Vibrant Royal Azure / Cyan)
                let core_r = 14u32;
                let core_g = (85 + wave_mod).min(180);
                let core_b = (185 + wave_mod * 2).min(245);

                // Smooth linear interpolation: base * (1 - t) + core * t
                let inv = 255 - smooth_factor;
                let r = ((base_r * inv) + (core_r * smooth_factor)) / 255;
                let g = ((base_g * inv) + (core_g * smooth_factor)) / 255;
                let b = ((base_b * inv) + (core_b * smooth_factor)) / 255;

                let color = (r << 16) | (g << 8) | b;
                prism.put_pixel(x, y, color);
            } else {
                let color = (base_r << 16) | (base_g << 8) | base_b;
                prism.put_pixel(x, y, color);
            }
        }
    }
}
