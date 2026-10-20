//! STM32F407 SD Card Bootloader Library
//!
//! This library provides a complete bootloader system for STM32F407 microcontrollers
//! with support for firmware upgrades from SD cards using the SDIO interface.
//!
//! # Features
//!
//! - **Dual-partition architecture**: Active/Update slots for safe firmware upgrades
//! - **CRC32 verification**: Ensures firmware integrity before flashing
//! - **Automatic rollback**: Recovers from failed upgrades (up to 3 attempts)
//! - **LED status indication**: Visual feedback during upgrade process
//! - **Button-triggered upgrades**: User-friendly upgrade initiation
//! - **SDIO 4-bit mode**: Fast firmware transfer from SD card
//! - **FAT32 support**: Standard filesystem for easy firmware deployment
//!
//! # Architecture
//!
//! The bootloader uses a dual-partition Flash layout:
//!
//! ```text
//! 0x08000000 - 0x08010000 (64KB)   : Bootloader
//! 0x08010000 - 0x08090000 (512KB)  : Application Slot A (Active)
//! 0x08090000 - 0x08110000 (512KB)  : Application Slot B (Update)
//! 0x08110000 - 0x08120000 (64KB)   : Boot State
//! ```
//!
//! # Usage
//!
//! ## Basic Example
//!
//! ```no_run
//! #![no_std]
//! #![no_main]
//!
//! use embassy_executor::Spawner;
//! use embassy_stm32::Config;
//! use stm32f407_sd_bootloader::boot_manager::BootManager;
//! use stm32f407_sd_bootloader::flash_manager::FlashManager;
//! use stm32f407_sd_bootloader::button_input::ButtonInput;
//! use stm32f407_sd_bootloader::led_indicator::LedIndicator;
//!
//! #[embassy_executor::main]
//! async fn main(_spawner: Spawner) {
//!     let p = embassy_stm32::init(Default::default());
//!     
//!     // Initialize components
//!     let flash = embassy_stm32::flash::Flash::new_blocking(p.FLASH);
//!     let flash_manager = FlashManager::new(flash);
//!     
//!     let button = ButtonInput::new(p.PE4.degrade());
//!     
//!     let led = LedIndicator::new(
//!         embassy_stm32::gpio::Output::new(p.PA7.degrade(), embassy_stm32::gpio::Level::Low, embassy_stm32::gpio::Speed::Low),
//!         embassy_stm32::gpio::Output::new(p.PA6.degrade(), embassy_stm32::gpio::Level::Low, embassy_stm32::gpio::Speed::Low),
//!     );
//!     
//!     // Create SDMMC peripheral for SD card access
//!     let sdmmc = embassy_stm32::sdmmc::Sdmmc::new_4bit(
//!         p.SDMMC1,
//!         p.DMA2_CH3,
//!         p.PC12,
//!         p.PD2,
//!         p.PC8,
//!         p.PC9,
//!         p.PC10,
//!         p.PC11,
//!         Default::default(),
//!     );
//!     
//!     // Run bootloader
//!     let mut boot_manager = BootManager::new(flash_manager, button, led);
//!     boot_manager.run(Some(sdmmc)).await;
//! }
//! ```
//!
//! ## Firmware Upgrade Process
//!
//! 1. Prepare firmware file (`firmware.bin`) and copy to SD card root directory
//! 2. Insert SD card into the device
//! 3. Press and hold the upgrade button (PE4)
//! 4. Reset the device
//! 5. Observe LED status:
//!    - Medium blink (200ms): Reading firmware
//!    - Slow blink (500ms): Writing firmware
//!    - Solid on: Upgrade successful
//!    - Fast blink (100ms): Upgrade failed
//! 6. Device automatically reboots into new firmware
//!
//! # Modules
//!
//! - [`boot_manager`]: Main bootloader orchestration and state machine
//! - [`flash_manager`]: Flash memory operations (erase, write, verify)
//! - [`sd_card_reader`]: SD card access via SDIO with FAT32 support
//! - [`firmware_verifier`]: CRC32 checksum calculation and size validation
//! - [`boot_state`]: Boot state management for partition switching
//! - [`led_indicator`]: LED status indication
//! - [`button_input`]: Button input with debouncing
//! - [`error`]: Error types and handling
//! - [`error_handler`]: Error classification and recovery strategies
//! - [`rollback`]: Automatic rollback mechanism
//! - [`config`]: Configuration constants and Flash layout
//! - [`timeout`]: Timeout utilities for I/O operations
//!
//! # Hardware Requirements
//!
//! ## Microcontroller
//! - STM32F407VG or compatible (1MB Flash, 128KB RAM)
//!
//! ## Peripherals
//! - SD card module (SDIO interface)
//! - Button for upgrade trigger (PE4)
//! - 2× LEDs for status indication (PA6, PA7)
//!
//! ## Pin Configuration
//!
//! | Pin  | Function        | Description          |
//! |------|-----------------|----------------------|
//! | PC8  | SDIO_D0         | SD card data line 0  |
//! | PC9  | SDIO_D1         | SD card data line 1  |
//! | PC10 | SDIO_D2         | SD card data line 2  |
//! | PC11 | SDIO_D3         | SD card data line 3  |
//! | PC12 | SDIO_CK         | SD card clock        |
//! | PD2  | SDIO_CMD        | SD card command      |
//! | PE4  | Button          | Upgrade trigger      |
//! | PA7  | LED1            | Status indicator     |
//! | PA6  | LED2            | Heartbeat indicator  |
//!
//! # Safety and Reliability
//!
//! The bootloader implements multiple safety mechanisms:
//!
//! - **CRC32 verification**: All firmware is verified before flashing
//! - **Write verification**: Each Flash block is read back and verified
//! - **Automatic rollback**: Failed upgrades automatically revert to stable firmware
//! - **Timeout protection**: All I/O operations have timeout limits
//! - **Partition validation**: Applications are validated before jumping
//! - **Error recovery**: Comprehensive error handling with graceful degradation
//!
//! # Performance
//!
//! - Firmware read speed: ≥100KB/s
//! - Firmware write speed: ≥50KB/s
//! - Complete upgrade time: ≤15 seconds (512KB firmware)
//! - Bootloader startup time: ≤500ms
//!
//! # Requirements Traceability
//!
//! This library implements the requirements specified in:
//! - `.kiro/specs/stm32-sd-bootloader/requirements.md`
//! - `.kiro/specs/stm32-sd-bootloader/design.md`
//!
//! Key requirements:
//! - Requirement 2.1: Complete firmware upgrade flow
//! - Requirement 2.2: Upgrade safety (CRC, dual-partition, rollback)
//! - Requirement 2.3: Status indication via LEDs
//! - Requirement 3.1: Hardware interfaces (SDIO, button, LED)
//! - Requirement 3.2: Flash partition layout
//! - Requirement 3.3: Firmware file format (FAT32, firmware.bin)
//! - Requirement 3.4: Upgrade flow (detect, read, verify, write, switch)
//! - Requirement 3.5: Error handling and timeout protection
//!
//! # Testing
//!
//! The library includes comprehensive tests:
//! - Unit tests for individual components
//! - Property-based tests for correctness properties
//! - Integration tests for complete workflows
//!
//! Run tests with:
//! ```bash
//! cargo test
//! ```
//!
//! # Examples
//!
//! See the `examples/` directory for:
//! - `sd_card_test.rs`: SD card initialization and testing
//! - `fat32_firmware_test.rs`: FAT32 filesystem and firmware reading
//!
//! # License
//!
//! This project is licensed under MIT or Apache-2.0.

#![no_std]
#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod boot_manager;
pub mod boot_state;
pub mod button_input;
pub mod config;
pub mod error;
pub mod error_handler;
pub mod firmware_verifier;
pub mod flash_manager;
pub mod led_indicator;
pub mod rollback;
pub mod sd_card_reader;
pub mod timeout;

#[cfg(test)]
extern crate std;
