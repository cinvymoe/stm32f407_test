# STM32F407 SD 卡 Bootloader 升级系统

基于 embassy-boot 框架的 STM32F407 bootloader 系统，支持从 SD 卡读取固件并进行安全升级，具备完整的回滚机制。

## 特性

- ✅ **双分区架构**：Active/Update 分区设计，支持原子切换
- ✅ **安全升级**：CRC32 完整性校验，防止损坏固件烧写
- ✅ **自动回滚**：升级失败时自动恢复到稳定版本（最多 3 次尝试）
- ✅ **用户友好**：LED 状态指示，按键触发升级
- ✅ **高性能**：SDIO 4-bit 模式，快速固件传输（≤15 秒完成 512KB 固件升级）
- ✅ **错误恢复**：完善的超时保护和错误处理机制

## 硬件要求

### 开发板
- STM32F407VG 开发板（1MB Flash, 128KB RAM）
- 或其他 STM32F407 系列芯片

### 外设连接

#### SD 卡模块（SDIO 接口）
| 引脚 | 功能 | 说明 |
|------|------|------|
| PC8  | SDIO_D0 | 数据线 0 |
| PC9  | SDIO_D1 | 数据线 1 |
| PC10 | SDIO_D2 | 数据线 2 |
| PC11 | SDIO_D3 | 数据线 3 |
| PC12 | SDIO_CK | 时钟线 |
| PD2  | SDIO_CMD | 命令线 |
| GND  | GND | 地线 |
| 3.3V | VCC | 电源（3.3V） |

#### 按键
| 引脚 | 功能 | 说明 |
|------|------|------|
| PE4  | 升级触发按键 | 上拉输入，低电平有效 |

#### LED 指示灯
| 引脚 | 功能 | 说明 |
|------|------|------|
| PA7  | LED1 - 状态指示 | 显示升级状态 |
| PA6  | LED2 - 心跳指示 | Bootloader 运行指示 |

### 接线图

```
STM32F407          SD Card Module
---------          --------------
PC8  ----------->  D0
PC9  ----------->  D1
PC10 ----------->  D2
PC11 ----------->  D3
PC12 ----------->  CLK
PD2  ----------->  CMD
GND  ----------->  GND
3.3V ----------->  VCC

PE4  ----------->  Button (to GND)
PA7  ----------->  LED1 (+ 限流电阻)
PA6  ----------->  LED2 (+ 限流电阻)
```

## Flash 分区布局

```
地址范围                    大小      用途
0x08000000 - 0x08010000    64KB     Bootloader
0x08010000 - 0x08090000    512KB    Application Slot A (Active)
0x08090000 - 0x08110000    512KB    Application Slot B (Update)
0x08110000 - 0x08120000    64KB     Boot State
```

## 编译和烧写

### 环境准备

1. **安装 Rust 工具链**
```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 ARM Cortex-M 目标
rustup target add thumbv7em-none-eabihf

# 安装 probe-rs（用于烧写和调试）
cargo install probe-rs --features cli
```

2. **安装依赖工具**
```bash
# 安装 cargo-binutils（用于生成 .bin 文件）
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

### 编译 Bootloader

```bash
# 编译 release 版本
cargo build --release

# 生成 .bin 文件
cargo objcopy --release -- -O binary bootloader.bin
```

### 烧写 Bootloader

使用 probe-rs 烧写：

```bash
# 烧写到 Flash 起始地址
probe-rs run --chip STM32F407VGTx --protocol swd target/thumbv7em-none-eabihf/release/stm32-sd-bootloader
```

或使用 OpenOCD：

```bash
# 启动 OpenOCD
openocd -f openocd.cfg

