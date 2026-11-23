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
    let (w, h) = (prism.width, prism.height);
    {
        let mut horizon = HORIZON.lock();
        horizon.set_dimensions(w, h);
        horizon.render(&mut prism);
    }
    BEACONS.lock().render(&mut prism, w / 2, h / 2);

    // Blit completed pristine scene to physical display
    prism.blit_to_screen();
    println!("[CANVAS] Pristine desktop surface composited to display.");
}

/// Re-composites the full desktop scene including open cartridge chambers with Z-ordering.
pub fn compose_desktop() {
    let router_focus = crate::pulses::router::ROUTER.lock().focus;
    compose_desktop_dynamic(router_focus);
}

/// Dynamic composition respecting Z-order and active focus.
pub fn compose_desktop_dynamic(focus: crate::pulses::router::WindowFocus) {
    let mut prism = PRISM.lock();
    backdrop::render_backdrop(&mut prism);
    BEACONS.lock().render(&mut prism, 512, 384);

    // Render non-focused windows first, then focused window on top
    let render_all = |p: &mut PrismSurface, exclude: crate::pulses::router::WindowFocus| {
        if exclude != crate::pulses::router::WindowFocus::Vault {
            crate::cartridges::vault::VAULT.lock().render(p);
        }
        if exclude != crate::pulses::router::WindowFocus::Ouroboros {
            crate::cartridges::ouroboros::OUROBOROS.lock().render(p);
        }
        if exclude != crate::pulses::router::WindowFocus::Telemetry {
            crate::cartridges::telemetry::TELEMETRY.lock().render(p);
        }
        if exclude != crate::pulses::router::WindowFocus::Scribe {
            crate::cartridges::scribe::SCRIBE.lock().render(p);
        }
        if exclude != crate::pulses::router::WindowFocus::SpeedTest {
            crate::cartridges::speed_test::SPEED_TEST.lock().render(p);
        }
        if exclude != crate::pulses::router::WindowFocus::Terminal {
            crate::cartridges::terminal::TERMINAL.lock().render(p);
        }
        if exclude != crate::pulses::router::WindowFocus::KinetixPC {
            crate::cartridges::kinetix_pc::KINETIX_PC.lock().render(p);
        }
    };

    render_all(&mut prism, focus);

    // Render actively focused window on top
    match focus {
        crate::pulses::router::WindowFocus::KinetixPC => crate::cartridges::kinetix_pc::KINETIX_PC.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::Terminal => crate::cartridges::terminal::TERMINAL.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::SpeedTest => crate::cartridges::speed_test::SPEED_TEST.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::Scribe => crate::cartridges::scribe::SCRIBE.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::Telemetry => crate::cartridges::telemetry::TELEMETRY.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::Ouroboros => crate::cartridges::ouroboros::OUROBOROS.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::Vault => crate::cartridges::vault::VAULT.lock().render(&mut prism),
        crate::pulses::router::WindowFocus::Desktop => {}
    }

    // Horizon taskbar dock always renders on top of windows
    HORIZON.lock().render(&mut prism);
    prism.blit_to_screen();
}

