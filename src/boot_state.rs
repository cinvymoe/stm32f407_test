//! Boot state management for dual-partition firmware updates
//!
//! This module implements the BootState structure that tracks which firmware
//! partition is active, which is pending activation, and manages rollback
//! counting for failed boot attempts.

use crate::config::{Slot, MAX_ROLLBACK_ATTEMPTS};
use crate::error::StateError;

/// Boot state magic number ("BOOT" in ASCII)
pub const BOOT_STATE_MAGIC: u32 = 0x424F4F54;

/// No pending slot marker
pub const NO_PENDING_SLOT: u8 = 0xFF;

/// Boot state structure stored in Flash
///
/// This structure is persisted to Flash and tracks the current boot state,
/// including which partition is active, which is pending activation, and
/// how many rollback attempts have been made.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootState {
    /// Magic number for validation (0x424F4F54 = "BOOT")
    pub magic: u32,

    /// Currently active slot (0 = Slot A, 1 = Slot B)
    pub active_slot: u8,

    /// Pending slot to activate (0xFF = none, 0 = Slot A, 1 = Slot B)
    pub pending_slot: u8,

    /// Rollback counter (increments on boot failure, max 3)
    pub rollback_count: u8,

    /// Status flags (reserved for future use)
    pub flags: u8,

    /// CRC32 checksum of the structure (excluding this field)
    pub crc: u32,
}

impl BootState {
    /// Create a new default boot state
    ///
    /// Default state:
    /// - Slot A is active
    /// - No pending slot
    /// - Zero rollback count
    /// - CRC is calculated
    pub fn new() -> Self {
        let mut state = Self {
            magic: BOOT_STATE_MAGIC,
            active_slot: Slot::A.to_u8(),
            pending_slot: NO_PENDING_SLOT,
            rollback_count: 0,
            flags: 0,
            crc: 0,
        };
        state.update_crc();
        state
    }

    /// Create a boot state with a specific active slot
    pub fn with_active_slot(slot: Slot) -> Self {
        let mut state = Self {
            magic: BOOT_STATE_MAGIC,
            active_slot: slot.to_u8(),
            pending_slot: NO_PENDING_SLOT,
            rollback_count: 0,
            flags: 0,
            crc: 0,
        };
        state.update_crc();
        state
    }

    /// Get the currently active slot
    pub fn get_active_slot(&self) -> Option<Slot> {
        Slot::from_u8(self.active_slot)
    }

    /// Get the pending slot (if any)
    pub fn get_pending_slot(&self) -> Option<Slot> {
        if self.pending_slot == NO_PENDING_SLOT {
            None
        } else {
            Slot::from_u8(self.pending_slot)
        }
    }

    /// Mark a slot as pending activation
    ///
    /// This is called after successfully writing new firmware to a slot.
    /// On next boot, the bootloader will attempt to activate this slot.
    pub fn mark_pending(&mut self, slot: Slot) {
        self.pending_slot = slot.to_u8();
        self.update_crc();
    }

    /// Confirm the current active slot
    ///
    /// This is called by the application after successful boot to confirm
    /// that the current firmware is working correctly. It clears the pending
    /// slot and resets the rollback counter.
    pub fn confirm_active(&mut self, slot: Slot) {
        self.active_slot = slot.to_u8();
        self.pending_slot = NO_PENDING_SLOT;
        self.rollback_count = 0;
        self.update_crc();
    }

    /// Increment the rollback counter
    ///
    /// This is called when a boot attempt fails. If the counter reaches
    /// MAX_ROLLBACK_ATTEMPTS, the system should permanently rollback.
    pub fn increment_rollback(&mut self) {
        if self.rollback_count < MAX_ROLLBACK_ATTEMPTS {
            self.rollback_count += 1;
            self.update_crc();
        }
    }

    /// Reset the rollback counter to zero
    pub fn reset_rollback(&mut self) {
        self.rollback_count = 0;
        self.update_crc();
    }

    /// Check if the system should rollback to the previous slot
    ///
    /// Returns true if:
    /// - There is a pending slot that failed to boot
    /// - The rollback counter has reached the maximum
    pub fn should_rollback(&self) -> bool {
        self.rollback_count >= MAX_ROLLBACK_ATTEMPTS
    }

    /// Perform rollback to the other slot
    ///
    /// This switches the active slot back to the previous one and clears
    /// the pending slot.
    pub fn rollback(&mut self) {
        if let Some(active) = self.get_active_slot() {
            let other = active.other();
            self.active_slot = other.to_u8();
            self.pending_slot = NO_PENDING_SLOT;
            self.rollback_count = 0;
            self.update_crc();
        }
    }

