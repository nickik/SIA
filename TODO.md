# SIA Architecture Completion TODO

Canonical checklist for taking SIA from the frozen integer VM baseline to a complete protected system for Cosmic, the Rust full-system VM, and later FPGA/custom implementation.

## Status

- `[ ]` not started
- `[~]` in progress
- `[x]` complete

---

# 1. SIA32-I VM baseline

Normative base ISA: [`SIA.md`](SIA.md).

- [x] Freeze complete SIA32-I VM primary opcode map.
- [x] Freeze `0xD = ADC`, `0xE = SBB`.
- [x] Freeze CLZ/CTZ/CPOP destination-zero escapes.
- [x] Freeze LI/ADDI signed 7-bit immediates.
- [x] Freeze logic/shift allocation and all immediate shift counts.
- [x] Keep Boolean source-negation forms outside the baseline.
- [x] Freeze scalar memory modes.
- [x] Freeze pair/quad transfer modes and fault atomicity.
- [x] Freeze `LDPC.W = align_down(PC+4,4) + signed disp8*4`.
- [x] Freeze BNZ/DBNZ and B/BL displacement rules.
- [x] Keep BZ unassigned.
- [x] Freeze JALR/JR/CALLR/RET.
- [x] Freeze odd JALR target as precise instruction-alignment fault; never mask bit 0.
- [x] Freeze BSET/BCLR/BINV/BEXT/REV8 base assignments.
- [x] Freeze TRAP/BREAK/NOP encodings.
- [x] Validate with canonical assembler, golden words, success/fault fixtures, and randomized differential tests.
- [x] Mark optional SIA-Zmul/SIA-M as separate from mandatory SIA32-I baseline freeze.

The 2026-09-10 freeze is a **VM baseline**, not the final immutable SIA v1.0 compiler-tested ISA.

---

# 2. SIA32-P privileged architecture

Normative semantics: [`SIA32-P.md`](SIA32-P.md).

Encoding baseline: [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md).

Architecture decisions complete:

- [x] exactly two privilege levels: U/S.
- [x] exactly six privileged registers: STATUS, EPC, CAUSE, BADADDR, SCRATCH, VMCTX.
- [x] no TVEC/VMROOT/ASID/IENABLE/IPENDING architectural registers.
- [x] fixed platform TRAP_VECTOR.
- [x] one CPU asynchronous platform-interrupt condition.
- [x] one CPU global interrupt gate: STATUS.IE.
- [x] source pending/mask/priority/claim/complete state belongs to platform controller.
- [x] deterministic reset state.
- [x] deterministic STATUS reserved-bit behavior.
- [x] exact STATUS/VMCTX retirement/serialization semantics.
- [x] precise synchronous-before-asynchronous priority rule.
- [x] deterministic BADADDR rules.
- [x] TRAP/BREAK architectural trap semantics.
- [x] SSWAP SCRATCH-only baseline.
- [x] SRET semantics.
- [x] SRETCTX fast context return semantics.
- [x] SRET/SRETCTX reject odd EPC before return-state changes.
- [x] define trap-entry failure as fatal entry failure, not recursive ordinary exception.
- [x] WFI wake/IE semantics.

Implementation work:

- [~] Treat compact `0xF` SYSTEM encoding as the implementation baseline.
- [ ] Add structured guest exception representation to Rust VM.
- [ ] Add U/S mode + six-register state.
- [ ] Implement SREAD/SWRITE/SSWAP.
- [ ] Implement central architectural trap entry.
- [ ] Convert SIA32-I faults/TRAP/BREAK into SIA32-P traps.
- [ ] Implement SRET/SRETCTX.
- [ ] Implement WFI/FENCE/SYNC.I baseline behavior.
- [ ] Add single platform-interrupt input.
- [ ] Add assembly-driven privileged conformance tests.
- [ ] Freeze SIA32-P binary encoding after executable conformance is green.

---

# 3. MMU

Normative semantics: [`SIA32-MMU.md`](SIA32-MMU.md).

- [x] 32-bit virtual address space.
- [x] 2 KiB normal pages.
- [x] 1 MiB superpages.
- [x] three-level `6/6/9/11` walk.
- [x] 12-bit / 4096 ASIDs.
- [x] `VMCTX = root>>12 + ASID`.
- [x] VMCTX is the only architectural MMU context register.
- [x] 4 KiB root alignment.
- [x] global G mappings.
- [x] `SWRITE VMCTX` never flushes TLB merely for a context change.
- [x] TLBFENCE/TLBFENCE.VA/TLBFENCE.ASID semantics.
- [x] unified physical page/frame model.
- [ ] Add page walker to Rust VM after base P trap machinery is green.
- [ ] Add TLB/ASID/global mapping model.
- [ ] Add MMU conformance tests.

---

# 4. Multi-register exception semantics

Normative semantics: [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md).

- [x] LDP/STP/LD4/ST4 all-or-nothing for recoverable faults.
- [x] prevalidate addresses/translations/permissions before commit.
- [x] base writeback only on success.
- [x] cross-page semantics.
- [x] MMIO prohibition.
- [x] distinguish fault atomicity from SMP multiword atomicity.
- [ ] Add MMU page-boundary/permission conformance tests after translation exists.

---

# 5. Memory model and caches

Normative semantics: [`SIA32-MEM.md`](SIA32-MEM.md).

- [x] strong TSO-like normal-memory model.
- [x] only Store -> later Load to different address relaxation.
- [x] full FENCE semantics.
- [x] strongly ordered non-speculative MMIO.
- [x] coherent Lighting DMA contract.
- [x] no baseline data-cache clean/invalidate instructions.
- [x] SYNC.I semantics.
- [x] local and cross-CPU instruction-publication model.
- [x] page-table-store / TLBFENCE ordering.
- [ ] Add memory-model litmus tests.
- [~] Document final Cosmic W^X policy.

