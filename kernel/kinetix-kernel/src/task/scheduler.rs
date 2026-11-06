//! Preemptive Multi-Policy CPU Scheduler for Kinetix OS.
//!
//! Implements three distinct scheduling policies:
//! 1. Round-Robin (`RoundRobin`): Uniform time-sliced circular task distribution.
//! 2. Priority-Based (`Priority`): Strict priority preemption with priority-weighted quantums.
//! 3. Fair-Share (`FairShare`): Dynamic least-consumed-CPU balancing (vruntime style).

use super::tcb::{Priority, Task, TaskState};

/// Scheduling policies supported by the Kinetix OS scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    RoundRobin,
    Priority,
    FairShare,
}

/// Chooses the next ready task index according to the configured scheduling policy.
pub fn select_next_task(tasks: &[Task], current_index: usize, policy: SchedulingPolicy) -> Option<usize> {
    if tasks.is_empty() {
        return None;
    }

    match policy {
        SchedulingPolicy::RoundRobin => select_round_robin(tasks, current_index),
        SchedulingPolicy::Priority => select_priority(tasks, current_index),
        SchedulingPolicy::FairShare => select_fair_share(tasks, current_index),
    }
}

/// Round-Robin: circular scan for the next ready task.
fn select_round_robin(tasks: &[Task], current_index: usize) -> Option<usize> {
    let count = tasks.len();
    for i in 1..=count {
        let idx = (current_index + i) % count;
        if tasks[idx].state == TaskState::Ready {
            return Some(idx);
        }
    }
    None
}

/// Priority: selects the ready task with the highest priority level.
/// Among equal priorities, selects using round-robin.
fn select_priority(tasks: &[Task], current_index: usize) -> Option<usize> {
    let mut best_idx = None;
    let mut highest_prio = Priority::Low;

    let count = tasks.len();
    for i in 1..=count {
        let idx = (current_index + i) % count;
        if tasks[idx].state == TaskState::Ready {
            if best_idx.is_none() || tasks[idx].priority > highest_prio {
                highest_prio = tasks[idx].priority;
                best_idx = Some(idx);
            }
        }
    }

    best_idx
}

/// Fair-Share: selects the ready task that has consumed the least total CPU ticks.
fn select_fair_share(tasks: &[Task], current_index: usize) -> Option<usize> {
    let mut best_idx = None;
    let mut min_ticks = u64::MAX;

    let count = tasks.len();
    for i in 1..=count {
        let idx = (current_index + i) % count;
        if tasks[idx].state == TaskState::Ready {
            if tasks[idx].total_ticks_consumed < min_ticks {
                min_ticks = tasks[idx].total_ticks_consumed;
                best_idx = Some(idx);
            }
        }
    }

    best_idx
}
