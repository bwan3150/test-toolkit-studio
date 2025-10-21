#!/bin/bash

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 显示现有tags
echo -e "${GREEN}=== 现有标签 ===${NC}"
git tag -l
echo ""

# 输入新tag
echo -e "${YELLOW}请输入新版本tag (例如: v0.5.8-beta):${NC}"
read -r NEW_TAG

if [ -z "$NEW_TAG" ]; then
    echo -e "${RED}错误: tag不能为空${NC}"
    exit 1
fi

# 检查tag是否已存在
if git tag -l | grep -q "^${NEW_TAG}$"; then
    echo -e "${RED}错误: tag ${NEW_TAG} 已存在${NC}"
    exit 1
fi

# 准备release notes
RELEASE_FILE="./build/RELEASE_NOTES.md"
TEMP_FILE=$(mktemp)

echo -e "${GREEN}=== 填写Release Notes ===${NC}\n"

# 新增功能
echo "## 新增功能" > "$TEMP_FILE"
echo -e "${yellow}输入新增功能 (每行一条,空行结束):${nc}"
while ifs= read -r line; do
    [ -z "$line" ] && break
    echo "- $line" >> "$temp_file"
done

# 改进优化
echo "" >> "$temp_file"
echo "## 改进优化" >> "$temp_file"
echo -e "${yellow}输入改进优化 (每行一条,空行结束):${nc}"
while ifs= read -r line; do
    [ -z "$line" ] && break
    echo "- $line" >> "$temp_file"
done

# 问题修复
echo "" >> "$temp_file"
echo "## 问题修复" >> "$temp_file"
echo -e "${yellow}输入问题修复 (每行一条,空行结束):${nc}"
while ifs= read -r line; do
    [ -z "$line" ] && break
    echo "- $line" >> "$temp_file"
done

echo "" >> "$temp_file"

# 预览
echo -e "\n${green}=== release notes 预览 ===${nc}"
cat "$temp_file"
echo ""

# 确认
echo -e "${yellow}确认提交? (y/n):${nc}"
read -r confirm

if [ "$confirm" != "y" ]; then
    echo -e "${red}已取消${nc}"
    rm "$tEMP_FILE"
    exit 0
fi

# 写入release notes
mv "$TEMP_FILE" "$RELEASE_FILE"

# Git操作
echo -e "${GREEN}=== 执行Git操作 ===${NC}"

git add "$RELEASE_FILE"
git commit -m "Update: release note for version $NEW_TAG"
git push

if [ $? -ne 0 ]; then
    echo -e "${RED}错误: git push失败${NC}"
    exit 1
fi

git tag "$NEW_TAG"
git push origin "$NEW_TAG"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ 成功创建并推送tag: $NEW_TAG${NC}"
else
    echo -e "${RED}错误: tag推送失败${NC}"
    exit 1
fi
