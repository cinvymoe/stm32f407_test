//! Configuration constants and Flash partition layout

#![allow(dead_code)] // Will be used in later tasks

use embassy_time::Duration;

/// Flash slot enumeration (A/B partitions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Slot {
    /// Slot A - Active partition
    A,
    /// Slot B - Update partition
    B,
}

impl Slot {
    /// Get the start address of the slot
    pub const fn start_address(&self) -> u32 {
        match self {
            Slot::A => FLASH_CONFIG.slot_a_start,
            Slot::B => FLASH_CONFIG.slot_b_start,
        }
    }

    /// Get the size of the slot
    pub const fn size(&self) -> u32 {
        match self {
            Slot::A => FLASH_CONFIG.slot_a_size,
            Slot::B => FLASH_CONFIG.slot_b_size,
        }
    }

    /// Get the end address of the slot (exclusive)
    pub const fn end_address(&self) -> u32 {
        match self {
            Slot::A => FLASH_CONFIG.slot_a_start + FLASH_CONFIG.slot_a_size,
            Slot::B => FLASH_CONFIG.slot_b_start + FLASH_CONFIG.slot_b_size,
        }
    }

    /// Convert to numeric value (0 for A, 1 for B)
    pub const fn to_u8(&self) -> u8 {
        match self {
            Slot::A => 0,
            Slot::B => 1,
        }
    }

    /// Convert from numeric value (0 for A, 1 for B)
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Slot::A),
            1 => Some(Slot::B),
            _ => None,
        }
    }

    /// Get the other slot
    pub const fn other(&self) -> Self {
        match self {
            Slot::A => Slot::B,
            Slot::B => Slot::A,
        }
    }
}

/// Flash partition configuration
///
/// Defines the memory layout for the bootloader, application slots,
/// and boot state storage in Flash memory.
#[derive(Debug, Clone, Copy)]
pub struct FlashConfig {
    /// Bootloader start address
    pub bootloader_start: u32,
    /// Bootloader size in bytes
    pub bootloader_size: u32,
    /// Slot A (Active) start address
    pub slot_a_start: u32,
    /// Slot A size in bytes
    pub slot_a_size: u32,
    /// Slot B (Update) start address
    pub slot_b_start: u32,
    /// Slot B size in bytes
    pub slot_b_size: u32,
    /// Boot state storage start address
    pub state_start: u32,
    /// Boot state storage size in bytes
    pub state_size: u32,
}

/// Flash partition layout for STM32F407VG (1MB Flash)
/// Total: 64KB + 448KB + 448KB + 64KB = 1024KB (1MB)
pub const FLASH_CONFIG: FlashConfig = FlashConfig {
    bootloader_start: 0x0800_0000,
    bootloader_size: 64 * 1024, // 64KB

    slot_a_start: 0x0801_0000,
    slot_a_size: 448 * 1024, // 448KB

    slot_b_start: 0x0808_0000,
    slot_b_size: 448 * 1024, // 448KB

    state_start: 0x080F_0000,
    state_size: 64 * 1024, // 64KB (ends at 0x0810_0000 = 1MB)
};

/// Firmware file name on SD card
pub const FIRMWARE_FILENAME: &str = "firmware.bin";

/// Maximum firmware size (448KB to fit in slots)
pub const MAX_FIRMWARE_SIZE: u32 = 448 * 1024;

/// Flash write block size (4KB)
pub const FLASH_BLOCK_SIZE: usize = 4096;

/// SD card read buffer size (4KB)
pub const SD_BUFFER_SIZE: usize = 4096;

/// SD card initialization timeout
pub const SD_INIT_TIMEOUT: Duration = Duration::from_secs(5);

/// SD card read timeout
pub const SD_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Flash write timeout
pub const FLASH_WRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// Button debounce time (milliseconds)
pub const BUTTON_DEBOUNCE_MS: u64 = 20;

/// Button check window (milliseconds)
pub const BUTTON_CHECK_WINDOW_MS: u64 = 2000;

/// LED blink periods (milliseconds)
pub const LED_FAST_BLINK_MS: u64 = 100;
/// LED medium blink period (milliseconds) - used during firmware reading
pub const LED_MEDIUM_BLINK_MS: u64 = 200;
/// LED slow blink period (milliseconds) - used during firmware writing
pub const LED_SLOW_BLINK_MS: u64 = 500;

/// Maximum rollback attempts
pub const MAX_ROLLBACK_ATTEMPTS: u8 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    // Import std test macros
    #[allow(unused_imports)]
    use std::{assert, assert_eq};

    #[test]
    fn test_flash_partitions_no_overlap() {
        let cfg = FLASH_CONFIG;

        // Check bootloader doesn't overlap with slot A
        assert!(cfg.bootloader_start + cfg.bootloader_size <= cfg.slot_a_start);

        // Check slot A doesn't overlap with slot B
        assert!(cfg.slot_a_start + cfg.slot_a_size <= cfg.slot_b_start);

        // Check slot B doesn't overlap with state
        assert!(cfg.slot_b_start + cfg.slot_b_size <= cfg.state_start);

        // Check state doesn't exceed 1MB Flash
        assert!(cfg.state_start + cfg.state_size <= 0x0800_0000 + 1024 * 1024);
    }

    #[test]
    fn test_flash_partitions_sizes() {
        let cfg = FLASH_CONFIG;

        assert_eq!(cfg.bootloader_size, 64 * 1024);
        assert_eq!(cfg.slot_a_size, 448 * 1024);
        assert_eq!(cfg.slot_b_size, 448 * 1024);
        assert_eq!(cfg.state_size, 64 * 1024);
    }

    #[test]
    fn test_max_firmware_size() {
        assert_eq!(MAX_FIRMWARE_SIZE, 448 * 1024);
        assert_eq!(MAX_FIRMWARE_SIZE, FLASH_CONFIG.slot_a_size);
        assert_eq!(MAX_FIRMWARE_SIZE, FLASH_CONFIG.slot_b_size);
    }

    #[test]
    fn test_slot_addresses() {
        assert_eq!(Slot::A.start_address(), 0x0801_0000);
        assert_eq!(Slot::B.start_address(), 0x0808_0000);

        assert_eq!(Slot::A.size(), 448 * 1024);
        assert_eq!(Slot::B.size(), 448 * 1024);

        assert_eq!(Slot::A.end_address(), 0x0808_0000);
        assert_eq!(Slot::B.end_address(), 0x080F_0000);
    }

    #[test]
    fn test_slot_conversion() {
        assert_eq!(Slot::A.to_u8(), 0);
        assert_eq!(Slot::B.to_u8(), 1);

        assert_eq!(Slot::from_u8(0), Some(Slot::A));
        assert_eq!(Slot::from_u8(1), Some(Slot::B));
        assert_eq!(Slot::from_u8(2), None);
        assert_eq!(Slot::from_u8(255), None);
    }

    #[test]
    fn test_slot_other() {
        assert_eq!(Slot::A.other(), Slot::B);
        assert_eq!(Slot::B.other(), Slot::A);
    }

    #[test]
    fn test_slot_no_overlap() {
        // Verify that Slot A and Slot B don't overlap
        assert!(Slot::A.end_address() <= Slot::B.start_address());
    }
}
