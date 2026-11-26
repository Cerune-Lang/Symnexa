# Compare the four bytes written by a relocation

[日本語](README.md)

[apply.ceru](apply.ceru) reads the patch location `P`, the referenced symbol location `S`, and the addend `A` from `input.o`. An `R_X86_64_PC32` relocation writes `S + A - P`. The patch is in `.text`. The symbol `anchor` can be in `.text` or in a read-only `PROGBITS` section.

| [Input](input.S) | Calculation | Four bytes written | ELF entry |
| --- | --- | --- | --- |
| Forward reference to `anchor` | `8 - 4 - 2 = 2` | `02 00 00 00` | `0xffffffff80001001` |
| Backward reference to `anchor` | `0 - 4 - 3 = -7` | `f9 ff ff ff` | `0xffffffff80001002` |
| In `.rodata`, text size 8 or 4,096 | `4096 + 19 - 4 - 2 = 4109` | `0d 10 00 00` | `0xffffffff80001001` |
| In `.rodata`, text size 4,097 | `8192 + 19 - 4 - 2 = 8205` | `0d 20 00 00` | `0xffffffff80001001` |

The calculations use offsets from the start of `.text`. The Rust linker rounds the end of the code up to a 4,096-byte page and starts `.rodata` there. The first input section, `.rodata.prefix`, occupies three bytes. The next, `.rodata.values`, requires 16-byte alignment and starts at offset 16. The symbol is at offset 3 within that section, placing `anchor` 19 bytes from the start of `.rodata`.

Cerune copies the code section, patches the four bytes, and creates `text.bin`. It prints the entry obtained by adding the section-relative value of `main` to the LiseOS text address `0xffffffff80001000`. The example takes an x86-64 ELF64 object up to 64 KiB with one executable section defining `main` and a referenced symbol defined in the same object.

## Run the comparison

From the Symnexa root on Linux, use Rust, Python 3, Clang, GNU objcopy and nm, and a Cerune compiler with binary file input and output. CI uses [Cerune commit `25a6f8f`](https://github.com/Cerune-Lang/Cerune/commit/25a6f8f2c6b76c411bedeeb61c4b1a759f37fa11). The initial Rust reference was [Symnexa commit `70489b5`](https://github.com/Cerune-Lang/Symnexa/commit/70489b57c072d47776bc6d8d6f578c54520b37bb).

```sh
cargo build
python3 examples/pc32/compare.py --cerune /path/to/cerune --symnexa target/debug/symnexa
```

The [comparison](compare.py) assembles one object per case and feeds identical bytes to the Rust linker and the Cerune HIR, MIR, SSA and VM executors. It extracts the entry and the entire `.text` from the Rust `ET_EXEC` and compares them with the printed entry and the saved bytes. It reads the Rust address of `anchor` with `nm -P` and checks it against the patch location, displacement and addend. Three more inputs cover signed 32-bit overflow, underflow, and a value that overflows only after adding the distance between sections. Rust exits with an error and the Cerune example prints `PC32 overflow` and returns before saving.