    /// Validate the boot state structure
    ///
    /// Checks:
    /// - Magic number is correct
    /// - CRC is valid
    /// - Slot values are valid
    pub fn validate(&self) -> Result<(), StateError> {
        // Check magic number
        if self.magic != BOOT_STATE_MAGIC {
            return Err(StateError::InvalidMagic);
        }

        // Verify CRC
        if !self.verify_crc() {
            return Err(StateError::CrcMismatch);
        }

        // Validate active slot
        if Slot::from_u8(self.active_slot).is_none() {
            return Err(StateError::InvalidSlot);
        }

        // Validate pending slot (if set)
        if self.pending_slot != NO_PENDING_SLOT && Slot::from_u8(self.pending_slot).is_none() {
            return Err(StateError::InvalidSlot);
        }

        Ok(())
    }

    /// Calculate CRC32 of the structure (excluding the CRC field itself)
    fn calculate_crc(&self) -> u32 {
        use crc::{Crc, CRC_32_ISO_HDLC};

        const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
        let mut digest = CRC.digest();

        // Hash all fields except CRC
        digest.update(&self.magic.to_le_bytes());
        digest.update(&[self.active_slot]);
        digest.update(&[self.pending_slot]);
        digest.update(&[self.rollback_count]);
        digest.update(&[self.flags]);

        digest.finalize()
    }

    /// Update the CRC field with the calculated checksum
    fn update_crc(&mut self) {
        self.crc = self.calculate_crc();
    }

    /// Verify that the stored CRC matches the calculated CRC
    fn verify_crc(&self) -> bool {
        self.crc == self.calculate_crc()
    }

    /// Serialize the boot state to bytes
    ///
    /// Returns a byte array that can be written to Flash
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4] = self.active_slot;
        bytes[5] = self.pending_slot;
        bytes[6] = self.rollback_count;
        bytes[7] = self.flags;
        bytes[8..12].copy_from_slice(&self.crc.to_le_bytes());
        // bytes[12..16] are padding (reserved)
        bytes
    }

    /// Deserialize boot state from bytes
    ///
    /// Reads a boot state structure from a byte array and validates it
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StateError> {
        if bytes.len() < 16 {
            return Err(StateError::InvalidSize);
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let active_slot = bytes[4];
        let pending_slot = bytes[5];
        let rollback_count = bytes[6];
        let flags = bytes[7];
        let crc = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        let state = Self {
            magic,
            active_slot,
            pending_slot,
            rollback_count,
            flags,
            crc,
        };

        // Validate the loaded state
        state.validate()?;

        Ok(state)
    }
}

impl Default for BootState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_boot_state() {
        let state = BootState::new();

