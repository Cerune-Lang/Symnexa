# LiseOS integration

[日本語](liseos.ja.md) · [Documentation](../README.md)

The first use case for Symnexa is [LiseOS](https://github.com/Cerune-Lang/LiseOS). **LiseOS currently uses `ld.lld`; Symnexa is not implemented yet.**

## Current build

```text
kernel/main.ceru
       ↓
tools/object-builder (host Rust)
       ↓
build/kernel.o
       ↓
ld.lld -T linker.ld
       ↓
build/liseos.elf
       ↓
Limine → QEMU
```

The Rust object builder calls Cerune's `compile_to_native_object`. It is a host build utility, not kernel code.

## Required linker settings

From the current [linker.ld](https://github.com/Cerune-Lang/LiseOS/blob/main/linker.ld):

| Property | Value |
| --- | --- |
| Output | x86-64 ELF64 executable |
| Entry | `main` |
| Virtual base | `0xffffffff80000000` |
| Segments | RX, R, RW `PT_LOAD` |
| Pages | Separate load permissions at 4 KiB intervals |
| Limine | Request start/end markers, base revision 3 |
| Linking | Static; unresolved symbols are errors |

The existing linker script also writes the Limine markers directly. A Symnexa replacement must preserve them.

## Verification

Run the established checks in the LiseOS repository:

```sh
bash scripts/iso.sh
bash scripts/smoke-qemu.sh
```

`scripts/audit_elf.py` verifies the ELF file, symbols, entry, and Limine markers. QEMU/GDB verifies the Cerune `main` is reached.

Only switch LiseOS to Symnexa after its ELF passes those checks. Keep the `ld.lld` path available for comparison and recovery.

[Linker implementation](../design/linking.en.md)
