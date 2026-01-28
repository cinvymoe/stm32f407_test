#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::sdmmc::{Config as SdmmcConfig, Sdmmc};
use embassy_stm32::{bind_interrupts, peripherals, sdmmc, Config};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use panic_probe as _;
use stm32f407_sd_bootloader::boot_manager::BootManager;
use stm32f407_sd_bootloader::button_input::ButtonInput;
use stm32f407_sd_bootloader::config;
use stm32f407_sd_bootloader::led_indicator::LedIndicator;

bind_interrupts!(struct Irqs {
    SDIO => sdmmc::InterruptHandler<peripherals::SDIO>;
});

// Global watchdog instance for feeding during long operations
type WatchdogType = embassy_stm32::wdg::IndependentWatchdog<'static, peripherals::IWDG>;
static WATCHDOG: Mutex<CriticalSectionRawMutex, Option<WatchdogType>> = Mutex::new(None);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("STM32F407 SD Card Bootloader v1.0.0");
    info!("Initializing hardware...");

    // Configure embassy with STM32F407 clock settings
    let mut embassy_config = Config::default();
    {
        use embassy_stm32::rcc::*;
        // Configure HSE (8 MHz external oscillator)
        embassy_config.rcc.hse = Some(Hse {
            freq: embassy_stm32::time::Hertz(8_000_000),
            mode: HseMode::Oscillator,
        });
        // Configure PLL for 168 MHz system clock
        embassy_config.rcc.pll_src = PllSource::HSE;
        embassy_config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL168,
            divp: Some(PllPDiv::DIV2), // 168 MHz
            divq: Some(PllQDiv::DIV7), // 48 MHz for USB/SDIO
            divr: None,
        });
        embassy_config.rcc.ahb_pre = AHBPrescaler::DIV1;
        embassy_config.rcc.apb1_pre = APBPrescaler::DIV4; // 42 MHz
        embassy_config.rcc.apb2_pre = APBPrescaler::DIV2; // 84 MHz
        embassy_config.rcc.sys = Sysclk::PLL1_P;
    }

    // Initialize STM32 peripherals
    let p = embassy_stm32::init(embassy_config);

    info!("Hardware initialized successfully");
    info!("System clock: 168 MHz");
    info!("APB1 clock: 42 MHz");
    info!("APB2 clock: 84 MHz");

    // Display flash partition configuration
    // Note: Partition layout is compatible with embassy-boot framework
    // - Bootloader: 64KB at 0x08000000
    // - Slot A (Active): 512KB at 0x08010000
    // - Slot B (Update): 512KB at 0x08090000
    // - Boot State: 64KB at 0x08110000
    // Boot state management is implemented in boot_state.rs and flash_manager.rs
    info!("Flash partitions configured:");
    info!(
        "  Bootloader: 0x{:08X} - 0x{:08X} ({}KB)",
        config::FLASH_CONFIG.bootloader_start,
        config::FLASH_CONFIG.bootloader_start + config::FLASH_CONFIG.bootloader_size,
        config::FLASH_CONFIG.bootloader_size / 1024
    );
    info!(
        "  Slot A:     0x{:08X} - 0x{:08X} ({}KB)",
        config::FLASH_CONFIG.slot_a_start,
        config::FLASH_CONFIG.slot_a_start + config::FLASH_CONFIG.slot_a_size,
        config::FLASH_CONFIG.slot_a_size / 1024
    );
    info!(
        "  Slot B:     0x{:08X} - 0x{:08X} ({}KB)",
        config::FLASH_CONFIG.slot_b_start,
        config::FLASH_CONFIG.slot_b_start + config::FLASH_CONFIG.slot_b_size,
        config::FLASH_CONFIG.slot_b_size / 1024
    );
    info!(
        "  State:      0x{:08X} - 0x{:08X} ({}KB)",
        config::FLASH_CONFIG.state_start,
        config::FLASH_CONFIG.state_start + config::FLASH_CONFIG.state_size,
        config::FLASH_CONFIG.state_size / 1024
    );

    // Initialize GPIO peripherals
    info!("Initializing GPIO peripherals...");

    // Initialize button input (PE4, pull-up, active low)
    let button = ButtonInput::new(p.PE4.into());
    info!("Button input initialized (PE4)");

    // Initialize LED indicators
    // PA7: Status LED
    // PA6: Heartbeat LED
    let led = LedIndicator::new(p.PA7.into(), p.PA6.into());
    info!("LED indicators initialized (PA6, PA7)");

    // Initialize Flash in blocking mode (STM32F4 limitation)
    // Note: FlashManager expects async Flash, but we'll use blocking mode
    // and the async operations will block internally
    info!("Initializing Flash manager...");
    use embassy_stm32::flash::Flash;
    let flash = Flash::new_blocking(p.FLASH);
    let flash_manager = stm32f407_sd_bootloader::flash_manager::FlashManager::new(flash);
    info!("Flash manager initialized");

    // Initialize Independent Watchdog (IWDG) for system reliability
    // Watchdog timeout: ~10 seconds (allows time for SD card operations and Flash writes)
    // The watchdog will automatically reset the system if it's not fed within this time
    // Note: The watchdog is fed periodically by a background task
    info!("Initializing watchdog...");
    use embassy_stm32::wdg::IndependentWatchdog;
    let mut watchdog = IndependentWatchdog::new(p.IWDG, 10_000_000); // 10 second timeout
    watchdog.unleash();

    // Store watchdog in global mutex
    {
        let mut wdg = WATCHDOG.lock().await;
        *wdg = Some(watchdog);
    }
    info!("Watchdog initialized (10s timeout)");

    // Spawn watchdog feeding task to prevent system reset during long operations
    _spawner.spawn(watchdog_feeder_task()).ok();

    // Initialize SDMMC for SD card access (will be passed to boot manager)
    info!("Preparing SDMMC peripheral...");
    let sdmmc_config = SdmmcConfig::default();

    let sdmmc = Sdmmc::new_4bit(
        p.SDIO,
        Irqs,
        p.DMA2_CH3,
        p.PC12, // CK
        p.PD2,  // CMD
        p.PC8,  // D0
        p.PC9,  // D1
        p.PC10, // D2
        p.PC11, // D3
        sdmmc_config,
    );
    info!("SDMMC peripheral configured");

    // Create and initialize BootManager
    info!("Creating boot manager...");
    let mut boot_manager = BootManager::new(flash_manager, button, led);

    info!("Bootloader initialization complete");
    info!("Starting bootloader main loop...");

    // Run bootloader main loop (never returns)
    boot_manager.run(Some(sdmmc)).await;
}

/// Watchdog feeder task
///
/// This task runs in the background and periodically feeds the watchdog
/// to prevent system reset during long operations like SD card reads
/// and Flash writes.
///
/// The task feeds the watchdog every 5 seconds (half of the 10 second timeout)
/// to ensure the system doesn't reset during normal operation.
#[embassy_executor::task]
async fn watchdog_feeder_task() {
    use embassy_time::{Duration, Timer};

    loop {
        // Wait 5 seconds (half of watchdog timeout)
        Timer::after(Duration::from_secs(5)).await;

        // Feed the watchdog
        let mut wdg = WATCHDOG.lock().await;
        if let Some(watchdog) = wdg.as_mut() {
            watchdog.pet();
            defmt::trace!("Watchdog fed");
        }
    }
}
