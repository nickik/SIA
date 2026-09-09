# SIA Architecture Completion TODO

This is the canonical checklist for taking SIA from a compact integer ISA plus initial privilege/atomic extensions to a complete system architecture suitable for Cosmic, the Rust full-system VM, and later FPGA implementation.

Work through this list in order unless a later item becomes a direct blocker for implementation or measurement.

## Status convention

- `[ ]` not started
- `[~]` in progress
- `[x]` complete

---

# 1. Finalize the instruction encoding

SIA32-I still contains provisional opcode assignments. Before the architecture can become a stable implementation target, the base and privileged instruction encodings need to be frozen.

Current proposal: [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md).

- [ ] Freeze the complete SIA32-I primary opcode map.
- [~] Resolve the current `ADC` / `SBB` opcode pressure — current recommendation is primary `0xD` = `ADC`, `0xE` = `SBB`.
- [ ] Freeze destination-zero escape encodings.
- [~] Freeze the `EXT` mechanism — current recommendation uses primary `0xF` for compact SYSTEM operations and reserves `0xFFxx` as the long-extension escape.
- [~] Allocate encodings for SIA32-P privileged operations:
  - [~] `SREAD`
  - [~] `SWRITE`
  - [~] `SSWAP`
  - [~] `SRET`
  - [~] `TLBFENCE`
  - [~] `TLBFENCE.VA`
  - [~] `TLBFENCE.ASID`
  - [~] `WFI`
- [~] Add `SYNC.I` instruction-fetch synchronization operation — semantics specified and compact encoding proposed.
- [x] Decide whether `SYNC.I` belongs in SIA32-I or a mandatory system/cache extension — defined by mandatory Lighting `SIA32-MEM` behavior and encoded in SYSTEM space.
- [~] Freeze illegal/reserved encoding behavior — proposal now distinguishes User privilege faults from reserved/invalid Supervisor encodings.
- [ ] Update interpreter/disassembler/assembler tables to use the frozen encoding.
- [ ] Mark v1.0 instruction encodings as stable and non-redefinable.

---

# 2. Define precise multi-register exception semantics

Normative semantics: [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md).

`LDP`, `STP`, `LD4`, and `ST4` are **all-or-nothing with respect to recoverable architectural faults and interrupts**.

- [x] Define whether multi-register transfers are architecturally atomic with respect to faults.
- [x] Define all-or-nothing architectural completion.
- [x] Define complete-transfer validation before architectural register or memory modification.
- [x] Define load destination update behavior on a fault.
- [x] Define store visibility on a fault.
- [x] Define base-register writeback behavior on a fault.
- [x] Define exception PC for a failed multi-register instruction.
- [x] Define precise behavior if a transfer crosses a page boundary.
- [x] Define behavior for MMIO mappings; multi-register transfers remain prohibited for MMIO.
- [x] Distinguish fault atomicity from SMP multiword atomicity.
- [ ] Add conformance tests for all page-boundary and permission combinations.

---

# 3. Freeze the SIA memory model

Normative model: [`SIA32-MEM.md`](SIA32-MEM.md).

SIA uses a strong **TSO-like** normal-memory model. The only ordinary relaxation is Store -> later Load to a different address. MMIO is stronger and fully ordered.

- [x] Define ordering of ordinary loads relative to older loads.
- [x] Define ordering of ordinary loads relative to older stores.
- [x] Define ordering of ordinary stores relative to older loads.
- [x] Define ordering of ordinary stores relative to older stores.
- [x] Define when speculative execution may become architecturally visible.
- [x] Define multi-copy/global visibility expectations for ordinary memory.
- [x] Define interaction with `SIA32-A` atomics.
- [x] Define exact semantics of `FENCE`.
- [x] Define that the strong baseline model makes `FENCE` rare in ordinary code.
- [x] Define MMIO ordering separately from normal RAM.
- [x] Define MMIO as strongly ordered and non-speculative.
- [x] Define DMA visibility requirements for Lighting.
- [x] Define CPU-to-device publish sequence.
- [x] Define device-to-CPU completion/readback sequence.
- [x] Define interaction between page-table writes and `TLBFENCE`.
- [x] Define interaction between code writes and `SYNC.I`.
- [x] Define instruction-fetch synchronization separately from ordinary data coherence.
- [ ] Add litmus tests to the Rust interpreter/VM.

