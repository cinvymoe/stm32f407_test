//! Rollback mechanism for failed firmware updates
//!
//! This module implements the rollback logic that allows the bootloader to
//! revert to a previous working firmware version if the new firmware fails
//! to boot successfully.
//!
//! Requirements:
//! - 3.4.5: Rollback mechanism with counter management
//! - 2.2.3: Upgrade failure rollback

use crate::boot_state::BootState;
use crate::config::{Slot, MAX_ROLLBACK_ATTEMPTS};
use crate::error::StateError;
use crate::flash_manager::FlashManager;
use defmt;

/// Rollback manager for handling firmware rollback
pub struct RollbackManager;

impl RollbackManager {
    /// Check if rollback is needed and perform it if necessary
    ///
    /// This function checks the boot state to determine if:
    /// 1. There is a pending firmware update that failed to boot
    /// 2. The rollback counter has reached the maximum attempts
    ///
    /// If rollback is needed, it switches back to the previous slot and
    /// resets the rollback counter.
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading/writing boot state
    ///
    /// # Returns
    /// * `Ok(Some(slot))` - Rollback performed, boot from this slot
    /// * `Ok(None)` - No rollback needed
    /// * `Err(StateError)` - Failed to read/write boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.5 - Rollback detection and execution
    pub async fn check_and_rollback(
        flash: &mut FlashManager<'_>,
    ) -> Result<Option<Slot>, StateError> {
        defmt::info!("Checking if rollback is needed...");

        // Load current boot state
        let mut state = flash.load_boot_state().await?;

        // Check if rollback is needed
        if state.should_rollback() {
            defmt::warn!(
                "Rollback required! Counter: {}/{}",
                state.rollback_count,
                MAX_ROLLBACK_ATTEMPTS
            );

            // Get the current active slot before rollback
            let old_slot = state.get_active_slot().ok_or(StateError::InvalidSlot)?;

            // Perform rollback
            state.rollback();

            let new_slot = state.get_active_slot().ok_or(StateError::InvalidSlot)?;

            defmt::info!("Rolling back from {:?} to {:?}", old_slot, new_slot);

            // Save the updated state
            flash.save_boot_state(&state).await?;

            defmt::info!("Rollback completed successfully");
            Ok(Some(new_slot))
        } else {
            defmt::info!("No rollback needed");
            Ok(None)
        }
    }

    /// Increment the rollback counter
    ///
    /// This should be called when a firmware boot attempt fails.
    /// After MAX_ROLLBACK_ATTEMPTS failures, the system will automatically
    /// rollback to the previous firmware.
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading/writing boot state
    ///
    /// # Returns
    /// * `Ok(())` - Counter incremented successfully
    /// * `Err(StateError)` - Failed to read/write boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.5 - Rollback counter management
    pub async fn increment_rollback_counter(
        flash: &mut FlashManager<'_>,
    ) -> Result<(), StateError> {
        defmt::info!("Incrementing rollback counter...");

        // Load current boot state
        let mut state = flash.load_boot_state().await?;

        // Increment counter
        let old_count = state.rollback_count;
        state.increment_rollback();
        let new_count = state.rollback_count;

        defmt::info!(
            "Rollback counter: {} -> {} (max: {})",
            old_count,
            new_count,
            MAX_ROLLBACK_ATTEMPTS
        );

        // Save updated state
        flash.save_boot_state(&state).await?;

        if state.should_rollback() {
            defmt::warn!("Rollback threshold reached! System will rollback on next boot");
        }

        Ok(())
    }

    /// Reset the rollback counter to zero
    ///
    /// This should be called by the application after successful boot to
    /// confirm that the firmware is working correctly.
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading/writing boot state
    ///
    /// # Returns
    /// * `Ok(())` - Counter reset successfully
    /// * `Err(StateError)` - Failed to read/write boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.5 - Rollback counter reset
    pub async fn reset_rollback_counter(flash: &mut FlashManager<'_>) -> Result<(), StateError> {
        defmt::info!("Resetting rollback counter...");

        // Load current boot state
        let mut state = flash.load_boot_state().await?;

        // Reset counter
        state.reset_rollback();

        defmt::info!("Rollback counter reset to 0");

        // Save updated state
        flash.save_boot_state(&state).await?;

        Ok(())
    }

    /// Confirm the current firmware as stable
    ///
    /// This marks the current slot as confirmed and resets the rollback counter.
    /// Should be called by the application after successful boot and validation.
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading/writing boot state
    /// * `slot` - The slot to confirm as stable
    ///
    /// # Returns
    /// * `Ok(())` - Firmware confirmed successfully
    /// * `Err(StateError)` - Failed to read/write boot state
    ///
    /// # Requirements
    /// Validates requirement 3.4.5 - Firmware confirmation
    pub async fn confirm_firmware(
        flash: &mut FlashManager<'_>,
        slot: Slot,
    ) -> Result<(), StateError> {
        defmt::info!("Confirming firmware in {:?} as stable", slot);

        // Load current boot state
        let mut state = flash.load_boot_state().await?;

        // Confirm the slot
        state.confirm_active(slot);

        defmt::info!("Firmware confirmed, rollback counter reset");

        // Save updated state
        flash.save_boot_state(&state).await?;

        Ok(())
    }

