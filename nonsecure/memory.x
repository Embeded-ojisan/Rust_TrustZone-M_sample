/* nonsecure/memory.x (MPS2-AN521 / TF-M map aligned) */
MEMORY
{
  /* NS Flash (from zephyr.map):
     FLASH 0x0010_0000 size 0x80000 (512KB) */
  FLASH : ORIGIN = 0x00100000, LENGTH = 512K

  /* NS RAM (from zephyr.map):
     RAM 0x2810_0000 size 0x8000 (32KB) */
  RAM   : ORIGIN = 0x28100000, LENGTH = 32K
}

REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