---

# 4. Define cache-management and instruction synchronization semantics

Specified in [`SIA32-MEM.md`](SIA32-MEM.md).

The baseline deliberately avoids software-managed data-cache coherence.

- [x] Define `SYNC.I` semantics:
  - [x] prior stores become visible to subsequent local instruction fetches;
  - [x] stale prefetch/decode/I-cache state cannot cause old instructions to execute after synchronization;
  - [x] operation remains valid on cacheless systems as a legal no-op or pipeline synchronization.
- [x] Define SMP expectations for remote instruction synchronization.
- [x] Require Cosmic to coordinate cross-CPU code publication when needed.
- [x] Define whether explicit data-cache clean/invalidate operations are required by baseline SIA — **no** for Lighting.
- [x] Require coherent normal RAM in the baseline Lighting profile.
- [x] Define MMIO as non-cacheable from the software-visible point of view.
- [x] Define normal-RAM cache behavior as transparent/coherent.
- [x] Define DMA/cache interaction.
- [x] Define PLIO/Lighting DMA as coherent with CPU caches.
- [x] Noncoherent DMA cache-maintenance operations are outside the baseline profile.
- [x] Define page tables as normal coherent physical memory.
- [x] Define self-modifying/JIT code sequence.
- [~] Document final recommended W^X policy for Cosmic in the Cosmic specification.

---

# 5. Define the Lighting platform interrupt-controller interface

SIA32-P defines the CPU-level interrupt classes:

```text
software
timer
external
```

The actual Lighting interrupt controller belongs in the platform specification rather than the generic SIA ISA.

- [ ] Define CPU-visible external interrupt line/condition.
- [ ] Define PLIO Notification to CPU interrupt mapping.
- [ ] Define pending state.
- [ ] Define per-source masking.
- [ ] Define claim/identify operation.
- [ ] Define acknowledge/complete operation.
- [ ] Define priority model.
- [ ] Define nesting/preemption policy.
- [ ] Define spurious interrupt behavior.
- [ ] Define interrupt-controller MMIO register layout.
- [ ] Define interaction with `IENABLE` / `IPENDING` in SIA32-P.
- [ ] Define software interrupt generation.
- [ ] Add a Rust VM interrupt-controller model.

---

# 6. Define architectural timer expectations

A seL4-style kernel needs a reliable scheduling timer and monotonic time source.

The timer should probably remain a platform MMIO device rather than become additional CPU privileged state.

- [ ] Define a 64-bit monotonic counter.
- [ ] Define counter frequency or discovery mechanism.
- [ ] Define read semantics on a 32-bit CPU.
- [ ] Define compare/deadline register.
- [ ] Define one-shot timer behavior.
- [ ] Define timer interrupt generation.
- [ ] Define acknowledgement/rearm behavior.
- [ ] Define behavior when programmed deadline is already in the past.
- [ ] Define wraparound expectations.
- [ ] Define reset value/behavior.
- [ ] Decide whether periodic mode exists or is synthesized in software.
- [ ] Define real-time clock separately from monotonic scheduling time.
- [ ] Implement timer in Rust VM before Cosmic scheduler work.

---

# 7. Freeze the SIA ABI

Forge and Cosmic need a normative SIA ABI rather than a suggested convention.

- [ ] Freeze argument registers.
- [ ] Freeze return-value registers.
- [ ] Freeze caller-saved registers.
- [ ] Freeze callee-saved registers.
- [ ] Freeze stack pointer role.
- [ ] Freeze link register role.
- [ ] Freeze frame pointer convention.
- [ ] Define stack growth direction.
- [ ] Define stack alignment.
- [ ] Define scalar argument extension rules.
- [ ] Define 64-bit integer argument/return conventions.
- [ ] Define struct/aggregate argument passing.
- [ ] Define small aggregate return convention.
- [ ] Define large aggregate return convention.
- [ ] Define varargs convention if Forge needs it.
- [ ] Define TLS model/register convention if required.
- [ ] Define syscall ABI.
- [ ] Define trap argument/result convention.
- [ ] Define kernel entry scratch-register usage.
- [ ] Define unwind/debug frame conventions if desired.
- [ ] Publish a normative `SIA32-ABI.md`.

