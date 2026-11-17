//! Task Control Block (TCB) & Task Representation for Kinetix OS.
//!
//! Encapsulates execution state, unique task identifiers, priorities,
//! execution telemetry, and dedicated kernel stacks.

use alloc::string::String;
use alloc::vec::Vec;
use super::context::task_trampoline;

/// Unique 64-bit identifier for a kernel task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

/// Lifecycle state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    #[allow(dead_code)]
    Blocked,
    Terminated,
}

/// Task execution priority levels.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Realtime = 4,
}

impl Priority {
    /// Standard time-slice quantum in timer ticks (each tick = 10 ms at 100 Hz).
    #[allow(dead_code)]
    pub fn default_quantum_ticks(&self) -> u32 {
        match self {
            Priority::Low => 2,       // 20 ms
            Priority::Normal => 4,    // 40 ms
            Priority::High => 8,      // 80 ms
            Priority::Realtime => 12, // 120 ms
        }
    }
}

/// Standard kernel task stack size: 64 KiB (16 pages).
pub const KERNEL_STACK_SIZE: usize = 64 * 1024;

/// Task Control Block managing a single schedulable thread of execution.
#[allow(dead_code)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TaskState,
    pub priority: Priority,
    pub rsp: u64,
    pub stack: Option<Vec<u8>>,

    // Telemetry and Quantum Scheduling Metrics
    pub quantum_ticks: u32,
    pub ticks_remaining: u32,
    pub total_ticks_consumed: u64,
    pub context_switches: u64,
    pub voluntary_yields: u64,
    pub preemptions: u64,

    // Research Writeup (25BTRCL255): Processor-Cache Affinity & Core Pinning
    pub affinity_mask: u64,
    pub last_core: u8,
    pub cache_affinity_hits: u64,
}

impl Task {
    /// Creates a new kernel task with specified priority and a dedicated 64 KiB stack.
    pub fn new_kernel_task(id: TaskId, name: &str, priority: Priority, entry_fn: fn()) -> Self {
        let stack = alloc::vec![0u8; KERNEL_STACK_SIZE];
        let stack_top = (stack.as_ptr() as usize + stack.len()) & !0x0F;
        let quantum = priority.default_quantum_ticks();

        let initial_rsp = unsafe {
            let ptr = stack_top as *mut usize;
            ptr.sub(1).write(task_trampoline as *const () as usize); // RIP return address
            ptr.sub(2).write(0);                                     // RBX
            ptr.sub(3).write(0);                                     // RBP
            ptr.sub(4).write(entry_fn as usize);                     // R12 = entry_fn
            ptr.sub(5).write(0);                                     // R13
            ptr.sub(6).write(0);                                     // R14
            ptr.sub(7).write(0);                                     // R15
            ptr.sub(7) as u64
        };

        Self {
            id,
            name: String::from(name),
            state: TaskState::Ready,
            priority,
            rsp: initial_rsp,
            stack: Some(stack),
            quantum_ticks: quantum,
            ticks_remaining: quantum,
            total_ticks_consumed: 0,
            context_switches: 0,
            voluntary_yields: 0,
            preemptions: 0,
            affinity_mask: u64::MAX, // All logical cores allowed by default
            last_core: 0,
            cache_affinity_hits: 0,
        }
    }

    /// Returns true if this task's processor affinity bitmask permits execution on `core_id`.
    #[inline]
    pub fn is_allowed_on_core(&self, core_id: u8) -> bool {
        if core_id >= 64 {
            return true;
        }
        (self.affinity_mask & (1u64 << core_id)) != 0
    }

    /// Wraps the current boot thread into Task 0 ("Kernel Main").
    pub fn new_main_task(id: TaskId, name: &str) -> Self {
        let priority = Priority::Normal;
        let quantum = priority.default_quantum_ticks();

        Self {
            id,
            name: String::from(name),
            state: TaskState::Running,
            priority,
            rsp: 0,
            stack: None,
            quantum_ticks: quantum,
            ticks_remaining: quantum,
            total_ticks_consumed: 0,
            context_switches: 0,
            voluntary_yields: 0,
            preemptions: 0,
            affinity_mask: u64::MAX,
            last_core: 0,
            cache_affinity_hits: 0,
        }
    }
}
