---
inclusion: always
description: "Git 提交信息规范 - 遵循 Conventional Commits 标准"
tags: ["git", "commit", "standards"]
---

# Git 提交信息规范

## Conventional Commits 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### 必需部分

**type**: 提交类型（必需）
**subject**: 简短描述（必需，不超过 50 字符）

### 可选部分

**scope**: 影响范围（可选）
**body**: 详细描述（可选）
**footer**: 备注信息（可选，如 Breaking Changes、Issue 引用）

## 提交类型 (type)

| 类型 | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | feat: 添加用户登录功能 |
| `fix` | Bug 修复 | fix: 修复内存泄漏问题 |
| `docs` | 文档变更 | docs: 更新 API 文档 |
| `style` | 代码格式（不影响功能） | style: 格式化代码 |
| `refactor` | 重构（不是新功能或修复） | refactor: 重构认证模块 |
| `perf` | 性能优化 | perf: 优化数据库查询 |
| `test` | 测试相关 | test: 添加单元测试 |
| `build` | 构建系统或依赖变更 | build: 升级 Rust 到 1.75 |
| `ci` | CI 配置变更 | ci: 添加 GitHub Actions |
| `chore` | 其他杂项 | chore: 更新 .gitignore |
| `revert` | 回滚提交 | revert: 回滚 commit abc123 |

## 影响范围 (scope)

根据项目模块定义，例如：
- `core`: 核心功能
- `api`: API 接口
- `ui`: 用户界面
- `db`: 数据库
- `auth`: 认证模块
- `config`: 配置相关

## 主题 (subject)

- 使用祈使句，现在时态："添加"而不是"已添加"或"添加了"
- 首字母小写
- 结尾不加句号
- 不超过 50 个字符
- 清晰描述做了什么

## 正文 (body)

- 详细说明"是什么"和"为什么"，而不是"怎么做"
- 每行不超过 72 个字符
- 与 subject 之间空一行
- 可以分多个段落

## 页脚 (footer)

**Breaking Changes**:
```
BREAKING CHANGE: 描述不兼容的变更
```

**Issue 引用**:
```
Closes #123
Fixes #456
Refs #789
```

## 提交示例

### 简单提交
```
feat: 添加 UART 通信功能
```

### 带范围的提交
```
fix(gpio): 修复引脚初始化错误
```

### 完整提交
```
feat(sensor): 添加温度传感器驱动

实现 DHT22 温度湿度传感器的驱动程序，支持：
- 温度读取（精度 0.1°C）
- 湿度读取（精度 0.1%）
- 错误处理和重试机制

Closes #42
```

### Breaking Change
```
refactor(api)!: 重构 API 接口

BREAKING CHANGE: API 端点从 /v1 改为 /v2
需要更新所有客户端代码
```

## 提交频率建议

- 每个逻辑变更一个提交
- 提交应该是原子性的（可独立回滚）
- 避免"WIP"、"临时提交"等无意义信息
- 频繁提交小改动，而不是一次提交大量变更

## 不良示例 ❌

```
update code
fix bug
WIP
临时保存
修改了一些东西
```

## 良好示例 ✅

```
feat(uart): 添加串口通信模块
fix(timer): 修复定时器溢出问题
docs: 更新嵌入式开发指南
refactor: 简化中断处理逻辑
perf(dma): 优化 DMA 传输性能
```

## Git 提交助手

使用 `.kiro/hooks/git-commit-helper.json` 自动检查和生成规范的提交信息。

在提交前，助手会：
1. 分析代码变更
2. 识别提交类型
3. 生成符合规范的提交信息
4. 提供修改建议

## 工具集成

### commitlint（可选）
```bash
npm install --save-dev @commitlint/cli @commitlint/config-conventional
```

### git hooks（可选）
```bash
# .git/hooks/commit-msg
#!/bin/sh
npx --no -- commitlint --edit $1
```

## 参考资源

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Angular Commit Guidelines](https://github.com/angular/angular/blob/main/CONTRIBUTING.md)
