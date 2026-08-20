# SIA reference interpreter

`SIA32-I` still leaves some grouped numeric encodings provisional. `siaemu` is a small dependency-free Rust interpreter for the current fixed-16-bit architecture so that compiler and ABI experiments can execute before v1.0 is frozen.

It follows `../SIA.md` Draft v0.5 and the initial software conventions in `../ABI.md`.

## Run

```sh
cargo run --release -- program.bin
cargo run --release -- program.bin --trace
cargo run --release -- program.bin --memory 4194304 --max-steps 1000000
```

The input is a raw little-endian image loaded at address `0`; execution starts at `PC=0`.

- Every instruction is 16 bits.
- RAM is zero-filled after the image; default size is 1 MiB.
- `r13` (`sp`) starts at the top of RAM rounded down to a 16-byte boundary.
- `r0` always reads zero and ordinary writes are discarded.
- Natural halfword/word alignment is enforced.
- Execution ends successfully when `PC` reaches exactly the end of the input image.
- Branching outside the executable image is an error.

The image may contain literal data as well as instructions. Code must branch around embedded literal pools.

## Emulator output / semihosting

Real SIA software will normally perform I/O through an OS/device interface. During bring-up, `siaemu` supplies a small semihosting convention through `TRAP`:

| Trap | Service |
|---|---|
| `TRAP 0` | Exit; status in `r1`. |
| `TRAP 1` | Write bytes: `r1` = fd (`1` stdout, `2` stderr), `r2` = guest address, `r3` = byte count. Returns count in `r1`. |
| `TRAP 2` | Write the low byte of `r1` to stdout. |

These trap services are simulator conventions, not architectural SIA I/O.

`BREAK` raises an emulator fault. `NOP` does nothing.

## Current primary map

| Primary | Use |
|---|---|
| `0` | `ADD rd,ra,rb`; `rd=0` escape currently encodes `CLZ` |
| `1` | `CMOV rd,rs,rc`; `rd=0` escape currently encodes `CTZ` |
| `2` | `LDA.W rd,[rb+ri*4]`; `rd=0` escape currently encodes `CPOP` |
| `3` | `STA.W rs,[rb+ri*4]` |
| `4` | destructive arithmetic / compare |
| `5` | destructive logic / shifts |
| `6` | `LI` / `ADDI` with signed 7-bit immediate |
| `7` | scalar memory plus `LWSP/SWSP/ADJSP` |
| `8` | `LDP/STP/LD4/ST4` |
| `9` | `LDPC.W` |
| `A` | `BNZ` / `DBNZ` |
| `B` | `B` / `BL` |
| `C` | indirect control, bit operations, optional multiply/divide, `REV8`, system |
| `D` | `ADC rd,rs,rc` |
| `E` | `SBB rd,rs,rc` |
| `F` | reserved `EXT`; execution faults |

`ADC`, `SBB`, `EXT`, and the v0.5 stack-relative scalar-memory subformat are architectural commitments of the current draft. Several other grouped assignments remain prototype choices until measurements justify freezing them.

## Arithmetic group (`4 rd rs fn`)

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

## Logic / shift group (`5 rd rs fn`)

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

The source-negation Boolean modifiers discussed in the architecture remain unassigned in the interpreter.

## Scalar memory primary (`7`)

General modes:

```text
0 LB       1 LBU      2 LH       3 LHU
4 LW       5 SB       6 SH       7 SW
C LW [rb]+ D SW [rb]+ E LW -[rb] F SW -[rb]
```

Modes `8..B` are the v0.5 implicit-SP subformat:

```text
15:12  11:8   7   6:4  3:2  1:0
0111    rv    S   imm   10   imm
```

with:

```text
S=0, rv!=0   LWSP rv, uimm5*4
S=1          SWSP rv, uimm5*4
S=0, rv=0    ADJSP simm5*16
```

`LWSP/SWSP` reach offsets `0..124` bytes from `r13/sp` in 4-byte increments.

`ADJSP` reaches `-256..+240` bytes in 16-byte increments.

`SWSP r0,off` stores zero and remains a normal store; only the otherwise-useless `LWSP r0,...` encodings are reclaimed for stack adjustment.

## Pair / quad modes (`8 first rb mode`)

```text
0 LDP
1 LDP [rb]+
2 STP
3 STP [rb]+
4 STP -[rb]
5 LD4
6 LD4 [rb]+
7 ST4
8 ST4 [rb]+
9 ST4 -[rb]
```

## PC-relative literals

The interpreter currently uses:

```text
base = align_down(PC + 4, 4)
address = base + sign_extend(disp8) * 4
```

This is still a prototype choice because the architecture intentionally leaves the final `LDPC.W` PC-base rule open for compiler/linker measurement.

## Misc group (`C rd rs fn`)

```text
0 JALR
1 BSET
2 BCLR
3 BINV
4 BEXT
5 MUL
6 MULH
7 MULHU
8 MULHSU
9 MULO
A DIV
B DIVU
C REM
D REMU
E REV8
F system
```

For `fn=F`, bits `11:4` are an 8-bit system immediate. `0xFE` is `BREAK`, `0xFF` is `NOP`, and other values are `TRAP imm8`.

The interpreter implements optional multiply/divide instructions so toolchain experiments can use them. Divide-by-zero and signed overflow behavior remains provisional until the SIA-M specification is frozen.

## Tests

The Rust unit tests cover, among other basics:

- arithmetic;
- post-increment scalar memory;
- counted loops;
- pair load/store;
- `ADJSP` allocation/restoration;
- `SWSP`/`LWSP` spill/reload;
- `SWSP r0` storing an actual zero word.
