# Scalable Instruction Architecture (SIA)

SIA is DEC's compact scalable RISC architecture for the Vision 2000 system family.

## Current architecture status

**SIA32-I VM baseline is frozen as of 2026-09-10.**

The freeze covers the tested integer encoding/semantics used by `nickik/LightingSimulation`, including `0xD=ADC`, `0xE=SBB`, destination-zero CLZ/CTZ/CPOP escapes, exact scalar and pair/quad memory modes, the aligned-PC+4 `LDPC.W` rule, BNZ/DBNZ/B/BL, no dedicated BZ, and `JALR` odd-target alignment faults.

This is an executable VM baseline rather than a claim that the final compiler-tested SIA v1.0 can never evolve. Any incompatible change requires an explicit successor baseline.

SIA32-P semantics are now specified tightly enough for implementation. Its binary encoding remains an implementation baseline until privileged conformance tests are green.

## Specifications

- [`SIA.md`](SIA.md) — frozen **SIA32-I VM baseline** integer architecture.
- [`SIA32-P.md`](SIA32-P.md) — **Privileged Architecture**: U/S privilege, deterministic trap/return semantics, six privileged registers (`STATUS`, `EPC`, `CAUSE`, `BADADDR`, `SCRATCH`, `VMCTX`), fixed platform trap vector, `SSWAP`, `SRET`, and `SRETCTX`.
- [`SIA32-MMU.md`](SIA32-MMU.md) — authoritative fast-context MMU: 2 KiB pages, 1 MiB superpages, 12-bit ASIDs, global mappings, and no TLB flush on ordinary `VMCTX` switches.
- [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md) — compact SYSTEM encoding implementation baseline; freeze follows executable privileged conformance.
- [`SIA32-MEM.md`](SIA32-MEM.md) — strong **SIA-TSO** memory model, `FENCE`, MMIO ordering, coherent DMA, caches, and `SYNC.I`.
- [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md) — normative all-or-nothing fault/restart semantics for `LDP/STP/LD4/ST4`.
- [`SIA32-A.md`](SIA32-A.md) — optional **Atomic and Multiprocessing Extension** for coherent shared-memory systems.
- [`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) — ROM ABI stability, global ROM libraries, and shared arena/slab allocator code with independent kernel/user allocator state.
- [`SIA-PLATFORM.md`](SIA-PLATFORM.md) — machine/platform contract: fixed reset/trap vectors, physical memory, platform-owned interrupt controller, timer, firmware, PLIO integration, discovery, and the first `Lighting-1` profile.
- [`TODO.md`](TODO.md) — ordered architecture/implementation checklist.

## Executable implementation

The Rust executable reference implementation is in [`nickik/LightingSimulation`](https://github.com/nickik/LightingSimulation).

`LightingSimulation` contains:

```text
siaasm
    canonical SIA32-I assembler

siaemu
    Rust executable reference interpreter

conformance tests
    assembly -> exact encoding -> execution -> expected architectural result
    exact golden encodings
    success and precise-fault cases
    deterministic randomized differential tests
```

The integer gate is complete. The next implementation milestone is SIA32-P privilege/trap machinery, beginning with structured architectural exceptions before MMU or platform-controller integration.

## Architecture model

```text
SIA32-I
    frozen base integer VM baseline

SIA32-P
    U/S privilege
    STATUS / EPC / CAUSE / BADADDR / SCRATCH / VMCTX
    fixed platform TRAP_VECTOR
    one global CPU interrupt gate: STATUS.IE

SIA32-MMU + SIA32-MEM
    protected virtual memory + ordering/coherence

SIA Platform
    RESET_VECTOR
    TRAP_VECTOR value/backing
    physical memory map
    interrupt pending/mask/priority/claim/complete
    timer
    firmware
    devices

Lighting-1
    first concrete workstation platform profile
```

The platform interrupt controller owns source-specific interrupt state. The CPU does not have `TVEC`, `IENABLE`, `IPENDING`, `VMROOT`, or `ASID` registers.

```text
small user/embedded CPU
    SIA32-I

protected single-CPU system
    SIA32-I + SIA32-P + SIA32-MMU + SIA32-MEM

coherent multiprocessor
    SIA32-I + SIA32-P + SIA32-MMU + SIA32-MEM + SIA32-A
```

Future extensions must not silently redefine an existing frozen-baseline encoding or semantic rule.
