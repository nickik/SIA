# Scalable Instruction Architecture (SIA)

SIA is DEC's compact scalable RISC architecture for the Vision 2000 system family.

## Specifications

- [`SIA.md`](SIA.md) — base **SIA32-I** integer architecture.
- [`SIA32-P.md`](SIA32-P.md) — **Privileged Architecture** for protected operating systems, virtual memory, traps, interrupts, and Cosmic.
- [`SIA32-A.md`](SIA32-A.md) — optional **Atomic and Multiprocessing Extension** for coherent shared-memory systems.

## Extension model

The base architecture remains intentionally small. Optional architectural capabilities are documented separately so implementations pay only for features they need.

```text
small user/embedded CPU
    SIA32-I

protected single-CPU system
    SIA32-I + SIA32-P

coherent multiprocessor
    SIA32-I + SIA32-P + SIA32-A
```

The first Lighting/Cosmic implementation requires `SIA32-P`.

The first required implementation of `SIA32-A` is DEC **Neutron**, targeted as a four-CPU ECL symmetric multiprocessor.

Future extensions must not redefine existing SIA encodings or architectural semantics.
