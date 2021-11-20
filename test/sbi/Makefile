image.bin:
	env CROSS_COMPILE=riscv32-unknown-elf- \
		PLATFORM_RISCV_XLEN=32 \
		PLATFORM=generic \
		PLATFORM_RISCV_ISA=rv32ima \
		FW_PIC=n \
		$(MAKE) -C opensbi
	cp opensbi/build/platform/generic/firmware/fw_payload.bin image.bin

.PHONY: clean
clean:
	rm -f image.bin
	$(MAKE) -C opensbi clean