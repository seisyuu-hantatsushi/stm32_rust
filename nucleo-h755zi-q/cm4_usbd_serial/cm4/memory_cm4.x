MEMORY
{
  /* FLASH and RAM are mandatory memory regions */

  /* STM32H745xI/747xI/755xI/757xI Flash back 2*/
  FLASH  : ORIGIN = 0x08100000, LENGTH = 512K

  /* AXISRAM */
  AXISRAM : ORIGIN = 0x24000000, LENGTH = 512K

  /* SRAM */
  RAM   : ORIGIN = 0x30000000, LENGTH = 128K
  SRAM2 : ORIGIN = 0x30020000, LENGTH = 128K
  SRAM3 : ORIGIN = 0x30040000, LENGTH = 32K
  SRAM4 : ORIGIN = 0x38000000, LENGTH = 64K

  /* Backup SRAM */
  BSRAM : ORIGIN = 0x38800000, LENGTH = 4K

  SRAM3_NONCACHE : ORIGIN = 0x10040000, LENGTH = 32K

  /* Instruction TCM */
  ITCM  : ORIGIN = 0x00000000, LENGTH = 64K
}

/* The location of the stack can be overridden using the
   `_stack_start` symbol.  Place the stack at the end of RAM */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

/* The location of the .text section can be overridden using the
   `_stext` symbol.  By default it will place after .vector_table */
/* _stext = ORIGIN(FLASH) + 0x40c; */

/* These sections are used for some of the examples */
SECTIONS {
  .sram3_uncache (NOLOAD) : ALIGN(4) {
    *(.sram3_uncache .sram3_uncache.*);
    . = ALIGN(4);
    } > SRAM3_NONCACHE
  .axisram (NOLOAD) : ALIGN(8) {
    *(.axisram .axisram.*);
    . = ALIGN(8);
    } > AXISRAM
  .sram2 (NOLOAD) : ALIGN(4) {
    *(.sram2 .sram2.*);
    . = ALIGN(4);
    } > SRAM2
  .sram3 (NOLOAD) : ALIGN(4) {
    *(.sram3 .sram3.*);
    . = ALIGN(4);
    } > SRAM3
  .sram4 (NOLOAD) : ALIGN(4) {
    *(.sram4 .sram4.*);
    . = ALIGN(4);
    } > SRAM4
};
