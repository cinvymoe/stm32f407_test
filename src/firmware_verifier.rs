//! Firmware verification module
//!
//! This module provides CRC32 checksum calculation and firmware size validation
//! to ensure firmware integrity before flashing.

use crate::config::MAX_FIRMWARE_SIZE;
use crate::error::VerifyError;
use crc::{Crc, Digest, CRC_32_ISO_HDLC};

/// CRC32 algorithm used for firmware verification
const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

/// Firmware verifier for CRC32 checksum calculation and size validation
///
/// This structure maintains the state of an incremental CRC32 calculation,
/// allowing firmware to be verified in chunks as it's read from the SD card.
///
/// # Example
///
/// ```no_run
/// use stm32f407_sd_bootloader::firmware_verifier::FirmwareVerifier;
///
/// let mut verifier = FirmwareVerifier::new();
///
/// // Process firmware in chunks
/// verifier.update(&chunk1);
/// verifier.update(&chunk2);
///
/// // Get final CRC32 value
/// let crc = verifier.finalize();
/// ```
pub struct FirmwareVerifier {
    /// CRC32 digest state
    digest: Digest<'static, u32>,

    /// Total bytes processed
    bytes_processed: u32,
}

impl FirmwareVerifier {
    /// Create a new firmware verifier
    ///
    /// Initializes the CRC32 calculator with the ISO HDLC polynomial.
    pub fn new() -> Self {
        Self {
            digest: CRC.digest(),
            bytes_processed: 0,
        }
    }

    /// Update the CRC calculation with new data
    ///
    /// This method can be called multiple times to incrementally process
    /// firmware data as it's read from the SD card.
    ///
    /// # Arguments
    ///
    /// * `data` - Slice of bytes to include in the CRC calculation
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use stm32f407_sd_bootloader::firmware_verifier::FirmwareVerifier;
    /// let mut verifier = FirmwareVerifier::new();
    /// verifier.update(&[0x01, 0x02, 0x03]);
    /// verifier.update(&[0x04, 0x05, 0x06]);
    /// ```
    pub fn update(&mut self, data: &[u8]) {
        self.digest.update(data);
        self.bytes_processed += data.len() as u32;
    }

    /// Finalize the CRC calculation and return the checksum
    ///
    /// This method returns the final CRC32 value. After calling this method,
    /// the verifier can be reset and reused for another calculation.
    ///
    /// # Returns
    ///
    /// The calculated CRC32 checksum as a u32 value
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use stm32f407_sd_bootloader::firmware_verifier::FirmwareVerifier;
    /// let mut verifier = FirmwareVerifier::new();
    /// verifier.update(&[0x01, 0x02, 0x03]);
    /// let crc = verifier.finalize();
    /// ```
    pub fn finalize(&self) -> u32 {
        self.digest.clone().finalize()
    }

    /// Reset the verifier to initial state
    ///
    /// This allows the verifier to be reused for a new CRC calculation
    /// without allocating a new instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use stm32f407_sd_bootloader::firmware_verifier::FirmwareVerifier;
    /// let mut verifier = FirmwareVerifier::new();
    /// verifier.update(&[0x01, 0x02, 0x03]);
    /// let crc1 = verifier.finalize();
    ///
    /// verifier.reset();
    /// verifier.update(&[0x04, 0x05, 0x06]);
    /// let crc2 = verifier.finalize();
    /// ```
    pub fn reset(&mut self) {
        self.digest = CRC.digest();
        self.bytes_processed = 0;
    }

    /// Validate that the firmware size is within acceptable limits
    ///
    /// Checks that the firmware size does not exceed MAX_FIRMWARE_SIZE (448KB).
    /// This should be called before starting the firmware write process.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the firmware in bytes
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the size is valid
    /// * `Err(VerifyError::SizeTooLarge)` if the size exceeds the maximum
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use stm32f407_sd_bootloader::firmware_verifier::FirmwareVerifier;
    /// let verifier = FirmwareVerifier::new();
    /// match verifier.validate_size(400_000) {
    ///     Ok(()) => println!("Size is valid"),
    ///     Err(e) => println!("Size validation failed: {:?}", e),
    /// }
    /// ```
    pub fn validate_size(&self, size: u32) -> Result<(), VerifyError> {
        if size > MAX_FIRMWARE_SIZE {
            Err(VerifyError::SizeTooLarge)
        } else {
            Ok(())
        }
    }

    /// Get the number of bytes processed so far
    ///
    /// This can be used to track progress during firmware verification.
    ///
    /// # Returns
    ///
    /// The total number of bytes that have been processed by `update()`
    pub fn bytes_processed(&self) -> u32 {
        self.bytes_processed
    }
}

