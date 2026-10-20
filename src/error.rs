//! Error types for the bootloader system
//!
//! This module defines all error types used throughout the bootloader,
//! including SD card errors, Flash errors, verification errors, and state errors.

use core::fmt;

/// Main upgrade error type that encompasses all possible errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum UpgradeError {
    /// SD card related error
    SdCard(SdError),

    /// Flash operation error
    Flash(FlashError),

    /// Firmware verification error
    Verification(VerifyError),

    /// Boot state error
    State(StateError),

    /// Operation timeout
    Timeout,
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SdCard(e) => write!(f, "SD card error: {}", e),
            Self::Flash(e) => write!(f, "Flash error: {}", e),
            Self::Verification(e) => write!(f, "Verification error: {}", e),
            Self::State(e) => write!(f, "State error: {}", e),
            Self::Timeout => write!(f, "Operation timeout"),
        }
    }
}

impl From<SdError> for UpgradeError {
    fn from(e: SdError) -> Self {
        Self::SdCard(e)
    }
}

impl From<FlashError> for UpgradeError {
    fn from(e: FlashError) -> Self {
        Self::Flash(e)
    }
}

impl From<VerifyError> for UpgradeError {
    fn from(e: VerifyError) -> Self {
        Self::Verification(e)
    }
}

impl From<StateError> for UpgradeError {
    fn from(e: StateError) -> Self {
        Self::State(e)
    }
}

/// SD card related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SdError {
    /// SD card initialization failed
    InitFailed,

    /// Failed to mount FAT32 filesystem
    MountFailed,

    /// Firmware file not found on SD card
    FileNotFound,

    /// Error reading from SD card
    ReadError,

    /// SD card operation timeout
    Timeout,
}

impl fmt::Display for SdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitFailed => write!(f, "SD card initialization failed"),
            Self::MountFailed => write!(f, "Failed to mount FAT32 filesystem"),
            Self::FileNotFound => write!(f, "Firmware file not found"),
            Self::ReadError => write!(f, "SD card read error"),
            Self::Timeout => write!(f, "SD card operation timeout"),
        }
    }
}

/// Flash operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum FlashError {
    /// Flash erase operation failed
    EraseFailed,

    /// Flash write operation failed
    WriteFailed,

    /// Flash verification failed (read-back mismatch)
    VerifyFailed,

    /// Invalid Flash address
    InvalidAddress,
}

impl fmt::Display for FlashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EraseFailed => write!(f, "Flash erase failed"),
            Self::WriteFailed => write!(f, "Flash write failed"),
            Self::VerifyFailed => write!(f, "Flash verification failed"),
            Self::InvalidAddress => write!(f, "Invalid Flash address"),
        }
    }
}

/// Firmware verification errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum VerifyError {
    /// Firmware size exceeds maximum allowed (512KB)
    SizeTooLarge,

    /// CRC32 checksum mismatch
    CrcMismatch,

    /// Invalid firmware format
    InvalidFormat,

    /// Size mismatch between expected and actual
    SizeMismatch,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeTooLarge => write!(f, "Firmware size exceeds 512KB limit"),
            Self::CrcMismatch => write!(f, "CRC32 checksum mismatch"),
            Self::InvalidFormat => write!(f, "Invalid firmware format"),
            Self::SizeMismatch => write!(f, "Firmware size mismatch"),
        }
    }
}

/// Boot state errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum StateError {
    /// Magic number is invalid
    InvalidMagic,

    /// CRC checksum mismatch
    CrcMismatch,

    /// Invalid slot value
    InvalidSlot,

    /// Invalid data size
    InvalidSize,

    /// Flash read failed
    ReadFailed,

    /// Flash write failed
    WriteFailed,
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid boot state magic number"),
            Self::CrcMismatch => write!(f, "Boot state CRC mismatch"),
            Self::InvalidSlot => write!(f, "Invalid slot value"),
            Self::InvalidSize => write!(f, "Invalid boot state size"),
            Self::ReadFailed => write!(f, "Failed to read boot state from Flash"),
            Self::WriteFailed => write!(f, "Failed to write boot state to Flash"),
        }
    }
}

