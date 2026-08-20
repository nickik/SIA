# SIA32 Software ABI

## Draft v0.1

This document defines the initial software ABI for `SIA32-I` programs.

The ISA itself is defined in [`SIA.md`](SIA.md). This ABI specifies the conventions required for separately compiled code to interoperate: register preservation, calls, returns, stack layout, argument passing, scalar data representation, and frame construction.

The initial ABI is intentionally small enough for the Forge bootstrap compiler and SIA interpreter while leaving room for later ELF, shared-library, floating-point, and operating-system ABI documents.

---

# 1. ABI goals

The SIA32 ABI is designed to:

- keep ordinary calls cheap on a machine with only 16 integer registers;
- exploit SIA's compact `LWSP`, `SWSP`, `ADJSP`, `LDP/STP`, and `LD4/ST4` instructions;
- permit leaf functions with no stack frame;
- keep frame pointers optional;
- make 32-bit integer/pointer code the fast path;
- support 64-bit integers and software floating point without hidden machine state;
- keep the stack mechanically simple for debuggers and unwinders;
- provide a stable target for Forge and other compilers.

---

# 2. Data model

The initial SIA32 ABI uses a 32-bit address space and little-endian data layout.

```text
pointer width   32 bits
usize/isize     32 bits
byte             8 bits
halfword        16 bits
word            32 bits
```

Natural ABI alignment:

| Type class | Size | Alignment |
|---|---:|---:|
| `i8/u8/byte/char8` | 1 | 1 |
| `i16/u16` | 2 | 2 |
| `i32/u32/pointer/usize/isize/f32 bits` | 4 | 4 |
| `i64/u64/f64 bits` | 8 | 4 |

The 4-byte alignment of 64-bit scalar values is intentional: base SIA32 performs 64-bit values as two 32-bit words and requires only word alignment. A future ABI variant may impose stronger alignment for a hardware 64-bit or floating-point extension, but it must use a distinct ABI name if it breaks layout compatibility.

Aggregate alignment is the maximum alignment of its members, subject to explicit representation attributes defined by the source language or system ABI.

---

# 3. Register convention

| Register | ABI name/role | Preservation |
|---|---|---|
| `r0` | zero | immutable |
| `r1-r6` | arguments / return values | caller-saved |
| `r7-r8` | temporaries | caller-saved |
| `r9-r12` | saved registers | callee-saved |
| `r13` | `sp` | restored by callee |
| `r14` | `lr` link register | overwritten by calls |
| `r15` | saved GPR / optional `fp` | callee-saved |

## 3.1 Caller-saved registers

A call may destroy:

```text
r1-r8
r14
```

A caller that needs one of these values after a call must preserve it before the call.

## 3.2 Callee-saved registers

A callee that modifies any of:

```text
r9-r12
r15
```

must restore the original value before returning.

`r13/sp` must equal its entry value when the callee returns.

A callee need only save the callee-saved registers it actually modifies.

## 3.3 Link register

`BL` and normal indirect calls place the return address in `r14`.

A leaf function that makes no further call may return directly with:

```asm
RET
```

without saving `r14`.

A non-leaf function must preserve its incoming return address before issuing a call that overwrites `r14`.

---

# 4. Boolean representation

SIA comparison instructions naturally produce full-register masks. The standard ABI therefore uses the canonical register representation:

```text
false = 0x00000000
true  = 0xFFFFFFFF
```

When a Boolean is stored as a standalone byte in memory:

```text
false = 0x00
true  = 0xFF
```

This permits:

```asm
SB   rX,[addr]
LB   rX,[addr]
```

to preserve the canonical register representation without extra conversion.

Noncanonical nonzero values may be accepted at foreign/system boundaries only when that boundary explicitly specifies truth-by-nonzero semantics. Compiler-generated SIA ABI calls use canonical Boolean values.

---

# 5. Integer and scalar register representation

Values narrower than 32 bits are canonicalized when passed in registers or returned from functions:

- signed 8/16-bit integers are sign-extended to 32 bits;
- unsigned 8/16-bit integers are zero-extended to 32 bits;
- Boolean values use the canonical mask representation above;
- pointers occupy one 32-bit register;
- `f32` under the soft-float ABI is passed as its raw 32-bit bit pattern.

