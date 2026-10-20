//! Button input module with debouncing support
//!
//! This module provides button input functionality with software debouncing
//! for reliable button state detection. The button is configured on PE4 with
//! pull-up resistor (active low).
//!
//! # Requirements
//! - Requirement 3.1.2: Button input on PE4, pull-up mode, active low
//! - 20ms software debounce time

#![allow(dead_code)] // Will be used in later tasks

use embassy_stm32::gpio::{AnyPin, Input, Level, Pull};
use embassy_time::{Duration, Timer};

use crate::config::BUTTON_DEBOUNCE_MS;

/// Button input handler with debouncing
///
/// Provides reliable button state detection with software debouncing
/// to filter out mechanical switch noise.
///
/// # Configuration
/// - Pin: PE4
/// - Mode: Input with pull-up
/// - Active: Low (button pressed = low level)
/// - Debounce: 20ms
pub struct ButtonInput<'d> {
    pin: Input<'d, AnyPin>,
}

impl<'d> ButtonInput<'d> {
    /// Create a new button input handler
    ///
    /// # Arguments
    /// * `pin` - GPIO input pin PE4 configured with pull-up
    ///
    /// # Example
    /// ```no_run
    /// use embassy_stm32::gpio::Input;
    /// use stm32_bootloader::button_input::ButtonInput;
    ///
    /// let button = ButtonInput::new(p.PE4.degrade());
    /// ```
    pub fn new(pin: AnyPin) -> Self {
        let pin = Input::new(pin, Pull::Up);
        Self { pin }
    }

    /// Check if button is currently pressed (with debouncing)
    ///
    /// This method implements software debouncing by checking the button
    /// state, waiting for the debounce period, and checking again.
    /// The button is considered pressed only if it remains low for the
    /// entire debounce period.
    ///
    /// # Returns
    /// * `true` if button is pressed (stable low state)
    /// * `false` if button is not pressed or state is unstable
    ///
    /// # Timing
    /// This method takes at least `debounce_time` to complete when the
    /// button is pressed, and returns immediately if not pressed.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example(mut button: ButtonInput) {
    /// if button.is_pressed().await {
    ///     // Button is reliably pressed
    ///     defmt::info!("Button pressed!");
    /// }
    /// # }
    /// ```
    pub async fn is_pressed(&mut self) -> bool {
        // Check initial state - button is active low
        if self.pin.get_level() == Level::High {
            // Button not pressed (high level)
            return false;
        }

        // Button appears pressed, wait for debounce period
        Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS)).await;

        // Check state again after debounce
        // Button is pressed only if it's still low
        self.pin.get_level() == Level::Low
    }

    /// Wait for button to be released
    ///
    /// This method blocks until the button is released (goes high).
    /// It includes debouncing to ensure the release is stable.
    ///
    /// # Behavior
    /// - If button is already released, returns immediately
    /// - If button is pressed, waits until it's released
    /// - Applies debouncing to ensure stable release detection
    ///
    /// # Example
    /// ```no_run
    /// # async fn example(mut button: ButtonInput) {
    /// // Wait for user to release the button
    /// button.wait_release().await;
    /// defmt::info!("Button released!");
    /// # }
    /// ```
    pub async fn wait_release(&mut self) {
        // Wait until button goes high (released)
        loop {
            if self.pin.get_level() == Level::High {
                // Button appears released, apply debouncing
                Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS)).await;

                // Check if still released after debounce
                if self.pin.get_level() == Level::High {
                    // Stable release detected
                    break;
                }
            }

            // Still pressed or bouncing, check again soon
            // Small delay to avoid busy-waiting
            Timer::after(Duration::from_millis(BUTTON_DEBOUNCE_MS / 4)).await;
        }
    }

    /// Get raw button state without debouncing
    ///
    /// Returns the immediate pin state. Useful for testing or
    /// special cases where debouncing is not needed.
    ///
    /// # Returns
    /// * `true` if button is pressed (pin is low)
    /// * `false` if button is not pressed (pin is high)
    pub fn is_pressed_raw(&self) -> bool {
        self.pin.get_level() == Level::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are unit tests that verify the logic.
    // Hardware-specific tests would require either:
    // 1. Running on actual hardware
    // 2. Using a mock GPIO implementation
    // 3. Integration tests with hardware simulation

    #[test]
    fn test_button_debounce_time() {
        // Verify default debounce time
        assert_eq!(BUTTON_DEBOUNCE_MS, 20);
    }
}
