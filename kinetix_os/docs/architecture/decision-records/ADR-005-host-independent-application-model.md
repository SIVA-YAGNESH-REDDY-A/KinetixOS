# ADR-005: Host-Independent Native Application Model

## Status
**Accepted**

## Context
Kinetix OS requires a native application execution architecture where applications are:
1. Written and compiled as separate binaries.
2. Independent of host runtime libraries (`glibc`, `musl`, Windows DLLs).
3. Capable of communicating with the Kinetix Kernel via our documented System Call ABI.

## Choices Considered
1. **Option A: Linux ABI Emulation**
   * *Advantages*: Could theoretically run standard Linux binaries (`/bin/sh`, busybox).
   * *Disadvantages*: Emulating hundreds of complex Linux syscalls (`clone`, `epoll`, `prctl`) is outside the scope of our research, introduces massive architectural complexity, and contradicts the goal of building an independent operating system.
2. **Option B: WebAssembly (Wasm) Micro-Runtime**
   * *Advantages*: Sandboxed, portable bytecode.
   * *Disadvantages*: Requires writing or embedding a Wasm interpreter/JIT inside the bare-metal kernel, adding layers of abstraction that obscure CPU multitasking and context switching experiments.
3. **Option C: Native Kinetix ELF64 Executables with `kinetix-runtime`**
   * *Advantages*:
     * True native machine code execution with hardware privilege separation.
     * Clean, documented, and minimal System Call interface designed specifically for Kinetix OS.
     * Applications are written in Rust (`no_std`) using `crates/kinetix-runtime`, compiling to static ELF64 binaries using `x86_64-unknown-none`.
     * Full control over process management, user stack allocation, and memory protection.
   * *Disadvantages*: Requires writing user-space runtime stubs and system call wrappers.

## Decision
We select **Option C**: Native Kinetix ELF64 executables linked against `crates/kinetix-runtime`.

## Consequences
* Applications are true native binaries that run directly on bare metal x86-64 hardware under Ring 3 privilege.
* No emulation layers or host runtimes exist in Kinetix OS.
* Demonstrates a complete, authentic operating-system user-space execution pipeline.
