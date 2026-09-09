# Scalable Instruction Architecture (SIA)

SIA is DEC's compact scalable RISC architecture for the Vision 2000 system family.

## Specifications

- [`SIA.md`](SIA.md) — base **SIA32-I** integer architecture.
- [`SIA32-P.md`](SIA32-P.md) — **Privileged Architecture** for protected operating systems, traps, interrupts, and Cosmic.
- [`SIA32-MMU.md`](SIA32-MMU.md) — authoritative fast-context MMU: 2 KiB pages, 1 MiB superpages, 12-bit ASIDs, global mappings, and no TLB flush on ordinary `VMCTX` switches.
- [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md) — proposed compact, non-conflicting v1 encoding for privileged/system operations.
- [`SIA32-MEM.md`](SIA32-MEM.md) — strong **SIA-TSO** memory model, `FENCE`, MMIO ordering, coherent DMA, caches, and `SYNC.I`.
- [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md) — normative all-or-nothing fault/restart semantics for `LDP/STP/LD4/ST4`.
- [`SIA32-A.md`](SIA32-A.md) — optional **Atomic and Multiprocessing Extension** for coherent shared-memory systems.
- [`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md) — ROM ABI stability, global ROM libraries, and shared arena/slab allocator code with independent kernel/user allocator state.
- [`TODO.md`](TODO.md) — ordered architecture-completion checklist.

## Extension model

The base architecture remains intentionally small. Optional architectural capabilities are documented separately so implementations pay only for features they need.

`SIA32-MEM` defines the baseline software-visible memory behavior used by protected Lighting systems; it does not require a particular cache implementation.

`SIA32-MMU.md` is the authoritative MMU contract and supersedes older draft MMU geometry still present in early `SIA32-P.md` text.

```text
small user/embedded CPU
    SIA32-I

protected single-CPU system
    SIA32-I + SIA32-P + SIA32-MMU + SIA32-MEM

coherent multiprocessor
    SIA32-I + SIA32-P + SIA32-MMU + SIA32-MEM + SIA32-A
```

The first Lighting/Cosmic implementation requires `SIA32-P`, `SIA32-MMU`, and the `SIA32-MEM` behavior.

The first required implementation of `SIA32-A` is DEC **Neutron**, targeted as a four-CPU ECL symmetric multiprocessor.

Future extensions must not redefine existing SIA encodings or architectural semantics.
