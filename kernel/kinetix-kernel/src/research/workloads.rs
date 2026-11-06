//! Research Workload Benchmarks for Kinetix OS Scheduler Performance Evaluation.
//!
//! Provides controlled, measurable workloads designed to expose the operational
//! characteristics of Round-Robin, Priority-Based, and Fair-Share CPU scheduling policies.

use core::hint::black_box;
use crate::task::yield_now;

/// Intensive polynomial and bitwise hashing calculation.
/// Runs a deterministic loop to consume CPU cycles without compiler optimization pruning.
#[inline(never)]
pub fn compute_intensive_math(iterations: u64) -> u64 {
    let mut acc: u64 = 0xA5A5_5A5A_0123_4567;
    for i in 0..iterations {
        acc = black_box(acc.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407));
        acc ^= acc >> 21;
        acc ^= acc << 35;
        acc ^= acc >> 4;
        acc = acc.wrapping_add(i);
    }
    black_box(acc)
}

// Workload 1: Pure Compute (Symmetric CPU-Bound Tasks)
// 20,000,000 iterations per worker (~60-80M cycles, ~2-3 PIT ticks)
pub fn worker_compute_a() {
    let res = compute_intensive_math(25_000_000);
    black_box(res);
}

pub fn worker_compute_b() {
    let res = compute_intensive_math(25_000_000);
    black_box(res);
}

pub fn worker_compute_c() {
    let res = compute_intensive_math(25_000_000);
    black_box(res);
}

// Workload 2: Interactive / I/O-Bound Simulation (Short compute bursts followed by voluntary yields)
pub fn worker_io_burst_a() {
    for _ in 0..10 {
        let res = compute_intensive_math(60_000);
        black_box(res);
        yield_now();
    }
}

pub fn worker_io_burst_b() {
    for _ in 0..10 {
        let res = compute_intensive_math(60_000);
        black_box(res);
        yield_now();
    }
}

pub fn worker_io_burst_c() {
    for _ in 0..10 {
        let res = compute_intensive_math(60_000);
        black_box(res);
        yield_now();
    }
}

// Workload 3: Heterogeneous Mixed Workload
pub fn worker_mixed_comp_1() {
    let res = compute_intensive_math(30_000_000);
    black_box(res);
}

pub fn worker_mixed_comp_2() {
    let res = compute_intensive_math(30_000_000);
    black_box(res);
}

pub fn worker_mixed_io_1() {
    for _ in 0..10 {
        let res = compute_intensive_math(50_000);
        black_box(res);
        yield_now();
    }
}

pub fn worker_mixed_io_2() {
    for _ in 0..10 {
        let res = compute_intensive_math(50_000);
        black_box(res);
        yield_now();
    }
}
