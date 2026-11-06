//! Cooperative Task Management & CPU Context Switching for Kinetix OS.
//!
//! Provides the core multitasking primitives: Task Control Blocks (TCB),
//! dedicated kernel stacks, voluntary CPU yielding (`yield_now`), and context switching.

pub mod context;
pub mod tcb;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
pub use tcb::{Task, TaskId, TaskState};

/// Global Cooperative Task Manager.
pub struct TaskManager {
    tasks: Vec<Task>,
    current_index: usize,
    next_id: u64,
    initialized: bool,
}

impl TaskManager {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_index: 0,
            next_id: 1,
            initialized: false,
        }
    }

    /// Initializes the multitasking subsystem and registers the boot thread as Task 0.
    pub fn init(&mut self) {
        if self.initialized {
            return;
        }

        let main_task = Task::new_main_task(TaskId(0), "Kernel Main");
        self.tasks.push(main_task);
        self.current_index = 0;
        self.next_id = 1;
        self.initialized = true;
    }

    /// Spawns a new cooperative kernel task with a dedicated 64 KiB stack.
    pub fn spawn(&mut self, name: &str, entry_fn: fn()) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        let task = Task::new_kernel_task(id, name, entry_fn);
        self.tasks.push(task);
        id
    }

    /// Finds the index of the next task in Ready state.
    fn pick_next_ready(&self) -> Option<usize> {
        if self.tasks.is_empty() {
            return None;
        }

        let count = self.tasks.len();
        for i in 1..=count {
            let idx = (self.current_index + i) % count;
            if self.tasks[idx].state == TaskState::Ready {
                return Some(idx);
            }
        }

        None
    }
}

pub static TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());

/// Initializes the multitasking subsystem.
pub fn init() {
    crate::println!("[TASK] Initializing Cooperative Multitasking Subsystem...");
    TASK_MANAGER.lock().init();
    crate::println!("  Task 0 ('Kernel Main') registered as active running thread.");
    crate::println!("[TASK] Task management initialized successfully.");
}

/// Spawns a new kernel task.
pub fn spawn(name: &str, entry_fn: fn()) -> TaskId {
    TASK_MANAGER.lock().spawn(name, entry_fn)
}

/// Voluntarily yields CPU execution to the next available ready task.
pub fn yield_now() {
    let switch_args = {
        let mut mgr = TASK_MANAGER.lock();
        if !mgr.initialized || mgr.tasks.len() <= 1 {
            return;
        }

        if let Some(next_idx) = mgr.pick_next_ready() {
            let cur_idx = mgr.current_index;
            if cur_idx == next_idx {
                return;
            }

            // Update states
            if mgr.tasks[cur_idx].state == TaskState::Running {
                mgr.tasks[cur_idx].state = TaskState::Ready;
            }
            mgr.tasks[next_idx].state = TaskState::Running;
            mgr.current_index = next_idx;

            // Extract pointers to stack pointer fields
            let old_rsp_ptr = &mut mgr.tasks[cur_idx].rsp as *mut u64;
            let new_rsp_ptr = &mgr.tasks[next_idx].rsp as *const u64;

            Some((old_rsp_ptr, new_rsp_ptr))
        } else {
            None
        }
    }; // Lock released here before switching context

    if let Some((old_rsp, new_rsp)) = switch_args {
        unsafe {
            context::switch_context(old_rsp, new_rsp);
        }
    }
}

/// Gracefully terminates the currently running task and switches to the next ready task.
pub fn exit() -> ! {
    let switch_args = {
        let mut mgr = TASK_MANAGER.lock();
        let cur_idx = mgr.current_index;
        mgr.tasks[cur_idx].state = TaskState::Terminated;

        if let Some(next_idx) = mgr.pick_next_ready() {
            mgr.tasks[next_idx].state = TaskState::Running;
            mgr.current_index = next_idx;

            let old_rsp_ptr = &mut mgr.tasks[cur_idx].rsp as *mut u64;
            let new_rsp_ptr = &mgr.tasks[next_idx].rsp as *const u64;
            Some((old_rsp_ptr, new_rsp_ptr))
        } else {
            None
        }
    };

    if let Some((old_rsp, new_rsp)) = switch_args {
        unsafe {
            context::switch_context(old_rsp, new_rsp);
        }
    }

    // If no more tasks exist, idle CPU
    crate::arch::cpu::halt_loop();
}

// Global Atomic Counters for Phase 10 Cooperative Multitasking Self-Test
static ALPHA_TICKS: AtomicU32 = AtomicU32::new(0);
static BETA_TICKS: AtomicU32 = AtomicU32::new(0);

fn task_alpha() {
    for i in 1..=3 {
        crate::println!("    [Task 1: Alpha] Step {}/3 running on CPU...", i);
        ALPHA_TICKS.fetch_add(1, Ordering::SeqCst);
        yield_now();
    }
    crate::println!("    [Task 1: Alpha] Finished all steps. Exiting.");
}

fn task_beta() {
    for i in 1..=3 {
        crate::println!("    [Task 2: Beta ] Step {}/3 running on CPU...", i);
        BETA_TICKS.fetch_add(1, Ordering::SeqCst);
        yield_now();
    }
    crate::println!("    [Task 2: Beta ] Finished all steps. Exiting.");
}

/// Runs verification tests for cooperative multitasking and CPU context switching.
pub fn run_task_tests() {
    crate::println!("\n[TEST] Running Cooperative Multitasking & Context Switch Tests...");

    // Spawn Task Alpha and Task Beta
    let id_a = spawn("Task Alpha", task_alpha);
    let id_b = spawn("Task Beta", task_beta);

    crate::println!("  Spawned Task 1 ('Task Alpha', ID: {:?})", id_a);
    crate::println!("  Spawned Task 2 ('Task Beta',  ID: {:?})", id_b);
    crate::println!("  Interleaving execution across tasks via yield_now()...");

    // Interleave execution by yielding from Task 0 until both tasks complete
    while ALPHA_TICKS.load(Ordering::SeqCst) < 3 || BETA_TICKS.load(Ordering::SeqCst) < 3 {
        yield_now();
    }

    assert_eq!(ALPHA_TICKS.load(Ordering::SeqCst), 3);
    assert_eq!(BETA_TICKS.load(Ordering::SeqCst), 3);

    crate::println!("  [PASS] Cooperative Task Management & CPU Context Switching verified!");
    crate::println!("[TEST] All Cooperative Task Management Tests PASSED!\n");
}
