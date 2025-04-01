/* STM32F407VG的内存布局 */
MEMORY
{
  /* 闪存和RAM的起始地址和大小 */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}

/* 这是用于cortex-m-rt的链接器脚本 */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);