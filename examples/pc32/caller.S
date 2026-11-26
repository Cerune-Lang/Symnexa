#ifndef REF
#define REF answer
#endif
.text
.balign 8
.globl main, REF
.byte 0x91
main:
    ret
field:
    .long 0x12345678
    .reloc field, R_X86_64_PC32, REF-4
#ifdef DUPLICATE
.globl shared
shared:
#endif
    .byte 0x92
