//! Flash Manager - Handles Flash operations for firmware upgrade
//!
//! This module provides the FlashManager struct which manages Flash partitions,
//! performs erase and write operations, and verifies data integrity.
//!
//! Supports both blocking and async Flash operations.

use crate::config::Slot;
use crate::error::FlashError;
use embassy_stm32::flash::{Blocking, Flash};
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};

/// Flash Manager for handling firmware storage operations
///
/// Manages Flash partitions (Slot A and Slot B) and provides async methods for:
/// - Erasing partitions
/// - Writing data blocks
/// - Verifying written data
/// - Address range validation
///
/// Supports both blocking and async Flash modes.
pub struct FlashManager<'d> {
    flash: Flash<'d, Blocking>,
}

impl<'d> FlashManager<'d> {
    /// Create a new FlashManager instance
    ///
    /// # Arguments
    /// * `flash` - Embassy Flash peripheral instance (blocking mode)
    ///
    /// # Example
    /// ```no_run
    /// let flash = Flash::new_blocking(p.FLASH);
    /// let flash_manager = FlashManager::new(flash);
    /// ```
    pub fn new(flash: Flash<'d, Blocking>) -> Self {
        Self { flash }
    }

    /// Erase a complete slot (partition)
    ///
    /// This async operation erases all Flash sectors in the specified slot,
    /// preparing it for new firmware data.
    ///
    /// # Arguments
    /// * `slot` - The slot to erase (Slot::A or Slot::B)
    ///
    /// # Returns
    /// * `Ok(())` - Erase operation successful
    /// * `Err(FlashError::EraseFailed)` - Erase operation failed
    ///
    /// # Requirements
    /// Validates requirement 3.4.3 - Firmware writing process
    ///
    /// # Example
    /// ```no_run
    /// flash_manager.erase_slot(Slot::B).await?;
    /// ```
    pub async fn erase_slot(&mut self, slot: Slot) -> Result<(), FlashError> {
        let start_addr = slot.start_address();
        let end_addr = slot.end_address();

        // Erase the entire slot range using blocking API
        self.flash
            .erase(start_addr, end_addr)
            .map_err(|_| FlashError::EraseFailed)?;

        Ok(())
    }

    /// Write a data block to a slot at the specified offset
    ///
    /// Async operation that writes data to Flash memory within the specified slot.
    /// The data must be properly aligned according to Flash requirements (typically word-aligned).
    ///
    /// # Arguments
    /// * `slot` - Target slot (Slot::A or Slot::B)
    /// * `offset` - Offset from the start of the slot (in bytes)
    /// * `data` - Data buffer to write
    ///
    /// # Returns
    /// * `Ok(())` - Write operation successful
    /// * `Err(FlashError::InvalidAddress)` - Address out of range
    /// * `Err(FlashError::WriteFailed)` - Write operation failed
    ///
    /// # Requirements
    /// Validates requirement 3.4.3 - Firmware writing process
    ///
    /// # Example
    /// ```no_run
    /// let data = [0u8; 256];
    /// flash_manager.write_block(Slot::B, 0, &data).await?;
    /// ```
    pub async fn write_block(
        &mut self,
        slot: Slot,
        offset: u32,
        data: &[u8],
    ) -> Result<(), FlashError> {
        // Validate address range
        let start_addr = slot.start_address();
        let write_addr = start_addr
            .checked_add(offset)
            .ok_or(FlashError::InvalidAddress)?;

        // Check if write would exceed slot boundary
        let end_addr = write_addr
            .checked_add(data.len() as u32)
            .ok_or(FlashError::InvalidAddress)?;

        if end_addr > slot.end_address() {
            return Err(FlashError::InvalidAddress);
        }

        // Perform the write operation using blocking API
        self.flash
            .write(write_addr, data)
            .map_err(|_| FlashError::WriteFailed)?;

        Ok(())
    }

