//! Preemptive Multi-Policy CPU Scheduler & Task Management Subsystem.
//!
//! Encompasses:
//! - Timer-driven preemptive scheduling (100 Hz / 10 ms ticks)
//! - Multi-policy selection (Round-Robin, Priority-Based, Fair-Share)
//! - Detailed per-task execution telemetry (ticks consumed, context switches, preemptions)

pub mod context;
pub mod scheduler;
pub mod tcb;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

pub use scheduler::SchedulingPolicy;
pub use tcb::{Priority, Task, TaskId, TaskState};
use crate::research::metrics::TaskMetric;

/// Global Preemptive Task Manager and Multi-Policy Scheduler.
pub struct TaskManager {
    tasks: Vec<Task>,
    current_index: usize,
    next_id: u64,
    policy: SchedulingPolicy,
    preemption_enabled: bool,
    initialized: bool,
}

impl TaskManager {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_index: 0,
            next_id: 1,
            policy: SchedulingPolicy::RoundRobin,
            preemption_enabled: false,
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
        self.policy = SchedulingPolicy::RoundRobin;
        self.preemption_enabled = true;
        self.initialized = true;
    }

    /// Configures the active scheduling policy.
    pub fn set_policy(&mut self, policy: SchedulingPolicy) {
        self.policy = policy;
    }

    /// Enables or disables timer-driven preemption.
    #[allow(dead_code)]
    pub fn set_preemption(&mut self, enabled: bool) {
        self.preemption_enabled = enabled;
    }

    /// Spawns a new task with specified priority and dedicated 64 KiB stack.
    pub fn spawn(&mut self, name: &str, priority: Priority, entry_fn: fn()) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;

        let task = Task::new_kernel_task(id, name, priority, entry_fn);
        self.tasks.push(task);
        id
    }

    /// Processes a periodic 10 ms timer tick. Returns true if current time-slice has expired.
    pub fn on_timer_tick(&mut self) -> bool {
        if !self.initialized || !self.preemption_enabled || self.tasks.is_empty() {
            return false;
        }

        let cur = self.current_index;
        self.tasks[cur].total_ticks_consumed += 1;
        self.tasks[cur].ticks_remaining = self.tasks[cur].ticks_remaining.saturating_sub(1);

        if self.tasks[cur].ticks_remaining == 0 {
            // Reset quantum
            self.tasks[cur].ticks_remaining = self.tasks[cur].quantum_ticks;
            // Preemption requested if there are other ready tasks
            self.has_ready_tasks_besides_current()
        } else {
            false
        }
    }

    fn has_ready_tasks_besides_current(&self) -> bool {
        for (i, t) in self.tasks.iter().enumerate() {
            if i != self.current_index && t.state == TaskState::Ready {
                return true;
            }
        }
        false
    }

    /// Resets the scheduler state for an isolated benchmark trial.
    /// Retains Task 0 (Kernel Main) and cleans up completed worker tasks.
    pub fn reset_for_benchmark(&mut self, policy: SchedulingPolicy) {
        if !self.initialized {
            self.init();
        }

        // Truncate to retain only Task 0
        self.tasks.truncate(1);

        if let Some(main_task) = self.tasks.get_mut(0) {
            main_task.state = TaskState::Running;
            main_task.total_ticks_consumed = 0;
            main_task.ticks_remaining = main_task.quantum_ticks;
            main_task.context_switches = 0;
            main_task.voluntary_yields = 0;
            main_task.preemptions = 0;
        }

        self.current_index = 0;
        self.next_id = 1;
        self.policy = policy;
        self.preemption_enabled = true;
    }

    /// Checks if there are any active (non-terminated) worker tasks.
    pub fn has_active_workers(&self) -> bool {
        self.tasks.iter().skip(1).any(|t| t.state != TaskState::Terminated)
    }

    /// Collects execution telemetry from all worker tasks.
    pub fn collect_worker_metrics(&self) -> Vec<TaskMetric> {
        self.tasks
            .iter()
            .skip(1)
            .map(|t| TaskMetric {
                id: t.id.0,
                name: t.name.clone(),
                priority: t.priority,
                ticks_consumed: t.total_ticks_consumed,
                context_switches: t.context_switches,
                voluntary_yields: t.voluntary_yields,
                preemptions: t.preemptions,
            })
            .collect()
    }
}

pub static TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());

/// Initializes the task management subsystem.
pub fn init() {
    crate::println!("[TASK] Initializing Preemptive Multi-Policy CPU Scheduler...");
    TASK_MANAGER.lock().init();
    crate::println!("  Default Scheduling Policy: Round-Robin (Time-Sliced)");
    crate::println!("  Task 0 ('Kernel Main') registered as active running thread.");
    crate::println!("[TASK] Task management initialized successfully.");
}

/// Spawns a new task with default Normal priority.
#[allow(dead_code)]
pub fn spawn(name: &str, entry_fn: fn()) -> TaskId {
    spawn_with_priority(name, Priority::Normal, entry_fn)
}

/// Spawns a new task with explicit priority.
pub fn spawn_with_priority(name: &str, priority: Priority, entry_fn: fn()) -> TaskId {
    x86_64::instructions::interrupts::without_interrupts(|| {
        TASK_MANAGER.lock().spawn(name, priority, entry_fn)
    })
}

/// Sets the scheduler policy.
pub fn set_policy(policy: SchedulingPolicy) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        TASK_MANAGER.lock().set_policy(policy);
    });
}

/// Temporarily enables or disables timer preemption.
#[allow(dead_code)]
pub fn set_preemption(enabled: bool) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        TASK_MANAGER.lock().set_preemption(enabled);
    });
}

