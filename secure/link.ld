MEMORY
{
  FLASH (rx)  : ORIGIN = 0x10000000, LENGTH = 522K
  NSC   (rx)  : ORIGIN = 0x10080800, LENGTH = 2K
  RAM   (rwx) : ORIGIN = 0x30000000, LENGTH = 128K
}

ENTRY(Reset);

SECTIONS
{
.vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));
    KEEP(*(.vector_table.reset_vector))
  } > FLASH

  .text :
  {
    *(.text .text.*)
  } > FLASH

  .rodata : { *(.rodata .rodata.*) } > FLASH

  /DISCARD/ :
  {
    /* Unused exception related info that only wastes space */
    *(.ARM.exidx);
    *(.ARM.exidx.*);
    *(.ARM.extab.*);
  }

  .data : AT(ADDR(.rodata) + SIZEOF(.rodata))
  {
    __sdata = .;
    *(.data .data.*)
    __edata = .;
  } > RAM

  __sidata = LOADADDR(.data);

  .bss :
  {
    __sbss = .;
    *(.bss .bss.*)
    *(COMMON)
    __ebss = .;
  } > RAM

  .gnu.sgstubs ORIGIN(NSC) :
  {
    __sgstubs_start = .;
    *(.gnu.sgstubs*)
    __sgstubs_end = .;
  } > NSC
}

PROVIDE(nonsecure_entry_function = 0x10080800);