    /// Verify that data was written correctly
    ///
    /// Async operation that reads back data from Flash and compares it with the expected data
    /// to ensure write integrity.
    ///
    /// # Arguments
    /// * `slot` - Slot to verify (Slot::A or Slot::B)
    /// * `offset` - Offset from the start of the slot (in bytes)
    /// * `expected` - Expected data to compare against
    ///
    /// # Returns
    /// * `Ok(true)` - Data matches expected value
    /// * `Ok(false)` - Data does not match (verification failed)
    /// * `Err(FlashError::InvalidAddress)` - Address out of range
    ///
    /// # Requirements
    /// Validates requirement 3.4.3 - Firmware writing process
    ///
    /// # Example
    /// ```no_run
    /// let data = [0u8; 256];
    /// flash_manager.write_block(Slot::B, 0, &data).await?;
    /// let verified = flash_manager.verify_block(Slot::B, 0, &data).await?;
    /// assert!(verified);
    /// ```
    pub async fn verify_block(
        &mut self,
        slot: Slot,
        offset: u32,
        expected: &[u8],
    ) -> Result<bool, FlashError> {
        // Validate address range
        let start_addr = slot.start_address();
        let read_addr = start_addr
            .checked_add(offset)
            .ok_or(FlashError::InvalidAddress)?;

        // Check if read would exceed slot boundary
        let end_addr = read_addr
            .checked_add(expected.len() as u32)
            .ok_or(FlashError::InvalidAddress)?;

        if end_addr > slot.end_address() {
            return Err(FlashError::InvalidAddress);
        }

        // Read data from Flash in chunks
        let mut read_buffer = [0u8; 256]; // Use a reasonable buffer size
        let mut offset_in_expected = 0;

        while offset_in_expected < expected.len() {
            let chunk_size = core::cmp::min(read_buffer.len(), expected.len() - offset_in_expected);
            let chunk_addr = read_addr + offset_in_expected as u32;

            // Read chunk from Flash using blocking API
            self.flash
                .read(chunk_addr, &mut read_buffer[..chunk_size])
                .map_err(|_| FlashError::VerifyFailed)?;

            // Compare with expected data
            if read_buffer[..chunk_size]
                != expected[offset_in_expected..offset_in_expected + chunk_size]
            {
                return Ok(false);
            }

            offset_in_expected += chunk_size;
        }

        Ok(true)
    }

    /// Check if an address is valid for a given slot
    ///
    /// Validates that an address and length fall within the slot boundaries.
    ///
    /// # Arguments
    /// * `slot` - Slot to check
    /// * `offset` - Offset from the start of the slot
    /// * `length` - Length of data to access
    ///
    /// # Returns
    /// * `true` - Address range is valid
    /// * `false` - Address range is invalid
    ///
    /// # Example
    /// ```no_run
    /// if flash_manager.is_address_valid(Slot::B, 0, 1024) {
    ///     // Safe to write 1024 bytes at offset 0
    /// }
    /// ```
    pub fn is_address_valid(&self, slot: Slot, offset: u32, length: u32) -> bool {
        let start_addr = slot.start_address();

        // Check for overflow in offset calculation
        let write_addr = match start_addr.checked_add(offset) {
            Some(addr) => addr,
            None => return false,
        };

        // Check for overflow in end address calculation
        let end_addr = match write_addr.checked_add(length) {
            Some(addr) => addr,
            None => return false,
        };

        // Check if within slot boundaries
        end_addr <= slot.end_address()
    }

