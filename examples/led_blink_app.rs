//! Simple LED Blink Application Example
//!
//! This is a minimal application that can be used to test the bootloader.
//! It blinks an LED to show that the application is running.
//!
//! # Building
//!
//! ```bash
//! cargo build --example led_blink_app --release
//! ```
//!
//! # Generating Firmware Binary
//!
//! ```bash
//! cargo objcopy --example led_blink_app --release -- -O binary firmware.bin
//! ```
//!
//! # Memory Configuration
//!
//! This application must be linked to run from Slot A (0x08010000).
//! Update memory.x accordingly:
//!
//! ```text
//! MEMORY
//! {
//!   FLASH : ORIGIN = 0x08010000, LENGTH = 448K
//!   RAM : ORIGIN = 0x20000000, LENGTH = 128K
//! }
//! ```

#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use panic_halt as _;

#[entry]
fn main() -> ! {
    // Initialize peripherals
    let p = embassy_stm32::init(Default::default());

    // Configure LED on PA5 (or adjust to your board)
    let mut led = Output::new(p.PA5, Level::Low, Speed::Low);

    // Blink LED forever
    loop {
        led.set_high();
        cortex_m::asm::delay(8_000_000); // ~1 second at 16MHz
        led.set_low();
        cortex_m::asm::delay(8_000_000); // ~1 second at 16MHz
    }
}
