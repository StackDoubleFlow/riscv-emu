.globl _start

_start:
    lui sp, 0x8
    lui gp, 0x8
    jal ra, main
    ebreak