A 64-bit scalar occupies two consecutive 32-bit words:

```text
lower-numbered register / lower-address word = bits 31:0
higher-numbered register / next word         = bits 63:32
```

For example:

```text
r1:r2 representation of 64-bit value:
r1 = low 32 bits
r2 = high 32 bits
```

This document writes such a value as `r1:r2` even though the low word is in `r1`.

---

# 6. Argument passing

Arguments are assigned left-to-right.

## 6.1 Register arguments

The first six available 32-bit argument words use:

```text
r1 r2 r3 r4 r5 r6
```

A scalar of 32 bits or less consumes one argument register.

A 64-bit scalar consumes two consecutive argument registers. It may begin at any of `r1` through `r5`; no even/odd pairing restriction is imposed.

If a multiword argument cannot fit entirely in the remaining argument registers, it is not split between registers and stack. It and all following arguments are passed on the stack.

This rule keeps argument reconstruction simple and avoids half-register/half-stack values.

## 6.2 Stack arguments

Once stack passing begins, arguments are placed at increasing addresses starting at the caller's stack pointer used for the call:

```text
callee entry sp + 0    first stack argument word
callee entry sp + 4    next word
callee entry sp + 8    ...
```

The caller allocates the stack-argument area before the call and releases it after the call.

Each stack argument is rounded to a multiple of 4 bytes. Scalar values use the representations defined above.

The call-boundary stack pointer must remain 16-byte aligned, so the caller may include padding after the final stack argument.

## 6.3 Examples

Six 32-bit arguments:

```text
a -> r1
b -> r2
c -> r3
d -> r4
e -> r5
f -> r6
```

Seven 32-bit arguments:

```text
a..f -> r1..r6
g    -> [entry_sp + 0]
```

Example with a 64-bit value:

```text
fn f(a:u32, b:u64, c:u32)

a       -> r1
b.low   -> r2
b.high  -> r3
c       -> r4
```

If only one register remains when a 64-bit argument is encountered, that argument and later arguments go entirely to the stack.

---

# 7. Return values

## 7.1 Scalars up to 32 bits

Returned in:

```text
r1
```

Narrow values use canonical extension rules.

## 7.2 64-bit scalars

Returned in:

```text
r1 = low 32 bits
r2 = high 32 bits
```

This includes `i64`, `u64`, and soft-float `f64` bit patterns.

## 7.3 Small aggregates

An aggregate of 1-4 bytes that is eligible for direct value return is returned in `r1`.

An aggregate of 5-8 bytes that is eligible for direct value return is returned in `r1:r2`, with the same little-endian word ordering as a 64-bit scalar.

Languages may choose memory return for aggregates with unusual alignment, nontrivial copy semantics, or source-level ABI attributes.

## 7.4 Large aggregate return

Aggregates larger than 8 bytes are returned through a caller-provided result buffer.

The hidden result pointer is passed in `r1`. User-visible argument assignment then begins at `r2`.

The callee writes the result to the supplied buffer and returns the same buffer pointer in `r1`.

This convention simplifies forwarding and debugging.

---

# 8. Stack model

The stack grows toward lower addresses.

`r13` is architecturally `sp`.

At every normal external function-call boundary:

```text
sp % 16 == 0
```

A function may temporarily have a differently aligned `sp` while constructing or dismantling its frame, but must restore 16-byte alignment before making another ABI call.

On return:

```text
sp == entry_sp
```

There is no red zone below `sp`. Memory below the current stack pointer may be used by interrupts, signal/trap machinery, debuggers, or future operating-system conventions.

Therefore code may not keep live locals below `sp` without first allocating that space by adjusting `sp`.

---

# 9. Stack-relative instructions and frame layout

SIA v0.5 provides:

```asm
LWSP rd, offset     ; 0..124, multiple of 4
SWSP rs, offset     ; 0..124, multiple of 4
ADJSP amount        ; signed multiple of 16, -256..+240
```

These are specifically intended to make compiler-generated frames efficient.

## 9.1 Stable-SP frame rule

For a fixed-size function frame, compilers should normally allocate the complete static frame near function entry and keep `sp` stable through the ordinary body.

