//! LED Indicator Module
//!
//! Provides LED status indication for the bootloader.
//! - PA7 (LED1): Status indicator (upgrade progress and errors)
//! - PA6 (LED2): Heartbeat indicator (bootloader running)

use embassy_stm32::gpio::{AnyPin, Level, Output, Speed};
use embassy_time::{Duration, Timer};

/// LED blinking patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedPattern {
    /// LED off
    Off,
    /// LED constantly on
    On,
    /// Fast blink (100ms period)
    FastBlink,
    /// Medium blink (200ms period)
    MediumBlink,
    /// Slow blink (500ms period)
    SlowBlink,
}

impl LedPattern {
    /// Get the blink period in milliseconds
    fn period_ms(&self) -> Option<u64> {
        match self {
            LedPattern::Off | LedPattern::On => None,
            LedPattern::FastBlink => Some(100),
            LedPattern::MediumBlink => Some(200),
            LedPattern::SlowBlink => Some(500),
        }
    }
}

/// LED Indicator controller
///
/// Controls two LEDs:
/// - Status LED (PA7): Shows upgrade status
/// - Heartbeat LED (PA6): Shows bootloader is running
pub struct LedIndicator {
    /// Status LED (PA7)
    status_led: Output<'static, AnyPin>,
    /// Heartbeat LED (PA6)
    heartbeat_led: Output<'static, AnyPin>,
    /// Current status LED pattern
    status_pattern: LedPattern,
    /// Whether heartbeat is enabled
    heartbeat_enabled: bool,
    /// Internal state for blinking
    blink_state: bool,
}

impl LedIndicator {
    /// Create a new LED indicator
    ///
    /// # Arguments
    /// * `status_pin` - Pin for status LED (PA7)
    /// * `heartbeat_pin` - Pin for heartbeat LED (PA6)
    pub fn new(status_pin: AnyPin, heartbeat_pin: AnyPin) -> Self {
        let status_led = Output::new(status_pin, Level::Low, Speed::Low);
        let heartbeat_led = Output::new(heartbeat_pin, Level::Low, Speed::Low);

        Self {
            status_led,
            heartbeat_led,
            status_pattern: LedPattern::Off,
            heartbeat_enabled: false,
            blink_state: false,
        }
    }

    /// Set the status LED pattern
    ///
    /// # Arguments
    /// * `pattern` - The LED pattern to display
    pub fn set_status(&mut self, pattern: LedPattern) {
        self.status_pattern = pattern;

        // Immediately apply static patterns
        match pattern {
            LedPattern::Off => {
                self.status_led.set_low();
            }
            LedPattern::On => {
                self.status_led.set_high();
            }
            _ => {
                // Blinking patterns will be handled by update()
            }
        }
    }

    /// Enable or disable heartbeat LED
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable heartbeat
    pub fn set_heartbeat(&mut self, enabled: bool) {
        self.heartbeat_enabled = enabled;
        if !enabled {
            self.heartbeat_led.set_low();
        }
    }

    /// Update LED states (should be called periodically)
    ///
    /// This method handles blinking patterns for both status and heartbeat LEDs.
    /// It should be called in a loop to maintain the blinking effect.
    pub async fn update(&mut self) {
        // Handle status LED blinking
        if let Some(period_ms) = self.status_pattern.period_ms() {
            self.blink_state = !self.blink_state;
            if self.blink_state {
                self.status_led.set_high();
            } else {
                self.status_led.set_low();
            }
            Timer::after(Duration::from_millis(period_ms)).await;
        } else {
            // For non-blinking patterns, wait a bit before next update
            Timer::after(Duration::from_millis(100)).await;
        }

        // Handle heartbeat LED (always slow blink when enabled)
        if self.heartbeat_enabled {
            // Toggle heartbeat at a slower rate (1Hz)
            static mut HEARTBEAT_COUNTER: u32 = 0;
            unsafe {
                HEARTBEAT_COUNTER += 1;
                if HEARTBEAT_COUNTER >= 5 {
                    // Toggle every ~500ms (5 * 100ms)
                    HEARTBEAT_COUNTER = 0;
                    if self.heartbeat_led.is_set_high() {
                        self.heartbeat_led.set_low();
                    } else {
                        self.heartbeat_led.set_high();
                    }
                }
            }
        }
    }

    /// Run the LED update loop indefinitely
    ///
    /// This is a convenience method that continuously calls update().
    /// It should be spawned as a separate task.
    pub async fn run(&mut self) -> ! {
        loop {
            self.update().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_led_pattern_period() {
        assert_eq!(LedPattern::Off.period_ms(), None);
        assert_eq!(LedPattern::On.period_ms(), None);
        assert_eq!(LedPattern::FastBlink.period_ms(), Some(100));
        assert_eq!(LedPattern::MediumBlink.period_ms(), Some(200));
        assert_eq!(LedPattern::SlowBlink.period_ms(), Some(500));
    }

    #[test]
    fn test_led_pattern_equality() {
        assert_eq!(LedPattern::Off, LedPattern::Off);
        assert_eq!(LedPattern::FastBlink, LedPattern::FastBlink);
        assert_ne!(LedPattern::Off, LedPattern::On);
        assert_ne!(LedPattern::FastBlink, LedPattern::SlowBlink);
    }
}
