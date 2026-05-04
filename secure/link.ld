MEMORY
{
  FLASH (rx)  : ORIGIN = 0x10000000, LENGTH = 522K  /* Secure Flash (NSC領域含む) */
  NSC   (rx)  : ORIGIN = 0x10080800, LENGTH = 2K    /* NSC veneer: SAU Region2に一致 */
  RAM   (rwx) : ORIGIN = 0x30000000, LENGTH = 128K  /* Secure RAM */
}

SECTIONS
{
.vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));        /* MSP初期値 */
    KEEP(*(.vector_table.reset_vector))     /* Reset + EXCEPTIONS配列 */
  } > FLASH

  .text :
  {
    *(.vector_table.reset_vector)
    *(.text .text.*)
  } > FLASH

  .rodata : { *(.rodata .rodata.*) } > FLASH

  .ARM.exidx :
  {
    *(.ARM.exidx* .gnu.linkonce.armexidx.*)
  } > FLASH

  .data : AT(LOADADDR(.ARM.exidx) + SIZEOF(.ARM.exidx))
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

  /* NSC veneer: SAU Region2 (0x10080800-0x10080FFF) に固定 */
  .gnu.sgstubs ORIGIN(NSC) :
  {
    __sgstubs_start = .;
    *(.gnu.sgstubs*)
    __sgstubs_end = .;
  } > NSC
}