That makes every stack slot have a fixed positive offset from `sp` and maximizes use of `LWSP/SWSP`.

A recommended layout is:

```text
higher addresses

caller frame / incoming stack arguments
---------------------------------------- entry_sp
saved registers / bookkeeping
fixed locals
cold spill slots
hot word spill slots            <- prefer within current sp + 0..124
---------------------------------------- current sp

lower addresses
```

The exact ordering of saved registers and locals is compiler policy, but the hottest word spills should be placed where `LWSP/SWSP` can reach them directly.

## 9.2 Frame allocation

For a 16-byte-multiple adjustment in range:

```asm
ADJSP -64
```

allocates 64 bytes.

For sizes not representable by one `ADJSP`, the compiler may use:

- multiple `ADJSP` instructions;
- `ADDI sp,imm` for small residual adjustments;
- a register-formed address for very large frames.

Because signed 5-bit `ADJSP` reaches `-256` but only `+240`, compilers that want identical single-instruction allocate/deallocate forms should keep the relevant adjustment at 240 bytes or less.

## 9.3 Call alignment inside a frame

Saving an odd number of words may temporarily break 16-byte alignment. The compiler is responsible for including sufficient frame padding before any nested call.

For example, saving 20 bytes of registers can be combined with 12 bytes of padding/locals so the total frame size remains 32 bytes.

---

# 10. Callee-save patterns

SIA multiple-register operations are intended to make common saves dense.

If all `r9-r12` are used:

```asm
ST4 r9:r12, -[sp]
```

saves them in one instruction.

If both `lr` and `r15` require preservation:

```asm
STP r14:r15, -[sp]
```

saves them together.

Restore in reverse frame order using post-increment forms:

```asm
LDP r14:r15, [sp]+
LD4 r9:r12,  [sp]+
```

A compiler must not save unused callee-saved registers merely to fit a preferred multiple-transfer pattern if doing so creates a material performance or frame-size penalty. Pair/quad transfers are optimizations, not requirements.

---

# 11. Frame pointer

`r15` is a callee-saved general register by default.

A function may designate it as `fp` when a stable frame reference is useful, for example:

- dynamic stack allocation;
- complex outgoing argument areas;
- debugging/unwinding modes that require a materialized frame pointer;
- functions whose stack offsets cannot otherwise be represented efficiently.

Ordinary fixed-frame functions should omit the frame pointer when possible because SIA has only 16 architectural registers.

When `r15` is used as a frame pointer, its incoming value remains callee-saved and must be restored before return.

The canonical frame address for unwind/debug purposes is the function's `entry_sp`. A materialized `fp` should normally be arranged to recover or directly represent that value. Exact unwind metadata encoding is deferred to the ELF/debug ABI.

---

# 12. Outgoing stack arguments

A caller that needs stack arguments may use either of two compiler strategies:

1. reserve a fixed outgoing argument area as part of its static frame; or
2. temporarily decrement `sp` immediately before a call and restore it immediately afterward.

At the instant of the call, `sp` must point to the first stack argument and be 16-byte aligned.

If the compiler temporarily moves `sp`, it must not issue `LWSP/SWSP` for preexisting frame slots using stale offsets until `sp` is restored or the offsets are recomputed.

For the initial Forge backend, reserving no outgoing area for functions whose callees use at most six argument words is preferred. Temporary outgoing areas are sufficient for larger calls.

---

# 13. Function calls

## 13.1 Direct call

```asm
BL target
```

sets:

```text
r14 = address after BL
pc  = target
```

## 13.2 Indirect call

```asm
CALLR rX
```

is the ABI alias for:

```asm
JALR r14, rX
```

## 13.3 Return

```asm
RET
```

is:

```asm
JALR r0, r14
```

The callee must restore `sp` and all modified callee-saved registers before executing the return.

---

# 14. Tail calls

A tail call is ABI-compatible when the caller has:

- restored its callee-saved registers;
- restored `sp` to the layout expected for the outgoing callee;
- placed the target function's arguments according to this ABI.

It may then branch/jump to the target without replacing the original caller's return address.

The Forge `return tail` feature should lower to this convention when the signature/layout permits.

---

