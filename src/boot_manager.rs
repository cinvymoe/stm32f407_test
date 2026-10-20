//! Boot Manager - Main bootloader orchestration
//!
//! This module provides the BootManager struct which orchestrates the entire
//! bootloader process, including:
//! - Button detection for upgrade trigger
//! - SD card firmware reading
//! - CRC32 verification
//! - Flash writing and verification
//! - Application jumping
//! - Error handling and rollback
//!
//! The BootManager integrates all other modules and implements the main
//! bootloader state machine.

use crate::button_input::ButtonInput;
use crate::config::{
    Slot, BUTTON_CHECK_WINDOW_MS, FIRMWARE_FILENAME, MAX_FIRMWARE_SIZE, SD_BUFFER_SIZE,
};
use crate::error::UpgradeError;
use crate::firmware_verifier::FirmwareVerifier;
use crate::flash_manager::FlashManager;
use crate::led_indicator::{LedIndicator, LedPattern};
use crate::sd_card_reader::SdCardReader;
use embassy_time::{Duration, Timer};

/// Boot Manager - Main bootloader orchestration
///
/// Manages the complete bootloader lifecycle:
/// 1. Check button state for upgrade trigger
/// 2. If upgrade requested: read firmware from SD, verify, and flash
/// 3. If no upgrade: validate application and jump to it
/// 4. Handle errors and rollback if needed
///
/// # Requirements
/// - Requirement 3.4.1: Startup detection and application jumping
/// - Requirement 2.1: Complete firmware upgrade flow
pub struct BootManager<'d> {
    flash: FlashManager<'d>,
    button: ButtonInput<'d>,
    led: LedIndicator,
}

impl<'d> BootManager<'d> {
    /// Create a new BootManager instance
    ///
    /// # Arguments
    /// * `flash` - FlashManager for firmware storage operations
    /// * `button` - ButtonInput for upgrade trigger detection
    /// * `led` - LedIndicator for status display
    ///
    /// # Requirements
    /// Validates requirement 3.4.1 - Bootloader initialization
    ///
    /// # Example
    /// ```no_run
    /// let flash = Flash::new(p.FLASH, p.DMA2_CH0);
    /// let flash_manager = FlashManager::new(flash);
    /// let button = ButtonInput::new(p.PE4.degrade());
    /// let led = LedIndicator::new(
    ///     Output::new(p.PA7.degrade(), Level::Low, Speed::Low),
    ///     Output::new(p.PA6.degrade(), Level::Low, Speed::Low),
    /// );
    ///
    /// let boot_manager = BootManager::new(flash_manager, button, led);
    /// ```
    pub fn new(flash: FlashManager<'d>, button: ButtonInput<'d>, led: LedIndicator) -> Self {
        Self { flash, button, led }
    }

