# Scalable Instruction Architecture (SIA)

SIA is DEC's compact scalable RISC architecture for the Vision 2000 system family.

## Specifications

- [`SIA.md`](SIA.md) — base **SIA32-I** integer architecture.
- [`SIA32-P.md`](SIA32-P.md) — reconciled **Privileged Architecture**: U/S protection, traps, interrupts, `VMCTX`, `SSWAP`, `SRET`, and `SRETCTX`.
- [`SIA32-MMU.md`](SIA32-MMU.md) — authoritative fast-context MMU: 2 KiB pages, 1 MiB superpages, 12-bit ASIDs, global mappings, and no TLB flush on ordinary `VMCTX` switches.
- [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md) — compact, non-conflicting v1 encoding proposal including `VMCTX` and `SRETCTX`.
- [`SIA32-MEM.md`](SIA32-MEM.md) — strong **SIA-TSO** memory model, `FENCE`, MMIO ordering, coherent DMA, caches, and `SYNC.I`.
- [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md) — normative all-or-nothing fault/restart semantics for `LDP/STP/LD4/ST4`.
- [`SIA32-A.md`](SIA32-A.md) — optional **Atomic and Multiprocessing Extension** for coherent shared-memory systems.
- [`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) — ROM ABI stability, global ROM libraries, and shared arena/slab allocator code with independent kernel/user allocator state.
- [`SIA-PLATFORM.md`](SIA-PLATFORM.md) — machine/platform contract and TODO for reset, physical memory, interrupts, timer, firmware, PLIO integration, discovery, and the first `Lighting-1` profile.
- [`TODO.md`](TODO.md) — ordered architecture-completion checklist.

## Architecture model

The instruction architecture remains intentionally separate from the machine/platform definition.

```text
SIA32-I
    base integer ISA

SIA32-P + SIA32-MMU + SIA32-MEM
    protected-system CPU architecture

SIA Platform Specification
    reset + memory map + timer + interrupts + firmware + devices

Lighting-1
    first concrete workstation platform profile
```

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