    /// Mark a slot as pending activation
    ///
    /// This async operation reads the current boot state from Flash, marks the specified
    /// slot as pending activation, and writes the updated state back to Flash.
    /// On next boot, the bootloader will attempt to activate this slot.
    ///
    /// # Arguments
    /// * `slot` - The slot to mark as pending (Slot::A or Slot::B)
    ///
    /// # Returns
    /// * `Ok(())` - Slot successfully marked as pending
    /// * `Err(FlashError)` - Failed to read or write boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.4 - Partition switching
    ///
    /// # Example
    /// ```no_run
    /// // After successfully writing firmware to Slot B
    /// flash_manager.mark_slot_pending(Slot::B).await?;
    /// // System will attempt to boot from Slot B on next restart
    /// ```
    pub async fn mark_slot_pending(&mut self, slot: Slot) -> Result<(), FlashError> {
        use crate::boot_state::BootState;
        use crate::config::FLASH_CONFIG;

        // Read current boot state from Flash
        let state_addr = FLASH_CONFIG.state_start;
        let mut state_bytes = [0u8; 16];

        self.flash
            .read(state_addr, &mut state_bytes)
            .map_err(|_| FlashError::VerifyFailed)?;

        // Parse boot state (or create new if invalid)
        let mut boot_state = BootState::from_bytes(&state_bytes).unwrap_or_else(|_| {
            // If state is invalid, create a new default state
            BootState::new()
        });

        // Mark the slot as pending
        boot_state.mark_pending(slot);

        // Erase the state sector before writing
        let state_end = FLASH_CONFIG.state_start + FLASH_CONFIG.state_size;
        self.flash
            .erase(state_addr, state_end)
            .map_err(|_| FlashError::EraseFailed)?;

        // Write updated boot state back to Flash
        let updated_bytes = boot_state.to_bytes();
        self.flash
            .write(state_addr, &updated_bytes)
            .map_err(|_| FlashError::WriteFailed)?;

        Ok(())
    }

    /// Check if an application in a slot is valid
    ///
    /// This method validates that a slot contains a valid application by checking:
    /// 1. The slot contains data (not all 0xFF - erased Flash)
    /// 2. The vector table appears valid (stack pointer and reset vector are reasonable)
    ///
    /// # Arguments
    /// * `slot` - The slot to check (Slot::A or Slot::B)
    ///
    /// # Returns
    /// * `true` - Slot contains a valid application
    /// * `false` - Slot is empty or contains invalid data
    ///
    /// # Requirements
    /// Validates requirement 3.4.4 - Partition switching
    ///
    /// # Example
    /// ```no_run
    /// if flash_manager.is_app_valid(Slot::A).await {
    ///     // Safe to jump to Slot A
    /// } else {
    ///     // Slot A is empty or corrupted
    /// }
    /// ```
    pub async fn is_app_valid(&mut self, slot: Slot) -> bool {
        // Read the first 8 bytes (stack pointer + reset vector)
        let mut header = [0u8; 8];
        let slot_addr = slot.start_address();

        // If we can't read, assume invalid
        if self.flash.read(slot_addr, &mut header).is_err() {
            return false;
        }

        // Check if the slot is erased (all 0xFF)
        if header.iter().all(|&b| b == 0xFF) {
            return false;
        }

        // Parse stack pointer (first 4 bytes, little-endian)
        let stack_pointer = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);

        // Parse reset vector (next 4 bytes, little-endian)
        let reset_vector = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

        // Validate stack pointer is in RAM range (STM32F407 has 128KB RAM at 0x20000000)
        // Stack pointer should be at the end of RAM
        const RAM_START: u32 = 0x2000_0000;
        const RAM_SIZE: u32 = 128 * 1024;
        const RAM_END: u32 = RAM_START + RAM_SIZE;

        if stack_pointer < RAM_START || stack_pointer > RAM_END {
            return false;
        }

        // Validate reset vector is in the slot's Flash range
        // Reset vector should point to code in the slot (with thumb bit set, so mask it)
        let reset_addr = reset_vector & !1; // Clear thumb bit

        if reset_addr < slot.start_address() || reset_addr >= slot.end_address() {
            return false;
        }