# 15. Soft-float convention

The base `SIA32-I` ABI is soft-float.

No floating-point register state is required by this ABI.

```text
f32 -> one 32-bit argument/return word containing IEEE-style bits chosen by the language/runtime ABI
f64 -> two 32-bit words, low word first
```

Floating operations are implemented by software routines or compiler-generated integer sequences unless a separately named hardware floating-point ABI is selected.

A future floating-point extension must not silently change the calling convention of binaries labeled with this base ABI.

---

# 16. System calls and simulator semihosting

Normal language/library I/O is not encoded into the function ABI.

A Forge operation such as `println` should lower conceptually through a runtime/library service:

```text
println -> formatting/runtime -> write -> OS/device service
```

The SIA reference interpreter may provide `TRAP`-based semihosting for bring-up. Those trap numbers are simulator conventions and are not ordinary function-call ABI semantics.

The operating-system syscall ABI will be specified separately from this document.

---

# 17. ELF and object ABI status

This draft does not yet freeze:

- ELF machine number;
- ELF flags;
- relocation numbers;
- symbol visibility details;
- PLT/GOT conventions;
- dynamic linking;
- TLS layout;
- DWARF register numbering;
- unwind record encoding.

The initial Forge/SIA integration may emit a linked flat image for `siaemu` before ELF exists.

The intended progression is:

```text
Forge -> SIA flat image -> siaemu
```

then:

```text
Forge -> SIA ELF32 relocatable object -> linker -> SIA ELF32 executable
```

ELF details should be frozen only after the assembler, backend, linker, and representative programs expose the required relocation set.

---

# 18. Recommended compiler behavior

A SIA compiler backend should initially:

1. allocate ordinary arguments in `r1-r6`;
2. treat `r1-r8` and `r14` as call-clobbered;
3. treat `r9-r12` and `r15` as callee-saved;
4. keep `sp` stable in fixed frames;
5. place hot spills in the first 128 bytes above `sp`;
6. use `LWSP/SWSP` instead of forming temporary stack addresses;
7. use `ADJSP` for suitable frame-size multiples of 16;
8. use `STP/ST4` and `LDP/LD4` when consecutive saved-register groups make them profitable;
9. omit `fp` unless needed;
10. preserve 16-byte stack alignment before every ABI call;
11. use `r1` or `r1:r2` for normal returns;
12. collect spill/reload and frame-size statistics so the ABI and ISA can be evaluated with real Forge programs.

---

# 19. Example: leaf function

Forge-like source:

```text
add(a:i32, b:i32) -> i32
```

may compile to:

```asm
ADD r1, r1, r2
RET
```

No frame is required.

---

# 20. Example: fixed frame with spills

Illustrative sequence:

```asm
; entry sp is 16-byte aligned

SW   r14, -[sp]      ; preserve return address
ADDI sp, -12         ; pad/save area to 16 bytes total
ADJSP -64            ; fixed locals/spills

SWSP r4, 0
SWSP r7, 4
...
LWSP r4, 0
LWSP r7, 4

ADJSP 64
ADDI sp, 12
LW   r14, [sp]+
RET
```

The exact save order is compiler policy. The important invariants are that the complete frame is allocated before ordinary body accesses, hot spills use small positive offsets from stable `sp`, call-boundary alignment is maintained, and the original `sp` is restored on return.

---

# 21. Open ABI decisions

Before ABI v1.0, toolchain experiments must settle:

1. final aggregate classification rules beyond 8 bytes and unusual alignments;
2. whether any aggregate class should receive stronger than 4-byte alignment;
3. variadic-function conventions, if variadics are standardized;
4. ELF machine/flags and relocation set;
5. DWARF register numbering and unwind metadata;
6. syscall ABI;
7. dynamic linking/TLS conventions;
8. hardware floating-point ABI variants;
9. vector/SIMD ABI variants if such extensions are ever standardized;
10. whether stack alignment remains 16 bytes after measurements on the smallest SIA implementations.

The scalar register convention, `sp=r13`, `lr=r14`, optional `fp=r15`, caller/callee-save split, little-endian word ordering, and `LWSP/SWSP/ADJSP` frame model are the initial commitments intended for Forge backend bring-up.