impl Default for FirmwareVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_verifier() {
        let verifier = FirmwareVerifier::new();
        assert_eq!(verifier.bytes_processed(), 0);
    }

    #[test]
    fn test_update_and_finalize() {
        let mut verifier = FirmwareVerifier::new();
        let data = b"Hello, World!";

        verifier.update(data);
        assert_eq!(verifier.bytes_processed(), data.len() as u32);

        let crc = verifier.finalize();
        assert_ne!(crc, 0); // CRC should be non-zero for non-empty data
    }

    #[test]
    fn test_incremental_update() {
        let mut verifier1 = FirmwareVerifier::new();
        let mut verifier2 = FirmwareVerifier::new();

        let data = b"Hello, World!";

        // Calculate CRC in one go
        verifier1.update(data);
        let crc1 = verifier1.finalize();

        // Calculate CRC incrementally
        verifier2.update(&data[0..5]);
        verifier2.update(&data[5..]);
        let crc2 = verifier2.finalize();

        // Both methods should produce the same CRC
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_reset() {
        let mut verifier = FirmwareVerifier::new();

        verifier.update(b"First data");
        let crc1 = verifier.finalize();
        assert_eq!(verifier.bytes_processed(), 10);

        verifier.reset();
        assert_eq!(verifier.bytes_processed(), 0);

        verifier.update(b"Second data");
        let crc2 = verifier.finalize();
        assert_eq!(verifier.bytes_processed(), 11);

        // Different data should produce different CRCs
        assert_ne!(crc1, crc2);
    }

    #[test]
    fn test_validate_size_valid() {
        let verifier = FirmwareVerifier::new();

        // Test valid sizes
        assert!(verifier.validate_size(0).is_ok());
        assert!(verifier.validate_size(1024).is_ok());
        assert!(verifier.validate_size(MAX_FIRMWARE_SIZE).is_ok());
    }

    #[test]
    fn test_validate_size_too_large() {
        let verifier = FirmwareVerifier::new();

        // Test sizes that exceed the maximum
        assert_eq!(
            verifier.validate_size(MAX_FIRMWARE_SIZE + 1),
            Err(VerifyError::SizeTooLarge)
        );
        assert_eq!(
            verifier.validate_size(1024 * 1024),
            Err(VerifyError::SizeTooLarge)
        );
    }

    #[test]
    fn test_validate_size_boundary() {
        let verifier = FirmwareVerifier::new();

        // Test boundary conditions
        assert!(verifier.validate_size(MAX_FIRMWARE_SIZE - 1).is_ok());
        assert!(verifier.validate_size(MAX_FIRMWARE_SIZE).is_ok());
        assert_eq!(
            verifier.validate_size(MAX_FIRMWARE_SIZE + 1),
            Err(VerifyError::SizeTooLarge)
        );
    }

    #[test]
    fn test_crc_deterministic() {
        let data = b"Test data for CRC calculation";

        let mut verifier1 = FirmwareVerifier::new();
        verifier1.update(data);
        let crc1 = verifier1.finalize();

        let mut verifier2 = FirmwareVerifier::new();
        verifier2.update(data);
        let crc2 = verifier2.finalize();

        // Same data should always produce the same CRC
        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_crc_different_data() {
        let mut verifier1 = FirmwareVerifier::new();
        verifier1.update(b"Data 1");
        let crc1 = verifier1.finalize();

        let mut verifier2 = FirmwareVerifier::new();
        verifier2.update(b"Data 2");
        let crc2 = verifier2.finalize();

        // Different data should produce different CRCs
        assert_ne!(crc1, crc2);
    }

    #[test]
    fn test_empty_data() {
        let mut verifier = FirmwareVerifier::new();
        verifier.update(&[]);

        assert_eq!(verifier.bytes_processed(), 0);
        let crc = verifier.finalize();

        // CRC of empty data is 0 for ISO HDLC
        assert_eq!(crc, 0);
    }

    #[test]
    fn test_large_data() {
        use std::vec;

        let mut verifier = FirmwareVerifier::new();
        let data = vec![0xAA; 4096]; // 4KB of data

        verifier.update(&data);
        assert_eq!(verifier.bytes_processed(), 4096);

        let crc = verifier.finalize();
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_multiple_updates_tracking() {
        let mut verifier = FirmwareVerifier::new();

        verifier.update(&[1, 2, 3]);
        assert_eq!(verifier.bytes_processed(), 3);

        verifier.update(&[4, 5, 6, 7]);
        assert_eq!(verifier.bytes_processed(), 7);

        verifier.update(&[8, 9]);
        assert_eq!(verifier.bytes_processed(), 9);
    }

    #[test]
    fn test_finalize_does_not_consume() {
        let mut verifier = FirmwareVerifier::new();
        verifier.update(b"Test");

        // Finalize can be called multiple times
        let crc1 = verifier.finalize();
        let crc2 = verifier.finalize();

        assert_eq!(crc1, crc2);
        assert_eq!(verifier.bytes_processed(), 4);
    }

    #[test]
    fn test_default_trait() {
        let verifier = FirmwareVerifier::default();
        assert_eq!(verifier.bytes_processed(), 0);
    }

    #[test]
    fn test_crc32_known_value() {
        // Test with a known CRC32 value
        // "123456789" has CRC32 of 0xCBF43926 (ISO HDLC)
        let mut verifier = FirmwareVerifier::new();
        verifier.update(b"123456789");
        let crc = verifier.finalize();

        assert_eq!(crc, 0xCBF43926);
    }

    #[test]
    fn test_order_matters() {
        let mut verifier1 = FirmwareVerifier::new();
        verifier1.update(b"AB");
        let crc1 = verifier1.finalize();

        let mut verifier2 = FirmwareVerifier::new();
        verifier2.update(b"BA");
        let crc2 = verifier2.finalize();

        // Order of bytes matters for CRC
        assert_ne!(crc1, crc2);
    }

    #[test]
    fn test_single_byte_difference() {
        let mut verifier1 = FirmwareVerifier::new();
        verifier1.update(b"Hello, World!");
        let crc1 = verifier1.finalize();

        let mut verifier2 = FirmwareVerifier::new();
        verifier2.update(b"Hello, World?"); // Changed last character
        let crc2 = verifier2.finalize();

        // Even a single byte difference should change the CRC
        assert_ne!(crc1, crc2);
    }
}
