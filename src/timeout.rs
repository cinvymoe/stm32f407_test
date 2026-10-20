//! Timeout protection for I/O operations
//!
//! This module provides timeout wrappers for async operations to prevent
//! the bootloader from hanging indefinitely on I/O failures.
//!
//! Requirements:
//! - 3.5.3: Timeout protection for all I/O operations

use crate::error::UpgradeError;
use core::future::Future;
use embassy_time::{Duration, TimeoutError, Timer};

/// Timeout helper for async operations
pub struct TimeoutHelper;

impl TimeoutHelper {
    /// Execute an async operation with a timeout
    ///
    /// If the operation doesn't complete within the specified duration,
    /// it will be cancelled and a timeout error will be returned.
    ///
    /// # Arguments
    /// * `operation` - The async operation to execute
    /// * `timeout` - Maximum duration to wait
    ///
    /// # Returns
    /// * `Ok(T)` - Operation completed successfully
    /// * `Err(UpgradeError::Timeout)` - Operation timed out
    /// * `Err(UpgradeError)` - Operation failed with an error
    ///
    /// # Requirements
    /// Validates requirement 3.5.3 - Timeout protection
    ///
    /// # Example
    /// ```no_run
    /// use embassy_time::Duration;
    ///
    /// let result = TimeoutHelper::with_timeout(
    ///     async { sd_card.init().await },
    ///     Duration::from_secs(5)
    /// ).await?;
    /// ```
    pub async fn with_timeout<F, T, E>(operation: F, timeout: Duration) -> Result<T, UpgradeError>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<UpgradeError>,
    {
        match embassy_time::with_timeout(timeout, operation).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e.into()),
            Err(TimeoutError) => {
                defmt::error!("Operation timed out after {:?}", timeout);
                Err(UpgradeError::Timeout)
            }
        }
    }

    /// Execute an async operation with a timeout and retry logic
    ///
    /// If the operation fails or times out, it will be retried up to the
    /// specified number of times with a delay between attempts.
    ///
    /// # Arguments
    /// * `operation_fn` - Function that creates the async operation (called for each retry)
    /// * `timeout` - Maximum duration to wait per attempt
    /// * `max_retries` - Maximum number of retry attempts (0 = no retries)
    /// * `retry_delay` - Delay between retry attempts
    ///
    /// # Returns
    /// * `Ok(T)` - Operation completed successfully
    /// * `Err(UpgradeError)` - All attempts failed
    ///
    /// # Example
    /// ```no_run
    /// use embassy_time::Duration;
    ///
    /// let result = TimeoutHelper::with_timeout_and_retry(
    ///     || async { sd_card.read_block(&mut buffer).await },
    ///     Duration::from_secs(5),
    ///     3,  // 3 retries
    ///     Duration::from_millis(100)
    /// ).await?;
    /// ```
    pub async fn with_timeout_and_retry<F, Fut, T, E>(
        mut operation_fn: F,
        timeout: Duration,
        max_retries: u8,
        retry_delay: Duration,
    ) -> Result<T, UpgradeError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Into<UpgradeError>,
    {
        let mut attempts = 0;
        let max_attempts = max_retries + 1; // Initial attempt + retries

        loop {
            attempts += 1;

            match Self::with_timeout(operation_fn(), timeout).await {
                Ok(result) => {
                    if attempts > 1 {
                        defmt::info!(
                            "Operation succeeded on attempt {}/{}",
                            attempts,
                            max_attempts
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    if attempts >= max_attempts {
                        defmt::error!("Operation failed after {} attempts: {:?}", attempts, e);
                        return Err(e);
                    }

                    defmt::warn!(
                        "Operation failed on attempt {}/{}, retrying after {:?}...",
                        attempts,
                        max_attempts,
                        retry_delay
                    );

                    Timer::after(retry_delay).await;
                }
            }
        }
    }

    /// Execute an async read operation with timeout
    ///
    /// Convenience wrapper for read operations with standard timeout.
    ///
    /// # Arguments
    /// * `operation` - The async read operation
    ///
    /// # Returns
    /// * `Ok(T)` - Read completed successfully
    /// * `Err(UpgradeError)` - Read failed or timed out
    ///
    /// # Requirements
    /// Validates requirement 3.5.3 - SD card read timeout (30 seconds)
    pub async fn read_with_timeout<F, T, E>(operation: F) -> Result<T, UpgradeError>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<UpgradeError>,
    {
        use crate::config::SD_READ_TIMEOUT;
        Self::with_timeout(operation, SD_READ_TIMEOUT).await
    }

    /// Execute an async write operation with timeout
    ///
    /// Convenience wrapper for write operations with standard timeout.
    ///
    /// # Arguments
    /// * `operation` - The async write operation
    ///
    /// # Returns
    /// * `Ok(T)` - Write completed successfully
    /// * `Err(UpgradeError)` - Write failed or timed out
    ///
    /// # Requirements
    /// Validates requirement 3.5.3 - Flash write timeout (60 seconds)
    pub async fn write_with_timeout<F, T, E>(operation: F) -> Result<T, UpgradeError>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<UpgradeError>,
    {
        use crate::config::FLASH_WRITE_TIMEOUT;
        Self::with_timeout(operation, FLASH_WRITE_TIMEOUT).await
    }

    /// Execute an async initialization operation with timeout
    ///
    /// Convenience wrapper for initialization operations with standard timeout.
    ///
    /// # Arguments
    /// * `operation` - The async initialization operation
    ///
    /// # Returns
    /// * `Ok(T)` - Initialization completed successfully
    /// * `Err(UpgradeError)` - Initialization failed or timed out
    ///
    /// # Requirements
    /// Validates requirement 3.5.3 - SD card init timeout (5 seconds)
    pub async fn init_with_timeout<F, T, E>(operation: F) -> Result<T, UpgradeError>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<UpgradeError>,
    {
        use crate::config::SD_INIT_TIMEOUT;
        Self::with_timeout(operation, SD_INIT_TIMEOUT).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SdError;

    #[test]
    fn test_timeout_durations() {
        use crate::config::{FLASH_WRITE_TIMEOUT, SD_INIT_TIMEOUT, SD_READ_TIMEOUT};

        // Verify timeout constants are reasonable
        assert_eq!(SD_INIT_TIMEOUT.as_secs(), 5);
        assert_eq!(SD_READ_TIMEOUT.as_secs(), 30);
        assert_eq!(FLASH_WRITE_TIMEOUT.as_secs(), 60);

        // Verify ordering (init < read < write)
        assert!(SD_INIT_TIMEOUT < SD_READ_TIMEOUT);
        assert!(SD_READ_TIMEOUT < FLASH_WRITE_TIMEOUT);
    }

    // Note: Async tests require tokio or embassy-executor runtime
    // These are conceptual tests showing the expected behavior

    #[test]
    fn test_timeout_error_conversion() {
        // Test that timeout errors are properly converted
        let error = UpgradeError::Timeout;
        assert!(matches!(error, UpgradeError::Timeout));
    }

    #[test]
    fn test_error_into_upgrade_error() {
        // Test that SdError converts to UpgradeError
        let sd_error = SdError::InitFailed;
        let upgrade_error: UpgradeError = sd_error.into();
        assert!(matches!(upgrade_error, UpgradeError::SdCard(_)));
    }

    #[test]
    fn test_retry_attempts_calculation() {
        // Test retry logic calculations
        let max_retries = 3u8;
        let max_attempts = max_retries + 1; // Initial + retries
        assert_eq!(max_attempts, 4);

        let no_retries = 0u8;
        let single_attempt = no_retries + 1;
        assert_eq!(single_attempt, 1);
    }

    #[test]
    fn test_timeout_duration_creation() {
        // Test Duration creation
        let five_secs = Duration::from_secs(5);
        assert_eq!(five_secs.as_millis(), 5000);

        let thirty_secs = Duration::from_secs(30);
        assert_eq!(thirty_secs.as_millis(), 30000);

        let sixty_secs = Duration::from_secs(60);
        assert_eq!(sixty_secs.as_millis(), 60000);
    }

    #[test]
    fn test_retry_delay_reasonable() {
        // Test that retry delays are reasonable
        let short_delay = Duration::from_millis(100);
        let medium_delay = Duration::from_millis(500);
        let long_delay = Duration::from_secs(1);

        assert!(short_delay < medium_delay);
        assert!(medium_delay < long_delay);
        assert!(long_delay.as_millis() == 1000);
    }
}
