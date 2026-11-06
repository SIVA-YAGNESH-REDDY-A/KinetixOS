# Kinetix OS — Native Demonstration Applications (Phase 17)

## 1. Overview & Architectural Scope

Kinetix OS provides native 64-bit user-mode application execution without hosting dependencies (no Windows, Linux, POSIX, glibc, or musl). Native binaries are compiled as pure Rust `no_std` ELF64 executables (`x86_64-unknown-none`) that link against `kinetix-runtime` and interact with the kernel exclusively through the fast hardware System Call ABI (`syscall` / `sysretq`).

During Phase 17, three native applications were implemented, packaged into the boot filesystem (`bootfs.tar`), and verified under Oracle VirtualBox 7.2:
1. **`/bin/hello-world`**: Primary sanity verification demonstrating Ring 3 privilege, system call printing, PID/uptime queries, and clean process termination.
2. **`/bin/sysinfo`**: System introspection utility displaying OS identity, architecture, firmware, Ring 3 privilege confirmation, syscall ABI linkage, and boot uptime.
3. **`/bin/bench`**: Compute-intensive arithmetic benchmark performing prime number discovery with cooperative multitasking yields, verifying register preservation and numerical consistency across context boundaries.

---

## 2. User-Space Runtime Architecture (`kinetix-runtime`)