/// Error codes for diagnostic purposes
///
/// Each error has a unique numeric code that can be used for debugging
/// and logging, especially useful when displaying errors via LED patterns
/// or limited output channels.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // SD card errors (0x10-0x1F)
    /// SD card initialization failed
    SdInitFailed = 0x10,
    /// Failed to mount FAT32 filesystem
    SdMountFailed = 0x11,
    /// Firmware file not found
    SdFileNotFound = 0x12,
    /// SD card read error
    SdReadError = 0x13,
    /// SD card operation timeout
    SdTimeout = 0x14,

    // Flash errors (0x20-0x2F)
    /// Flash erase failed
    FlashEraseFailed = 0x20,
    /// Flash write failed
    FlashWriteFailed = 0x21,
    /// Flash verification failed
    FlashVerifyFailed = 0x22,
    /// Invalid Flash address
    FlashInvalidAddress = 0x23,

    // Firmware verification errors (0x30-0x3F)
    /// Firmware size too large
    FirmwareTooLarge = 0x30,
    /// CRC32 checksum mismatch
    FirmwareCrcMismatch = 0x31,
    /// Invalid firmware format
    FirmwareInvalidFormat = 0x32,
    /// Firmware size mismatch
    FirmwareSizeMismatch = 0x33,

    // State errors (0x40-0x4F)
    /// Invalid boot state magic number
    StateInvalidMagic = 0x40,
    /// Boot state CRC mismatch
    StateCrcMismatch = 0x41,
    /// Failed to read boot state
    StateReadFailed = 0x42,
    /// Failed to write boot state
    StateWriteFailed = 0x43,
    /// Invalid slot value
    StateInvalidSlot = 0x44,
    /// Invalid boot state size
    StateInvalidSize = 0x45,

    // Timeout errors (0x50-0x5F)
    /// General operation timeout
    TimeoutGeneral = 0x50,
}

impl ErrorCode {
    /// Get a human-readable description of the error
    pub const fn description(&self) -> &'static str {
        match self {
            // SD card errors
            Self::SdInitFailed => "SD card initialization failed",
            Self::SdMountFailed => "Failed to mount FAT32 filesystem",
            Self::SdFileNotFound => "Firmware file not found on SD card",
            Self::SdReadError => "Error reading from SD card",
            Self::SdTimeout => "SD card operation timeout",

            // Flash errors
            Self::FlashEraseFailed => "Flash erase operation failed",
            Self::FlashWriteFailed => "Flash write operation failed",
            Self::FlashVerifyFailed => "Flash verification failed (read-back mismatch)",
            Self::FlashInvalidAddress => "Invalid Flash address specified",

            // Firmware verification errors
            Self::FirmwareTooLarge => "Firmware size exceeds 512KB maximum",
            Self::FirmwareCrcMismatch => "Firmware CRC32 checksum mismatch",
            Self::FirmwareInvalidFormat => "Invalid firmware file format",
            Self::FirmwareSizeMismatch => "Firmware size mismatch between expected and actual",

            // State errors
            Self::StateInvalidMagic => "Invalid boot state magic number",
            Self::StateCrcMismatch => "Boot state CRC checksum mismatch",
            Self::StateReadFailed => "Failed to read boot state from Flash",
            Self::StateWriteFailed => "Failed to write boot state to Flash",
            Self::StateInvalidSlot => "Invalid slot value in boot state",
            Self::StateInvalidSize => "Invalid boot state data size",

            // Timeout errors
            Self::TimeoutGeneral => "Operation timeout",
        }
    }

    /// Check if this error is fatal (requires system halt or reset)
    ///
    /// Fatal errors indicate serious hardware or system problems that
    /// cannot be recovered from without intervention.
    pub const fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::FlashEraseFailed | Self::StateReadFailed | Self::StateWriteFailed
        )
    }

    /// Check if this error is recoverable (can continue to application)
    ///
    /// Recoverable errors allow the system to skip the upgrade and
    /// boot into the existing application.
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::SdInitFailed
                | Self::SdMountFailed
                | Self::SdFileNotFound
                | Self::SdReadError
                | Self::SdTimeout
                | Self::FirmwareTooLarge
                | Self::FirmwareCrcMismatch
                | Self::FirmwareInvalidFormat
                | Self::FlashInvalidAddress
        )
    }

    /// Check if this error requires rollback
    ///
    /// Rollback errors occur during the upgrade process and require
    /// reverting to the previous firmware version.
    pub const fn requires_rollback(&self) -> bool {
        matches!(
            self,
            Self::FlashWriteFailed
                | Self::FlashVerifyFailed
                | Self::TimeoutGeneral
                | Self::StateInvalidMagic
                | Self::StateCrcMismatch
                | Self::StateInvalidSlot
                | Self::StateInvalidSize
        )
    }

    /// Get the numeric error code value
    pub const fn code(&self) -> u8 {
        *self as u8
    }
}

