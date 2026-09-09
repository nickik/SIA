# SIA Architecture Completion TODO

Canonical checklist for taking SIA to a complete protected-system architecture for Cosmic, the Rust full-system VM, and later FPGA/custom implementation.

## Status

- `[ ]` not started
- `[~]` in progress
- `[x]` complete

---

# 1. Finalize instruction encoding

Current privileged proposal: [`SIA32-P-ENCODING.md`](SIA32-P-ENCODING.md).

- [ ] Freeze complete SIA32-I primary opcode map.
- [~] Freeze `0xD = ADC`, `0xE = SBB` direction.
- [ ] Freeze destination-zero escape encodings.
- [~] Freeze primary `0xF` SYSTEM/EXT mechanism.
- [~] Freeze privileged/system encodings:
  - [~] `SREAD`
  - [~] `SWRITE`
  - [~] `SSWAP`
  - [~] `SRET`
  - [~] `SRETCTX`
  - [~] `TLBFENCE`
  - [~] `TLBFENCE.VA`
  - [~] `TLBFENCE.ASID`
  - [~] `WFI`
  - [~] `SYNC.I`
  - [~] `FENCE`
- [x] Reduce baseline privileged system-register namespace to six registers:
  - [x] `STATUS`
  - [x] `EPC`
  - [x] `CAUSE`
  - [x] `BADADDR`
  - [x] `SCRATCH`
  - [x] `VMCTX`
- [x] Remove `TVEC`, `VMROOT`, `ASID`, `IENABLE`, and `IPENDING` as separate privileged registers.
- [~] Freeze reserved/illegal encoding behavior.
- [ ] Update interpreter/disassembler/assembler tables.
- [ ] Mark v1.0 instruction encodings stable and non-redefinable.

---

# 2. Privileged architecture

Normative semantics: [`SIA32-P.md`](SIA32-P.md).

- [x] U/S privilege model.
- [x] Minimal six-register privileged state.
- [x] One fixed platform-defined `TRAP_VECTOR`; no writable trap-vector register.
- [x] One CPU global interrupt gate: `STATUS.IE`.
- [x] One CPU-level asynchronous cause: `PLATFORM_INTERRUPT`.
- [x] Interrupt pending/mask/source/priority state moved to platform interrupt controller.
- [x] `SSWAP sp,SCRATCH` trap-stack transition.
- [x] `SRET` semantics.
- [x] `SRETCTX` fast context-switch return semantics.
- [x] `SRETCTX` does not flush TLB or pre-walk EPC.
- [ ] Add privileged conformance tests.

---

# 3. MMU

Normative semantics: [`SIA32-MMU.md`](SIA32-MMU.md).

- [x] 32-bit virtual address space.
- [x] 2 KiB normal pages.
- [x] 1 MiB superpages.
- [x] Three-level `6/6/9/11` walk.
- [x] 12-bit / 4096 ASIDs.
- [x] `VMCTX = root>>12 + ASID`.
- [x] `VMCTX` is the only architectural MMU context register.
- [x] 4 KiB root alignment.
- [x] Global `G` mappings.
- [x] Global trap/kernel/ROM mappings supported.
- [x] `SWRITE VMCTX` never flushes TLB.
- [x] `TLBFENCE*` mapping-change semantics.
- [x] Unified physical page/frame model; no instruction/data page types.
- [ ] Add MMU conformance tests.

---

# 4. Multi-register exception semantics

Normative semantics: [`SIA32-MULTI-TRANSFER.md`](SIA32-MULTI-TRANSFER.md).

- [x] `LDP/STP/LD4/ST4` all-or-nothing for recoverable faults and interrupts.
- [x] Prevalidate all addresses/translations/permissions before commit.
- [x] Base writeback only on success.
- [x] Cross-page semantics.
- [x] MMIO prohibition.
- [x] Distinguish fault atomicity from SMP multiword atomicity.
- [ ] Add page-boundary/permission conformance tests.

---

# 5. Memory model and caches

Normative semantics: [`SIA32-MEM.md`](SIA32-MEM.md).

- [x] Strong TSO-like normal-memory model.
- [x] Only ordinary Store -> later Load to different address relaxation.
- [x] Full `FENCE` semantics.
- [x] Strongly ordered non-speculative MMIO.
- [x] Coherent Lighting DMA.
- [x] No baseline data-cache clean/invalidate instructions.
- [x] `SYNC.I` semantics.
- [x] Local and cross-CPU instruction-publication model.
- [x] Page-table-store / `TLBFENCE` ordering.
- [ ] Add memory-model litmus tests.
- [~] Document final Cosmic W^X policy.

---

# 6. SIA Platform Specification

Platform skeleton: [`SIA-PLATFORM.md`](SIA-PLATFORM.md).

## 6.1 Fixed vectors and memory map

- [ ] Freeze `Lighting-1 RESET_VECTOR`.
- [ ] Freeze `Lighting-1 TRAP_VECTOR`.
- [ ] Define physical-mode trap backing/stub.
- [ ] Define Cosmic `G=1` supervisor trap mapping.
- [ ] Freeze RAM/ROM/MMIO map.
- [ ] Freeze memory attributes.

## 6.2 Interrupt controller

The CPU has no `IENABLE`/`IPENDING` registers and no source-specific interrupt state.

