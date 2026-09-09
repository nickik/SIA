# Scalable Instruction Architecture (SIA)

SIA is DEC's compact scalable RISC architecture for the Vision 2000 system family.

## Specifications

- [`SIA.md`](SIA.md) — base **SIA32-I** integer architecture.
- [`SIA32-P.md`](SIA32-P.md) — reconciled **Privileged Architecture** with only six privileged registers: `STATUS`, `EPC`, `CAUSE`, `BADADDR`, `SCRATCH`, and `VMCTX`; fixed platform trap vector; `SSWAP`, `SRET`, and `SRETCTX`.
- [`SIA32-MMU.md`](SIA32-MMU.md) — authoritative fast-context MMU: 2 KiB pages, 1 MiB superpages, 12-bit ASIDs, global mappings, and no TLB flush on ordinary `VMCTX` switches.
- [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md) — compact, non-conflicting v1 SYSTEM encoding proposal for the six-register privileged state and `SRETCTX`.
- [`SIA32-MEM.md`](SIA32-MEM.md) — strong **SIA-TSO** memory model, `FENCE`, MMIO ordering, coherent DMA, caches, and `SYNC.I`.
- [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md) — normative all-or-nothing fault/restart semantics for `LDP/STP/LD4/ST4`.
- [`SIA32-A.md`](SIA32-A.md) — optional **Atomic and Multiprocessing Extension** for coherent shared-memory systems.
- [`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) — ROM ABI stability, global ROM libraries, and shared arena/slab allocator code with independent kernel/user allocator state.
- [`SIA-PLATFORM.md`](SIA-PLATFORM.md) — machine/platform contract: fixed reset/trap vectors, physical memory, platform-owned interrupt controller, timer, firmware, PLIO integration, discovery, and the first `Lighting-1` profile.
- [`TODO.md`](TODO.md) — ordered architecture-completion checklist.

## Architecture model

The CPU architecture remains intentionally separate from platform facilities.

```text
SIA32-I
    base integer ISA

SIA32-P
    U/S privilege
    STATUS / EPC / CAUSE / BADADDR / SCRATCH / VMCTX
    one fixed platform TRAP_VECTOR
    one global CPU interrupt gate: STATUS.IE

SIA32-MMU + SIA32-MEM
    protected virtual memory + ordering/coherence

SIA Platform Specification
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

The first Lighting/Cosmic implementation requires the protected-system architecture plus a concrete `Lighting-1` platform profile.

The first required implementation of `SIA32-A` is DEC **Neutron**, targeted as a four-CPU ECL symmetric multiprocessor.

Future extensions must not redefine existing SIA encodings or architectural semantics.
