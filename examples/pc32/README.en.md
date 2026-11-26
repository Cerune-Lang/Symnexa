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

## Resolve a definition in another object

[pair.ceru](pair.ceru) places the code from `first.o` and `second.o` in that order, filling alignment gaps with zeroes. The relocation's symbol index is local to the [caller](caller.S). The example follows it to the name in that object's string table and compares the full name with public definitions in the [provider](provider.S). It adds the symbol's section offset to the start of the defining object's code.

| Input order | Target + addend − patch, relative to the start of `.text` | Displacement | Patched bytes | Entry of `main` |
| --- | --- | ---: | --- | --- |
| Caller then provider | `19 - 4 - 2` | 13 | `0d 00 00 00` | `0xffffffff80001001` |
| Provider then caller | `3 - 4 - 10` | -11 | `f5 ff ff ff` | `0xffffffff80001009` |

The caller occupies seven bytes with eight-byte alignment. The provider occupies five bytes with 16-byte alignment. With the provider first, the caller starts at offset 8 and its patch field is another two bytes into the code. Run the first row in a fresh directory:

```sh
mkdir pair-input
clang --target=x86_64-unknown-linux-gnu -c examples/pc32/caller.S -o pair-input/first.o
clang --target=x86_64-unknown-linux-gnu -c examples/pc32/provider.S -o pair-input/second.o
cerune run examples/pc32/pair.ceru --file-root pair-input
```

Each input has one executable section containing the definitions used by the example. The pair has one relocation in total. Each input and the merged code are limited to 64 KiB. Both `GLOBAL` and `WEAK` definitions participate in name resolution. As in Rust, duplicate public definitions stop the example before it saves the code. For an unresolved or duplicate symbol it prints the reason and the bytes of the name. The comparison exercises reversed order, a renamed reference, a weak definition, a name with a different suffix, a local-only definition, and a duplicate public name.

## Save an ELF with an entry point and a load address

With the same `first.o` and `second.o`, run [pair_exec.ceru](pair_exec.ceru) to create `pair-input/pair.elf`.

```sh
cerune run examples/pc32/pair_exec.ceru --file-root pair-input
readelf -h -l pair-input/pair.elf
```

The entry point is the address of `main` in the table above. For caller-first order, the `LOAD` row maps 21 bytes at file offset `0x1000` to address `0xffffffff80001000`. The example writes one readable, executable segment and reports an error before saving if another input section requires space in memory. The test compares the entry, every field of the executable segment header, and all relocated bytes with the Rust ELF. Booting LiseOS will also require a segment containing the Limine requests.

## Run the comparison

From the Symnexa root on Linux, use Rust, Python 3, Clang, GNU objcopy and nm, and a Cerune compiler with binary file input and output. CI uses [Cerune commit `25a6f8f`](https://github.com/Cerune-Lang/Cerune/commit/25a6f8f2c6b76c411bedeeb61c4b1a759f37fa11). The initial Rust reference was [Symnexa commit `70489b5`](https://github.com/Cerune-Lang/Symnexa/commit/70489b57c072d47776bc6d8d6f578c54520b37bb).

```sh
cargo build
python3 examples/pc32/compare.py --cerune /path/to/cerune --symnexa target/debug/symnexa
```

The [comparison](compare.py) assembles one object per case and feeds identical bytes to the Rust linker and the Cerune HIR, MIR, SSA and VM executors. It extracts the entry and the entire `.text` from the Rust `ET_EXEC` and compares them with the printed entry and the saved bytes. It reads the Rust address of `anchor` with `nm -P` and checks it against the patch location, displacement and addend. Three more inputs cover signed 32-bit overflow, underflow, and a value that overflows only after adding the distance between sections. Rust exits with an error and the Cerune example prints `PC32 overflow` and returns before saving.
