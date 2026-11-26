# Compare the four bytes written by a relocation

[日本語](README.md)

[apply.ceru](apply.ceru) reads the patch location `P`, the referenced symbol location `S`, and the addend `A` from `input.o`. An `R_X86_64_PC32` relocation writes `S + A - P`. Here both locations are in the same section, so their offsets from the section start suffice.

| [Input](input.S) | Calculation | Four bytes written | ELF entry |
| --- | --- | --- | --- |
| Forward reference to `anchor` | `8 - 4 - 2 = 2` | `02 00 00 00` | `0xffffffff80001001` |
| Backward reference to `anchor` | `0 - 4 - 3 = -7` | `f9 ff ff ff` | `0xffffffff80001002` |

Cerune copies the code section, patches the four bytes, and creates `text.bin`. It prints the entry obtained by adding the section-relative value of `main` to the LiseOS text address `0xffffffff80001000`. The example takes an x86-64 ELF64 object up to 64 KiB with one executable section containing both `main` and the referenced definition.

## Run the comparison

From the Symnexa root on Linux, use Rust, Python 3, Clang, GNU objcopy, and a Cerune compiler with binary file input and output. CI uses [Cerune commit `25a6f8f`](https://github.com/Cerune-Lang/Cerune/commit/25a6f8f2c6b76c411bedeeb61c4b1a759f37fa11). The initial Rust reference was [Symnexa commit `70489b5`](https://github.com/Cerune-Lang/Symnexa/commit/70489b57c072d47776bc6d8d6f578c54520b37bb).

```sh
cargo build
python3 examples/pc32/compare.py --cerune /path/to/cerune --symnexa target/debug/symnexa
```

The [comparison](compare.py) assembles one object per case and feeds identical bytes to the Rust linker and the Cerune HIR, MIR, SSA and VM executors. It extracts the entry and the entire `.text` from the Rust `ET_EXEC` and compares them with the printed entry and the saved bytes. Two more inputs exercise results above the signed 32-bit maximum and below the minimum. Rust exits with an error and the Cerune example prints `PC32 overflow` and returns before saving.
