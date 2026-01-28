---
inclusion: fileMatch
fileMatchPattern: "*.rs"
description: "Rust 嵌入式开发最佳实践"
tags: ["rust", "embedded", "stm32"]
---

# Rust 嵌入式开发最佳实践

当处理 Rust 嵌入式项目时，请遵循以下指南。

## 内存管理

- 避免使用堆分配（`Box`, `Vec`, `String`）
- 优先使用栈分配和静态变量
- 使用 `heapless` crate 替代标准库集合
- 注意栈大小限制

## 中断处理

- 中断处理函数应尽可能短小
- 使用原子操作或临界区保护共享数据
- 避免在中断中使用阻塞操作
- 使用 `cortex_m::interrupt::free` 创建临界区

## 外设访问

- 使用 PAC (Peripheral Access Crate) 或 HAL
- 确保外设单例只被获取一次
- 使用类型状态模式确保配置正确
- 注意寄存器访问的原子性

## 错误处理

- 使用 `Result` 类型处理可能失败的操作
- 避免 `panic!`，使用 `panic-halt` 或自定义 panic handler
- 考虑使用 `defmt` 进行调试输出

## 性能优化

- 使用 `#[inline]` 标记小函数
- 启用 LTO (Link Time Optimization)
- 使用 `opt-level = "z"` 优化代码大小
- 避免不必要的边界检查

## 代码示例

```rust
// 好的做法：使用静态变量和原子操作
use core::sync::atomic::{AtomicBool, Ordering};
static FLAG: AtomicBool = AtomicBool::new(false);

// 好的做法：使用 heapless 集合
use heapless::Vec;
let mut buffer: Vec<u8, 32> = Vec::new();

// 好的做法：临界区保护
cortex_m::interrupt::free(|_cs| {
    // 访问共享资源
});
```

## 项目配置参考

- Cargo.toml: #[[file:Cargo.toml]]
- 内存布局: #[[file:memory.x]]