---

# 6. SIA Platform Specification

Platform skeleton: [`SIA-PLATFORM.md`](SIA-PLATFORM.md).

## 6.1 Fixed vectors and memory map

- [ ] Freeze Lighting-1 RESET_VECTOR.
- [ ] Freeze Lighting-1 TRAP_VECTOR.
- [ ] Define physical-mode trap backing/stub.
- [ ] Define Cosmic G=1 supervisor trap mapping.
- [ ] Freeze RAM/ROM/MMIO map and memory attributes.

## 6.2 Interrupt controller

- [ ] Define source namespace/ID width and maximum sources.
- [ ] Define pending representation.
- [ ] Define per-source enable/mask.
- [ ] Define CLAIM/COMPLETE.
- [ ] Decide priority and optional threshold model.
- [ ] Define spurious claim value.
- [ ] Define edge/level behavior.
- [ ] Define software-interrupt source.
- [ ] Define timer source.
- [ ] Define PLIO Notification mapping.
- [ ] Freeze controller MMIO layout.
- [ ] Add Rust VM controller only after the one-line CPU interrupt contract is tested.

## 6.3 Monotonic timer

- [ ] Define 64-bit monotonic counter and discovery frequency.
- [ ] Define stable 32-bit CPU read sequence.
- [ ] Define 64-bit deadline/compare and one-shot behavior.
- [ ] Define past-deadline, pending/rearm, wrap/reset behavior.
- [ ] Freeze timer MMIO layout.

## 6.4 Platform discovery / boot

- [ ] Define Platform Information Block.
- [ ] Define ROM layout/versioning.
- [ ] Decide minimal boot console.
- [ ] Define PLIO host and QDX boot-device profile.
- [ ] Define firmware -> Cosmic handoff.
- [ ] Define power/reset MMIO.

---

# 7. SIA ABI

Need normative `SIA32-ABI.md`.

Current direction:

```text
r0       zero
r1-r6    arguments / returns / fast IPC message registers
r7-r8    caller-saved temporaries
r9-r12   callee-saved
r13      sp
r14      lr / caller-saved link
r15      callee-saved general register; optional frame pointer
```

- [ ] Freeze arguments/returns/caller-save/callee-save.
- [ ] Freeze stack growth and 16-byte public-call alignment.
- [ ] Freeze optional frame-pointer convention.
- [ ] Define 64-bit values and aggregate passing.
- [ ] Define varargs if required.
- [ ] Decide TLS model.
- [ ] Define normal TRAP syscall ABI.
- [ ] Define fast IPC register ABI.
- [ ] Define asynchronous trap preservation and kernel trap-frame layout.
- [ ] Define unwind/debug metadata.

---

# 8. Object format and relocations

- [ ] Choose object container.
- [ ] Define sections/symbols.
- [ ] Define absolute, branch/call, and LDPC.W relocations.
- [ ] Define far veneers and literal-pool placement.
- [ ] Define relocation overflow behavior.
- [ ] Define data/TLS relocations as needed.
- [ ] Define Cosmic executable image and linker requirements.
- [ ] Add Forge relocation tests.

---

# 9. ROM/runtime ABI

Strategy: [`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md).

- [ ] Define fixed ROM ABI header/versioning/vector table.
- [ ] Define feature bits and compatibility rules.
- [ ] Define ROM allocator/runtime entry conventions.
- [ ] Keep shared allocator algorithms with independent process/kernel state.

---

# 10. Debug architecture — later

Possible SIA32-D work:

- [ ] instruction breakpoints.
- [ ] data watchpoints.
- [ ] single step.
- [ ] debug halt/resume.
- [ ] register/memory inspection.
- [ ] external debug interface.

---

# 11. SMP platform — later

SIA32-A supplies atomics/order; platform work still needs CPU IDs/startup, interrupt routing, software IPIs, per-CPU timers, coherent-memory contract, TLB shootdown, and CPU park/restart.

No new baseline privileged registers should be added merely for SMP.

---

# Immediate order

```text
1. implement structured exceptions in LightingSimulation
2. implement U/S + six-register SIA32-P state
3. implement SYSTEM decode + trap entry + SRET/SRETCTX
4. add one platform interrupt input and conformance tests
5. implement MMU/VMCTX/TLB/TLBFENCE
6. freeze SIA32-P binary encoding after executable tests
7. freeze SIA ABI
8. freeze Lighting RESET/TRAP vectors + physical memory map
9. define Lighting interrupt controller and timer
10. define Platform Information Block / boot contract
11. define object + relocation format
12. boot Cosmic
```

---

# Definition of first protected SIA system complete

- [x] SIA32-I VM baseline frozen.
- [x] SIA32-P semantics sufficiently specified for implementation.
- [x] six-register privileged state model defined.
- [x] MMU semantics defined.
- [x] multi-register fault semantics defined.
- [x] memory model/cache/DMA semantics defined.
- [ ] SIA32-P executable conformance green and binary encoding frozen.
- [ ] SIA ABI frozen.
- [ ] object/relocation format sufficient for Forge.
- [ ] Lighting fixed vectors/memory map frozen.
- [ ] Lighting interrupt controller/timer frozen.
- [ ] boot/platform-discovery contract frozen.
- [ ] Rust VM can implement all behavior without inventing unspecified rules.
- [ ] Cosmic boots and runs protected user processes entirely against documented architecture.
