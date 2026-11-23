# Kinetix OS — Preemptive Multi-Policy CPU Scheduler Architecture

**Document ID:** SPEC-KERNEL-SCHED-005  
**Status:** Implemented & Verified in VirtualBox  
**Target Architecture:** x86_64 Long Mode (100 Hz Preemptive Timer Interrupts)  
**Implementation Language:** Pure Rust (`core`, `alloc`, `no_std`)  

---

## 1. Overview and Architecture

Phase 11 fulfills the CPU scheduling core of Kinetix OS, establishing a timer-driven preemptive multitasking engine supporting multiple scheduling disciplines and execution telemetry:

```
+-------------------------------------------------------------------------+
|                  100 Hz Hardware PIT Timer (IRQ 0 / Vector 32)          |
|      - Fires every 10 ms via 8254 Programmable Interval Timer           |
|      - Increments running task's total ticks consumed                   |
|      - Decrements time-slice quantum (ticks_remaining)                  |
|      - Involuntary Preemption Triggered when ticks_remaining == 0      |
+-------------------------------------------------------------------------+
                                    |
                                    v (Quantum Expired)
+-------------------------------------------------------------------------+
|               Multi-Policy Scheduler Engine (TASK_MANAGER)              |
|      - Dynamically Configurable Scheduling Disciplines:                 |
|         1. Round-Robin (`RoundRobin`): Uniform time-sliced circular     |
|         2. Priority-Based (`Priority`): Strict priority preemption      |
|         3. Fair-Share (`FairShare`): Least-consumed CPU balancing       |
+-------------------------------------------------------------------------+
                                    |
                                    v (Select Next Task)
+-------------------------------------------------------------------------+
|               Involuntary Preemptive Context Switch                     |
|      - Updates Per-Task Telemetry (preemptions, context_switches)       |
|      - Releases TASK_MANAGER Spinlock before Switch                     |
|      - Invokes switch_context(old_rsp, new_rsp)                         |
|      - Resumes Interrupted Task Seamlessly via iretq Stack Frame        |
+-------------------------------------------------------------------------+
```

---

## 2. Multi-Policy Scheduling Disciplines

### 2.1 Round-Robin (`RoundRobin`)
- Circular FIFO scan across all ready tasks starting from `(current_index + 1)`.
- Ensures equal opportunity and prevents task starvation.
- Default quantum: 4 ticks (40 ms).

### 2.2 Priority-Based Scheduling (`Priority`)
- Supports 4 priority tiers:
  - `Low` (Priority 1): 2 ticks (20 ms quantum)
  - `Normal` (Priority 2): 4 ticks (40 ms quantum)
  - `High` (Priority 3): 8 ticks (80 ms quantum)
  - `Realtime` (Priority 4): 12 ticks (120 ms quantum)
- Selects the task with the strictly highest priority; among equal priorities, round-robins.

### 2.3 Fair-Share Scheduling (`FairShare`)
- Modeled after modern completely fair scheduling (CFS) principles.
- Inspects `total_ticks_consumed` across all ready tasks and selects the task that has received the least CPU time.
- Dynamically balances execution time across disparate CPU-intensive workloads.

---

## 3. Timer-Driven Involuntary Preemption Flow

When an involuntary preemption occurs:
1. The CPU is executing instructions in a task thread.
2. PIT Channel 0 triggers hardware IRQ 0.
3. CPU saves `SS`, `RSP`, `RFLAGS`, `CS`, `RIP` onto the running task's stack.
4. `common_isr_stub` pushes `RAX..R15` and invokes `interrupt_dispatch`.
5. `handle_irq` sends EOI to the dual 8259 PIC.
6. `task::handle_timer_tick()` decrements `ticks_remaining`.
7. When `ticks_remaining == 0`:
   - Increments task's `preemptions` telemetry counter.
   - Drops `TASK_MANAGER` lock.
   - Invokes `switch_context`, which pushes callee-saved registers and swaps the stack pointer to the next task's stack.
8. When the preempted task is later rescheduled, `switch_context` returns directly back into `handle_irq`, which finishes and executes `iretq`, perfectly restoring the exact CPU state at the instant of preemption.

---

## 4. Live VirtualBox Verification Evidence

Booting in Oracle VirtualBox 7.2 produced the following serial log:

```text
[TASK] Initializing Preemptive Multi-Policy CPU Scheduler...
  Default Scheduling Policy: Round-Robin (Time-Sliced)
  Task 0 ('Kernel Main') registered as active running thread.
[TASK] Task management initialized successfully.

[TEST] Running Preemptive Multi-Policy Scheduler Verification Tests...
  [PASS] Configured Priority-Based Scheduling Policy.
  [PASS] Configured Fair-Share Scheduling Policy.
  [PASS] Configured Round-Robin (Time-Sliced) Policy.
  Spawning compute-intensive Worker Alpha and Worker Beta (NO voluntary yields)...
  Spawned Worker Alpha (TaskId(1)) and Worker Beta (TaskId(2))
  Enabling CPU interrupts to trigger 100 Hz PIT timer preemption...
    [Worker Alpha] Completed compute step 1 (involuntary preemption enabled)
    [Worker Alpha] Completed compute step 2 (involuntary preemption enabled)
    [Worker Alpha] Completed compute step 3 (involuntary preemption enabled)
    [Worker Beta ] Completed compute step 1 (involuntary preemption enabled)
    [Worker Beta ] Completed compute step 2 (involuntary preemption enabled)
    [Worker Beta ] Completed compute step 3 (involuntary preemption enabled)
  [PASS] Timer Preemption verified: Both busy-compute workers ran concurrently to completion!
[TEST] All Preemptive Scheduler Verification Tests PASSED!

[KERNEL] Phase 11 Milestone achieved: Preemptive Multi-Policy CPU Scheduler operational.
```
