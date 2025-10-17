#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::{peripherals, Config};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use panic_halt as _;
use static_cell::StaticCell;

// 使用Embassy的时间驱动 - TIM2作为时间源
// 时间驱动已在Cargo.toml中通过特性启用
// 创建两个全局信号分别用于控制两个LED状态
static LED1_CONTROL: StaticCell<Signal<CriticalSectionRawMutex, bool>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Program start");

    // 配置系统
    let config = Config::default();
    let p = embassy_stm32::init(config);

    // 初始化LED控制信号
    let led1_signal = LED1_CONTROL.init(Signal::new());

    // 启动按键监控任务 - PE4控制PA7，PE3控制PA6
    spawner.spawn(button1_task(p.PE4, led1_signal)).unwrap();

    // 启动LED控制任务
    spawner
        .spawn(led1_control_task(p.PA7, led1_signal))
        .unwrap();
    spawner.spawn(led2_control_task(p.PA6)).unwrap();
}

// PE4按键监控任务 - 控制PA7 LED
#[embassy_executor::task]
async fn button1_task(
    pin: peripherals::PE4,
    signal: &'static Signal<CriticalSectionRawMutex, bool>,
) {
    let button = Input::new(pin, Pull::Up);
    let mut led_state = false;
    let mut prev_state = true; // 上拉模式下，未按下时为高电平

    loop {
        let current_state = button.is_high();

        // 检测按键按下（从高电平变为低电平）
        if prev_state && !current_state {
            // 按键按下，切换LED状态
            led_state = !led_state;
            signal.signal(led_state);
            info!("LED1 state changed: {}", led_state);
        }
        // info!("LED1 state: {}", led_state);

        prev_state = current_state;
        Timer::after(Duration::from_millis(20)).await; // 消抖延时
    }
}

// PE3按键监控任务 - 控制PA6 LED
#[embassy_executor::task]
async fn button2_task(
    pin: peripherals::PE3,
    signal: &'static Signal<CriticalSectionRawMutex, bool>,
) {
    let button = Input::new(pin, Pull::Up);
    let mut led_state = false;
    let mut prev_state = true; // 上拉模式下，未按下时为高电平

    loop {
        let current_state = button.is_high();

        // 检测按键按下（从高电平变为低电平）
        if prev_state && !current_state {
            // 按键按下，切换LED状态
            led_state = !led_state;
            signal.signal(led_state);
        }

        prev_state = current_state;
        Timer::after(Duration::from_millis(20)).await; // 消抖延时
    }
}

// PA7 LED控制任务
#[embassy_executor::task]
async fn led1_control_task(
    pin: peripherals::PA7,
    signal: &'static Signal<CriticalSectionRawMutex, bool>,
) {
    let mut led = Output::new(pin, Level::Low, Speed::Low);

    loop {
        // 等待信号变化
        let led_on = signal.wait().await;

        if led_on {
            // 打开LED
            led.set_high();
            info!("LED1 OFF");
        } else {
            // 关闭LED
            led.set_low();
            info!("LED1 ON");
        }
    }
}

// PA6 LED控制任务
#[embassy_executor::task]
async fn led2_control_task(pin: peripherals::PA6) {
    let mut led = Output::new(pin, Level::Low, Speed::Low);

    loop {
        led.toggle();
        let state = if led.is_set_high() { "ON " } else { "OFF " };
        info!("LED2 state: {}", state);
        Timer::after(Duration::from_millis(1000)).await;
    }
}

// 注意：原来的button_task和led_control_task已被拆分为独立的任务，实现PE4控制PA7，PE3控制PA6
