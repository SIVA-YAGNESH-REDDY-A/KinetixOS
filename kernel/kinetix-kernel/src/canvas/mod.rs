//! The Canvas: Visual Presentation & Desktop Compositor Subsystem for Kinetix OS.
//!
//! Encompasses the Prism double-buffer surface compositor, procedural blue bloom
//! backdrop generator, Horizon taskbar dock, Cartridge beacons, and Flow chambers.

pub mod backdrop;
pub mod beacons;
pub mod chamber;
pub mod horizon;
pub mod prism;

use crate::println;
use kinetix_boot_protocol::FramebufferInfo;

#[allow(unused_imports)]
pub use beacons::{BeaconId, CartridgeBeacon, BEACONS};
#[allow(unused_imports)]
pub use chamber::FlowChamber;
#[allow(unused_imports)]
pub use horizon::{HorizonDock, HORIZON};
#[allow(unused_imports)]
pub use prism::{PrismSurface, PRISM};

/// Initializes the complete Canvas subsystem with the GOP linear framebuffer.
pub fn init(fb: FramebufferInfo) {
    let mut prism = PRISM.lock();
    prism.init(fb);

    println!("[CANVAS] Initialized Prism 32-bit double buffer ({}x{}).", prism.width, prism.height);
    println!("[CANVAS] Synthesizing procedural blue bloom backdrop...");
    backdrop::render_backdrop(&mut prism);

    println!("[CANVAS] Projecting Horizon dock and Cartridge beacons...");
    HORIZON.lock().render(&mut prism);
    BEACONS.lock().render(&mut prism, 512, 384);

    // Blit completed pristine scene to physical display
    prism.blit_to_screen();
    println!("[CANVAS] Pristine desktop surface composited to display.");
}

/// Re-composites the full desktop scene including open cartridge chambers.
pub fn compose_desktop() {
    let mut prism = PRISM.lock();
    backdrop::render_backdrop(&mut prism);
    HORIZON.lock().render(&mut prism);
    BEACONS.lock().render(&mut prism, 512, 384);
    crate::cartridges::scribe::SCRIBE.lock().render(&mut prism);
    crate::cartridges::ouroboros::OUROBOROS.lock().render(&mut prism);
    prism.blit_to_screen();
}

/// Executes Phase 19 verification tests for the Canvas and Horizon subsystem.
pub fn run_canvas_tests() {
    println!("\n================================================================================");
    println!("             KINETIX OS CANVAS & HORIZON VERIFICATION (PHASE 19)                ");
    println!("================================================================================");

    let prism = PRISM.lock();
    println!("  [TEST 1] Verifying Prism Double-Buffer Geometry...");
    assert_eq!(prism.width, 1024, "Surface width must match 1024");
    assert_eq!(prism.height, 768, "Surface height must match 768");
    assert!(prism.buffer.len() >= 1024 * 768, "Buffer must contain at least 786,432 pixels");
    println!("  [PASS] Prism double buffer verified: 1024x768x4 ({} bytes).", prism.buffer.len() * 4);

    println!("  [TEST 2] Verifying Procedural Blue Bloom Backdrop synthesis...");
    let sky_pixel = prism.get_pixel(100, 100);
    let bloom_pixel = prism.get_pixel(540, 420);
    assert_ne!(sky_pixel, 0, "Sky pixel must be non-zero");
    assert_ne!(bloom_pixel, 0, "Bloom center pixel must be non-zero");
    println!("  [PASS] Wallpaper synthesized: Sky pixel=0x{:08X}, Bloom pixel=0x{:08X}.", sky_pixel, bloom_pixel);

    println!("  [TEST 3] Verifying Horizon Dock layout & Chronometer...");
    let horizon_pixel = prism.get_pixel(512, 740);
    assert_eq!(horizon_pixel, prism.to_native(0x0017_2644), "Search pill pixel should match dock color");
    println!("  [PASS] Horizon dock verified at Y=724..768 with Kinetix spark emblem.");

    println!("  [TEST 4] Verifying Desktop Cartridge Beacons & Hover Hit-Testing...");
    let beacons = BEACONS.lock();
    let hit_scribe = beacons.hit_test(50, 60);
    assert_eq!(hit_scribe, Some(BeaconId::Scribe), "Hit test at (50, 60) should identify Scribe");
    let hit_ouroboros = beacons.hit_test(50, 160);
    assert_eq!(hit_ouroboros, Some(BeaconId::Ouroboros), "Hit test at (50, 160) should identify Ouroboros");
    let hit_telemetry = beacons.hit_test(50, 260);
    assert_eq!(hit_telemetry, Some(BeaconId::Telemetry), "Hit test at (50, 260) should identify Telemetry");
    let hit_empty = beacons.hit_test(500, 500);
    assert_eq!(hit_empty, None, "Hit test at (500, 500) should be empty");
    println!("  [PASS] All 3 Cartridge beacons (Scribe, Ouroboros, Telemetry) hit-testing verified.");

    println!("  [TEST 5] Verifying Flow Chamber window primitives & drag bounds...");
    let mut chamber = FlowChamber::new(1, "Test Window", 200, 150, 300, 200, 0x000F_172A);
    chamber.is_open = true;
    assert!(chamber.contains(250, 200), "Chamber body should contain (250, 200)");
    assert!(chamber.contains_header(250, 160), "Chamber header should contain (250, 160)");
    assert!(chamber.contains_close(chamber.x + chamber.width - 15, chamber.y + 10), "Chamber close button should match");
    assert!(!chamber.contains(100, 100), "Chamber should not contain out-of-bounds point");
    println!("  [PASS] Flow Chamber window frame, header, and close button hit-testing verified.");

    println!("\n[TEST] All Canvas & Horizon Desktop Verification Tests PASSED!\n");
}