# 在另一个终端使用 GDB
arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/stm32-sd-bootloader
(gdb) target remote :3333
(gdb) load
(gdb) monitor reset halt
(gdb) continue
```

### 编译应用程序

应用程序需要配置正确的链接地址（0x08010000）：

**memory.x** (应用程序)：
```
MEMORY
{
  FLASH : ORIGIN = 0x08010000, LENGTH = 512K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
```

编译并生成固件：

```bash
# 在应用程序目录
cargo build --release

# 生成 firmware.bin
cargo objcopy --release -- -O binary firmware.bin

# 计算 CRC32（可选，用于验证）
crc32 firmware.bin
```

## 使用说明

### 固件升级流程

1. **准备固件文件**
   - 将编译好的 `firmware.bin` 复制到 SD 卡根目录
   - 确保文件名为 `firmware.bin`（区分大小写）
   - 固件大小不超过 512KB

2. **插入 SD 卡**
   - 将 SD 卡插入 SD 卡模块
   - 确保 SD 卡格式为 FAT32

3. **触发升级**
   - 按住 PE4 按键
   - 重启设备（按复位按钮或重新上电）
   - 保持按键按下约 2 秒

4. **观察升级过程**
   - LED1 中速闪烁（200ms）：正在读取固件
   - LED1 慢速闪烁（500ms）：正在烧写固件
   - LED1 常亮：升级成功
   - LED1 快速闪烁（100ms）：升级失败

5. **完成升级**
   - 升级成功后，设备会自动重启
   - 新固件开始运行
   - 可以移除 SD 卡

### LED 状态说明

| LED 模式 | 含义 | 说明 |
|---------|------|------|
| LED2 慢闪 | Bootloader 运行中 | 心跳指示 |
| LED1 中速闪烁（200ms） | 读取固件 | 正在从 SD 卡读取 |
| LED1 慢速闪烁（500ms） | 烧写固件 | 正在写入 Flash |
| LED1 常亮 | 升级成功 | 2 秒后自动重启 |
| LED1 快速闪烁（100ms） | 升级失败 | 检查 SD 卡和固件 |

### 正常启动（不升级）

如果不需要升级固件：
1. 不按 PE4 按键
2. 直接重启设备
3. Bootloader 会自动跳转到应用程序

### 回滚机制

如果新固件无法正常启动：
1. Bootloader 会自动检测应用启动失败
2. 自动回滚到旧版本固件
3. 最多尝试 3 次启动新固件
4. 超过 3 次后永久回滚到稳定版本

## 调试和日志

### RTT 日志

Bootloader 使用 defmt + RTT 输出详细日志：

```bash
# 使用 probe-rs 查看日志
probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32-sd-bootloader

# 或使用 defmt-print
cargo install defmt-print
probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32-sd-bootloader | defmt-print -e target/thumbv7em-none-eabihf/release/stm32-sd-bootloader
```

### 日志级别

在 `Cargo.toml` 中配置日志级别：

```toml
[profile.dev.package.defmt]
opt-level = 3

[profile.release.package.defmt]
opt-level = 3
```

环境变量：
```bash
# 设置日志级别
export DEFMT_LOG=trace  # trace, debug, info, warn, error
```

## 故障排查

详细的故障排查指南请参考 [docs/troubleshooting.md](docs/troubleshooting.md)

### 常见问题

**Q: LED1 快速闪烁，无法升级**
- 检查 SD 卡是否正确插入
- 确认 SD 卡格式为 FAT32
- 确认固件文件名为 `firmware.bin`
- 检查固件大小是否超过 512KB

**Q: 升级后设备无法启动**
- 系统会自动回滚到旧版本
- 检查新固件的链接地址是否正确（0x08010000）
- 确认固件包含正确的向量表

**Q: 按键无法触发升级**
- 检查 PE4 引脚连接
- 确保按键在重启时按下并保持 2 秒
- 检查按键是否正确接地

**Q: SD 卡读取速度慢**
- 使用高速 SD 卡（Class 10 或 UHS-I）
- 检查 SDIO 接线是否过长或有干扰
- 确认供电稳定（3.3V）

## 性能指标

- **固件读取速度**：≥100KB/s
- **固件烧写速度**：≥50KB/s
- **完整升级时间**：≤15 秒（512KB 固件）
- **Bootloader 启动时间**：≤500ms
- **升级成功率**：≥99%（正常条件下）

## 项目结构

```
.
├── src/
│   ├── main.rs              # 主程序入口
│   ├── lib.rs               # 库接口
│   ├── boot_manager.rs      # 启动管理器
│   ├── boot_state.rs        # 启动状态管理
│   ├── flash_manager.rs     # Flash 管理器
│   ├── sd_card_reader.rs    # SD 卡读取器
│   ├── firmware_verifier.rs # 固件校验器
│   ├── led_indicator.rs     # LED 指示器
│   ├── button_input.rs      # 按键输入
│   ├── error.rs             # 错误类型定义
│   ├── error_handler.rs     # 错误处理
│   ├── rollback.rs          # 回滚机制
│   ├── timeout.rs           # 超时处理
│   └── config.rs            # 配置常量
├── examples/
│   ├── sd_card_test.rs      # SD 卡测试示例
│   └── fat32_firmware_test.rs # FAT32 固件读取测试
├── docs/
│   ├── project-overview.md  # 项目概述
│   └── troubleshooting.md   # 故障排查指南
├── scripts/
│   └── git-commit-template.sh # Git 提交模板
├── .kiro/
│   └── specs/               # 规格文档
├── Cargo.toml               # 项目配置
├── memory.x                 # 链接脚本
├── openocd.cfg              # OpenOCD 配置
└── README.md                # 本文件
```

## API 文档

生成 API 文档：

```bash
cargo doc --open
```

详细的 API 文档请参考生成的 rustdoc 文档。

## 示例程序

项目包含以下示例：

1. **SD 卡测试** (`examples/sd_card_test.rs`)
   - 测试 SDIO 接口初始化
   - 测试 FAT32 文件系统挂载

2. **固件读取测试** (`examples/fat32_firmware_test.rs`)
   - 测试固件文件读取
   - 测试 CRC32 计算

运行示例：

```bash
cargo run --example sd_card_test --release
cargo run --example fat32_firmware_test --release
```

## 开发指南

### 添加新功能

1. 在对应模块添加代码
2. 更新 API 文档注释
3. 添加单元测试
4. 更新 README.md

### 代码风格

项目遵循 Rust 标准代码风格：

```bash
# 格式化代码
cargo fmt

# 检查代码
cargo clippy -- -D warnings
```

### 提交规范

遵循 Conventional Commits 规范，详见 `.kiro/steering/git-commit-standards.md`

## 依赖项

主要依赖：

- `embassy-boot-stm32`: Bootloader 框架
- `embassy-stm32`: STM32 HAL
- `embassy-executor`: 异步执行器
- `embedded-sdmmc-rs`: FAT32 文件系统
- `crc`: CRC32 校验
- `defmt`: 日志系统
- `cortex-m-rt`: Cortex-M 运行时

完整依赖列表请参考 `Cargo.toml`

## 许可证

本项目采用 MIT 或 Apache-2.0 双许可证。

## 贡献

欢迎提交 Issue 和 Pull Request！

## 参考资料

- [embassy-boot 文档](https://embassy.dev/book/#_bootloader)
- [STM32F407 参考手册](https://www.st.com/resource/en/reference_manual/dm00031020.pdf)
- [embedded-sdmmc-rs](https://github.com/rust-embedded-community/embedded-sdmmc-rs)
- [embassy-stm32 示例](https://github.com/embassy-rs/embassy/tree/main/examples/stm32f4)

## 联系方式

如有问题或建议，请提交 Issue。

---

**注意**：本项目处于开发阶段，部分功能可能尚未完全实现。使用前请仔细阅读文档并进行充分测试。