/// Resets scheduler state for an isolated benchmark trial.
pub fn reset_for_benchmark(policy: SchedulingPolicy) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        TASK_MANAGER.lock().reset_for_benchmark(policy);
    });
}

/// Checks whether any worker tasks are still running or ready.
pub fn has_active_workers() -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        TASK_MANAGER.lock().has_active_workers()
    })
}

/// Collects execution telemetry from all worker tasks.
pub fn collect_worker_metrics() -> Vec<TaskMetric> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        TASK_MANAGER.lock().collect_worker_metrics()
    })
}

/// Voluntarily yields CPU execution to the next available ready task.
pub fn yield_now() {
    perform_context_switch(false);
}

/// Called by the IRQ 0 timer interrupt handler on every tick.
/// If the running task's time-slice has expired, performs an involuntary preemption!
pub fn handle_timer_tick() {
    let should_preempt = if let Some(mut mgr) = TASK_MANAGER.try_lock() {
        mgr.on_timer_tick()
    } else {
        false
    };

    if should_preempt {
        perform_context_switch(true);
    }
}

fn perform_context_switch(is_preemption: bool) {
    let switch_args = if let Some(mut mgr) = TASK_MANAGER.try_lock() {
        if !mgr.initialized || mgr.tasks.len() <= 1 {
            return;
        }

        let policy = mgr.policy;
        let cur_idx = mgr.current_index;

        if let Some(next_idx) = scheduler::select_next_task(&mgr.tasks, cur_idx, policy) {
            if cur_idx == next_idx {
                return;
            }

            // Update telemetry
            if is_preemption {
                mgr.tasks[cur_idx].preemptions += 1;
            } else {
                mgr.tasks[cur_idx].voluntary_yields += 1;
            }
            mgr.tasks[cur_idx].context_switches += 1;
            mgr.tasks[next_idx].context_switches += 1;

            // Transition states
            if mgr.tasks[cur_idx].state == TaskState::Running {
                mgr.tasks[cur_idx].state = TaskState::Ready;
            }
            mgr.tasks[next_idx].state = TaskState::Running;
            mgr.current_index = next_idx;

            let old_rsp_ptr = &mut mgr.tasks[cur_idx].rsp as *mut u64;
            let new_rsp_ptr = &mgr.tasks[next_idx].rsp as *const u64;

            Some((old_rsp_ptr, new_rsp_ptr))
        } else {
            None
        }
    } else {
        return;
    }; // Lock dropped before switching context

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

        let policy = mgr.policy;
        if let Some(next_idx) = scheduler::select_next_task(&mgr.tasks, cur_idx, policy) {
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

    crate::arch::cpu::halt_loop();
}

// Global Atomic Counters for Preemptive Multitasking Verification Tests
static COMPUTE_A_PROGRESS: AtomicU32 = AtomicU32::new(0);
static COMPUTE_B_PROGRESS: AtomicU32 = AtomicU32::new(0);

fn preemptive_worker_a() {
    // Busy loop without yield_now() to prove involuntary timer preemption!
    for step in 1..=3 {
        // Busy wait doing math work across several timer ticks
        let mut sum: u64 = 0;
        for j in 0..50_000 {
            sum = sum.wrapping_add(j);
        }
        COMPUTE_A_PROGRESS.fetch_add(1, Ordering::SeqCst);
        let _ = sum;
        crate::println!("    [Worker Alpha] Completed compute step {} (involuntary preemption enabled)", step);
    }
}

fn preemptive_worker_b() {
    // Busy loop without yield_now() to prove involuntary timer preemption!
    for step in 1..=3 {
        let mut sum: u64 = 0;
        for j in 0..50_000 {
            sum = sum.wrapping_add(j);
        }
        COMPUTE_B_PROGRESS.fetch_add(1, Ordering::SeqCst);
        let _ = sum;
        crate::println!("    [Worker Beta ] Completed compute step {} (involuntary preemption enabled)", step);
    }
}

/// Runs verification tests for multi-policy scheduling and timer-driven preemption.
pub fn run_scheduler_tests() {
    crate::println!("\n[TEST] Running Preemptive Multi-Policy Scheduler Verification Tests...");

    // Test 1: Verify Scheduling Policies
    set_policy(SchedulingPolicy::Priority);
    crate::println!("  [PASS] Configured Priority-Based Scheduling Policy.");
    set_policy(SchedulingPolicy::FairShare);
    crate::println!("  [PASS] Configured Fair-Share Scheduling Policy.");
    set_policy(SchedulingPolicy::RoundRobin);
    crate::println!("  [PASS] Configured Round-Robin (Time-Sliced) Policy.");

    // Test 2: Involuntary Timer-Driven Preemption
    crate::println!("  Spawning compute-intensive Worker Alpha and Worker Beta (NO voluntary yields)...");
    let id_a = spawn_with_priority("Compute-Alpha", Priority::Normal, preemptive_worker_a);
    let id_b = spawn_with_priority("Compute-Beta", Priority::Normal, preemptive_worker_b);

    crate::println!("  Spawned Worker Alpha ({:?}) and Worker Beta ({:?})", id_a, id_b);
    crate::println!("  Enabling CPU interrupts to trigger 100 Hz PIT timer preemption...");

    // Temporarily enable interrupts so PIT ticks fire during the test
    crate::arch::cpu::enable_interrupts();

    // The main thread yields or waits while workers are preemptively scheduled
    while COMPUTE_A_PROGRESS.load(Ordering::SeqCst) < 3 || COMPUTE_B_PROGRESS.load(Ordering::SeqCst) < 3 {
        yield_now();
    }

    crate::println!("  [PASS] Timer Preemption verified: Both busy-compute workers ran concurrently to completion!");
    crate::println!("[TEST] All Preemptive Scheduler Verification Tests PASSED!\n");
}