impl From<UpgradeError> for ErrorCode {
    fn from(error: UpgradeError) -> Self {
        match error {
            UpgradeError::SdCard(e) => e.into(),
            UpgradeError::Flash(e) => e.into(),
            UpgradeError::Verification(e) => e.into(),
            UpgradeError::State(e) => e.into(),
            UpgradeError::Timeout => Self::TimeoutGeneral,
        }
    }
}

impl From<SdError> for ErrorCode {
    fn from(error: SdError) -> Self {
        match error {
            SdError::InitFailed => Self::SdInitFailed,
            SdError::MountFailed => Self::SdMountFailed,
            SdError::FileNotFound => Self::SdFileNotFound,
            SdError::ReadError => Self::SdReadError,
            SdError::Timeout => Self::SdTimeout,
        }
    }
}

impl From<FlashError> for ErrorCode {
    fn from(error: FlashError) -> Self {
        match error {
            FlashError::EraseFailed => Self::FlashEraseFailed,
            FlashError::WriteFailed => Self::FlashWriteFailed,
            FlashError::VerifyFailed => Self::FlashVerifyFailed,
            FlashError::InvalidAddress => Self::FlashInvalidAddress,
        }
    }
}

impl From<VerifyError> for ErrorCode {
    fn from(error: VerifyError) -> Self {
        match error {
            VerifyError::SizeTooLarge => Self::FirmwareTooLarge,
            VerifyError::CrcMismatch => Self::FirmwareCrcMismatch,
            VerifyError::InvalidFormat => Self::FirmwareInvalidFormat,
            VerifyError::SizeMismatch => Self::FirmwareSizeMismatch,
        }
    }
}

