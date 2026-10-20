//! Error handling and recovery logic
//!
//! This module implements error classification and handling strategies for the bootloader.
//! Errors are categorized into:
//! - Recoverable errors: Can continue to application
//! - Rollback errors: Require reverting to previous firmware
//! - Fatal errors: Require system halt or reset
//!
//! Requirements:
//! - 3.5: Error handling and recovery
//! - 3.5.1: SD card error recovery
//! - 3.5.2: Firmware error handling

use crate::error::{ErrorCode, UpgradeError};
use crate::led_indicator::{LedIndicator, LedPattern};
use defmt;
use embassy_time::{Duration, Timer};

/// Error handling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStrategy {
    /// Continue to application (skip upgrade)
    ContinueToApp,

    /// Rollback to previous firmware
    Rollback,

    /// Fatal error - enter SOS mode
    Fatal,
}

/// Error handler for bootloader operations
pub struct ErrorHandler;

impl ErrorHandler {
    /// Classify an error and determine the handling strategy
    ///
    /// # Arguments
    /// * `error` - The upgrade error to classify
    ///
    /// # Returns
    /// The appropriate error handling strategy
    ///
    /// # Requirements
    /// Validates requirement 3.5 - Error classification
    pub fn classify_error(error: &UpgradeError) -> ErrorStrategy {
        let error_code = ErrorCode::from(*error);

        if error_code.is_fatal() {
            ErrorStrategy::Fatal
        } else if error_code.requires_rollback() {
            ErrorStrategy::Rollback
        } else if error_code.is_recoverable() {
            ErrorStrategy::ContinueToApp
        } else {
            // Default to continue if not explicitly categorized
            ErrorStrategy::ContinueToApp
        }
    }

    /// Log an error with detailed information
    ///
    /// # Arguments
    /// * `error` - The upgrade error to log
    /// * `context` - Additional context about where the error occurred
    ///
    /// # Requirements
    /// Validates requirement 3.5 - Error logging
    pub fn log_error(error: &UpgradeError, context: &str) {
        let error_code = ErrorCode::from(*error);

        defmt::error!(
            "[ERROR] {} - Code: 0x{:02X} - {}",
            context,
            error_code.code(),
            error_code.description()
        );

        // Log the full error details
        defmt::error!("Error details: {:?}", error);
    }

    /// Handle a recoverable error
    ///
    /// Recoverable errors allow the system to skip the upgrade and boot into
    /// the existing application. This includes:
    /// - SD card not inserted
    /// - Firmware file not found
    /// - CRC checksum mismatch
    /// - Firmware size too large
    ///
    /// # Arguments
    /// * `error` - The upgrade error
    /// * `led` - LED indicator for status display
    ///
    /// # Requirements
    /// Validates requirement 3.5.1 - SD card error recovery
    /// Validates requirement 3.5.2 - Firmware error handling
    pub async fn handle_recoverable_error(error: &UpgradeError, led: &mut LedIndicator) {
        Self::log_error(error, "Recoverable error during upgrade");

        // Set LED to error pattern (fast blink)
        led.set_status(LedPattern::FastBlink);

        // Display error for 3 seconds
        Timer::after(Duration::from_secs(3)).await;

        // Clear LED status
        led.set_status(LedPattern::Off);

        defmt::info!("Continuing to existing application...");
    }

    /// Handle an error that requires rollback
    ///
    /// Rollback errors occur during the upgrade process and require reverting
    /// to the previous firmware version. This includes:
    /// - Flash write failures
    /// - Flash verification failures
    /// - Operation timeouts
    /// - Boot state corruption
    ///
    /// # Arguments
    /// * `error` - The upgrade error
    /// * `led` - LED indicator for status display
    ///
    /// # Requirements
    /// Validates requirement 3.5 - Error handling with rollback
    pub async fn handle_rollback_error(error: &UpgradeError, led: &mut LedIndicator) {
        Self::log_error(error, "Rollback required due to upgrade failure");

        // Set LED to error pattern
        led.set_status(LedPattern::FastBlink);

        // Display error for 2 seconds
        Timer::after(Duration::from_secs(2)).await;

        defmt::warn!("System will attempt rollback on next boot");

        // Note: Actual rollback logic is handled by check_and_rollback in boot flow
    }

