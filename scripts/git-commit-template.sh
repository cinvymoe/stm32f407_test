#!/bin/bash
# Git 提交信息模板生成脚本
# 用法: ./scripts/git-commit-template.sh

set -e

echo "=== Git 提交助手 ==="
echo ""

# 检查是否有变更
if ! git diff --cached --quiet 2>/dev/null; then
    echo "检测到暂存的变更："
    git diff --cached --name-status
    echo ""
else
    echo "没有暂存的变更。请先使用 'git add' 暂存文件。"
    exit 1
fi

# 提示输入提交类型
echo "选择提交类型："
echo "1) feat     - 新功能"
echo "2) fix      - Bug 修复"
echo "3) docs     - 文档变更"
echo "4) style    - 代码格式"
echo "5) refactor - 重构"
echo "6) perf     - 性能优化"
echo "7) test     - 测试"
echo "8) build    - 构建系统"
echo "9) ci       - CI 配置"
echo "10) chore   - 其他杂项"
echo ""
read -p "请输入数字 (1-10): " type_choice

case $type_choice in
    1) TYPE="feat" ;;
    2) TYPE="fix" ;;
    3) TYPE="docs" ;;
    4) TYPE="style" ;;
    5) TYPE="refactor" ;;
    6) TYPE="perf" ;;
    7) TYPE="test" ;;
    8) TYPE="build" ;;
    9) TYPE="ci" ;;
    10) TYPE="chore" ;;
    *) echo "无效选择"; exit 1 ;;
esac

# 输入范围（可选）
read -p "输入影响范围 (可选，直接回车跳过): " SCOPE

# 输入主题
read -p "输入简短描述 (必需): " SUBJECT

if [ -z "$SUBJECT" ]; then
    echo "错误：描述不能为空"
    exit 1
fi

# 构建提交信息
if [ -n "$SCOPE" ]; then
    COMMIT_MSG="$TYPE($SCOPE): $SUBJECT"
else
    COMMIT_MSG="$TYPE: $SUBJECT"
fi

# 显示预览
echo ""
echo "生成的提交信息："
echo "---"
echo "$COMMIT_MSG"
echo "---"
echo ""

# 确认提交
read -p "是否执行提交? (y/n): " confirm

if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
    git commit -m "$COMMIT_MSG"
    echo "✓ 提交成功！"
else
    echo "已取消提交"
    exit 0
fi
