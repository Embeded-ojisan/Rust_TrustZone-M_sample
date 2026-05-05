MEMORY
{
  FLASH (rx)  : ORIGIN = 0x00100000, LENGTH = 512K  /* SAU Region0に一致 */
  RAM   (rwx) : ORIGIN = 0x28100000, LENGTH = 32K   /* SAU Region1に一致 */
}

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
}