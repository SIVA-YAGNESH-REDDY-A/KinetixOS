# Kinetix OS — Cooperative Task Management & CPU Context Switching

**Document ID:** SPEC-KERNEL-TASK-004  
**Status:** Implemented & Verified in VirtualBox  
**Target Architecture:** x86_64 Long Mode (System V AMD64 ABI)  
**Implementation Language:** Pure Rust (`core`, `alloc`, `no_std`) + Low-Level Assembly Context Switch (`global_asm!`)  

---

## 1. Overview and Architecture

Phase 10 implements the foundational task abstractions and low-level context switching mechanisms that power multitasking in Kinetix OS:

```
+-------------------------------------------------------------------------+
|                  Cooperative Task Manager (TASK_MANAGER)                |
|      - Task Lifecycle State Machine (Ready, Running, Blocked, Terminated)|
|      - Unique 64-bit Task Identifiers (TaskId)                          |
|      - Dedicated 64 KiB Kernel Stacks per Task                          |
|      - Task 0 ("Kernel Main") Boot Thread Tracking                      |
+-------------------------------------------------------------------------+
                                    |
                                    v (yield_now)
+-------------------------------------------------------------------------+
|               Low-Level Context Switch (switch_context)                 |
|      - Preserves Callee-Saved Registers: RBX, RBP, R12, R13, R14, R15   |
|      - Hardware Stack Pointer (RSP) Swap: mov [rdi], rsp; mov rsp, [rsi]|
|      - Restores Callee-Saved Registers: pop r15..rbx                    |
|      - Return (ret) into Target Task's RIP                              |
+-------------------------------------------------------------------------+
                                    |
                                    v (Initial Dispatch)
+-------------------------------------------------------------------------+
|                     Task Trampoline & Wrapper                           |
|      - task_trampoline: Entry wrapper for newly spawned tasks           |
|      - Enforces 16-Byte Stack Alignment required by System V ABI        |
|      - Calls task entry function via R12                                |
|      - Automatically invokes task::exit() upon completion               |
+-------------------------------------------------------------------------+
```

---

## 2. Task Control Block (TCB) & Stack Initialization

### 2.1 Task Structure
```rust
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TaskState,
    pub rsp: u64,
    pub stack: Option<Vec<u8>>,
}
```

### 2.2 Synthetic Initial Stack Frame
When a new task is created (`Task::new_kernel_task`), a 64 KiB stack is allocated and populated with a synthetic stack frame matching what `switch_context` expects to pop:

```
High Memory (stack_top, 16-byte aligned)
+----------------------------------------+
| [stack_top - 8 ]: Address of trampoline| <--- Return address popped by `ret`
+----------------------------------------+
| [stack_top - 16]: Initial RBX (0)      | <--- Popped into RBX
+----------------------------------------+
| [stack_top - 24]: Initial RBP (0)      | <--- Popped into RBP
+----------------------------------------+
| [stack_top - 32]: Initial R12 (entry)  | <--- Popped into R12 (function pointer)
+----------------------------------------+
| [stack_top - 40]: Initial R13 (0)      | <--- Popped into R13
+----------------------------------------+
| [stack_top - 48]: Initial R14 (0)      | <--- Popped into R14
+----------------------------------------+
| [stack_top - 56]: Initial R15 (0)      | <--- Popped into R15
+----------------------------------------+ <--- Initial task.rsp points here
Low Memory
```

When `switch_context` is invoked to switch to this task, it pops `r15..rbx`, leaving the stack 16-byte aligned, and executes `ret` into `task_trampoline`, which moves `R12` to `RDI` and calls the task entry function.

---

## 3. Cooperative Yielding (`yield_now`)

The cooperative scheduler allows tasks to voluntarily yield control:
1. Acquires `TASK_MANAGER` lock to inspect the task queue.
2. Identifies the next task with `state == TaskState::Ready`.
3. Marks current task as `Ready` and next task as `Running`.
4. Extracts raw pointers to the respective `rsp` fields (`old_rsp_ptr` and `new_rsp_ptr`).
5. **Releases `TASK_MANAGER` spinlock** before executing `switch_context` to prevent cross-task deadlocks.
6. Calls `switch_context(old_rsp_ptr, new_rsp_ptr)`.

---

## 4. Live VirtualBox Verification Evidence

Booting in Oracle VirtualBox 7.2 produced the following serial log:

```text
[TASK] Initializing Cooperative Multitasking Subsystem...
  Task 0 ('Kernel Main') registered as active running thread.
[TASK] Task management initialized successfully.

[TEST] Running Cooperative Multitasking & Context Switch Tests...
  Spawned Task 1 ('Task Alpha', ID: TaskId(1))
  Spawned Task 2 ('Task Beta',  ID: TaskId(2))
  Interleaving execution across tasks via yield_now()...
    [Task 1: Alpha] Step 1/3 running on CPU...
    [Task 2: Beta ] Step 1/3 running on CPU...
    [Task 1: Alpha] Step 2/3 running on CPU...
    [Task 2: Beta ] Step 2/3 running on CPU...
    [Task 1: Alpha] Step 3/3 running on CPU...
    [Task 2: Beta ] Step 3/3 running on CPU...
  [PASS] Cooperative Task Management & CPU Context Switching verified!
[TEST] All Cooperative Task Management Tests PASSED!

[KERNEL] Phase 10 Milestone achieved: Cooperative Task Management & Context Switching operational.
```
