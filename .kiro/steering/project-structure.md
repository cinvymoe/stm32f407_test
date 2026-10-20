---
inclusion: always
description: "项目文件组织规范 - 保持代码结构清晰"
tags: ["structure", "organization", "standards"]
---

# 项目文件组织规范

## 标准目录结构

```
project/
├── src/              # 源代码
├── examples/         # 示例和演示代码
├── tests/            # 测试文件
├── docs/             # 项目文档
├── config/           # 配置文件
├── scripts/          # 工具脚本
├── temp/             # 临时文件（添加到 .gitignore）
└── .kiro/            # Kiro 配置
    ├── hooks/        # 自动化 hooks
    └── steering/     # 知识库文件
```

## 文件命名规范

- **源代码**: 使用 snake_case（Rust）或 camelCase（JS/TS）
- **示例**: 前缀 `example_` 或后缀 `_example`
- **测试**: 后缀 `_test` 或 `_spec`
- **文档**: 使用描述性名称，如 `api-guide.md`
- **临时**: 前缀 `temp_` 或 `tmp_`

## 创建文件时的默认位置

当 Agent 创建新文件时，应遵循：

1. **源代码** -> `src/` 目录
2. **示例代码** -> `examples/` 目录
3. **测试文件** -> `tests/` 目录
4. **文档** -> `docs/` 目录
5. **脚本** -> `scripts/` 目录
6. **实验代码** -> `temp/` 目录

## .gitignore 建议

```gitignore
# 临时文件
temp/
*.tmp
*_temp.*

# 构建产物
target/
dist/
build/

# IDE 文件
.vscode/
.idea/
```

## 注意事项

- 代码归档助手会自动整理文件位置
- 保持根目录简洁，只放置必要的配置文件
- 定期清理 temp/ 目录
- 重要文件及时提交到 git
