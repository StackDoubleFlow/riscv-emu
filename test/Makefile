CC=riscv32-unknown-elf-gcc
OBJCOPY=llvm-objcopy
CFLAGS=-nostdlib -lgcc -Tlinker.ld

image.bin: main.o
	llvm-objcopy -O binary main.o image.bin

main.o: main.c start.s
	$(CC) -o main.o start.s main.c $(CFLAGS)