    /// Get the current rollback counter value
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading boot state
    ///
    /// # Returns
    /// * `Ok(count)` - Current rollback counter value
    /// * `Err(StateError)` - Failed to read boot state
    pub async fn get_rollback_count(flash: &mut FlashManager<'_>) -> Result<u8, StateError> {
        let state = flash.load_boot_state().await?;
        Ok(state.rollback_count)
    }

    /// Check if the system should rollback (counter >= max attempts)
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading boot state
    ///
    /// # Returns
    /// * `Ok(true)` - System should rollback
    /// * `Ok(false)` - System should not rollback
    /// * `Err(StateError)` - Failed to read boot state
    pub async fn should_rollback(flash: &mut FlashManager<'_>) -> Result<bool, StateError> {
        let state = flash.load_boot_state().await?;
        Ok(state.should_rollback())
    }

    /// Perform a manual rollback to the other slot
    ///
    /// This can be used to force a rollback without waiting for boot failures.
    ///
    /// # Arguments
    /// * `flash` - Flash manager for reading/writing boot state
    ///
    /// # Returns
    /// * `Ok(slot)` - Rolled back to this slot
    /// * `Err(StateError)` - Failed to perform rollback
    pub async fn force_rollback(flash: &mut FlashManager<'_>) -> Result<Slot, StateError> {
        defmt::warn!("Forcing manual rollback...");

        // Load current boot state
        let mut state = flash.load_boot_state().await?;

        let old_slot = state.get_active_slot().ok_or(StateError::InvalidSlot)?;

        // Perform rollback
        state.rollback();

        let new_slot = state.get_active_slot().ok_or(StateError::InvalidSlot)?;

        defmt::info!("Manual rollback from {:?} to {:?}", old_slot, new_slot);

        // Save updated state
        flash.save_boot_state(&state).await?;

        Ok(new_slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_rollback_attempts() {
        // Verify the maximum rollback attempts constant
        assert_eq!(MAX_ROLLBACK_ATTEMPTS, 3);
        assert!(MAX_ROLLBACK_ATTEMPTS > 0);
        assert!(MAX_ROLLBACK_ATTEMPTS < 10); // Reasonable limit
    }

    #[test]
    fn test_rollback_threshold() {
        // Test that boot state correctly identifies rollback threshold
        let mut state = BootState::new();

        assert!(!state.should_rollback());

        state.increment_rollback();
        assert!(!state.should_rollback());

        state.increment_rollback();
        assert!(!state.should_rollback());

        state.increment_rollback();
        assert!(state.should_rollback());
    }

    #[test]
    fn test_rollback_counter_saturation() {
        // Test that counter doesn't overflow
        let mut state = BootState::new();

        for _ in 0..10 {
            state.increment_rollback();
        }

        // Should saturate at MAX_ROLLBACK_ATTEMPTS
        assert_eq!(state.rollback_count, MAX_ROLLBACK_ATTEMPTS);
    }

    #[test]
    fn test_rollback_slot_switching() {
        // Test that rollback switches to the other slot
        let mut state = BootState::with_active_slot(Slot::A);
        state.mark_pending(Slot::B);
        state.rollback_count = MAX_ROLLBACK_ATTEMPTS;

        assert_eq!(state.get_active_slot(), Some(Slot::A));

        state.rollback();

        // After rollback, should be on the "other" slot
        assert_eq!(state.get_active_slot(), Some(Slot::B));
        assert_eq!(state.get_pending_slot(), None);
        assert_eq!(state.rollback_count, 0);
    }

    #[test]
    fn test_confirm_firmware_resets_counter() {
        // Test that confirming firmware resets the rollback counter
        let mut state = BootState::new();
        state.rollback_count = 2;
        state.mark_pending(Slot::B);

        state.confirm_active(Slot::B);

        assert_eq!(state.get_active_slot(), Some(Slot::B));
        assert_eq!(state.get_pending_slot(), None);
        assert_eq!(state.rollback_count, 0);
    }

    #[test]
    fn test_rollback_clears_pending() {
        // Test that rollback clears the pending slot
        let mut state = BootState::with_active_slot(Slot::A);
        state.mark_pending(Slot::B);

        assert_eq!(state.get_pending_slot(), Some(Slot::B));

        state.rollback();

        assert_eq!(state.get_pending_slot(), None);
    }

    #[test]
    fn test_multiple_rollback_cycles() {
        // Test multiple rollback cycles
        let mut state = BootState::with_active_slot(Slot::A);

        // First cycle: A -> B (fail) -> A
        state.mark_pending(Slot::B);
        state.rollback_count = MAX_ROLLBACK_ATTEMPTS;
        state.rollback();
        assert_eq!(state.get_active_slot(), Some(Slot::B));

        // Second cycle: B -> A (fail) -> B
        state.mark_pending(Slot::A);
        state.rollback_count = MAX_ROLLBACK_ATTEMPTS;
        state.rollback();
        assert_eq!(state.get_active_slot(), Some(Slot::A));
    }

    #[test]
    fn test_rollback_without_pending() {
        // Test rollback when there's no pending slot
        let mut state = BootState::with_active_slot(Slot::A);
        state.rollback_count = MAX_ROLLBACK_ATTEMPTS;

        assert_eq!(state.get_pending_slot(), None);

        state.rollback();

        // Should still switch to other slot
        assert_eq!(state.get_active_slot(), Some(Slot::B));
        assert_eq!(state.rollback_count, 0);
    }
}
