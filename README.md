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