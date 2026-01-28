# STM32F407 SD 卡 Bootloader 故障排查指南

本文档提供常见问题的诊断和解决方法。

## 目录

- [LED 状态指示](#led-状态指示)
- [错误码参考](#错误码参考)
- [常见问题](#常见问题)
- [硬件检查](#硬件检查)
- [调试方法](#调试方法)
- [高级诊断](#高级诊断)

## LED 状态指示

### LED2 (PA6) - 心跳指示

| 状态 | 含义 |
|------|------|
| 慢速闪烁 (500ms) | Bootloader 正常运行 |
| 熄灭 | Bootloader 未运行或已跳转到应用 |

### LED1 (PA7) - 状态指示

| 状态 | 含义 | 说明 |
|------|------|------|
| 熄灭 | 空闲状态 | 等待操作或已跳转到应用 |
| 中速闪烁 (200ms) | 读取固件中 | 正在从 SD 卡读取 firmware.bin |
| 慢速闪烁 (500ms) | 烧写固件中 | 正在写入 Flash 存储器 |
| 常亮 | 升级成功 | 固件升级完成，即将重启 |
| 快速闪烁 (100ms) | 升级失败 | 发生错误，检查日志或错误码 |

## 错误码参考

错误码可通过 RTT 日志查看，或通过 LED 闪烁模式判断。

### SD 卡错误 (0x10-0x1F)

| 错误码 | 名称 | 描述 | 解决方法 |
|--------|------|------|----------|
| 0x10 | SdInitFailed | SD 卡初始化失败 | 检查 SD 卡连接、供电、格式 |
| 0x11 | SdMountFailed | FAT32 挂载失败 | 确认 SD 卡格式为 FAT32 |
| 0x12 | SdFileNotFound | 固件文件未找到 | 确认 firmware.bin 在根目录 |
| 0x13 | SdReadError | SD 卡读取错误 | 检查 SD 卡是否损坏 |
| 0x14 | SdTimeout | SD 卡操作超时 | 更换高速 SD 卡，检查接线 |

### Flash 错误 (0x20-0x2F)

| 错误码 | 名称 | 描述 | 解决方法 |
|--------|------|------|----------|
| 0x20 | FlashEraseFailed | Flash 擦除失败 | 硬件故障，联系技术支持 |
| 0x21 | FlashWriteFailed | Flash 写入失败 | 检查电源稳定性 |
| 0x22 | FlashVerifyFailed | Flash 验证失败 | 重试升级，检查电源 |
| 0x23 | FlashInvalidAddress | 无效 Flash 地址 | 固件配置错误，重新编译 |

### 固件验证错误 (0x30-0x3F)

| 错误码 | 名称 | 描述 | 解决方法 |
|--------|------|------|----------|
| 0x30 | FirmwareTooLarge | 固件过大 | 固件必须 ≤448KB |
| 0x31 | FirmwareCrcMismatch | CRC 校验失败 | 固件损坏，重新生成 |
| 0x32 | FirmwareInvalidFormat | 固件格式无效 | 使用正确的 .bin 格式 |
| 0x33 | FirmwareSizeMismatch | 固件大小不匹配 | 重新生成固件 |

### 状态错误 (0x40-0x4F)

| 错误码 | 名称 | 描述 | 解决方法 |
|--------|------|------|----------|
| 0x40 | StateInvalidMagic | 启动状态魔数无效 | 重新烧写 bootloader |
| 0x41 | StateCrcMismatch | 状态 CRC 不匹配 | 重新烧写 bootloader |
| 0x42 | StateReadFailed | 状态读取失败 | Flash 硬件故障 |
| 0x43 | StateWriteFailed | 状态写入失败 | Flash 硬件故障 |
| 0x44 | StateInvalidSlot | 无效分区值 | 重新烧写 bootloader |
| 0x45 | StateInvalidSize | 状态大小无效 | 重新烧写 bootloader |

### 超时错误 (0x50-0x5F)

| 错误码 | 名称 | 描述 | 解决方法 |
|--------|------|------|----------|
| 0x50 | TimeoutGeneral | 操作超时 | 检查硬件连接和性能 |

## 常见问题

### 1. LED1 快速闪烁，无法升级

**可能原因：**
- SD 卡未插入或接触不良
- SD 卡格式不是 FAT32
- firmware.bin 文件不存在或名称错误
- 固件文件损坏或过大

**解决步骤：**

1. **检查 SD 卡**
   ```bash
   # 在电脑上检查 SD 卡
   # Windows: 右键 -> 属性 -> 文件系统应显示 FAT32
   # Linux: lsblk -f
   # macOS: diskutil info /dev/diskX
   ```

2. **验证文件**
   ```bash
   # 确认文件存在
   ls -lh /path/to/sdcard/firmware.bin
   
   # 检查文件大小
   # 应该 ≤ 458752 字节 (448KB)
   ```

3. **重新格式化 SD 卡**
   ```bash
   # Linux
   sudo mkfs.vfat -F 32 /dev/sdX1
   
   # macOS
   diskutil eraseDisk FAT32 SDCARD /dev/diskX
   
   # Windows: 使用磁盘管理工具格式化为 FAT32
   ```

4. **重新生成固件**
   ```bash
   ./scripts/generate_firmware.sh led_blink_app
   cp target/firmware/firmware.bin /path/to/sdcard/
   ```

### 2. 升级后设备无法启动

**可能原因：**
- 新固件链接地址错误
- 新固件缺少向量表
- 新固件有严重 bug

**解决步骤：**

1. **自动回滚**
   - Bootloader 会自动检测启动失败
   - 最多尝试 3 次后回滚到旧版本
   - 等待约 10 秒，设备应自动恢复

2. **检查固件链接地址**
   
   应用程序的 `memory.x` 应该是：
   ```
   MEMORY
   {
     FLASH : ORIGIN = 0x08010000, LENGTH = 448K
     RAM : ORIGIN = 0x20000000, LENGTH = 128K
   }
   ```

3. **验证向量表**
   ```bash
   # 检查固件前 8 字节
   hexdump -C firmware.bin | head -1
   
   # 应该看到：
   # 00000000  xx xx xx 20 xx xx 01 08  ...
   #           ^^^^^^^^^ ^^^^^^^^^^^
   #           栈指针    复位向量
   # 栈指针应在 0x20000000-0x20020000 范围
   # 复位向量应在 0x08010000-0x08080000 范围
   ```

4. **手动恢复**
   
   如果自动回滚失败，重新烧写 bootloader：
   ```bash
   probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader
   ```

### 3. 按键无法触发升级

**可能原因：**
- 按键未正确连接
- 按键时序不对
- 按键引脚配置错误

**解决步骤：**

1. **检查硬件连接**
   - PE4 引脚应连接到按键
   - 按键另一端接地 (GND)
   - 按下时 PE4 应为低电平

2. **测试按键**
   ```bash
   # 使用万用表测试
   # 按键未按下：PE4 应为 3.3V (上拉)
   # 按键按下：PE4 应为 0V (接地)
   ```

3. **检查时序**
   - 必须在设备启动时按住按键
   - 保持按下至少 2 秒
   - 看到 LED2 开始闪烁后可以释放

4. **调整检测窗口**
   
   如果需要更长的检测时间，修改 `src/config.rs`：
   ```rust
   pub const BUTTON_CHECK_WINDOW_MS: u64 = 5000; // 改为 5 秒
   ```

### 4. SD 卡读取速度慢

**可能原因：**
- SD 卡速度等级低
- SDIO 接线过长或有干扰
- 供电不稳定

**解决步骤：**

1. **使用高速 SD 卡**
   - 推荐 Class 10 或 UHS-I 卡
   - 避免使用老旧或低速卡

2. **检查接线**
   - SDIO 信号线应尽量短 (< 10cm)
   - 使用屏蔽线或双绞线
   - 避免与高频信号并行走线

3. **稳定供电**
   - SD 卡需要稳定的 3.3V 供电
   - 添加去耦电容 (100nF + 10uF)
   - 检查电源纹波

4. **降低时钟频率**
   
   如果问题持续，修改 `src/sd_card_reader.rs`：
   ```rust
   // 将初始化时钟从 400kHz 降低到 200kHz
   sdmmc.init_card(Hertz::khz(200)).await
   ```

### 5. 升级过程中断电

**可能原因：**
- 电源不稳定
- 电池电量不足
- 意外断电

**恢复步骤：**

1. **重新上电**
   - Bootloader 会检测到升级未完成
   - 自动回滚到旧版本
   - 设备应能正常启动

2. **重试升级**
   - 确保电源稳定
   - 重新执行升级流程

3. **最坏情况**
   
   如果两个分区都损坏：
   ```bash
   # 重新烧写 bootloader 和初始应用
   probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader
   ```

### 6. 无 SD 卡时无法启动

**可能原因：**
- 应用分区为空或损坏
- Bootloader 配置错误

**解决步骤：**

1. **检查应用分区**
   - 确认至少一个分区有有效应用
   - 初次使用需要先烧写初始应用

2. **烧写初始应用**
   ```bash
   # 使用 probe-rs 直接烧写到 Slot A
   probe-rs download --chip STM32F407VGTx \
     --format bin \
     --base-address 0x08010000 \
     firmware.bin
   ```

3. **验证启动**
   - 移除 SD 卡
   - 不按升级按键
   - 重启设备
   - 应直接跳转到应用

## 硬件检查

### SD 卡模块检查清单

- [ ] 供电电压：3.3V ±5%
- [ ] 所有信号线已连接 (D0-D3, CLK, CMD)
- [ ] 接地线已连接
- [ ] 信号线长度 < 10cm
- [ ] 去耦电容已安装 (100nF + 10uF)
- [ ] SD 卡已正确插入
- [ ] SD 卡格式为 FAT32
- [ ] SD 卡容量 4GB-32GB (SDHC)

### 按键检查清单

- [ ] PE4 连接到按键一端
- [ ] 按键另一端接地
- [ ] 按键未按下时 PE4 为高电平 (3.3V)
- [ ] 按键按下时 PE4 为低电平 (0V)
- [ ] 无需外部上拉电阻 (内部上拉已启用)

### LED 检查清单

- [ ] PA7 (LED1) 连接到 LED 正极
- [ ] PA6 (LED2) 连接到 LED 正极
- [ ] LED 负极通过限流电阻 (330Ω-1kΩ) 接地
- [ ] LED 极性正确
- [ ] LED 能正常点亮

## 调试方法

### 使用 RTT 日志

RTT (Real-Time Transfer) 提供实时日志输出，无需 UART。

1. **启动 RTT 会话**
   ```bash
   probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader
   ```

2. **查看日志**
   ```
   INFO  STM32F407 Bootloader starting...
   INFO  Checking for upgrade button (2 second window)...
   INFO  No upgrade button detected
   INFO  Checking application validity...
   INFO  Slot A contains valid application
   INFO  Jumping to application at 0x08010000
   ```

3. **日志级别**
   
   设置环境变量调整日志详细程度：
   ```bash
   export DEFMT_LOG=trace  # 最详细
   export DEFMT_LOG=debug
   export DEFMT_LOG=info   # 默认
   export DEFMT_LOG=warn
   export DEFMT_LOG=error  # 最简洁
   ```

### 使用 GDB 调试

1. **启动 OpenOCD**
   ```bash
   openocd -f openocd.cfg
   ```

2. **连接 GDB**
   ```bash
   arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader
   (gdb) target remote :3333
   (gdb) load
   (gdb) monitor reset halt
   ```

3. **设置断点**
   ```gdb
   (gdb) break main
   (gdb) break BootManager::perform_upgrade
   (gdb) continue
   ```

4. **检查变量**
   ```gdb
   (gdb) print firmware_size
   (gdb) print crc32
   (gdb) x/16x 0x08010000  # 查看 Slot A 内容
   ```

### 检查 Flash 内容

1. **读取 Flash**
   ```bash
   probe-rs read --chip STM32F407VGTx 0x08010000 512 > slot_a.bin
   probe-rs read --chip STM32F407VGTx 0x08080000 512 > slot_b.bin
   ```

2. **检查向量表**
   ```bash
   hexdump -C slot_a.bin | head -2
   ```

3. **验证 CRC**
   ```bash
   crc32 slot_a.bin
   ```

## 高级诊断

### 分析启动失败

如果应用无法启动，检查以下内容：

1. **向量表偏移 (VTOR)**
   
   应用应设置 VTOR 到正确地址：
   ```rust
   // 在应用的 main 函数开始处
   unsafe {
       let scb = &*cortex_m::peripheral::SCB::PTR;
       scb.vtor.write(0x08010000); // Slot A 地址
   }
   ```

2. **栈指针**
   
   向量表第一个字 (4 字节) 应该是有效的栈指针：
   - 范围：0x20000000 - 0x20020000 (128KB RAM)
   - 通常指向 RAM 末尾：0x20020000

3. **复位向量**
   
   向量表第二个字应该是复位处理函数地址：
   - 范围：0x08010000 - 0x08080000 (Slot A)
   - 最低位应为 1 (Thumb 模式)

### 性能分析

测量升级时间：

1. **启用时间戳**
   
   在关键点添加日志：
   ```rust
   let start = embassy_time::Instant::now();
   // ... 操作 ...
   let elapsed = start.elapsed();
   defmt::info!("Operation took {} ms", elapsed.as_millis());
   ```

2. **预期时间**
   - SD 卡初始化：< 1 秒
   - 读取 448KB 固件：< 5 秒 (@ 100KB/s)
   - 擦除 Slot B：< 2 秒
   - 写入 448KB 固件：< 9 秒 (@ 50KB/s)
   - 总计：< 15 秒

3. **性能优化**
   - 使用 DMA 加速 Flash 操作
   - 增大缓冲区大小 (当前 4KB)
   - 使用更快的 SD 卡

### 回滚机制测试

测试自动回滚功能：

1. **创建故意失败的固件**
   ```rust
   #[entry]
   fn main() -> ! {
       // 不标记自身为有效，导致回滚
       loop {
           cortex_m::asm::wfi();
       }
   }
   ```

2. **观察回滚**
   - 升级到故意失败的固件
   - 设备重启 3 次尝试启动
   - 第 4 次重启应回滚到旧版本
   - 旧版本应正常运行

3. **验证回滚计数器**
   ```bash
   # 通过 RTT 日志查看
   WARN  Application failed to start, rolling back
   INFO  Rollback count: 1
   INFO  Rollback count: 2
   INFO  Rollback count: 3
   ERROR Max rollback attempts reached
   INFO  Rolling back to Slot A
   ```

## 获取帮助

如果以上方法都无法解决问题：

1. **收集信息**
   - 完整的 RTT 日志
   - LED 闪烁模式
   - 硬件连接照片
   - 固件生成命令和输出

2. **提交 Issue**
   - 访问项目 GitHub 仓库
   - 创建新 Issue
   - 附上收集的信息

3. **社区支持**
   - 查看已有 Issues
   - 参考示例代码
   - 阅读设计文档

## 附录

### 快速诊断流程图

```
设备无法启动
    ├─ LED2 不闪烁
    │   ├─ 检查电源
    │   ├─ 检查 bootloader 是否烧写
    │   └─ 重新烧写 bootloader
    │
    ├─ LED1 快速闪烁
    │   ├─ 检查 SD 卡
    │   ├─ 检查固件文件
    │   └─ 查看 RTT 日志获取错误码
    │
    └─ LED 都不亮
        ├─ 检查 LED 连接
        ├─ 检查 LED 极性
        └─ 检查限流电阻
```

### 常用命令速查

```bash
# 编译 bootloader
cargo build --release

# 生成固件
./scripts/generate_firmware.sh led_blink_app

# 烧写 bootloader
probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader

# 查看 RTT 日志
probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/stm32f407-sd-bootloader

# 读取 Flash
probe-rs read --chip STM32F407VGTx 0x08010000 512 > slot_a.bin

# 格式化 SD 卡 (Linux)
sudo mkfs.vfat -F 32 /dev/sdX1

# 计算 CRC32
crc32 firmware.bin
```

---

**最后更新：** 2026-01-28  
**版本：** 1.0.0
