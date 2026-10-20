/* STM32F407VG Bootloader Memory Layout
 * 
 * Flash Partitions (1MB total):
 * 0x08000000 - 0x08010000 (64KB)   : Bootloader
 * 0x08010000 - 0x08090000 (512KB)  : Application Slot A (Active)
 * 0x08090000 - 0x08110000 (512KB)  : Application Slot B (Update)
 * 0x08110000 - 0x08120000 (64KB)   : Boot State (embassy-boot state storage)
 * 
 * RAM: 128KB
 */

MEMORY
{
  /* Bootloader Flash - 64KB */
  FLASH : ORIGIN = 0x08000000, LENGTH = 64K
  
  /* RAM - 128KB */
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}

/* Stack starts at the end of RAM */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

/* Flash partition addresses for bootloader logic */
/* These are used by the bootloader code, not by the linker */
__bootloader_start = 0x08000000;
__bootloader_end = 0x08010000;
__slot_a_start = 0x08010000;
__slot_a_end = 0x08090000;
__slot_b_start = 0x08090000;
__slot_b_end = 0x08110000;
__state_start = 0x08110000;
__state_end = 0x08120000;