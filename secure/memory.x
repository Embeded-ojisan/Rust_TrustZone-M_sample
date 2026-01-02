/* secure/memory.x (MPS2-AN521 / minimal PoC) */
MEMORY
{
  /* QEMU reset expects vectors at Flash base (0x1000_0000) */
  FLASH : ORIGIN = 0x10000000, LENGTH = 512K

  /* Secure RAM (TF-M map showed 0x3800_0000 .. but for PoC we keep it) */
  RAM   : ORIGIN = 0x38000000, LENGTH = 1M
}

SECTIONS
{
  /* vector_table直後付近にNSC veneerを置く（既存構造を維持） */
  .gnu.sgstubs 0x10000800 :
  {
    *(.gnu.sgstubs*)
  } > FLASH

  .text_ns :
  {
    *(.text_nonsecure_entry*)
  } > FLASH
}

/* cortex-m-rtに text の開始を教える */
PROVIDE(_stext = 0x10000900);
