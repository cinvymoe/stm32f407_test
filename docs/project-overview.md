# STM32F407 嵌入式 Rust 项目

## 项目概述

基于 STM32F407VG 微控制器的 Rust 嵌入式开发项目，使用 Embassy 异步框架实现 GPIO 控制和任务调度。

## 技术栈

- **语言**: Rust 2021 Edition
- **目标平台**: STM32F407VG (ARM Cortex-M4F)
- **异步框架**: Embassy (executor + time + stm32 HAL)
- **调试工具**: OpenOCD / probe-rs
- **日志系统**: defmt + RTT

## 核心依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| embassy-executor | 0.7.0 | 异步任务执行器 |
| embassy-time | 0.4.0 | 异步定时器 |
| embassy-stm32 | 0.2.0 | STM32 HAL 抽象层 |
| cortex-m-rt | 0.7.3 | Cortex-M 运行时 |
| defmt | 1.0 | 高效日志框架 |

## 硬件资源

### 内存布局
- **Flash**: 0x08000000 - 1024KB
- **RAM**: 0x20000000 - 128KB

### GPIO 配置
- **PA6**: LED2 输出（1Hz 闪烁）
- **PA7**: LED1 输出（按键控制）
- **PE4**: 按键输入（控制 LED1）
- **PE3**: 按键输入（预留，代码中未启用）

## 功能特性

### 1. 异步任务架构
- 使用 Embassy executor 管理多个并发任务
- 基于信号量的任务间通信

### 2. LED 控制
- **LED1 (PA7)**: 通过 PE4 按键切换状态
- **LED2 (PA6)**: 自动 1 秒闪烁

### 3. 按键处理
- 上拉输入模式
- 20ms 软件消抖
- 边沿检测（下降沿触发）

### 4. 日志系统
- defmt 格式化日志
- RTT 实时传输
- 支持切换到纯文本 RTT 模式

## 开发工具链

### 编译目标
```bash
thumbv7em-none-eabihf
```

### 调试方式
1. **OpenOCD + GDB**: 传统调试方案
2. **probe-rs**: 现代 Rust 调试工具（当前配置）

### 烧录方法
- VSCode Task: Flash (DAPLink)
- 命令行: `probe-rs run --chip STM32F407VG`

## 项目结构

```
stm32f407_test/
├── src/
│   └── main.rs          # 主程序（任务定义）
├── docs/                # 项目文档
├── scripts/             # 工具脚本
├── .cargo/
│   └── config.toml      # Cargo 配置
├── .vscode/             # VSCode 配置
├── memory.x             # 链接器内存布局
├── openocd.cfg          # OpenOCD 配置
├── openocd.gdb          # GDB 启动脚本
└── Cargo.toml           # 项目依赖
```

## 编译配置

### Release 优化
- 优化级别: `s` (体积优化)
- LTO: 启用
- 保留调试信息

### Dev 配置
- 优化级别: `0`
- 关键依赖强制 O3 优化（避免多重 panic handler）

## RTT 日志方案

### 方案 A: defmt + RTT（当前）
- 二进制格式，需 `defmt-print` 解码
- 高效、低开销

### 方案 B: rtt-target（可选）
- 纯文本输出，直接可读
- 通过 `nc localhost 9090` 查看
- 详见 README.md 切换步骤

## 快速开始

### 1. 安装依赖
```bash
rustup target add thumbv7em-none-eabihf
cargo install probe-rs
```

### 2. 编译
```bash
cargo build
```

### 3. 烧录运行
```bash
cargo run
```

### 4. 查看日志
```bash
# defmt 模式
defmt-print -e target/thumbv7em-none-eabihf/debug/stm32f407_test

# 或使用 probe-rs
probe-rs run --chip STM32F407VG
```

## 注意事项

1. **LED 电平逻辑**: 当前代码中 `set_high()` 对应 "OFF"，`set_low()` 对应 "ON"，需根据硬件调整
2. **按键 PE3**: 代码中已定义 `button2_task` 但未在 main 中启动
3. **时间驱动**: 使用 `time-driver-any` 特性，自动选择可用定时器

## 扩展方向

- [ ] 添加 UART 通信
- [ ] 实现 I2C/SPI 外设驱动
- [ ] 集成传感器模块
- [ ] 添加低功耗模式
- [ ] 实现 DMA 传输

## 参考资源

- [Embassy 文档](https://embassy.dev/)
- [STM32F407 数据手册](https://www.st.com/resource/en/datasheet/stm32f407vg.pdf)
- [probe-rs 文档](https://probe.rs/)
