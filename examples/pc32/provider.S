#ifndef DEF
#define DEF answer
#endif
.text
.balign 16
    .byte 0xa1, 0xa2, 0xa3
#ifndef LOCAL
#ifdef WEAK
.weak DEF
#else
.globl DEF
#endif
#endif
DEF:
    ret
.globl shared
shared:
    .byte 0xa4
