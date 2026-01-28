//! FAT32 Firmware File Test Example
//!
//! This example demonstrates the complete FAT32 filesystem operations:
//! - Mounting FAT32 filesystem
//! - Finding firmware.bin file
//! - Reading firmware file contents
//!
//! Hardware connections:
//! - PC8:  SDIO_D0
//! - PC9:  SDIO_D1
//! - PC10: SDIO_D2
//! - PC11: SDIO_D3
//! - PC12: SDIO_CK (Clock)
//! - PD2:  SDIO_CMD (Command)
//!
//! Requirements:
//! - SD card formatted as FAT32
//! - firmware.bin file in root directory

#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::sdmmc::{Config as SdmmcConfig, Sdmmc};
use embassy_stm32::{bind_interrupts, peripherals, sdmmc, Config};
use panic_probe as _;
use stm32f407_sd_bootloader::sd_card_reader::SdCardReader;

bind_interrupts!(struct Irqs {
    SDIO => sdmmc::InterruptHandler<peripherals::SDIO>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("FAT32 Firmware File Test");

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

    // Initialize SD card reader
    let mut reader = match SdCardReader::init(sdmmc).await {
        Ok(reader) => {
            info!("✓ SD card initialized successfully");
            reader
        }
        Err(e) => {
            error!("✗ SD card initialization failed: {:?}", e);
            error!("Please check SD card is inserted and connections are correct");
            loop {
                embassy_time::Timer::after_secs(1).await;
            }
        }
    };

    // Get card information
    match reader.card_info() {
        Ok(info) => {
            info!(
                "✓ SD card: {} MB ({} blocks)",
                info.capacity_mb(),
                info.block_count
            );
        }
        Err(e) => {
            error!("✗ Failed to get card info: {:?}", e);
        }
    }

    // Mount FAT32 filesystem
    match reader.mount().await {
        Ok(()) => {
            info!("✓ FAT32 filesystem mounted successfully");
        }
        Err(e) => {
            error!("✗ Failed to mount FAT32 filesystem: {:?}", e);
            error!("Please ensure SD card is formatted as FAT32");
            loop {
                embassy_time::Timer::after_secs(1).await;
            }
        }
    }

    // Open firmware file
    let mut firmware_file = match reader.open_firmware().await {
        Ok(file) => {
            info!("✓ Found firmware.bin: {} bytes", file.size());
            file
        }
        Err(e) => {
            error!("✗ Failed to open firmware.bin: {:?}", e);
            error!("Please ensure firmware.bin exists in root directory");
            loop {
                embassy_time::Timer::after_secs(1).await;
            }
        }
    };

    // Read firmware file in chunks
    info!("Reading firmware file...");
    let mut buffer = [0u8; 512];
    let mut total_read = 0u32;
    let mut chunk_count = 0u32;

    loop {
        match reader.read(&mut firmware_file, &mut buffer).await {
            Ok(0) => {
                // End of file
                info!("✓ Reached end of file");
                break;
            }
            Ok(bytes_read) => {
                total_read += bytes_read as u32;
                chunk_count += 1;

                if chunk_count % 10 == 0 {
                    let percent = (total_read * 100) / firmware_file.size();
                    info!(
                        "  Read {} bytes ({} / {} bytes, {}%)",
                        bytes_read,
                        total_read,
                        firmware_file.size(),
                        percent
                    );
                }

                // Show first 16 bytes of first chunk
                if chunk_count == 1 {
                    info!("  First 16 bytes: {:02X}", &buffer[..16]);
                }
            }
            Err(e) => {
                error!("✗ Failed to read firmware file: {:?}", e);
                break;
            }
        }
    }

    info!("✓ Firmware file read complete");
    info!("  Total bytes read: {}", total_read);
    info!("  Total chunks: {}", chunk_count);
    info!("  File size: {}", firmware_file.size());

    if total_read == firmware_file.size() {
        info!("✓ All bytes read successfully!");
    } else {
        error!(
            "✗ Mismatch: read {} bytes but file size is {}",
            total_read,
            firmware_file.size()
        );
    }

    info!("Test completed");

    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}
