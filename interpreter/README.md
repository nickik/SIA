# SIA reference interpreter

`SIA32-I` v0.4 deliberately leaves the numeric instruction encoding provisional. `siaemu` is a small, dependency-free Rust interpreter that gives the current fixed-16-bit design an executable **prototype encoding** so that the ISA can be tested before v1.0 is frozen.

It intentionally implements the v0.4 design in `../SIA.md`; it does not add packed/SIMD or other later brainstorming.

## Run

```sh
cargo run --release -- program.bin
cargo run --release -- program.bin --trace
cargo run --release -- program.bin --memory 4194304 --max-steps 1000000
```

The binary is a raw little-endian image loaded at address `0`. Execution begins at `PC=0`.

- Every instruction is exactly 16 bits.
- RAM is zero-filled after the image; default size is 1 MiB.
- `r13` (`sp`) starts at the top of RAM, rounded down to a 4-byte boundary.
- `r0` always reads as zero and discards writes.
- Natural halfword/word alignment is enforced.
- Execution ends successfully when `PC` reaches exactly the end of the input image.
- Branching outside the executable image is an error.

The image may contain literal data as well as instructions. Code must branch around embedded literal pools, just as a real linked image would.

## Emulator output / semihosting

A real SIA machine would normally produce output through a device interface (for example a terminal/UART, memory-mapped device, or later QDX/OS service). A simulator usually needs something simpler before that hardware exists, so `siaemu` uses **semihosting**: selected `TRAP` instructions call host services.

These services are emulator conventions, not architectural SIA I/O requirements.

| Trap | Service |
|---|---|
| `TRAP 0` | Exit. Exit status is in `r1`. |
| `TRAP 1` | Write bytes. `r1` = fd (`1` stdout, `2` stderr), `r2` = guest address, `r3` = byte count. Returns byte count in `r1`. |
| `TRAP 2` | Write the low byte of `r1` to stdout. Useful for tiny hand-written tests. |

`BREAK` raises an emulator fault. `NOP` does nothing.

## Prototype encoding

The high nibble follows the provisional v0.4 primary map:

| Primary | Prototype use |
|---|---|
| `0` | `ADD rd,ra,rb`; `rd=0` escape encodes `CLZ` |
| `1` | `CMOV rd,rs,rc`; `rd=0` escape encodes `CTZ` |
| `2` | `LDA.W rd,[rb+ri*4]`; `rd=0` escape encodes `CPOP` |
| `3` | `STA.W rs,[rb+ri*4]` |
| `4` | destructive arithmetic / compare (`rd,rs,fn`) |
| `5` | destructive logic / shifts (`rd,rs,fn`) |
| `6` | `LI` / `ADDI` with signed 7-bit immediate |
| `7` | scalar memory (`rv,rb,mode`) |
| `8` | `LDP/STP/LD4/ST4` (`first,rb,mode`) |
| `9` | `LDPC.W rd,disp8` |
| `A` | `BNZ` / `DBNZ` |
| `B` | `B` / `BL` |
| `C` | indirect control, bit operations, optional multiply/divide, `REV8`, system |
| `D` | **prototype `ADC rd,rs,rc`** |
| `E` | **prototype `SBB rd,rs,rc`** |
| `F` | reserved `EXT`; executing it faults |

### Important encoding experiment: `ADC` / `SBB`

The v0.4 semantic specification requires explicit three-register `ADC` and `SBB`, but the provisional primary map does not yet assign them a legal 16-bit encoding. A three-register operation consumes all twelve payload bits, so each operation needs a full primary opcode unless its semantics change.

The interpreter therefore temporarily assigns primaries `D` and `E` to `ADC` and `SBB`. The spec currently calls these regions “reserved base growth.” This is intentional: the emulator is exposing an unresolved encoding-pressure decision rather than silently changing the architecture. Do not treat these two assignments as v1.0-frozen.

### Arithmetic group (`4 rd rs fn`)

| `fn` | Operation |
|---|---|
| `0` | `SUB` |
| `1` | `ADDO` |
| `2` | `SUBO` |
| `3` | `CMPEQ` |
| `4` | `CMPLT` |
| `5` | `CMPLTU` |
| `6` | `MIN` |
| `7` | `MINU` |
| `8` | `MAX` |
| `9` | `MAXU` |

### Logic / shift group (`5 rd rs fn`)

| `fn` | Operation |
|---|---|
| `0` | `AND` |
| `1` | `OR` |
| `2` | `XOR` |
| `3` | `SHL` register count |
| `4` | `SHR` register count |
| `5` | `SAR` register count |
| `6`,`7` | `SHLI` 0..15 / 16..31 |
| `8`,`9` | `SHRI` 0..15 / 16..31 |
| `A`,`B` | `SARI` 0..15 / 16..31 |

The source-negation Boolean modifiers discussed in the spec remain unassigned; the interpreter does not invent an encoding for them yet.

### Scalar memory modes (`7 rv rb mode`)

`0` `LB`, `1` `LBU`, `2` `LH`, `3` `LHU`, `4` `LW`, `5` `SB`, `6` `SH`, `7` `SW`, `C` `LW [rb]+`, `D` `SW [rb]+`, `E` `LW -[rb]`, `F` `SW -[rb]`.

Modes `8..B` remain reserved.

### Pair / quad modes (`8 first rb mode`)

`0` `LDP`, `1` `LDP [rb]+`, `2` `STP`, `3` `STP [rb]+`, `4` `STP -[rb]`, `5` `LD4`, `6` `LD4 [rb]+`, `7` `ST4`, `8` `ST4 [rb]+`, `9` `ST4 -[rb]`.

### PC-relative literals

For the prototype interpreter, `LDPC.W` uses:

```text
base = align_down(PC + 4, 4)
address = base + sign_extend(disp8) * 4
```

This gives an aligned word load and approximately ±512 bytes of literal reach. v0.4 deliberately leaves the exact PC-base rule open, so this is another value we can change after experiments.

### Misc group (`C rd rs fn`)

`0` `JALR`, `1` `BSET`, `2` `BCLR`, `3` `BINV`, `4` `BEXT`, `5` `MUL`, `6` `MULH`, `7` `MULHU`, `8` `MULHSU`, `9` `MULO`, `A` `DIV`, `B` `DIVU`, `C` `REM`, `D` `REMU`, `E` `REV8`, `F` system.

For `fn=F`, bits `11:4` are an 8-bit system immediate. `0xFE` is `BREAK`, `0xFF` is `NOP`, and other values are `TRAP imm8`.

The interpreter implements the optional multiply/divide operations so binaries can experiment with them. Divide-by-zero and signed `INT_MIN / -1` currently fault; those architectural semantics remain open in v0.4.