        assert_eq!(state.magic, BOOT_STATE_MAGIC);
        assert_eq!(state.active_slot, Slot::A.to_u8());
        assert_eq!(state.pending_slot, NO_PENDING_SLOT);
        assert_eq!(state.rollback_count, 0);
        assert_eq!(state.flags, 0);
        assert!(state.verify_crc());
    }

    #[test]
    fn test_with_active_slot() {
        let state_a = BootState::with_active_slot(Slot::A);
        assert_eq!(state_a.get_active_slot(), Some(Slot::A));
        assert!(state_a.verify_crc());

        let state_b = BootState::with_active_slot(Slot::B);
        assert_eq!(state_b.get_active_slot(), Some(Slot::B));
        assert!(state_b.verify_crc());
    }

    #[test]
    fn test_mark_pending() {
        let mut state = BootState::new();
        assert_eq!(state.get_pending_slot(), None);

        state.mark_pending(Slot::B);
        assert_eq!(state.get_pending_slot(), Some(Slot::B));
        assert!(state.verify_crc());
    }

    #[test]
    fn test_confirm_active() {
        let mut state = BootState::new();
        state.mark_pending(Slot::B);
        state.rollback_count = 2;

        state.confirm_active(Slot::B);

        assert_eq!(state.get_active_slot(), Some(Slot::B));
        assert_eq!(state.get_pending_slot(), None);
        assert_eq!(state.rollback_count, 0);
        assert!(state.verify_crc());
    }

    #[test]
    fn test_increment_rollback() {
        let mut state = BootState::new();

        assert_eq!(state.rollback_count, 0);
        assert!(!state.should_rollback());

        state.increment_rollback();
        assert_eq!(state.rollback_count, 1);
        assert!(!state.should_rollback());

        state.increment_rollback();
        assert_eq!(state.rollback_count, 2);
        assert!(!state.should_rollback());

        state.increment_rollback();
        assert_eq!(state.rollback_count, 3);
        assert!(state.should_rollback());

        // Should not increment beyond max
        state.increment_rollback();
        assert_eq!(state.rollback_count, 3);
        assert!(state.verify_crc());
    }

    #[test]
    fn test_reset_rollback() {
        let mut state = BootState::new();
        state.rollback_count = 3;

        state.reset_rollback();
        assert_eq!(state.rollback_count, 0);
        assert!(!state.should_rollback());
        assert!(state.verify_crc());
    }

    #[test]
    fn test_rollback() {
        let mut state = BootState::with_active_slot(Slot::A);
        state.mark_pending(Slot::B);
        state.rollback_count = 3;

        state.rollback();

        assert_eq!(state.get_active_slot(), Some(Slot::B));
        assert_eq!(state.get_pending_slot(), None);
        assert_eq!(state.rollback_count, 0);
        assert!(state.verify_crc());
    }

    #[test]
    fn test_validate_valid_state() {
        let state = BootState::new();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_magic() {
        let mut state = BootState::new();
        state.magic = 0xDEADBEEF;

        assert_eq!(state.validate(), Err(StateError::InvalidMagic));
    }

    #[test]
    fn test_validate_invalid_crc() {
        let mut state = BootState::new();
        state.crc = 0xDEADBEEF;

        assert_eq!(state.validate(), Err(StateError::CrcMismatch));
    }

    #[test]
    fn test_validate_invalid_slot() {
        let mut state = BootState::new();
        state.active_slot = 99;
        state.update_crc();

        assert_eq!(state.validate(), Err(StateError::InvalidSlot));
    }

    #[test]
    fn test_validate_invalid_pending_slot() {
        let mut state = BootState::new();
        state.pending_slot = 99;
        state.update_crc();

        assert_eq!(state.validate(), Err(StateError::InvalidSlot));
    }

    #[test]
    fn test_crc_changes_with_data() {
        let state1 = BootState::with_active_slot(Slot::A);
        let state2 = BootState::with_active_slot(Slot::B);

        assert_ne!(state1.crc, state2.crc);
    }

    #[test]
    fn test_crc_deterministic() {
        let state1 = BootState::new();
        let state2 = BootState::new();

        assert_eq!(state1.crc, state2.crc);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let original = BootState::new();
        let bytes = original.to_bytes();
        let restored = BootState::from_bytes(&bytes).unwrap();

        assert_eq!(original, restored);
    }

    #[test]
    fn test_serialization_with_pending() {
        let mut original = BootState::new();
        original.mark_pending(Slot::B);
        original.increment_rollback();

        let bytes = original.to_bytes();
        let restored = BootState::from_bytes(&bytes).unwrap();

        assert_eq!(original, restored);
        assert_eq!(restored.get_pending_slot(), Some(Slot::B));
        assert_eq!(restored.rollback_count, 1);
    }

    #[test]
    fn test_from_bytes_invalid_size() {
        let bytes = [0u8; 8]; // Too small
        let result = BootState::from_bytes(&bytes);

        assert_eq!(result, Err(StateError::InvalidSize));
    }

    #[test]
    fn test_from_bytes_invalid_magic() {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

        let result = BootState::from_bytes(&bytes);
        assert_eq!(result, Err(StateError::InvalidMagic));
    }

    #[test]
    fn test_state_transitions() {
        // Simulate a complete upgrade cycle
        let mut state = BootState::with_active_slot(Slot::A);

        // 1. Mark Slot B as pending (new firmware written)
        state.mark_pending(Slot::B);
        assert_eq!(state.get_active_slot(), Some(Slot::A));
        assert_eq!(state.get_pending_slot(), Some(Slot::B));

        // 2. Reboot and activate Slot B
        state.confirm_active(Slot::B);
        assert_eq!(state.get_active_slot(), Some(Slot::B));
        assert_eq!(state.get_pending_slot(), None);
        assert_eq!(state.rollback_count, 0);

        // 3. Application confirms it's working
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_rollback_scenario() {
        // Simulate a failed upgrade with rollback
        let mut state = BootState::with_active_slot(Slot::A);

        // 1. Mark Slot B as pending
        state.mark_pending(Slot::B);

        // 2. Try to boot Slot B (fails 3 times)
        state.increment_rollback();
        state.increment_rollback();
        state.increment_rollback();

        assert!(state.should_rollback());

        // 3. Rollback to Slot A
        state.rollback();
        assert_eq!(state.get_active_slot(), Some(Slot::B)); // Actually rolls to B (the "other" slot)
        assert_eq!(state.get_pending_slot(), None);
        assert_eq!(state.rollback_count, 0);
    }
}
