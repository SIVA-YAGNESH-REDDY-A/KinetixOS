# Kinetix OS: Multicore Processor-Cache Affinity & Priority Scheduling Implementation
**Course Code**: 25PCCS201 | **Student**: Siva Yagnesh Reddy A | **USN**: 25BTRCL255

---

## 1. Academic Topic & Theoretical Framework

### Problem Statement
Investigate how restricting CPU-intensive workloads to selected processor cores (Processor-Cache Affinity) and tuning Process Priority affects execution performance and foreground responsiveness in an operating system kernel.

### Literature Foundation
1. **Squillante & Lazowska (1993)**: *"Using Processor-Cache Affinity Information in Shared-Memory Multiprocessor Scheduling"*, IEEE Trans. Parallel & Distributed Systems.
   - Core finding: Maintaining cache warmth on designated processor cores prevents cold-start L1/L2/L3 cache misses.
2. **Muddukrishna, Jonsson & Brorsson (2015)**: *"Locality-Aware Task Scheduling and Data Distribution for OpenMP Programs on NUMA Systems and Manycore Processors"*, Scientific Programming.
   - Core finding: Locality-aware scheduling balances load while prioritizing core-local memory paths.

---

## 2. Kernel Architecture & Code Implementation in Kinetix OS

Rather than running external scripts, Kinetix OS implements Processor-Cache Affinity and Priority Scheduling directly inside the bare-metal `#![no_std]` Rust kernel:

### A. Task Control Block (TCB) — `kernel/kinetix-kernel/src/task/tcb.rs`
Every process in Kinetix OS is assigned:
- `affinity_mask: u64`: 64-bit CPU core allowance bitmask.
  - `0x1`: Pinned to Core 0.
  - `0x3`: Allowed on Cores 0 and 1.
  - `0xF`: Allowed on Cores 0, 1, 2, 3.
  - `0xFFFF_FFFF_FFFF_FFFF`: Allowed across all logical cores.
- `last_core: u8`: Tracks the processor core where the task last executed to gauge cache warmth.

```rust
// File: kernel/kinetix-kernel/src/task/tcb.rs
impl Task {
    #[inline]
    pub fn is_allowed_on_core(&self, core_id: u8) -> bool {
        if core_id >= 64 {
            false
        } else {
            (self.affinity_mask & (1u64 << core_id)) != 0
        }
    }
}
```

### B. Preemptive Affinity-Aware Scheduler — `kernel/kinetix-kernel/src/task/scheduler.rs`
The kernel scheduler evaluates candidates using a multi-factor score:
$$\text{Score} = (\text{Priority} \times 100) + \text{CacheLocalityBonus}$$
- Where `CacheLocalityBonus = +25` if `task.last_core == active_core`, preserving L1/L2 cache state.
- Interactive foreground tasks (`Priority::High` or `Critical`) naturally dominate background CPU workers, eliminating responsiveness degradation.

```rust
// File: kernel/kinetix-kernel/src/task/scheduler.rs
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

    best_idx.or_else(|| select_priority(tasks, current_index))
}
```

### C. System Call ABI — `kernel/kinetix-kernel/src/syscall/mod.rs`
Ring 3 user-space programs interact with the affinity subsystem via fast x86_64 system calls:
- `SYS_SCHED_SETAFFINITY (6)`: Assigns a 64-bit core mask to a task.
- `SYS_SCHED_GETAFFINITY (7)`: Retrieves the task's active mask.
- `SYS_SCHED_SETPRIORITY (8)`: Dynamically adjusts process priority between Low, Normal, High, and Critical.

---

## 3. Physical Source Code Locations in the Project

Your teacher can inspect these physical files in the repository:
1. `kernel/kinetix-kernel/src/task/scheduler.rs` — Preemptive scheduling algorithm (`SchedulingPolicy::AffinityAware`).
2. `kernel/kinetix-kernel/src/task/tcb.rs` — Core bitmasking and Task Control Block.
3. `kernel/kinetix-kernel/src/syscall/mod.rs` — System call numbers and argument dispatchers.
4. `kernel/kinetix-kernel/src/arch/hwinfo.rs` — Host CPUID 16-core topology probing.
