MEMORY
{
  FLASH : ORIGIN = 0x00200000, LENGTH = 512K
  RAM   : ORIGIN = 0x28200000, LENGTH = 256K
}

SECTIONS {
  /* hello_from_ns を 0x00200800 に置く */
  .ns_callable 0x00200800 :
  {
    KEEP(*(.ns_callable_fn))     /* 関数本体 */
    KEEP(*(.ns_callable_ptr))    /* ポインタ */
  } > FLASH

  /* cortex-m-rt に渡す _stext の定義 */
  PROVIDE(_stext = 0x00200900);
}