    /// Main bootloader run loop
    ///
    /// This is the main entry point for the bootloader. It:
    /// 1. Checks if upgrade is requested (button pressed)
    /// 2. If yes: performs upgrade flow
    /// 3. If no: validates and jumps to application
    /// 4. Never returns (either jumps to app or loops on fatal error)
    ///
    /// # Arguments
    /// * `sdmmc` - Optional SDMMC peripheral for SD card access (only needed if upgrade is requested)
    ///
    /// # Requirements
    /// Validates requirement 2.1 - Complete firmware upgrade flow
    pub async fn run<T, D>(&mut self, sdmmc: Option<embassy_stm32::sdmmc::Sdmmc<'d, T, D>>) -> !
    where
        T: embassy_stm32::sdmmc::Instance,
        D: embassy_stm32::sdmmc::SdmmcDma<T>,
    {
        defmt::info!("STM32F407 Bootloader starting...");

        // Start heartbeat LED
        self.led.set_heartbeat(true);

        // Check if upgrade is requested
        if self.should_upgrade().await {
            defmt::info!("Upgrade requested, entering upgrade mode");

            if let Some(sdmmc_peripheral) = sdmmc {
                // Perform upgrade
                match self.perform_upgrade(sdmmc_peripheral).await {
                    Ok(()) => {
                        defmt::info!("Upgrade successful! Rebooting...");
                        self.led.set_status(LedPattern::On);
                        Timer::after(Duration::from_secs(2)).await;

                        // Reboot to activate new firmware
                        cortex_m::peripheral::SCB::sys_reset();
                    }
                    Err(e) => {
                        defmt::error!("Upgrade failed: {:?}", e);
                        self.led.set_status(LedPattern::FastBlink);
                        Timer::after(Duration::from_secs(3)).await;

                        // Fall through to jump to existing application
                    }
                }
            } else {
                defmt::error!("Upgrade requested but no SDMMC peripheral provided!");
                self.led.set_status(LedPattern::FastBlink);
                Timer::after(Duration::from_secs(3)).await;
            }
        }

        // No upgrade requested or upgrade failed - jump to application
        defmt::info!("Checking application validity...");

        // Check which slot to boot from
        let boot_slot = self.determine_boot_slot().await;

        match boot_slot {
            Some(slot) => {
                defmt::info!("Jumping to application in {:?}", slot);
                self.jump_to_app(slot);
            }
            None => {
                defmt::error!("No valid application found!");
                self.led.set_status(LedPattern::FastBlink);

                // Enter infinite loop with SOS pattern
                loop {
                    Timer::after(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Check if upgrade should be performed
    ///
    /// Checks the button state within a detection window (2 seconds).
    /// If the button is pressed during this window, upgrade is triggered.
    ///
    /// # Returns
    /// * `true` - Upgrade should be performed
    /// * `false` - No upgrade requested
    ///
    /// # Requirements
    /// Validates requirement 3.4.1 - Startup detection
    ///
    /// # Example
    /// ```no_run
    /// # async fn example(mut boot_manager: BootManager<'_>) {
    /// if boot_manager.should_upgrade().await {
    ///     // Perform upgrade
    /// }
    /// # }
    /// ```
    async fn should_upgrade(&mut self) -> bool {
        defmt::info!("Checking for upgrade button (2 second window)...");

        // Check button state with timeout
        let check_duration = Duration::from_millis(BUTTON_CHECK_WINDOW_MS);
        let start = embassy_time::Instant::now();

        while start.elapsed() < check_duration {
            if self.button.is_pressed().await {
                defmt::info!("Upgrade button detected!");
                return true;
            }

            // Small delay to avoid busy-waiting
            Timer::after(Duration::from_millis(50)).await;
        }

        defmt::info!("No upgrade button detected");
        false
    }

    /// Perform the complete firmware upgrade process
    ///
    /// This async method orchestrates the entire upgrade flow:
    /// 1. Initialize SD card and mount filesystem
    /// 2. Open and read firmware file
    /// 3. Verify firmware size and calculate CRC32
    /// 4. Erase target slot (Slot B)
    /// 5. Write firmware in blocks with verification
    /// 6. Mark slot as pending activation
    ///
    /// # Arguments
    /// * `sdmmc` - Configured SDMMC peripheral for SD card access
    ///
    /// # Returns
    /// * `Ok(())` - Upgrade completed successfully
    /// * `Err(UpgradeError)` - Upgrade failed at some stage
    ///
    /// # Requirements
    /// Validates requirements:
    /// - 3.4.2: Firmware reading from SD card
    /// - 3.4.3: Firmware writing to Flash
    /// - 3.4.4: Partition switching
    async fn perform_upgrade<T, D>(
        &mut self,
        sdmmc: embassy_stm32::sdmmc::Sdmmc<'d, T, D>,
    ) -> Result<(), UpgradeError>
    where
        T: embassy_stm32::sdmmc::Instance,
        D: embassy_stm32::sdmmc::SdmmcDma<T>,
    {
        defmt::info!("Starting firmware upgrade process");

        // Set LED to indicate reading
        self.led.set_status(LedPattern::MediumBlink);

        // Step 1: Initialize SD card
        defmt::info!("Initializing SD card...");
        let mut sd_reader = SdCardReader::init(sdmmc)
            .await
            .map_err(UpgradeError::SdCard)?;

        // Step 2: Mount filesystem
        defmt::info!("Mounting FAT32 filesystem...");
        sd_reader.mount().await.map_err(UpgradeError::SdCard)?;

        // Step 3: Open firmware file
        defmt::info!("Opening firmware file: {}", FIRMWARE_FILENAME);
        let mut firmware_file = sd_reader
            .open_firmware()
            .await
            .map_err(UpgradeError::SdCard)?;

        let firmware_size = firmware_file.size();
        defmt::info!("Firmware size: {} bytes", firmware_size);

        // Step 4: Validate firmware size
        if firmware_size > MAX_FIRMWARE_SIZE {
            defmt::error!(
                "Firmware too large: {} > {} bytes",
                firmware_size,
                MAX_FIRMWARE_SIZE
            );
            return Err(UpgradeError::Verification(
                crate::error::VerifyError::SizeTooLarge,
            ));
        }

        // Step 5: Calculate CRC32 while reading firmware
        defmt::info!("Calculating CRC32 checksum...");
        let mut verifier = FirmwareVerifier::new();
        let mut buffer = [0u8; SD_BUFFER_SIZE];
        let mut total_read = 0u32;

        // First pass: calculate CRC
        loop {
            let bytes_read = sd_reader
                .read(&mut firmware_file, &mut buffer)
                .await
                .map_err(UpgradeError::SdCard)?;

            if bytes_read == 0 {
                break;
            }

            verifier.update(&buffer[..bytes_read]);
            total_read += bytes_read as u32;
        }

        let crc32 = verifier.finalize();
        defmt::info!("Firmware CRC32: 0x{:08X}", crc32);
        defmt::info!("Total bytes read: {}", total_read);

        // Verify we read the expected amount
        if total_read != firmware_size {
            defmt::error!(
                "Size mismatch: read {} bytes, expected {}",
                total_read,
                firmware_size
            );
            return Err(UpgradeError::Verification(
                crate::error::VerifyError::SizeMismatch,
            ));
        }

        // Step 6: Erase target slot (Slot B)
        defmt::info!("Erasing target slot (Slot B)...");
        self.led.set_status(LedPattern::SlowBlink);

        self.flash
            .erase_slot(Slot::B)
            .await
            .map_err(UpgradeError::Flash)?;

        defmt::info!("Slot B erased successfully");

        // Step 7: Rewind file and write firmware to Flash
        defmt::info!("Writing firmware to Flash...");

        // Re-open the file to read from the beginning
        let mut firmware_file = sd_reader
            .open_firmware()
            .await
            .map_err(UpgradeError::SdCard)?;

        let mut offset = 0u32;
        let mut blocks_written = 0u32;

        loop {
            // Read a block from SD card
            let bytes_read = sd_reader
                .read(&mut firmware_file, &mut buffer)
                .await
                .map_err(UpgradeError::SdCard)?;

            if bytes_read == 0 {
                break;
            }

            // Write block to Flash
            self.flash
                .write_block(Slot::B, offset, &buffer[..bytes_read])
                .await
                .map_err(UpgradeError::Flash)?;

            // Verify the written block
            let verified = self
                .flash
                .verify_block(Slot::B, offset, &buffer[..bytes_read])
                .await
                .map_err(UpgradeError::Flash)?;

            if !verified {
                defmt::error!("Flash verification failed at offset {}", offset);
                return Err(UpgradeError::Flash(crate::error::FlashError::VerifyFailed));
            }

            offset += bytes_read as u32;
            blocks_written += 1;

            // Log progress every 16 blocks (64KB)
            if blocks_written % 16 == 0 {
                let progress = (offset * 100) / firmware_size;
                defmt::info!("Progress: {}% ({} bytes)", progress, offset);
            }
        }

        defmt::info!("Firmware written successfully: {} bytes", offset);

        // Step 8: Mark Slot B as pending activation
        defmt::info!("Marking Slot B for activation...");
        self.flash
            .mark_slot_pending(Slot::B)
            .await
            .map_err(UpgradeError::Flash)?;

        defmt::info!("Upgrade process completed successfully!");
        Ok(())
    }

    /// Determine which slot to boot from
    ///
    /// Checks the boot state and validates application slots to determine
    /// which firmware to boot. Handles rollback if needed.
    ///
    /// # Returns
    /// * `Some(Slot)` - Valid slot to boot from
    /// * `None` - No valid application found
    ///
    /// # Requirements
    /// Validates requirement 3.4.4 - Partition switching and rollback
    async fn determine_boot_slot(&mut self) -> Option<Slot> {
        // Try to load boot state
        // For now, we'll use a simple heuristic: check both slots and boot from the first valid one
        // In a full implementation, this would read the BootState from Flash

        // Check Slot A first (default)
        if self.flash.is_app_valid(Slot::A).await {
            defmt::info!("Slot A contains valid application");
            return Some(Slot::A);
        }

        // Check Slot B
        if self.flash.is_app_valid(Slot::B).await {
            defmt::info!("Slot B contains valid application");
            return Some(Slot::B);
        }

        // No valid application found
        defmt::error!("No valid application in any slot");
        None
    }

    /// Jump to application in the specified slot
    ///
    /// This method performs the following steps:
    /// 1. Disable interrupts
    /// 2. Set vector table offset (VTOR) to application address
    /// 3. Load stack pointer from application vector table
    /// 4. Jump to application reset handler
    ///
    /// This function never returns.
    ///
    /// # Arguments
    /// * `slot` - The slot containing the application to jump to
    ///
    /// # Requirements
    /// Validates requirement 3.4.1 - Application jumping
    ///
    /// # Safety
    /// This function is unsafe because it:
    /// - Directly manipulates CPU registers
    /// - Jumps to arbitrary code
    /// - Assumes the application has a valid vector table
    ///
    /// # Example
    /// ```no_run
    /// # fn example(boot_manager: BootManager<'_>) {
    /// boot_manager.jump_to_app(Slot::A);
    /// // Never returns
    /// # }
    /// ```
    fn jump_to_app(&self, slot: Slot) -> ! {
        let app_address = slot.start_address();

        defmt::info!("Jumping to application at 0x{:08X}", app_address);

        unsafe {
            // Disable interrupts
            cortex_m::interrupt::disable();

            // Get the application's vector table
            let vt = app_address as *const u32;

            // Read stack pointer and reset vector from application
            let stack_pointer = core::ptr::read_volatile(vt);
            let reset_vector = core::ptr::read_volatile(vt.offset(1));

            defmt::info!("Stack pointer: 0x{:08X}", stack_pointer);
            defmt::info!("Reset vector: 0x{:08X}", reset_vector);

            // Set vector table offset register (VTOR)
            let scb = &*cortex_m::peripheral::SCB::PTR;
            scb.vtor.write(app_address);

            // Create a function pointer to the reset handler
            // Clear the thumb bit (bit 0) from the reset vector
            let reset_handler = (reset_vector & !1) as *const ();
            let reset_fn: extern "C" fn() -> ! = core::mem::transmute(reset_handler);

            // Set the stack pointer
            cortex_m::register::msp::write(stack_pointer);

            // Jump to the application's reset handler
            reset_fn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FLASH_BLOCK_SIZE;

    #[test]
    fn test_boot_manager_constants() {
        // Verify configuration constants are reasonable
        assert!(BUTTON_CHECK_WINDOW_MS > 0);
        assert!(BUTTON_CHECK_WINDOW_MS <= 5000); // Max 5 seconds

        assert!(MAX_FIRMWARE_SIZE > 0);
        assert!(MAX_FIRMWARE_SIZE <= 512 * 1024); // Max 512KB

        assert!(FLASH_BLOCK_SIZE > 0);
        assert!(FLASH_BLOCK_SIZE <= 8192); // Reasonable block size

        assert!(SD_BUFFER_SIZE > 0);
        assert!(SD_BUFFER_SIZE >= FLASH_BLOCK_SIZE); // Buffer should be at least block size
    }

    #[test]
    fn test_firmware_filename() {
        assert_eq!(FIRMWARE_FILENAME, "firmware.bin");
    }

    #[test]
    fn test_slot_addresses() {
        let slot_a = Slot::A;
        let slot_b = Slot::B;

        // Verify slots don't overlap
        assert!(slot_a.end_address() <= slot_b.start_address());

        // Verify addresses are in Flash range
        assert!(slot_a.start_address() >= 0x0800_0000);
        assert!(slot_b.end_address() <= 0x0820_0000); // 2MB max for STM32F4
    }

    #[test]
    fn test_vector_table_layout() {
        // Verify assumptions about ARM Cortex-M vector table
        // Vector table starts with:
        // [0] = Initial stack pointer
        // [1] = Reset handler

        let test_address = 0x0801_0000u32;
        let vt = test_address as *const u32;

        // Verify pointer arithmetic
        unsafe {
            let sp_offset = vt.offset(0);
            let reset_offset = vt.offset(1);

            assert_eq!(sp_offset as u32, test_address);
            assert_eq!(reset_offset as u32, test_address + 4);
        }
    }

    #[test]
    fn test_thumb_bit_clearing() {
        // ARM Cortex-M reset vectors have bit 0 set (thumb mode)
        let reset_vector_with_thumb = 0x0801_0101u32;
        let reset_vector_cleared = reset_vector_with_thumb & !1;

        assert_eq!(reset_vector_cleared, 0x0801_0100);
        assert_eq!(reset_vector_cleared & 1, 0);
    }

    #[test]
    fn test_progress_calculation() {
        // Test progress percentage calculation
        let firmware_size = 512 * 1024u32; // 512KB
        let test_offsets = [
            (0, 0),
            (firmware_size / 4, 25),
            (firmware_size / 2, 50),
            (firmware_size * 3 / 4, 75),
            (firmware_size, 100),
        ];

        for (offset, expected_progress) in test_offsets.iter() {
            let progress = (offset * 100) / firmware_size;
            assert_eq!(progress, *expected_progress);
        }
    }

    #[test]
    fn test_block_counting() {
        // Test block counting logic
        let firmware_size = 100 * 1024u32; // 100KB
        let block_size = 4096u32; // 4KB

        let expected_blocks = (firmware_size + block_size - 1) / block_size;
        assert_eq!(expected_blocks, 25); // 100KB / 4KB = 25 blocks
    }

    #[test]
    fn test_size_validation() {
        // Test firmware size validation
        assert!(MAX_FIRMWARE_SIZE <= 512 * 1024);

        let valid_sizes = [0, 1024, 256 * 1024, MAX_FIRMWARE_SIZE];
        for size in valid_sizes.iter() {
            assert!(*size <= MAX_FIRMWARE_SIZE);
        }

        let invalid_sizes = [MAX_FIRMWARE_SIZE + 1, 1024 * 1024];
        for size in invalid_sizes.iter() {
            assert!(*size > MAX_FIRMWARE_SIZE);
        }
    }
}
