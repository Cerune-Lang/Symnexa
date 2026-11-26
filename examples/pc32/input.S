// A nonzero entry offset and nonzero placeholder bytes make both comparisons observable.
.text
.globl main, anchor
#ifdef BACKWARD
anchor:
    .byte 0x90
#endif
    .byte 0x90
main:
    ret
field:
    .long 0x12345678
#ifdef PLACEMENT_OVERFLOW
    .reloc field, R_X86_64_PC32, anchor+2147480000
#elif defined(OVERFLOW)
    .reloc field, R_X86_64_PC32, anchor+2147483647
#elif defined(UNDERFLOW)
    .reloc field, R_X86_64_PC32, anchor-2147483648
#else
    .reloc field, R_X86_64_PC32, anchor-4
#endif
    .byte 0x90, 0x90
#ifdef RODATA
#ifdef TEXT_SIZE
    .org TEXT_SIZE, 0x90
#endif
    .section .rodata.prefix,"a",@progbits
    .byte 0x31, 0x32, 0x33
    .section .rodata.values,"a",@progbits
    .balign 16
    .byte 0x41, 0x42, 0x43
anchor:
    .byte 0x44
#elif !defined(BACKWARD)
anchor:
    ret
#endif
