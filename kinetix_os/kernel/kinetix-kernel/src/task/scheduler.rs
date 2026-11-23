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
    /// Research Writeup (25BTRCL255): Processor-Cache Affinity + Priority Isolation.
    AffinityAware,
}

/// Chooses the next ready task index according to the configured scheduling policy on Core 0.
pub fn select_next_task(tasks: &[Task], current_index: usize, policy: SchedulingPolicy) -> Option<usize> {
    select_next_task_on_core(tasks, current_index, policy, 0)
}

/// Chooses the next ready task index for `active_core` respecting processor affinity masks.
pub fn select_next_task_on_core(
    tasks: &[Task],
    current_index: usize,
    policy: SchedulingPolicy,
    active_core: u8,
) -> Option<usize> {
    if tasks.is_empty() {
        return None;
    }

    match policy {
        SchedulingPolicy::RoundRobin => select_round_robin(tasks, current_index),
        SchedulingPolicy::Priority => select_priority(tasks, current_index),
        SchedulingPolicy::FairShare => select_fair_share(tasks, current_index),
        SchedulingPolicy::AffinityAware => select_affinity_aware(tasks, current_index, active_core),
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

/// Affinity-Aware Scheduler (Research Topic 25BTRCL255):
/// 1. Filters tasks whose `affinity_mask` permits `active_core`.
/// 2. Scores each candidate by `(priority * 100) + cache_locality_bonus(last_core == active_core)`.
/// 3. Protects foreground responsiveness against background CPU workers.
fn select_affinity_aware(tasks: &[Task], current_index: usize, active_core: u8) -> Option<usize> {
    let mut best_idx = None;
    let mut best_score: i64 = -1;

    let count = tasks.len();
    for i in 1..=count {
        let idx = (current_index + i) % count;
        let t = &tasks[idx];
        if t.state == TaskState::Ready && t.is_allowed_on_core(active_core) {
            let cache_bonus: i64 = if t.last_core == active_core { 25 } else { 0 };
            let score = (t.priority as i64) * 100 + cache_bonus;
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }
    }

    // Fallback to any ready task if no task matches the core mask
    best_idx.or_else(|| select_priority(tasks, current_index))
}
