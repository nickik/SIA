# Scalable Instruction Architecture (SIA)

SIA is DEC's compact scalable RISC architecture for the Vision 2000 system family.

## Specifications

- [`SIA.md`](SIA.md) — base **SIA32-I** integer architecture.
- [`SIA32-A.md`](SIA32-A.md) — optional **Atomic and Multiprocessing Extension** for coherent shared-memory systems.

## Extension model

The base architecture remains intentionally small. Optional architectural capabilities are documented separately so implementations pay only for features they need.

```text
small single-CPU system
    SIA32-I

coherent multiprocessor
    SIA32-I + SIA32-A
```

The first required implementation of `SIA32-A` is DEC **Neutron**, targeted as a four-CPU ECL symmetric multiprocessor.

Future extensions must not redefine existing SIA encodings or architectural semantics.
