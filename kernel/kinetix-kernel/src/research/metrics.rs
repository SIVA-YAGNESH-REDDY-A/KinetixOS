//! Research Telemetry & CPU Cycle Measurement for Kinetix OS.
//!
//! Provides high-precision hardware Time Stamp Counter (TSC) cycle timing,
//! Jain's Fairness Index computation, and empirical multitasking benchmark data structures.

use alloc::string::String;
use alloc::vec::Vec;
use crate::task::Priority;
use crate::task::scheduler::SchedulingPolicy;

/// Reads the current CPU Time Stamp Counter (TSC) via the `rdtsc` instruction.
#[inline(always)]
pub fn read_tsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
    }
    ((high as u64) << 32) | (low as u64)
}

/// Computes Jain's Fairness Index as an integer percentage (0 to 100%).
///
/// Formulation:
///   J(x_1, ..., x_n) = (sum(x_i))^2 / (n * sum(x_i^2))
///
/// Uses `u128` arithmetic to prevent integer overflow.
pub fn compute_jains_fairness(allocations: &[u64]) -> u32 {
    if allocations.is_empty() {
        return 100;
    }
    let n = allocations.len() as u128;
    let sum: u128 = allocations.iter().map(|&x| x as u128).sum();
    if sum == 0 {
        return 100;
    }
    let sum_sq: u128 = allocations.iter().map(|&x| (x as u128) * (x as u128)).sum();
    if sum_sq == 0 {
        return 100;
    }

    let numerator = sum * sum * 100;
    let denominator = n * sum_sq;
    (numerator / denominator) as u32
}

/// Detailed per-task execution telemetry record.
#[derive(Debug, Clone)]
pub struct TaskMetric {
    pub id: u64,
    pub name: String,
    pub priority: Priority,
    pub ticks_consumed: u64,
    pub context_switches: u64,
    pub voluntary_yields: u64,
    pub preemptions: u64,
}

/// Comprehensive empirical telemetry record for a benchmark trial.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub trial_id: usize,
    pub policy: SchedulingPolicy,
    pub workload_name: &'static str,
    pub elapsed_cycles: u64,
    pub elapsed_ticks: u64,
    pub total_context_switches: u64,
    pub total_preemptions: u64,
    pub total_voluntary_yields: u64,
    #[allow(dead_code)]
    pub task_count: usize,
    pub fairness_pct: u32,
    pub task_metrics: Vec<TaskMetric>,
}
