# clang -O1 -nostdlib --target=riscv32 -march=rv32i -fuse-ld=/home/stack/ledump/riscv-test/toolchains/riscv32i/bin/riscv32-unknown-elf-ld --gcc-toolchain=/home/stack/ledump/riscv-test/toolchains/riscv32i -lgcc -T linker.ld -o fib.o start.s fib.c 
riscv32-unknown-elf-gcc -nostdlib -o fib.o start.s fib.c -lgcc -T linker.ld
llvm-objcopy -O binary fib.o fib.bin