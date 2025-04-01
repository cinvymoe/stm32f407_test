#![no_std]
#![no_main]

use panic_halt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::{peripherals, Config};
use embassy_time::{Duration, Timer};

// 使用Embassy的时间驱动 - TIM2作为时间源
// 时间驱动已在Cargo.toml中通过特性启用

#[embassy_executor::main]

async fn main(spawner: Spawner) {
    // 配置系统
    let config = Config::default();
    let p = embassy_stm32::init(config);
    
    // 启动两个LED闪烁任务
    spawner.spawn(blink_task1(p.PA7, Duration::from_millis(500))).unwrap();
    spawner.spawn(blink_task2(p.PA6, Duration::from_millis(1000))).unwrap();
}

// 第一个LED闪烁任务 - 快速闪烁 (500ms)
#[embassy_executor::task]
async fn blink_task1(
    pin: peripherals::PA7,
    interval: Duration,
) {
    let mut led = Output::new(pin, Level::Low, Speed::Low);
    
    loop {
        led.set_high();
        Timer::after(interval).await;
        led.set_low();
        Timer::after(interval).await;
    }
}

// 第二个LED闪烁任务 - 慢速闪烁 (1000ms)
#[embassy_executor::task]
async fn blink_task2(
    pin: peripherals::PA6,
    interval: Duration,
) {
    let mut led = Output::new(pin, Level::Low, Speed::Low);
    
    loop {
        led.set_high();
        Timer::after(interval).await;
        led.set_low();
        Timer::after(interval).await;
    }
}