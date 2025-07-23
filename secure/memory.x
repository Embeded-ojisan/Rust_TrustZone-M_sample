MEMORY
{
  FLASH : ORIGIN = 0x10000000, LENGTH = 256K
  RAM   : ORIGIN = 0x38000000, LENGTH = 128K
}

SECTIONS
{
  /* Veneerエリア：vector_table の直後＝0x10000800 (2KBベクタの場合) */
  .gnu.sgstubs 0x10000800 :
  {
    *(.gnu.sgstubs*)
  } > FLASH

  .text_ns :
  {
    *(.text_nonsecure_entry*)
  } > FLASH

  /* ...（RAM, bss, dataなどはcortex-m-rtのデフォルトにまかせる）... */
}

/* cortex-m-rtに「.textセクション開始アドレスはここ」と教える */
PROVIDE(_stext = 0x10000900);