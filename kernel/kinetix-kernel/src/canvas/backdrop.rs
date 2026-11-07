//! The Signature Backdrop: Procedural Blue Bloom & Harmonic Wave Synthesizer.
//!
//! Generates the custom Kinetix OS desktop wallpaper inspired by the blue abstract
//! floral bloom aesthetic, rendered entirely in mathematical fixed-point code
//! with zero external image dependencies.

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

#[inline(always)]
fn fast_cos(angle: i32) -> i32 {
    fast_sin(angle + 64)
}

/// Generates the complete Kinetix OS desktop backdrop into the Prism backbuffer.
pub fn render_backdrop(prism: &mut PrismSurface) {
    let width = prism.width;
    let height = prism.height;

    let center_x: i32 = 540;
    let center_y: i32 = 420;

    for y in 0..height {
        // Stop rendering backdrop below taskbar horizon (y = 724)
        if y >= 724 {
            break;
        }

        let y_i32 = y as i32;

        // Base atmospheric background gradient: light sky mist -> deep marine blue
        let t = (y * 255) / 724;
        let base_r = (185 - (t * 145) / 255) as u32;
        let base_g = (210 - (t * 135) / 255) as u32;
        let base_b = (235 - (t * 95) / 255) as u32;

        for x in 0..width {
            let x_i32 = x as i32;

            let dx = x_i32 - center_x;
            let dy = y_i32 - center_y;

            // Approximate Euclidean distance: max(|dx|, |dy|) + min(|dx|, |dy|)/2
            let abs_dx = dx.abs();
            let abs_dy = dy.abs();
            let dist = core::cmp::max(abs_dx, abs_dy) + core::cmp::min(abs_dx, abs_dy) / 2;

            if dist < 360 {
                // Inside the central bloom zone: evaluate layered sinusoidal ribbon waves
                let angle_approx = ((dx * 64) / (dist + 1)) + ((dy * 64) / (dist + 1));
                let wave1 = fast_sin((dist / 3) + angle_approx * 2 + fast_sin(dx / 12));
                let wave2 = fast_cos((dist / 5) - angle_approx + fast_sin(dy / 16));
                let ribbon = (wave1 + wave2) / 2; // Range -127..127

                // Sculpt swirling petal ribbons
                let dist_u32 = dist as u32;
                let edge_factor = (360 - dist) as u32;

                if ribbon > 15 {
                    // Petal crest and highlight
                    let intensity = ((ribbon - 15) * 255) / 112; // 0..255
                    let bloom_r = (15 + (intensity * 45) / 255) as u32;
                    let bloom_g = (70 + (intensity * 125) / 255) as u32;
                    let bloom_b = (160 + (intensity * 95) / 255) as u32;

                    // Blend with distance falloff for soft edges
                    let r = ((bloom_r * edge_factor) + (base_r * dist_u32)) / 360;
                    let g = ((bloom_g * edge_factor) + (base_g * dist_u32)) / 360;
                    let b = ((bloom_b * edge_factor) + (base_b * dist_u32)) / 360;

                    let color = (r << 16) | (g << 8) | b;
                    prism.put_pixel(x, y, color);
                    continue;
                } else if ribbon < -20 {
                    // Petal deep blue shadow & crease
                    let shadow_r = 8u32;
                    let shadow_g = 25u32;
                    let shadow_b = 75u32;

                    let r = ((shadow_r * edge_factor) + (base_r * dist_u32)) / 360;
                    let g = ((shadow_g * edge_factor) + (base_g * dist_u32)) / 360;
                    let b = ((shadow_b * edge_factor) + (base_b * dist_u32)) / 360;

                    let color = (r << 16) | (g << 8) | b;
                    prism.put_pixel(x, y, color);
                    continue;
                } else {
                    // Midtone petal body: rich royal blue
                    let mid_r = 25u32;
                    let mid_g = 95u32;
                    let mid_b = 190u32;

                    let r = ((mid_r * edge_factor) + (base_r * dist_u32)) / 360;
                    let g = ((mid_g * edge_factor) + (base_g * dist_u32)) / 360;
                    let b = ((mid_b * edge_factor) + (base_b * dist_u32)) / 360;

                    let color = (r << 16) | (g << 8) | b;
                    prism.put_pixel(x, y, color);
                    continue;
                }
            }

            // Outside bloom: render base ambient sky gradient
            let color = (base_r << 16) | (base_g << 8) | base_b;
            prism.put_pixel(x, y, color);
        }
    }
}
