## 新增功能
- 为toolkit-engine核心内置了aapt, 用于识别apk包名等信息

## 改进优化
- 将原有aapt调用方法替换使用tke aapt指令
- 统一MacOS和Windows的apk安装流程

## 问题修复
- 修复MacOS自动更新机制中, 本地获取与云端推荐格式不匹配的问题(使用.dmg而不是.zip)