---

# 8. Define object format and relocations

SIA uses short branches and PC-relative literal pools, so the assembler/linker ABI is particularly important.

- [ ] Choose or define object-file container format.
- [ ] Define section types.
- [ ] Define symbol representation.
- [ ] Define absolute 32-bit relocation.
- [ ] Define PC-relative branch relocation.
- [ ] Define PC-relative call relocation.
- [ ] Define `LDPC.W` literal relocation.
- [ ] Define far call/jump relocation strategy.
- [ ] Define linker veneers/trampolines.
- [ ] Define literal-pool placement rules.
- [ ] Define overflow handling for out-of-range branches.
- [ ] Define data-pointer relocations.
- [ ] Define TLS relocations if TLS is adopted.
- [ ] Define executable image format for Cosmic user programs.
- [ ] Define kernel/firmware linker requirements.
- [ ] Add relocation tests to Forge toolchain.

---

# 9. Add a debug architecture later

This is not required for first Cosmic boot because the Rust VM can provide richer debugging externally, but real FPGA systems eventually need hardware debug support.

Possible `SIA32-D` work:

- [ ] hardware instruction breakpoint.
- [ ] hardware data watchpoint.
- [ ] single-step support.
- [ ] debug halt.
- [ ] debug resume.
- [ ] debug cause/status.
- [ ] register inspection/modification mechanism.
- [ ] memory inspection/modification mechanism.
- [ ] external debug-port architecture.
- [ ] interaction with interrupts and privilege state.
- [ ] interaction with SMP.

---

# 10. Define SMP platform architecture when needed

`SIA32-A` supplies atomics and ordering, but a complete SMP platform also needs processor-management mechanisms.

These should be specified separately from `SIA32-P`, likely as a Neutron/SIA SMP platform document.

- [ ] CPU/hart identification.
- [ ] discover number of CPUs.
- [ ] secondary CPU reset/startup state.
- [ ] secondary CPU release/start address.
- [ ] interprocessor interrupt generation.
- [ ] interprocessor interrupt acknowledgement.
- [ ] CPU halt/park.
- [ ] CPU restart.
- [ ] TLB shootdown protocol expectations.
- [ ] cache-coherence contract.
- [ ] memory-ordering implications for SMP.
- [ ] `SYNC.I` cross-CPU publication requirements.
- [ ] per-CPU timer/interrupt expectations.
- [ ] CPU failure/offline semantics if required.

---

# Immediate specification order

Work through the architecture in this sequence:

```text
SIA32-I                mostly defined
   |
   v
SIA32-P                defined
   |
   v
1. encoding freeze     proposal written; still needs final freeze
   |
   v
2. multi-register      semantics specified
   |
   v
3. memory model        SIA-TSO specified
   |
   v
4. cache + SYNC.I      baseline specified
   |
   v
5. Lighting interrupt controller
   |
   v
6. Lighting timer
   |
   v
7. SIA ABI
   |
   v
8. object format + relocations
   |
   v
Rust full-system VM
   |
   v
Cosmic development
```

The debug and SMP specifications can follow once the single-CPU Lighting/Cosmic platform is working.

---

# Definition of "SIA32 system architecture complete"

For the first protected single-CPU Lighting implementation, SIA is complete enough when all of the following are stable:

- [ ] SIA32-I instruction semantics and encodings are frozen.
- [ ] SIA32-P is frozen.
- [x] multi-register exception/restart behavior is frozen.
- [x] memory ordering is specified; final v1 freeze follows encoding integration/tests.
- [~] `FENCE`, `SYNC.I`, and `TLBFENCE` relationships are specified; encodings still need final freeze.
- [x] MMIO ordering/cacheability behavior is specified.
- [x] DMA visibility/coherency rules are specified for Lighting.
- [ ] SIA ABI is frozen.
- [ ] object/relocation formats are sufficient for Forge.
- [ ] Lighting timer and interrupt-controller contracts are frozen.
- [ ] the Rust VM can implement all architectural behavior without inventing unspecified rules.
- [ ] Cosmic can boot and run protected user processes entirely against documented architecture.
