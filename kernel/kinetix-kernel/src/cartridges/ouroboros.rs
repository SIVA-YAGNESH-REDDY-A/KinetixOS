//! Ouroboros Arena: Native Interactive Snake Arcade Cartridge for Kinetix OS.
//!
//! Provides a 20x20 grid arcade chamber with serpentine body dynamics,
//! directional physics, food spawning, score tracking, and restart overlay.

use alloc::vec::Vec;
use crate::canvas::chamber::FlowChamber;
use crate::canvas::prism::PrismSurface;
use spin::Mutex;

pub const OUROBOROS_X: usize = 300;
pub const OUROBOROS_Y: usize = 120;
pub const OUROBOROS_WIDTH: usize = 424;
pub const OUROBOROS_HEIGHT: usize = 460;

const GRID_COLS: usize = 20;
const GRID_ROWS: usize = 20;
const CELL_SIZE: usize = 19;
const CELL_PAD: usize = 1;
const GRID_ORIGIN_X: usize = OUROBOROS_X + 12;
const GRID_ORIGIN_Y: usize = OUROBOROS_Y + 36;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Running,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPoint {
    pub x: i32,
    pub y: i32,
}

pub struct OuroborosArena {
    pub chamber: FlowChamber,
    pub body: Vec<GridPoint>,
    pub direction: Direction,
    pub next_direction: Direction,
    pub food: GridPoint,
    pub score: u32,
    pub state: GameState,
    pub rng_seed: u32,
}

impl OuroborosArena {
    pub const fn new() -> Self {
        Self {
            chamber: FlowChamber::new(
                2,
                "Ouroboros Arena — Score: 0",
                OUROBOROS_X,
                OUROBOROS_Y,
                OUROBOROS_WIDTH,
                OUROBOROS_HEIGHT,
                0x000A_1C14, // Dark forest plate
            ),
            body: Vec::new(),
            direction: Direction::Right,
            next_direction: Direction::Right,
            food: GridPoint { x: 15, y: 10 },
            score: 0,
            state: GameState::Running,
            rng_seed: 0x1234_5678,
        }
    }

    /// Opens the Ouroboros game window and starts a new match.
    pub fn open(&mut self) {
        self.chamber.is_open = true;
        self.chamber.is_focused = true;
        if self.body.is_empty() {
            self.reset();
        }
    }

    /// Closes the Ouroboros window.
    #[allow(dead_code)]
    pub fn close(&mut self) {
        self.chamber.is_open = false;
        self.chamber.is_focused = false;
    }

    /// Returns whether the Ouroboros window is open.
    pub fn is_open(&self) -> bool {
        self.chamber.is_open
    }

    /// Resets the game to initial state: 3-segment snake centered, moving right.
    pub fn reset(&mut self) {
        self.body.clear();
        self.body.push(GridPoint { x: 10, y: 10 }); // Head
        self.body.push(GridPoint { x: 9, y: 10 });
        self.body.push(GridPoint { x: 8, y: 10 });  // Tail

        self.direction = Direction::Right;
        self.next_direction = Direction::Right;
        self.score = 0;
        self.state = GameState::Running;
        self.chamber.title = "Ouroboros Arena — Score: 0";
        self.spawn_food();
    }

    /// Changes snake movement direction, preventing instant 180-degree self-reversals.
    pub fn change_direction(&mut self, new_dir: Direction) {
        let is_opposite = match (self.direction, new_dir) {
            (Direction::Up, Direction::Down) => true,
            (Direction::Down, Direction::Up) => true,
            (Direction::Left, Direction::Right) => true,
            (Direction::Right, Direction::Left) => true,
            _ => false,
        };

        if !is_opposite {
            self.next_direction = new_dir;
        }
    }

    /// Advances the game world by one tick step.
    pub fn tick(&mut self) {
        if self.state != GameState::Running || self.body.is_empty() {
            return;
        }

        self.direction = self.next_direction;
        let head = self.body[0];

        let new_head = match self.direction {
            Direction::Up => GridPoint { x: head.x, y: head.y - 1 },
            Direction::Down => GridPoint { x: head.x, y: head.y + 1 },
            Direction::Left => GridPoint { x: head.x - 1, y: head.y },
            Direction::Right => GridPoint { x: head.x + 1, y: head.y },
        };

        // 1. Wall Collision Check
        if new_head.x < 0 || new_head.x >= GRID_COLS as i32 || new_head.y < 0 || new_head.y >= GRID_ROWS as i32 {
            self.state = GameState::GameOver;
            return;
        }

        // 2. Self Collision Check
        for &seg in self.body.iter() {
            if seg == new_head {
                self.state = GameState::GameOver;
                return;
            }
        }

        // 3. Move Head
        self.body.insert(0, new_head);

        // 4. Food Check
        if new_head == self.food {
            self.score += 10;
            self.spawn_food();
            // Don't pop tail -> snake grows longer!
        } else {
            self.body.pop(); // Remove tail segment to maintain length
        }
    }

