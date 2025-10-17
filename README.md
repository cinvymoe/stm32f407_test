# STM32F407 Rust项目模板

这是一个使用Rust语言开发STM32F407微控制器的初始模板项目。该模板配置了使用DAPLink进行调试的环境。

## 项目结构

- `src/main.rs`: 主程序源代码，实现了一个简单的LED闪烁功能
- `memory.x`: 定义了STM32F407的内存布局
- `.vscode/`: VSCode配置文件，包含调试和任务设置
- `.cargo/config.toml`: Cargo配置文件，设置编译目标和链接器选项

## 开发环境要求

- Rust和Cargo
- `thumbv7em-none-eabihf` 目标支持: `rustup target add thumbv7em-none-eabihf`
- ARM GCC工具链: `arm-none-eabi-gcc`, `arm-none-eabi-gdb`等
- OpenOCD
- VSCode插件:
  - Rust Analyzer
  - Cortex-Debug

## 使用方法

### 编译项目

```bash
cargo build --target thumbv7em-none-eabihf
```

### 烧录程序

在VSCode中，按`Ctrl+Shift+P`打开命令面板，选择`Tasks: Run Task`，然后选择`Flash (DAPLink)`任务。

### 调试程序

在VSCode中，按`F5`启动调试，或者在调试面板中选择`Debug (DAPLink)`配置并点击开始按钮。

## 硬件连接

- 将DAPLink调试器连接到STM32F407开发板的SWD接口
- 绿色LED连接到PD12引脚

## 注意事项

- 确保安装了所有必要的工具和依赖
- 如果调试时遇到问题，检查DAPLink连接和OpenOCD配置
- 实际使用时，请下载完整的STM32F407.svd文件替换.vscode目录下的简化版文件

## Windows 远程调试开启gdb server
- ./openocd -f interface/picoprobe.cfg -f .\target\stm32f4x.cfg -c "bindto 0.0.0.0"

## 改用纯 RTT 文本打印（rtt-target）

本节整理从 defmt+RTT 切换到纯 RTT 文本打印的完整流程。目标是让主机端无需解码，直接在 RTT 通道看到可读的 ASCII 文本。

### 1. 依赖与代码改动

- 在 `Cargo.toml` 中移除 defmt 相关依赖，加入 `rtt-target` 并启用 `cortex-m` 特性：

  ```toml
  [dependencies]
  # ... 其他依赖
  rtt-target = { version = "0.3", features = ["cortex-m"] }
  ```

- 在 `src/main.rs` 中：
  - 引入 `rtt-target`：`use rtt_target::{rprintln, rtt_init_print};`
  - 在 `main` 开始处初始化：`rtt_init_print!();`
  - 使用 `rprintln!("...")` 代替原来的 defmt `info!(...)` 输出日志。

示例（节选）：

```rust
use rtt_target::{rprintln, rtt_init_print};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    rtt_init_print!();
    rprintln!("Program start");
    // ...
    rprintln!("LED1 ON");
}
```

### 2. 配置调整

- `.cargo/config.toml`
  - 保持 `runner` 为 GDB（便于加载/调试）：
    ```toml
    [target.thumbv7em-none-eabihf]
    runner = "arm-none-eabi-gdb -q -x openocd.gdb"
    rustflags = [
      "-C", "link-arg=--nmagic",
      "-C", "link-arg=-Tlink.x",
    ]
    [build]
    target = "thumbv7em-none-eabihf"
    ```
  - 移除 defmt 相关配置（如 `-Tdefmt.x` 与 `DEFMT_LOG`）。

- `openocd.cfg`
  - 启用 RTT 并开启 TCP 服务（通道 0，端口 9090）：
    ```tcl
    rtt setup 0x20000000 0x4000 "SEGGER RTT"
    rtt start
    rtt server start 9090 0
    ```

### 3. 构建、烧录与运行

- 构建：
  ```bash
  cargo build
  ```

- 烧录（OpenOCD 直接编程并复位）：
  ```bash
  openocd -f openocd.cfg -c "program target/thumbv7em-none-eabihf/debug/stm32f407_test verify reset exit"
  ```

- 或者通过 GDB 运行（使用 `openocd.gdb` 脚本）：
  ```bash
  arm-none-eabi-gdb -q -x openocd.gdb target/thumbv7em-none-eabihf/debug/stm32f407_test
  ```

- 在查看日志前，单独启动一个 OpenOCD 会话以维持 RTT 服务器运行：
  ```bash
  openocd -f openocd.cfg
  ```

### 4. 查看纯文本 RTT 输出

- 使用 `nc`（netcat）直接连接 RTT 服务器：
  ```bash
  nc localhost 9090
  ```
  你将看到类似：
  ```
  Program start
  LED1 ON
  LED2 OFF
  ```

- 如果系统没有 `nc`，可用 `socat`：
  ```bash
  socat - TCP:localhost:9090
  ```

### 5. 常见问题与提示

- RTT 控制块未找到：调整 `rtt setup` 的搜索起始地址/长度，参考 OpenOCD 日志（例如 `Control block found at 0x20000008`）。
- LED 电平语义：确保日志文案与实际硬件“高电平亮/低电平亮”的行为一致，避免混淆。
- 不再需要 `defmt-print`：纯文本模式下无需解码工具；如果仍需使用 defmt，请改回 `defmt + defmt-rtt` 并在主机端使用 `defmt-print`。

以上即为切换到纯 RTT 文本打印的完整步骤，按需选择 GDB 或直接 `program` 方式进行运行与查看。