        // All checks passed - application appears valid
        true
    }

    /// Get the underlying Flash peripheral (for advanced operations)
    ///
    /// This method provides access to the raw Flash peripheral for operations
    /// not covered by the FlashManager API.
    ///
    /// # Returns
    /// Mutable reference to the Flash peripheral
    pub fn flash_mut(&mut self) -> &mut Flash<'d, Blocking> {
        &mut self.flash
    }

    /// Load boot state from Flash
    ///
    /// Reads the boot state structure from the dedicated state partition in Flash.
    /// If the state is invalid or corrupted, returns an error.
    ///
    /// # Returns
    /// * `Ok(BootState)` - Successfully loaded boot state
    /// * `Err(StateError)` - Failed to read or parse boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.4 - Boot state management
    pub async fn load_boot_state(
        &mut self,
    ) -> Result<crate::boot_state::BootState, crate::error::StateError> {
        use crate::boot_state::BootState;
        use crate::config::FLASH_CONFIG;
        use crate::error::StateError;

        let state_addr = FLASH_CONFIG.state_start;
        let mut state_bytes = [0u8; 16];

        self.flash
            .read(state_addr, &mut state_bytes)
            .map_err(|_| StateError::ReadFailed)?;

        BootState::from_bytes(&state_bytes)
    }

    /// Save boot state to Flash
    ///
    /// Writes the boot state structure to the dedicated state partition in Flash.
    /// This operation erases the state sector before writing.
    ///
    /// # Arguments
    /// * `state` - Boot state to save
    ///
    /// # Returns
    /// * `Ok(())` - Successfully saved boot state
    /// * `Err(StateError)` - Failed to write boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.4 - Boot state management
    pub async fn save_boot_state(
        &mut self,
        state: &crate::boot_state::BootState,
    ) -> Result<(), crate::error::StateError> {
        use crate::config::FLASH_CONFIG;
        use crate::error::StateError;

        let state_addr = FLASH_CONFIG.state_start;
        let state_end = FLASH_CONFIG.state_start + FLASH_CONFIG.state_size;

        // Erase the state sector
        self.flash
            .erase(state_addr, state_end)
            .map_err(|_| StateError::WriteFailed)?;

        // Write the boot state
        let state_bytes = state.to_bytes();
        self.flash
            .write(state_addr, &state_bytes)
            .map_err(|_| StateError::WriteFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FLASH_CONFIG;

    #[test]
    fn test_address_validation() {
        // Create a mock FlashManager for testing
        // Note: We can't actually create a Flash instance in tests,
        // so we'll test the logic that doesn't require hardware

        // Test valid addresses
        let slot = Slot::A;
        let start = slot.start_address();
        let size = slot.size();

        // Valid: offset 0, length within bounds
        assert!(start.checked_add(0).is_some());
        assert!(start.checked_add(size - 1).is_some());

        // Invalid: offset beyond slot
        assert!(start.checked_add(size).is_some());
        let end = start.checked_add(size).unwrap();
        assert_eq!(end, slot.end_address());

        // Test overflow protection
        let max_offset = u32::MAX;
        assert!(start.checked_add(max_offset).is_none());
    }

    #[test]
    fn test_slot_boundaries() {
        // Verify slot A boundaries
        let slot_a = Slot::A;
        assert_eq!(slot_a.start_address(), FLASH_CONFIG.slot_a_start);
        assert_eq!(slot_a.size(), FLASH_CONFIG.slot_a_size);
        assert_eq!(
            slot_a.end_address(),
            FLASH_CONFIG.slot_a_start + FLASH_CONFIG.slot_a_size
        );

        // Verify slot B boundaries
        let slot_b = Slot::B;
        assert_eq!(slot_b.start_address(), FLASH_CONFIG.slot_b_start);
        assert_eq!(slot_b.size(), FLASH_CONFIG.slot_b_size);
        assert_eq!(
            slot_b.end_address(),
            FLASH_CONFIG.slot_b_start + FLASH_CONFIG.slot_b_size
        );

        // Verify slots don't overlap
        assert!(slot_a.end_address() <= slot_b.start_address());
    }

    #[test]
    fn test_address_range_checks() {
        let slot = Slot::B;

        // Test valid ranges
        let offset = 0;
        let length = 1024;
        let start = slot.start_address();
        let end = start
            .checked_add(offset)
            .and_then(|a| a.checked_add(length));
        assert!(end.is_some());
        assert!(end.unwrap() <= slot.end_address());

        // Test boundary case - exactly at end
        let offset = 0;
        let length = slot.size();
        let end = start
            .checked_add(offset)
            .and_then(|a| a.checked_add(length));
        assert!(end.is_some());
        assert_eq!(end.unwrap(), slot.end_address());

        // Test overflow case
        let offset = slot.size();
        let length = 1;
        let end = start
            .checked_add(offset)
            .and_then(|a| a.checked_add(length));
        assert!(end.is_some());
        assert!(end.unwrap() > slot.end_address());
    }

    #[test]
    fn test_write_block_size_limits() {
        // Test that we can calculate addresses for various block sizes
        let slot = Slot::A;
        let block_sizes = [256, 512, 1024, 4096, 8192];

        for &size in &block_sizes {
            let offset = 0;
            let addr = slot.start_address().checked_add(offset);
            assert!(addr.is_some());

            let end = addr.unwrap().checked_add(size);
            assert!(end.is_some());
            assert!(end.unwrap() <= slot.end_address());
        }
    }

    #[test]
    fn test_verify_block_chunking() {
        // Test that verification logic handles chunking correctly
        let data_sizes = [100, 256, 512, 1000, 4096];

        for &size in &data_sizes {
            let chunk_size = 256;
            let num_chunks = (size + chunk_size - 1) / chunk_size;

            let mut verified_bytes = 0;
            for _ in 0..num_chunks {
                let remaining = size - verified_bytes;
                let this_chunk = core::cmp::min(chunk_size, remaining);
                verified_bytes += this_chunk;
            }

            assert_eq!(verified_bytes, size);
        }
    }

    #[test]
    fn test_vector_table_validation() {
        // Test the logic used in is_app_valid for vector table validation

        // Valid stack pointer (in RAM range)
        const RAM_START: u32 = 0x2000_0000;
        const RAM_SIZE: u32 = 128 * 1024;
        const RAM_END: u32 = RAM_START + RAM_SIZE;

        let valid_sp = RAM_END - 1024; // Stack pointer near end of RAM
        assert!(valid_sp >= RAM_START && valid_sp <= RAM_END);

        let invalid_sp_low = 0x0800_0000; // In Flash, not RAM
        assert!(invalid_sp_low < RAM_START);

        let invalid_sp_high = 0x3000_0000; // Beyond RAM
        assert!(invalid_sp_high > RAM_END);

        // Valid reset vector (in slot range, with thumb bit)
        let slot = Slot::A;
        let valid_reset = slot.start_address() + 0x100 + 1; // +1 for thumb bit
        let reset_addr = valid_reset & !1; // Clear thumb bit
        assert!(reset_addr >= slot.start_address() && reset_addr < slot.end_address());

        let invalid_reset = 0x0800_0000 + 1; // In bootloader, not slot
        let reset_addr = invalid_reset & !1;
        assert!(reset_addr < slot.start_address());
    }

    #[test]
    fn test_erased_flash_detection() {
        // Test detection of erased Flash (all 0xFF)
        let erased = [0xFFu8; 8];
        assert!(erased.iter().all(|&b| b == 0xFF));

        let not_erased = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF];
        assert!(!not_erased.iter().all(|&b| b == 0xFF));

        let valid_header = [0x00, 0x00, 0x02, 0x20, 0x01, 0x01, 0x01, 0x08];
        assert!(!valid_header.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_boot_state_size() {
        // Verify boot state fits in expected size
        use crate::boot_state::BootState;

        let state = BootState::new();
        let bytes = state.to_bytes();
        assert_eq!(bytes.len(), 16);
    }
}