    /// Generates food at an unoccupied grid coordinate.
    pub fn spawn_food(&mut self) {
        for _ in 0..100 {
            self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let fx = ((self.rng_seed >> 16) % (GRID_COLS as u32)) as i32;
            self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let fy = ((self.rng_seed >> 16) % (GRID_ROWS as u32)) as i32;

            let candidate = GridPoint { x: fx, y: fy };
            let occupied = self.body.iter().any(|&seg| seg == candidate);
            if !occupied {
                self.food = candidate;
                return;
            }
        }
        self.food = GridPoint { x: 5, y: 5 };
    }

    /// Renders the complete Ouroboros Arena and playing grid into the Prism surface.
    pub fn render(&self, prism: &mut PrismSurface) {
        if !self.chamber.is_open {
            return;
        }

        // 1. Draw window frame, header, and close button
        self.chamber.render(prism);

        // 2. Draw Playing Grid Background (400x400)
        prism.fill_rect(GRID_ORIGIN_X, GRID_ORIGIN_Y, 400, 400, 0x0006_140D);
        prism.draw_rect_outline(GRID_ORIGIN_X, GRID_ORIGIN_Y, 400, 400, 0x0022_C55E);

        // 3. Draw Food (Ruby red orb with highlight)
        let food_px = GRID_ORIGIN_X + (self.food.x as usize) * (CELL_SIZE + CELL_PAD);
        let food_py = GRID_ORIGIN_Y + (self.food.y as usize) * (CELL_SIZE + CELL_PAD);
        prism.fill_rect(food_px + 2, food_py + 2, CELL_SIZE - 4, CELL_SIZE - 4, 0x00EF_4444);
        prism.put_pixel(food_px + 4, food_py + 4, 0x00FF_FFFF); // Glimmer

        // 4. Draw Snake Body
        for (idx, &seg) in self.body.iter().enumerate() {
            let seg_px = GRID_ORIGIN_X + (seg.x as usize) * (CELL_SIZE + CELL_PAD);
            let seg_py = GRID_ORIGIN_Y + (seg.y as usize) * (CELL_SIZE + CELL_PAD);

            if idx == 0 {
                // Head: Lime green with eyes
                prism.fill_rect(seg_px, seg_py, CELL_SIZE, CELL_SIZE, 0x004A_DE80);
                // Eyes
                prism.put_pixel(seg_px + 4, seg_py + 4, 0x000F_172A);
                prism.put_pixel(seg_px + 14, seg_py + 4, 0x000F_172A);
            } else {
                // Body segment: Emerald green with soft border
                prism.fill_rect(seg_px, seg_py, CELL_SIZE, CELL_SIZE, 0x0022_C55E);
                prism.draw_rect_outline(seg_px, seg_py, CELL_SIZE, CELL_SIZE, 0x0015_803D);
            }
        }

        // 5. Draw Game Over Overlay if crashed
        if self.state == GameState::GameOver {
            // Translucent dark plate across center grid
            prism.fill_rect(GRID_ORIGIN_X + 40, GRID_ORIGIN_Y + 140, 320, 100, 0x000A_1124);
            prism.draw_rect_outline(GRID_ORIGIN_X + 40, GRID_ORIGIN_Y + 140, 320, 100, 0x00EF_4444);
            prism.draw_text(GRID_ORIGIN_X + 110, GRID_ORIGIN_Y + 165, "GAME OVER!", 0x00EF_4444, None);
            prism.draw_text(GRID_ORIGIN_X + 60, GRID_ORIGIN_Y + 195, "Press SPACE to Restart", 0x00FA_CC15, None);
        }

        // 6. Draw Bottom Status Bar inside window
        let status_y = self.chamber.y + self.chamber.height.saturating_sub(22);
        prism.fill_rect(self.chamber.x + 1, status_y, self.chamber.width.saturating_sub(2), 21, 0x0006_1109);

        let mut status_buf = [0u8; 48];
        let status_str = format_game_status(&mut status_buf, self.score, self.body.len());
        prism.draw_text(self.chamber.x + 14, status_y + 3, status_str, 0x0086_EFAC, None);
    }
}