The [kinetix-runtime](file:///d:/kinetix_os/crates/kinetix-runtime/src/lib.rs) crate acts as the minimal C/Rust standard library equivalent for Kinetix OS user applications.

### 2.1 System Call Stubs (AMD64 ABI)
System calls are invoked via inline x86_64 assembly following the System V AMD64 calling convention:
- Syscall number passed in `rax`.
- First argument in `rdi`.
- Second argument in `rsi`.
- Kernel return value received in `rax`.
- Hardware preserves RIP in `rcx` and RFLAGS in `r11`.

```rust
#[inline(always)]
pub unsafe fn syscall0(num: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall1(num: u64, arg1: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

#[inline(always)]
pub unsafe fn syscall2(num: u64, arg1: u64, arg2: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        in("rax") num,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}
```

### 2.2 Standard Library Functions
`kinetix-runtime` exports high-level ergonomic functions:
- `print_str(s: &str)`: Emits string to kernel console via `SYS_PRINT` (`syscall 1`).
- `print_u64(val: u64)`: Prints unsigned 64-bit integer formatted as ASCII.
- `print_i64(val: i64)`: Prints signed 64-bit integer formatted as ASCII.
- `yield_now()`: Relinquishes CPU time-slice via `SYS_YIELD` (`syscall 2`).
- `getpid() -> u64`: Queries active process ID via `SYS_GETPID` (`syscall 4`).
- `uptime_ms() -> u64`: Queries system boot uptime via `SYS_UPTIME_MS` (`syscall 5`).
- `exit(code: i32) -> !`: Terminates application execution via `SYS_EXIT` (`syscall 3`).

---

## 3. Native Applications

### 3.1 `hello-world` ([apps/hello-world](file:///d:/kinetix_os/apps/hello-world/src/main.rs))
Verifies foundational user-space startup and execution:
- Emits banner `"  [APP: hello-world] Hello from Kinetix OS Native Application!"`.
- Queries and prints PID (`1`).
- Queries and prints boot uptime in milliseconds.
- Gracefully terminates with exit code `0`.

### 3.2 `sysinfo` ([apps/sysinfo](file:///d:/kinetix_os/apps/sysinfo/src/main.rs))
Provides diagnostic introspection of Kinetix OS from unprivileged Ring 3:
- Reads `CS` register to verify privilege level (CPL == 3).
- Outputs OS Name: `Kinetix OS (Pure Rust no_std)`.
- Outputs Target Architecture: `x86_64 (AMD64 Long Mode)`.
- Outputs Firmware Interface: `UEFI (64-bit Native)`.
- Outputs Syscall ABI: `Fast System Call (SYSCALL / SYSRET)`.
- Displays Process ID and elapsed system boot time.
- Exits cleanly with status code `0`.

### 3.3 `bench` ([apps/bench](file:///d:/kinetix_os/apps/bench/src/main.rs))
Performs an arithmetic prime search over range `2..3000`:
- Utilizes trial division algorithm.
- Every 500 candidate numbers, voluntarily yields CPU execution via `yield_now()`.
- **Register Preservation Verification**: Ensures the loop counter `n` (stored in `RSI` by the compiler) is preserved across `syscall` entry, dispatch, and return.
- Results computed:
  - Candidates checked: 2,999.
  - Primes discovered: 430.
  - Prime sum checksum: 593,823.
  - Duration: < 10 ms (0 ms resolution).
- Terminates with status code `0`.

---

## 4. Address Space Isolation & Resource Lifecycle

Each application executes within a completely isolated address space:
1. **Creation**: `ProcessAddressSpace::new()` allocates a dedicated PML4 and copies identity-mapped kernel lower 1 GiB (`PDPT[0]`) and framebuffer regions (`PDPT[2..511]`).
2. **Virtual Mapping**: Application code is mapped starting at `0x4000_0000` with User (`PAGE_USER`) and Present permissions.
3. **User Stack**: Dedicated 16 KiB stack allocated at `0x7FFF_C000..0x8000_0000` with initial RSP `0x7FFF_EFF0`.
4. **Execution**: Kernel switches CR3 to the process PML4 and executes `iretq` privilege drop into Ring 3.
5. **Reclamation**: Upon `SYS_EXIT`, the kernel regains control on its dedicated kernel stack (`KERNEL_STACK`), restores kernel CR3, and drops `ProcessAddressSpace`. The `Drop` implementation walks all 11 allocated physical frames (PML4, PDPT, PD, PTs, code frames, stack frames) and deallocates them back to the global bitmap frame allocator:
   ```text
   [APP LAUNCHER] '/bin/hello-world' terminated with exit status 0.
   [APP LAUNCHER] Reclaimed 11 physical frames for '/bin/hello-world'.
   ```

---

## 5. Live VirtualBox Verification Evidence

Log capture from serial port COM1 under Oracle VirtualBox 7.2:

```text
================================================================================
          KINETIX OS NATIVE DEMONSTRATION APPLICATIONS (PHASE 17)               
================================================================================

--------------------------------------------------------------------------------
[APP LAUNCHER] Loading '/bin/hello-world' (Hello World Native Application)...
  Address Space: PML4 0x100000, Entry 0x40000000, RSP 0x7FFFEFF0 (11 frames)
  Executing '/bin/hello-world' in Ring 3...

  [APP: hello-world] Hello from Kinetix OS Native Application!
  [APP: hello-world] Process ID : 1
  [APP: hello-world] Boot Uptime: 710 ms
  [APP: hello-world] Exiting cleanly with status 0.

  [APP LAUNCHER] '/bin/hello-world' terminated with exit status 0.
  [APP LAUNCHER] Reclaimed 11 physical frames for '/bin/hello-world'.

--------------------------------------------------------------------------------
[APP LAUNCHER] Loading '/bin/sysinfo' (System Information Utility)...
  Address Space: PML4 0x100000, Entry 0x40000000, RSP 0x7FFFEFF0 (11 frames)
  Executing '/bin/sysinfo' in Ring 3...

  [APP: sysinfo] ========================================
  [APP: sysinfo]        KINETIX OS SYSTEM INFORMATION     
  [APP: sysinfo] ========================================
  [APP: sysinfo] OS Name     : Kinetix OS (Pure Rust no_std)
  [APP: sysinfo] Target Arch : x86_64 (AMD64 Long Mode)
  [APP: sysinfo] Firmware    : UEFI (64-bit Native)
  [APP: sysinfo] Privilege   : Ring 3 (Unprivileged User Mode)
  [APP: sysinfo] ABI Linkage : Fast System Call (SYSCALL / SYSRET)
  [APP: sysinfo] Process ID  : 1
  [APP: sysinfo] System Time : 730 ms elapsed
  [APP: sysinfo] ========================================

  [APP LAUNCHER] '/bin/sysinfo' terminated with exit status 0.
  [APP LAUNCHER] Reclaimed 11 physical frames for '/bin/sysinfo'.

--------------------------------------------------------------------------------
[APP LAUNCHER] Loading '/bin/bench' (User-Space Arithmetic Benchmark)...
  Address Space: PML4 0x100000, Entry 0x40000000, RSP 0x7FFFEFF0 (11 frames)
  Executing '/bin/bench' in Ring 3...

  [APP: bench] Starting user-space arithmetic benchmark...
  [APP: bench] Search complete (range: 2..3000).
  [APP: bench] Primes discovered  : 430
  [APP: bench] Prime sum check    : 593823
  [APP: bench] Benchmark duration : 0 ms
  [APP: bench] Benchmark completed successfully with status 0.

  [APP LAUNCHER] '/bin/bench' terminated with exit status 0.
  [APP LAUNCHER] Reclaimed 11 physical frames for '/bin/bench'.

[TEST] All Native Demonstration Applications executed successfully (exit code 0)!

[KERNEL] Phase 17 Milestone achieved: Native Demonstration Applications (hello-world, sysinfo, bench) operational.
```