impl From<StateError> for ErrorCode {
    fn from(error: StateError) -> Self {
        match error {
            StateError::InvalidMagic => Self::StateInvalidMagic,
            StateError::CrcMismatch => Self::StateCrcMismatch,
            StateError::ReadFailed => Self::StateReadFailed,
            StateError::WriteFailed => Self::StateWriteFailed,
            StateError::InvalidSlot => Self::StateInvalidSlot,
            StateError::InvalidSize => Self::StateInvalidSize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Import std for tests
    #[allow(unused_imports)]
    use std::format;

    #[test]
    fn test_error_code_values() {
        // Verify error codes are in correct ranges
        assert_eq!(ErrorCode::SdInitFailed.code(), 0x10);
        assert_eq!(ErrorCode::SdTimeout.code(), 0x14);

        assert_eq!(ErrorCode::FlashEraseFailed.code(), 0x20);
        assert_eq!(ErrorCode::FlashInvalidAddress.code(), 0x23);

        assert_eq!(ErrorCode::FirmwareTooLarge.code(), 0x30);
        assert_eq!(ErrorCode::FirmwareInvalidFormat.code(), 0x32);

        assert_eq!(ErrorCode::StateInvalidMagic.code(), 0x40);
        assert_eq!(ErrorCode::StateInvalidSize.code(), 0x45);

        assert_eq!(ErrorCode::TimeoutGeneral.code(), 0x50);
    }

    #[test]
    fn test_error_code_descriptions() {
        // Verify all error codes have descriptions
        assert!(!ErrorCode::SdInitFailed.description().is_empty());
        assert!(!ErrorCode::FlashEraseFailed.description().is_empty());
        assert!(!ErrorCode::FirmwareTooLarge.description().is_empty());
        assert!(!ErrorCode::StateInvalidMagic.description().is_empty());
        assert!(!ErrorCode::TimeoutGeneral.description().is_empty());
    }

    #[test]
    fn test_fatal_errors() {
        assert!(ErrorCode::FlashEraseFailed.is_fatal());
        assert!(ErrorCode::StateReadFailed.is_fatal());
        assert!(ErrorCode::StateWriteFailed.is_fatal());

        assert!(!ErrorCode::SdInitFailed.is_fatal());
        assert!(!ErrorCode::FirmwareTooLarge.is_fatal());
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(ErrorCode::SdInitFailed.is_recoverable());
        assert!(ErrorCode::SdMountFailed.is_recoverable());
        assert!(ErrorCode::SdFileNotFound.is_recoverable());
        assert!(ErrorCode::FirmwareTooLarge.is_recoverable());
        assert!(ErrorCode::FirmwareCrcMismatch.is_recoverable());

        assert!(!ErrorCode::FlashEraseFailed.is_recoverable());
        assert!(!ErrorCode::FlashWriteFailed.is_recoverable());
    }

    #[test]
    fn test_rollback_errors() {
        assert!(ErrorCode::FlashWriteFailed.requires_rollback());
        assert!(ErrorCode::FlashVerifyFailed.requires_rollback());
        assert!(ErrorCode::TimeoutGeneral.requires_rollback());

        assert!(!ErrorCode::SdInitFailed.requires_rollback());
        assert!(!ErrorCode::FirmwareTooLarge.requires_rollback());
    }

    #[test]
    fn test_upgrade_error_conversions() {
        let sd_err = UpgradeError::from(SdError::InitFailed);
        assert_eq!(sd_err, UpgradeError::SdCard(SdError::InitFailed));

        let flash_err = UpgradeError::from(FlashError::EraseFailed);
        assert_eq!(flash_err, UpgradeError::Flash(FlashError::EraseFailed));

        let verify_err = UpgradeError::from(VerifyError::CrcMismatch);
        assert_eq!(
            verify_err,
            UpgradeError::Verification(VerifyError::CrcMismatch)
        );

        let state_err = UpgradeError::from(StateError::InvalidMagic);
        assert_eq!(state_err, UpgradeError::State(StateError::InvalidMagic));
    }

    #[test]
    fn test_error_code_from_upgrade_error() {
        let code = ErrorCode::from(UpgradeError::SdCard(SdError::InitFailed));
        assert_eq!(code, ErrorCode::SdInitFailed);

        let code = ErrorCode::from(UpgradeError::Flash(FlashError::WriteFailed));
        assert_eq!(code, ErrorCode::FlashWriteFailed);

        let code = ErrorCode::from(UpgradeError::Verification(VerifyError::SizeTooLarge));
        assert_eq!(code, ErrorCode::FirmwareTooLarge);

        let code = ErrorCode::from(UpgradeError::State(StateError::CrcMismatch));
        assert_eq!(code, ErrorCode::StateCrcMismatch);

        let code = ErrorCode::from(UpgradeError::Timeout);
        assert_eq!(code, ErrorCode::TimeoutGeneral);
    }

    #[test]
    fn test_error_code_from_specific_errors() {
        assert_eq!(
            ErrorCode::from(SdError::MountFailed),
            ErrorCode::SdMountFailed
        );
        assert_eq!(
            ErrorCode::from(FlashError::VerifyFailed),
            ErrorCode::FlashVerifyFailed
        );
        assert_eq!(
            ErrorCode::from(VerifyError::InvalidFormat),
            ErrorCode::FirmwareInvalidFormat
        );
        assert_eq!(
            ErrorCode::from(StateError::InvalidSlot),
            ErrorCode::StateInvalidSlot
        );
    }

    #[test]
    fn test_error_display() {
        let err = SdError::InitFailed;
        assert_eq!(format!("{}", err), "SD card initialization failed");

        let err = FlashError::WriteFailed;
        assert_eq!(format!("{}", err), "Flash write failed");

        let err = VerifyError::CrcMismatch;
        assert_eq!(format!("{}", err), "CRC32 checksum mismatch");

        let err = StateError::InvalidMagic;
        assert_eq!(format!("{}", err), "Invalid boot state magic number");
    }

    #[test]
    fn test_upgrade_error_display() {
        let err = UpgradeError::SdCard(SdError::FileNotFound);
        assert!(format!("{}", err).contains("SD card error"));
        assert!(format!("{}", err).contains("not found"));

        let err = UpgradeError::Timeout;
        assert_eq!(format!("{}", err), "Operation timeout");
    }

    #[test]
    fn test_error_categorization_complete() {
        // Every error should be in exactly one category
        let all_codes = [
            ErrorCode::SdInitFailed,
            ErrorCode::SdMountFailed,
            ErrorCode::SdFileNotFound,
            ErrorCode::SdReadError,
            ErrorCode::SdTimeout,
            ErrorCode::FlashEraseFailed,
            ErrorCode::FlashWriteFailed,
            ErrorCode::FlashVerifyFailed,
            ErrorCode::FlashInvalidAddress,
            ErrorCode::FirmwareTooLarge,
            ErrorCode::FirmwareCrcMismatch,
            ErrorCode::FirmwareInvalidFormat,
            ErrorCode::StateInvalidMagic,
            ErrorCode::StateCrcMismatch,
            ErrorCode::StateReadFailed,
            ErrorCode::StateWriteFailed,
            ErrorCode::StateInvalidSlot,
            ErrorCode::StateInvalidSize,
            ErrorCode::TimeoutGeneral,
        ];

        for code in &all_codes {
            let fatal = code.is_fatal();
            let recoverable = code.is_recoverable();
            let rollback = code.requires_rollback();

            // Each error should be in at least one category
            assert!(
                fatal || recoverable || rollback,
                "Error {:?} is not categorized",
                code
            );
        }
    }
}