fn format_game_status<'a>(buf: &'a mut [u8], score: u32, length: usize) -> &'a str {
    let mut idx = 0;

    let p1 = b"Score: ";
    for &b in p1 { buf[idx] = b; idx += 1; }
    idx = append_num(buf, idx, score as usize);

    let p2 = b"  |  Length: ";
    for &b in p2 { buf[idx] = b; idx += 1; }
    idx = append_num(buf, idx, length);

    let p3 = b"  |  Speed: 10 ticks/s";
    for &b in p3 { buf[idx] = b; idx += 1; }

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

pub static OUROBOROS: Mutex<OuroborosArena> = Mutex::new(OuroborosArena::new());

/// Executes Phase 21 verification tests for the Ouroboros Snake arcade chamber.
pub fn run_ouroboros_tests() {
    crate::println!("\n================================================================================");
    crate::println!("             KINETIX OS OUROBOROS (CARTRIDGE II) VERIFICATION (PHASE 21)        ");
    crate::println!("================================================================================");

    let mut arena = OUROBOROS.lock();

    crate::println!("  [TEST 1] Verifying Ouroboros Arena Window Launch & Bounds...");
    arena.open();
    assert!(arena.is_open(), "Ouroboros window must report is_open() == true");
    assert_eq!(arena.chamber.width, OUROBOROS_WIDTH);
    assert_eq!(arena.chamber.height, OUROBOROS_HEIGHT);
    crate::println!("  [PASS] Ouroboros arena launched at ({}, {}) [{}x{}].", OUROBOROS_X, OUROBOROS_Y, OUROBOROS_WIDTH, OUROBOROS_HEIGHT);

    crate::println!("  [TEST 2] Verifying Initial Snake State & Food Placement...");
    assert_eq!(arena.body.len(), 3, "Initial snake length must be 3");
    assert_eq!(arena.body[0], GridPoint { x: 10, y: 10 }, "Snake head should be at (10, 10)");
    assert_eq!(arena.score, 0, "Initial score must be 0");
    assert_eq!(arena.state, GameState::Running);
    crate::println!("  [PASS] Snake initialized: 3 segments, head at (10, 10), food at ({}, {}).", arena.food.x, arena.food.y);

    crate::println!("  [TEST 3] Verifying Movement Dynamics & Directional Physics...");
    arena.change_direction(Direction::Up);
    arena.tick();
    assert_eq!(arena.body[0], GridPoint { x: 10, y: 9 }, "Head moved Up to (10, 9)");
    arena.change_direction(Direction::Left);
    arena.tick();
    assert_eq!(arena.body[0], GridPoint { x: 9, y: 9 }, "Head moved Left to (9, 9)");
    crate::println!("  [PASS] Movement steps and directional turns verified.");

    crate::println!("  [TEST 4] Verifying 180-Degree Reversal Prevention...");
    // Moving Left, attempting instant Right must be ignored
    arena.change_direction(Direction::Right);
    arena.tick();
    assert_eq!(arena.body[0], GridPoint { x: 8, y: 9 }, "Instant right turn rejected; continued Left to (8, 9)");
    crate::println!("  [PASS] Instant self-reversal prevented.");

    crate::println!("  [TEST 5] Verifying Food Consumption & Growth...");
    // Place food directly in path
    arena.food = GridPoint { x: 7, y: 9 };
    arena.tick(); // Eats food at (7, 9)
    assert_eq!(arena.body[0], GridPoint { x: 7, y: 9 });
    assert_eq!(arena.score, 10, "Score should increment by 10");
    assert_eq!(arena.body.len(), 4, "Snake length should grow from 3 to 4");
    crate::println!("  [PASS] Food consumed: score incremented to 10, length grew to 4.");

    crate::println!("  [TEST 6] Verifying Wall Collision & Game Over State...");
    // Drive snake into left wall (x < 0)
    for _ in 0..10 {
        arena.tick();
    }
    assert_eq!(arena.state, GameState::GameOver, "Crashing into wall must trigger GameOver state");
    crate::println!("  [PASS] Wall collision detected; Game Over state triggered.");

    crate::println!("  [TEST 7] Verifying Match Restart...");
    arena.reset();
    assert_eq!(arena.state, GameState::Running);
    assert_eq!(arena.body.len(), 3);
    assert_eq!(arena.score, 0);
    crate::println!("  [PASS] Game reset verified: state restored to Running with clean score.");

    crate::println!("\n[TEST] All Ouroboros (Cartridge II) Verification Tests PASSED!\n");
}