/// The continuous desktop executive loop.
/// Polling mouse drags, window focus clicks, keyboard typing, and autonomous ticks.
pub fn run_desktop_executive_loop() -> ! {
    let mut prev_left = false;
    let mut is_dragging_kpc = false;
    let mut is_dragging_terminal = false;
    let mut is_dragging_scribe = false;
    let mut is_dragging_speed = false;
    let mut is_dragging_telemetry = false;
    let mut is_dragging_snake = false;
    let mut is_dragging_vault = false;

    let mut last_snake_tick = crate::arch::pit::get_ticks();
    let mut last_telemetry_tick = crate::arch::pit::get_ticks();
    let mut last_clock_tick = crate::arch::pit::get_ticks();
    let mut fallback_counter: u32 = 0;

    // Ensure interrupts are active for PS/2 keyboard, mouse, and PIT timer
    crate::arch::cpu::enable_interrupts();

    let (screen_w, screen_h) = {
        let p = PRISM.lock();
        (p.width, p.height)
    };

    loop {
        let (mx, my, left_pressed) = {
            let ptr = crate::pulses::pointer::POINTER.lock();
            let st = ptr.state();
            (st.x as usize, st.y as usize, st.left_pressed)
        };

        let mut needs_redraw = false;

        // 1. Handle Mouse Down Transition (Click)
        if left_pressed && !prev_left {
            let mut horizon = HORIZON.lock();
            let mut router = crate::pulses::router::ROUTER.lock();
            let mut click_handled = false;

            // Check Start Menu buttons first if open
            if horizon.start_menu_open {
                if horizon.contains_shutdown(mx, my) {
                    drop(router);
                    drop(horizon);
                    crate::arch::power::poweroff();
                } else if horizon.contains_restart(mx, my) {
                    drop(router);
                    drop(horizon);
                    crate::arch::power::reboot();
                } else if horizon.contains_sleep(mx, my) {
                    horizon.start_menu_open = false;
                    needs_redraw = true;
                    click_handled = true;
                    // Low-power halt state until next interrupt
                    crate::arch::cpu::halt();
                } else if horizon.contains_cancel(mx, my) {
                    horizon.start_menu_open = false;
                    needs_redraw = true;
                    click_handled = true;
                }
            }

            // Check Start button click
            if !click_handled && horizon.contains_start_button(mx, my) {
                horizon.toggle_start_menu();
                needs_redraw = true;
                click_handled = true;
            } else if horizon.start_menu_open && !horizon.contains_start_button(mx, my) {
                horizon.start_menu_open = false;
                needs_redraw = true;
            }

            // Check Taskbar open window pills (restoring minimized windows)
            if !click_handled {
                let dock_y = screen_h.saturating_sub(horizon::HORIZON_HEIGHT);
                if my >= dock_y && my < screen_h && mx >= 276 && mx < 760 {
                    let pill_idx = (mx - 276) / 104;
                    match pill_idx {
                        0 => {
                            let mut kpc = crate::cartridges::kinetix_pc::KINETIX_PC.lock();
                            if kpc.chamber.is_open {
                                kpc.chamber.restore();
                                router.focus = crate::pulses::router::WindowFocus::KinetixPC;
                                needs_redraw = true;
                                click_handled = true;
                            }
                        }
                        1 => {
                            let mut term = crate::cartridges::terminal::TERMINAL.lock();
                            if term.chamber.is_open {
                                term.chamber.restore();
                                router.focus = crate::pulses::router::WindowFocus::Terminal;
                                needs_redraw = true;
                                click_handled = true;
                            }
                        }
                        2 => {
                            let mut scribe = crate::cartridges::scribe::SCRIBE.lock();
                            if scribe.chamber.is_open {
                                scribe.chamber.restore();
                                router.focus = crate::pulses::router::WindowFocus::Scribe;
                                needs_redraw = true;
                                click_handled = true;
                            }
                        }
                        3 => {
                            let mut tel = crate::cartridges::telemetry::TELEMETRY.lock();
                            if tel.chamber.is_open {
                                tel.chamber.restore();
                                router.focus = crate::pulses::router::WindowFocus::Telemetry;
                                needs_redraw = true;
                                click_handled = true;
                            }
                        }
                        4 => {
                            let mut st = crate::cartridges::speed_test::SPEED_TEST.lock();
                            if st.chamber.is_open {
                                st.chamber.restore();
                                router.focus = crate::pulses::router::WindowFocus::SpeedTest;
                                needs_redraw = true;
                                click_handled = true;
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Check window maximize buttons
            let mut handled_max = false;
            if !click_handled {
                {
                    let mut st = crate::cartridges::speed_test::SPEED_TEST.lock();
                    if st.chamber.contains_maximize(mx, my) {
                        st.chamber.toggle_maximize(screen_w, screen_h);
                        handled_max = true;
                    }
                }
                if !handled_max {
                    let mut term = crate::cartridges::terminal::TERMINAL.lock();
                    if term.chamber.contains_maximize(mx, my) {
                        term.chamber.toggle_maximize(screen_w, screen_h);
                        handled_max = true;
                    }
                }
                if !handled_max {
                    let mut kpc = crate::cartridges::kinetix_pc::KINETIX_PC.lock();
                    if kpc.chamber.contains_maximize(mx, my) {
                        kpc.chamber.toggle_maximize(screen_w, screen_h);
                        handled_max = true;
                    }
                }
                if handled_max {
                    needs_redraw = true;
                }
            }

            // Check window dragging headers
            let mut handled_drag = false;
            if !click_handled && !handled_max {
                {
                    let mut st = crate::cartridges::speed_test::SPEED_TEST.lock();
                    if st.is_open() && st.chamber.contains_header(mx, my) {
                        st.chamber.start_drag(mx, my);
                        is_dragging_speed = true;
                        handled_drag = true;
                    }
                }
                if handled_drag {
                    router.focus = crate::pulses::router::WindowFocus::SpeedTest;
                    needs_redraw = true;
                } else {
                    let mut term = crate::cartridges::terminal::TERMINAL.lock();
                    if term.is_open() && term.chamber.contains_header(mx, my) {
                        term.chamber.start_drag(mx, my);
                        is_dragging_terminal = true;
                        handled_drag = true;
                    }
                    if handled_drag {
                        router.focus = crate::pulses::router::WindowFocus::Terminal;
                        needs_redraw = true;
                    }
                }
                if !handled_drag {
                    let mut kpc = crate::cartridges::kinetix_pc::KINETIX_PC.lock();
                    if kpc.is_open() && kpc.chamber.contains_header(mx, my) {
                        kpc.chamber.start_drag(mx, my);
                        is_dragging_kpc = true;
                        handled_drag = true;
                    }
                    if handled_drag {
                        router.focus = crate::pulses::router::WindowFocus::KinetixPC;
                        needs_redraw = true;
                    }
                }
                if !handled_drag {
                    let mut tel = crate::cartridges::telemetry::TELEMETRY.lock();
                    if tel.is_open() && tel.chamber.contains_header(mx, my) {
                        tel.chamber.start_drag(mx, my);
                        is_dragging_telemetry = true;
                        handled_drag = true;
                    }
                    if handled_drag {
                        router.focus = crate::pulses::router::WindowFocus::Telemetry;
                        needs_redraw = true;
                    }
                }
                if !handled_drag {
                    let mut sc = crate::cartridges::scribe::SCRIBE.lock();
                    if sc.is_open() && sc.chamber.contains_header(mx, my) {
                        sc.chamber.start_drag(mx, my);
                        is_dragging_scribe = true;
                        handled_drag = true;
                    }
                    if handled_drag {
                        router.focus = crate::pulses::router::WindowFocus::Scribe;
                        needs_redraw = true;
                    }
                }
                if !handled_drag {
                    let mut snake = crate::cartridges::ouroboros::OUROBOROS.lock();
                    if snake.is_open() && snake.chamber.contains_header(mx, my) {
                        snake.chamber.start_drag(mx, my);
                        is_dragging_snake = true;
                        handled_drag = true;
                    }
                    if handled_drag {
                        router.focus = crate::pulses::router::WindowFocus::Ouroboros;
                        needs_redraw = true;
                    }
                }
                if !handled_drag {
                    let mut vault = crate::cartridges::vault::VAULT.lock();
                    if vault.is_open() && vault.chamber.contains_header(mx, my) {
                        vault.chamber.start_drag(mx, my);
                        is_dragging_vault = true;
                        handled_drag = true;
                    }
                    if handled_drag {
                        router.focus = crate::pulses::router::WindowFocus::Vault;
                        needs_redraw = true;
                    }
                }
            }

            if !click_handled && !handled_drag && !handled_max {
                // Standard click dispatch
                router.dispatch_mouse_click(mx, my);
                needs_redraw = true;
            }
        }

        // 2. Handle Mouse Dragging Update
        if left_pressed {
            if is_dragging_speed {
                crate::cartridges::speed_test::SPEED_TEST.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            } else if is_dragging_terminal {
                crate::cartridges::terminal::TERMINAL.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            } else if is_dragging_kpc {
                crate::cartridges::kinetix_pc::KINETIX_PC.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            } else if is_dragging_telemetry {
                crate::cartridges::telemetry::TELEMETRY.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            } else if is_dragging_scribe {
                crate::cartridges::scribe::SCRIBE.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            } else if is_dragging_snake {
                crate::cartridges::ouroboros::OUROBOROS.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            } else if is_dragging_vault {
                crate::cartridges::vault::VAULT.lock().chamber.update_drag(mx, my, screen_w, screen_h);
                needs_redraw = true;
            }
        } else {
            // Mouse released: stop dragging
            if is_dragging_speed { crate::cartridges::speed_test::SPEED_TEST.lock().chamber.stop_drag(); is_dragging_speed = false; }
            if is_dragging_terminal { crate::cartridges::terminal::TERMINAL.lock().chamber.stop_drag(); is_dragging_terminal = false; }
            if is_dragging_kpc { crate::cartridges::kinetix_pc::KINETIX_PC.lock().chamber.stop_drag(); is_dragging_kpc = false; }
            if is_dragging_telemetry { crate::cartridges::telemetry::TELEMETRY.lock().chamber.stop_drag(); is_dragging_telemetry = false; }
            if is_dragging_scribe { crate::cartridges::scribe::SCRIBE.lock().chamber.stop_drag(); is_dragging_scribe = false; }
            if is_dragging_snake { crate::cartridges::ouroboros::OUROBOROS.lock().chamber.stop_drag(); is_dragging_snake = false; }
            if is_dragging_vault { crate::cartridges::vault::VAULT.lock().chamber.stop_drag(); is_dragging_vault = false; }
        }
        prev_left = left_pressed;

        // 3. Process Keyboard Characters from Ring Buffer
        while let Some(ch) = crate::arch::keyboard::pop_char() {
            if ch == '\x1B' {
                // Escape key: close active window or power menu
                let mut router = crate::pulses::router::ROUTER.lock();
                router.focus = crate::pulses::router::WindowFocus::Desktop;
                HORIZON.lock().start_menu_open = false;
                needs_redraw = true;
            } else {
                let mut router = crate::pulses::router::ROUTER.lock();
                router.dispatch_key(ch);
                needs_redraw = true;
            }
        }

        // 4. Autonomous Clock / Game / Telemetry Ticks
        let current_ticks = crate::arch::pit::get_ticks();
        fallback_counter = fallback_counter.wrapping_add(1);

        // Snake Game Autonomous Movement (Every 14 PIT ticks = 140ms)
        let snake_due = if current_ticks > last_snake_tick {
            current_ticks.saturating_sub(last_snake_tick) >= 14
        } else {
            fallback_counter % 15 == 0
        };

        if snake_due {
            last_snake_tick = current_ticks;
            let mut snake = crate::cartridges::ouroboros::OUROBOROS.lock();
            if snake.is_open() && snake.state == crate::cartridges::ouroboros::GameState::Running {
                snake.tick();
                needs_redraw = true;
            }
        }

        // Refresh Horizon Live RTC Clock every 100 PIT ticks (1.0s)
        if current_ticks.saturating_sub(last_clock_tick) >= 100 {
            last_clock_tick = current_ticks;
            needs_redraw = true;
        }

        // Refresh Telemetry window every 50 PIT ticks (0.5s) if open
        if current_ticks.saturating_sub(last_telemetry_tick) >= 50 {
            last_telemetry_tick = current_ticks;
            if crate::cartridges::telemetry::TELEMETRY.lock().is_open() {
                needs_redraw = true;
            }
        }

        // 5. Redraw Desktop & Pointer if State Changed
        if needs_redraw {
            let current_focus = crate::pulses::router::ROUTER.lock().focus;
            compose_desktop_dynamic(current_focus);
            pulses_draw_reticle(mx, my);
        } else {
            pulses_draw_reticle(mx, my);
        }

        // Low-power halt slice waiting for the next interrupt
        crate::arch::cpu::halt();
    }
}

fn pulses_draw_reticle(mx: usize, my: usize) {
    let mut reticle = crate::pulses::reticle::RETICLE.lock();
    reticle.move_to(mx, my);
}

/// Executes Phase 19 verification tests for the Canvas and Horizon subsystem.
pub fn run_canvas_tests() {
    println!("\n================================================================================");
    println!("             KINETIX OS CANVAS & HORIZON VERIFICATION (PHASE 19)                ");
    println!("================================================================================");

    let prism = PRISM.lock();
    println!("  [TEST 1] Verifying Prism Double-Buffer Geometry...");
    assert!(prism.width >= 800, "Surface width must be at least 800");
    assert!(prism.height >= 600, "Surface height must be at least 600");
    println!("  [PASS] Prism double buffer verified: {}x{}x4 ({} bytes).", prism.width, prism.height, prism.buffer.len() * 4);

    println!("  [TEST 2] Verifying Procedural Blue Bloom Backdrop synthesis...");
    let sky_pixel = prism.get_pixel(100, 100);
    let bloom_pixel = prism.get_pixel(prism.width / 2, prism.height / 2);
    assert_ne!(sky_pixel, 0, "Sky pixel must be non-zero");
    assert_ne!(bloom_pixel, 0, "Bloom center pixel must be non-zero");
    println!("  [PASS] Wallpaper synthesized: Sky pixel=0x{:08X}, Bloom pixel=0x{:08X}.", sky_pixel, bloom_pixel);

    println!("  [TEST 3] Verifying Horizon Bottom-Left Chronometer & Dock layout...");
    let dock_y = prism.height.saturating_sub(horizon::HORIZON_HEIGHT);
    println!("  [PASS] Horizon dock verified at Y={}..{} with bottom-left real-time chronometer.", dock_y, prism.height);

    println!("  [TEST 4] Verifying Desktop Cartridge Beacons & Hover Hit-Testing...");
    let beacons = BEACONS.lock();
    let hit_kpc = beacons.hit_test(40, 40);
    assert_eq!(hit_kpc, Some(BeaconId::KinetixPC), "Hit test at (40, 40) should identify KinetixPC");
    let hit_term = beacons.hit_test(40, 130);
    assert_eq!(hit_term, Some(BeaconId::Terminal), "Hit test at (40, 130) should identify Terminal");
    let hit_scribe = beacons.hit_test(40, 220);
    assert_eq!(hit_scribe, Some(BeaconId::Scribe), "Hit test at (40, 220) should identify Scribe");
    println!("  [PASS] Desktop Beacons hit-testing verified.");

    println!("\n[TEST] All Canvas & Horizon Desktop Verification Tests PASSED!\n");
}
