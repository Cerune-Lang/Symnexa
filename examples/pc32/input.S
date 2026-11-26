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
#ifdef OVERFLOW
    .reloc field, R_X86_64_PC32, anchor+2147483647
#elif defined(UNDERFLOW)
    .reloc field, R_X86_64_PC32, anchor-2147483648
#else
    .reloc field, R_X86_64_PC32, anchor-4
#endif
    .byte 0x90, 0x90
#ifndef BACKWARD
anchor:
    ret
#endif