- [ ] Define interrupt-source namespace/ID width.
- [ ] Define maximum sources.
- [ ] Define pending representation.
- [ ] Define per-source enable/mask.
- [ ] Define `CLAIM` operation.
- [ ] Define `COMPLETE` operation.
- [ ] Decide fixed/programmed priority model.
- [ ] Define optional threshold/nesting model.
- [ ] Define spurious claim value.
- [ ] Define edge/level behavior.
- [ ] Define software-interrupt source.
- [ ] Define timer source.
- [ ] Define PLIO Notification mapping.
- [ ] Freeze controller MMIO layout.
- [ ] Add Rust VM interrupt-controller model.

## 6.3 Monotonic timer

- [ ] Define 64-bit monotonic counter.
- [ ] Define frequency/discovery.
- [ ] Define stable 32-bit CPU read sequence.
- [ ] Define 64-bit deadline/compare.
- [ ] Define one-shot behavior.
- [ ] Define past-deadline behavior.
- [ ] Define controller pending/rearm behavior.
- [ ] Define wraparound/reset behavior.
- [ ] Freeze timer MMIO layout.

## 6.4 Platform discovery / boot

- [ ] Define Platform Information Block.
- [ ] Define ROM layout and versioning.
- [ ] Decide minimal boot console.
- [ ] Define PLIO host integration.
- [ ] Define QDX boot-device profile.
- [ ] Define firmware -> Cosmic handoff.
- [ ] Define power/reset MMIO.

---

# 7. Freeze SIA ABI

Need a normative [`SIA32-ABI.md`] document.

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

- [ ] Freeze argument registers.
- [ ] Freeze return-value registers.
- [ ] Freeze caller/callee-save sets.
- [ ] Freeze stack growth and 16-byte call-boundary alignment.
- [ ] Freeze optional frame-pointer convention.
- [ ] Define 64-bit integer arguments/results.
- [ ] Define aggregate passing/returns.
- [ ] Define varargs if required.
- [ ] Decide TLS model.
- [ ] Define normal `TRAP` syscall ABI.
- [ ] Define fast IPC register ABI.
- [ ] Define asynchronous trap/interrupt preservation rules.
- [ ] Define kernel trap-frame layout.
- [ ] Define unwind/debug metadata convention.

---

# 8. Object format and relocations

- [ ] Choose/define object container.
- [ ] Define sections/symbols.
- [ ] Define absolute 32-bit relocation.
- [ ] Define branch/call relocations.
- [ ] Define `LDPC.W` literal relocation.
- [ ] Define far call/jump veneers.
- [ ] Define literal-pool placement.
- [ ] Define overflow behavior.
- [ ] Define data-pointer relocations.
- [ ] Define TLS relocations if TLS is adopted.
- [ ] Define Cosmic executable image format.
- [ ] Define kernel/firmware linker requirements.
- [ ] Add Forge relocation tests.

---

# 9. ROM/runtime ABI

Strategy: [`SIA-ROM-RUNTIME.md`](SIA-ROM-RUNTIME.md).

- [ ] Define fixed ROM ABI header.
- [ ] Define ABI versioning.
- [ ] Define stable vector/interface table.
- [ ] Define feature bits.
- [ ] Define compatibility rules.
- [ ] Define ROM allocator/runtime entry conventions.
- [ ] Keep allocator algorithms shared while allocator state remains per-kernel/per-process.

---

# 10. Debug architecture — later

Possible `SIA32-D` work:

- [ ] instruction breakpoints.
- [ ] data watchpoints.
- [ ] single step.
- [ ] debug halt/resume.
- [ ] register/memory inspection.
- [ ] external debug interface.

The Rust VM can provide richer non-architectural debugging before this exists.

---

# 11. SMP platform — later

`SIA32-A` supplies atomics/order; platform work still needs:

- [ ] CPU IDs/count.
- [ ] secondary CPU reset/start.
- [ ] interrupt-controller routing.
- [ ] software IPIs.
- [ ] per-CPU timers.
- [ ] coherent-memory contract.
- [ ] TLB-shootdown protocol.
- [ ] CPU halt/park/restart.

No new baseline privileged registers should be added merely for SMP.

---

# Immediate order

```text
1. freeze remaining SIA32-I instruction encodings
2. freeze compact SYSTEM encoding
3. write/freeze SIA32 ABI
4. freeze Lighting RESET/TRAP vectors + physical memory map
5. define Lighting interrupt controller
6. define monotonic timer
7. define Platform Information Block / boot contract
8. define object + relocation format
9. implement Rust full-system VM
10. boot Cosmic
```

---

# Definition of first protected SIA system complete

- [ ] SIA32-I encodings frozen.
- [~] SIA32-P semantics frozen; tests pending.
- [x] six-register privileged state model defined.
- [x] MMU semantics defined.
- [x] multi-register fault semantics defined.
- [x] memory model/cache/DMA semantics defined.
- [ ] SIA ABI frozen.
- [ ] object/relocation format sufficient for Forge.
- [ ] Lighting fixed vectors/memory map frozen.
- [ ] Lighting interrupt controller frozen.
- [ ] Lighting timer frozen.
- [ ] boot/platform-discovery contract frozen.
- [ ] Rust VM can implement all behavior without inventing unspecified rules.
- [ ] Cosmic boots and runs protected user processes entirely against documented architecture.
