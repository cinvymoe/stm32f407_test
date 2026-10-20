//! SD Card Reader Test Example
//!
//! This example demonstrates how to initialize and use the SD card reader
//! with the SDIO interface on STM32F407.
//!
//! Hardware connections:
//! - PC8:  SDIO_D0
//! - PC9:  SDIO_D1
//! - PC10: SDIO_D2
//! - PC11: SDIO_D3
//! - PC12: SDIO_CK (Clock)
//! - PD2:  SDIO_CMD (Command)

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::sdmmc::{Config as SdmmcConfig, DataBlock, Sdmmc};
use embassy_stm32::{bind_interrupts, peripherals, sdmmc, Config};
use panic_probe as _;
use stm32f407_sd_bootloader::sd_card_reader::SdCardReader;

bind_interrupts!(struct Irqs {
    SDIO => sdmmc::InterruptHandler<peripherals::SDIO>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("SD Card Reader Test");

    // Initialize embassy with default configuration
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: embassy_stm32::time::Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL168,
            divp: Some(PllPDiv::DIV2), // 168 MHz
            divq: Some(PllQDiv::DIV7), // 48 MHz for SDIO
            divr: None,
        });
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV4;
        config.rcc.apb2_pre = APBPrescaler::DIV2;
        config.rcc.sys = Sysclk::PLL1_P;
    }

    let p = embassy_stm32::init(config);

    info!("Hardware initialized");

    // Configure SDIO peripheral with 4-bit data bus
    let sdmmc_config = SdmmcConfig::default();

    let sdmmc = Sdmmc::new_4bit(
        p.SDIO,
        Irqs,
        p.DMA2_CH3,
        p.PC12, // clk
        p.PD2,  // cmd
        p.PC8,  // d0
        p.PC9,  // d1
        p.PC10, // d2
        p.PC11, // d3
        sdmmc_config,
    );

    info!("SDIO peripheral configured (4-bit mode)");

    // Initialize SD card reader with timeout protection
    match SdCardReader::init(sdmmc).await {
        Ok(mut reader) => {
            info!("SD card initialized successfully!");

            // Get card information
            match reader.card_info() {
                Ok(info) => {
                    info!(
                        "SD card capacity: {} MB ({} blocks)",
                        info.capacity_mb(),
                        info.block_count
                    );
                }
                Err(e) => {
                    error!("Failed to get card info: {:?}", e);
                }
            }

            // Test reading first block
            let mut block = DataBlock([0u8; 512]);
            match reader.read_block(0, &mut block).await {
                Ok(()) => {
                    info!("Successfully read block 0");
                    info!("First 16 bytes: {:02X}", &block.0[..16]);
                }
                Err(e) => {
                    error!("Failed to read block 0: {:?}", e);
                }
            }

            info!("SD card test completed successfully");
        }
        Err(e) => {
            error!("SD card initialization failed: {:?}", e);
            error!("Please check:");
            error!("  1. SD card is inserted");
            error!("  2. Connections are correct (PC8-PC12, PD2)");
            error!("  3. SD card is formatted as FAT32");
        }
    }

    info!("Test finished");

    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}
