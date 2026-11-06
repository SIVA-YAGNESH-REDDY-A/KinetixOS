//! Task Control Block (TCB) & Task Representation for Kinetix OS.
//!
//! Encapsulates execution state, unique task identifiers, and dedicated kernel stacks.

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

/// Standard kernel task stack size: 64 KiB (16 pages).
pub const KERNEL_STACK_SIZE: usize = 64 * 1024;

/// Task Control Block managing a single schedulable thread of execution.
#[allow(dead_code)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TaskState,
    pub rsp: u64,
    pub stack: Option<Vec<u8>>,
}

impl Task {
    /// Creates a new cooperative kernel task with its own dedicated 64 KiB stack.
    pub fn new_kernel_task(id: TaskId, name: &str, entry_fn: fn()) -> Self {
        let stack = alloc::vec![0u8; KERNEL_STACK_SIZE];

        // Align stack top to 16 bytes
        let stack_top = (stack.as_ptr() as usize + stack.len()) & !0x0F;

        // Initialize stack frame to simulate a suspended context:
        // [stack_top - 8 ]: Address of task_trampoline (popped by ret in switch_context)
        // [stack_top - 16]: Initial RBX
        // [stack_top - 24]: Initial RBP
        // [stack_top - 32]: Initial R12 = entry_fn function pointer
        // [stack_top - 40]: Initial R13
        // [stack_top - 48]: Initial R14
        // [stack_top - 56]: Initial R15
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
            rsp: initial_rsp,
            stack: Some(stack),
        }
    }

    /// Wraps the current boot thread into Task 0 ("Kernel Main").
    pub fn new_main_task(id: TaskId, name: &str) -> Self {
        Self {
            id,
            name: String::from(name),
            state: TaskState::Running,
            rsp: 0, // Will be set when Task 0 yields
            stack: None,
        }
    }
}