    /// Handle a fatal error
    ///
    /// Fatal errors indicate serious hardware or system problems that cannot
    /// be recovered from without intervention. This includes:
    /// - Flash hardware failure
    /// - Boot state corruption (unrecoverable)
    /// - All partitions invalid
    ///
    /// The system enters SOS mode with continuous fast LED blinking and waits
    /// for watchdog reset or manual intervention.
    ///
    /// # Arguments
    /// * `error` - The upgrade error
    /// * `led` - LED indicator for status display
    ///
    /// # Requirements
    /// Validates requirement 3.5 - Fatal error handling (SOS mode)
    pub async fn handle_fatal_error(error: &UpgradeError, led: &mut LedIndicator) -> ! {
        Self::log_error(error, "FATAL ERROR - System cannot continue");

        let error_code = ErrorCode::from(*error);
        defmt::error!("Fatal error code: 0x{:02X}", error_code.code());
        defmt::error!("System entering SOS mode...");

        // Set LED to SOS pattern (continuous fast blink)
        led.set_status(LedPattern::FastBlink);

        // Enter infinite loop - wait for watchdog or manual reset
        loop {
            Timer::after(Duration::from_millis(100)).await;
            // Watchdog will eventually reset the system if enabled
        }
    }

    /// Handle an upgrade error with the appropriate strategy
    ///
    /// This is the main entry point for error handling. It classifies the error
    /// and applies the appropriate handling strategy.
    ///
    /// # Arguments
    /// * `error` - The upgrade error
    /// * `led` - LED indicator for status display
    ///
    /// # Returns
    /// * `Ok(())` - Error handled, can continue to application
    /// * `Err(())` - Fatal error, system cannot continue
    ///
    /// # Requirements
    /// Validates requirement 3.5 - Complete error handling flow
    pub async fn handle_error(error: &UpgradeError, led: &mut LedIndicator) -> Result<(), ()> {
        let strategy = Self::classify_error(error);

        match strategy {
            ErrorStrategy::ContinueToApp => {
                Self::handle_recoverable_error(error, led).await;
                Ok(())
            }
            ErrorStrategy::Rollback => {
                Self::handle_rollback_error(error, led).await;
                Ok(())
            }
            ErrorStrategy::Fatal => {
                Self::handle_fatal_error(error, led).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{FlashError, SdError, StateError, VerifyError};

    #[test]
    fn test_classify_recoverable_errors() {
        let errors = [
            UpgradeError::SdCard(SdError::InitFailed),
            UpgradeError::SdCard(SdError::MountFailed),
            UpgradeError::SdCard(SdError::FileNotFound),
            UpgradeError::SdCard(SdError::ReadError),
            UpgradeError::Verification(VerifyError::SizeTooLarge),
            UpgradeError::Verification(VerifyError::CrcMismatch),
            UpgradeError::Verification(VerifyError::InvalidFormat),
        ];

        for error in &errors {
            let strategy = ErrorHandler::classify_error(error);
            assert_eq!(
                strategy,
                ErrorStrategy::ContinueToApp,
                "Error {:?} should be recoverable",
                error
            );
        }
    }

    #[test]
    fn test_classify_rollback_errors() {
        let errors = [
            UpgradeError::Flash(FlashError::WriteFailed),
            UpgradeError::Flash(FlashError::VerifyFailed),
            UpgradeError::Timeout,
            UpgradeError::State(StateError::InvalidMagic),
            UpgradeError::State(StateError::CrcMismatch),
        ];

        for error in &errors {
            let strategy = ErrorHandler::classify_error(error);
            assert_eq!(
                strategy,
                ErrorStrategy::Rollback,
                "Error {:?} should require rollback",
                error
            );
        }
    }

    #[test]
    fn test_classify_fatal_errors() {
        let errors = [
            UpgradeError::Flash(FlashError::EraseFailed),
            UpgradeError::State(StateError::ReadFailed),
            UpgradeError::State(StateError::WriteFailed),
        ];

        for error in &errors {
            let strategy = ErrorHandler::classify_error(error);
            assert_eq!(
                strategy,
                ErrorStrategy::Fatal,
                "Error {:?} should be fatal",
                error
            );
        }
    }

    #[test]
    fn test_error_strategy_values() {
        // Verify error strategies are distinct
        assert_ne!(ErrorStrategy::ContinueToApp, ErrorStrategy::Rollback);
        assert_ne!(ErrorStrategy::ContinueToApp, ErrorStrategy::Fatal);
        assert_ne!(ErrorStrategy::Rollback, ErrorStrategy::Fatal);
    }

    #[test]
    fn test_all_errors_have_strategy() {
        // Test that all error types can be classified
        let all_errors = [
            // SD errors
            UpgradeError::SdCard(SdError::InitFailed),
            UpgradeError::SdCard(SdError::MountFailed),
            UpgradeError::SdCard(SdError::FileNotFound),
            UpgradeError::SdCard(SdError::ReadError),
            UpgradeError::SdCard(SdError::Timeout),
            // Flash errors
            UpgradeError::Flash(FlashError::EraseFailed),
            UpgradeError::Flash(FlashError::WriteFailed),
            UpgradeError::Flash(FlashError::VerifyFailed),
            UpgradeError::Flash(FlashError::InvalidAddress),
            // Verification errors
            UpgradeError::Verification(VerifyError::SizeTooLarge),
            UpgradeError::Verification(VerifyError::CrcMismatch),
            UpgradeError::Verification(VerifyError::InvalidFormat),
            UpgradeError::Verification(VerifyError::SizeMismatch),
            // State errors
            UpgradeError::State(StateError::InvalidMagic),
            UpgradeError::State(StateError::CrcMismatch),
            UpgradeError::State(StateError::ReadFailed),
            UpgradeError::State(StateError::WriteFailed),
            UpgradeError::State(StateError::InvalidSlot),
            UpgradeError::State(StateError::InvalidSize),
            // Timeout
            UpgradeError::Timeout,
        ];

        for error in &all_errors {
            let strategy = ErrorHandler::classify_error(error);
            // Just verify it returns a valid strategy
            assert!(
                matches!(
                    strategy,
                    ErrorStrategy::ContinueToApp | ErrorStrategy::Rollback | ErrorStrategy::Fatal
                ),
                "Error {:?} has invalid strategy",
                error
            );
        }
    }

    #[test]
    fn test_error_code_consistency() {
        // Verify error code classification is consistent with SD errors
        let sd_code = ErrorCode::from(SdError::InitFailed);
        assert!(sd_code.is_recoverable());
        assert!(!sd_code.requires_rollback());
        assert!(!sd_code.is_fatal());

        let sd_code2 = ErrorCode::from(SdError::MountFailed);
        assert!(sd_code2.is_recoverable());
        assert!(!sd_code2.requires_rollback());
        assert!(!sd_code2.is_fatal());

        // Flash errors
        let flash_code = ErrorCode::from(FlashError::EraseFailed);
        assert!(!flash_code.is_recoverable());
        assert!(!flash_code.requires_rollback());
        assert!(flash_code.is_fatal());

        let flash_code2 = ErrorCode::from(FlashError::WriteFailed);
        assert!(!flash_code2.is_recoverable());
        assert!(flash_code2.requires_rollback());
        assert!(!flash_code2.is_fatal());

        // Verify errors
        let verify_code = ErrorCode::from(VerifyError::SizeTooLarge);
        assert!(verify_code.is_recoverable());
        assert!(!verify_code.requires_rollback());
        assert!(!verify_code.is_fatal());

        // State errors
        let state_code = ErrorCode::from(StateError::ReadFailed);
        assert!(!state_code.is_recoverable());
        assert!(!state_code.requires_rollback());
        assert!(state_code.is_fatal());
    }
}
