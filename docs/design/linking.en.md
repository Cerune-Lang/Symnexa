# Symnexa linker implementation

[日本語](linking.ja.md) · [Documentation](../README.md)

Symnexa will resolve symbols and relocations in object files and produce executable images. The first use case is the x86-64 ELF64 kernel for LiseOS. **This document describes the intended implementation; no linker is implemented yet.**

## Inputs and outputs

| Property | Scope |
| --- | --- |
| Input | x86-64 little-endian ELF64 ET_REL |
| Output | ELF64 ET_EXEC |
| Entry | `main` symbol |
| Virtual base | `0xffffffff80000000` |
| Sections | `.text`, `.rodata`, `.data`, `.bss`, Limine data |
| Program headers | Read-only, executable, and writable `PT_LOAD` segments |
| Excluded | Dynamic linking, shared libraries, TLS, PE executables |

File format and target ABI are separate concerns. COFF input can be investigated after the ELF boot case works.

## Linking

1. Parse the ELF header, sections, symbols, and relocation entries.
2. Check duplicate and unresolved symbols; calculate their final addresses.
3. Place sections using their alignment and size. Allocate `.bss` in memory without writing its contents to the file.
4. Apply supported relocations, reporting unsupported kinds explicitly.
5. Emit ELF and `PT_LOAD` headers and set the entry address.
6. Validate unresolved references and segment layout.

Start with the relocation types actually emitted by Sashimi for the LiseOS kernel rather than implementing hypothetical types.

## Segment placement

The current LiseOS linker script marks `.text` read/execute, `.rodata` read-only, and `.data` read/write. Limine request data belongs in a data segment.

Segments with different permissions must not share a 4 KiB page. ELF's `p_offset` and `p_vaddr` must satisfy the congruence required by `p_align`.

## Limine data

The LiseOS linker script currently emits request start/end markers and base revision 3 using `QUAD` directives. Symnexa will need to produce equivalent data or accept those bytes from Cerune-generated inputs.

Limine-specific data should be selected explicitly for the LiseOS use case, not inserted into every ELF output.

## Verification

- Reject malformed ELF objects and unsupported relocations.
- Validate symbol addresses, program headers, and load permissions.
- Compare entry, segments, and Limine markers against `ld.lld` output.
- Boot LiseOS in QEMU and confirm entry into the Cerune `main` through GDB.

[Boot verification details](../integration/liseos.en.md)
