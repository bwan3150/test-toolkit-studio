# Toolkit Studio

测试工具箱系列产品, 深度融合AI的UI自动化测试脚本编写集成开发环境

## 主要功能

- **项目管理**：自动创建结构化测试项目文件夹，从CSV导入测试用例(未来支持从 Notion, GitHub 或 Toolkit云盘 同步)
- **AI仿生测试**: 允许AI通过此集成环境观察App和并与设备交互, 能够根据测试需求初步进行无监督测试, 并且自动记录为自动化脚本供后续审查与修订
- **脚本编辑**：自动化编辑器，支持语法高亮和多标签, 运行时实时反馈运行步骤和失败位置
- **设备连接**：多设备设备管理，实时状态检测和屏幕预览
- **元素定位**：点击屏幕获取元素，查看XML结构, 并直接根据获取的元素创建自动化脚本
- **测试执行**：运行测试脚本，查看执行日志
- **AI修正**: App版本更新后, 运行旧脚本时遇到问题, 可以开启AI辅助驾驶, AI会根据页面环境自动判断和修正当前脚本中存在的元素定位缺失/偏移等问题

## 本项目结构

```
test-toolkit-studio/
├── main.js              # Electron主进程
├── renderer/            # UI渲染进程
│   ├── index.html      # 四合一主界面
│   ├── login.html      # 登录页
│   ├── styles/         # css
│   │   ├── common.css
│   │   ├── login.css
│   │   └── main.css
│   └── js/             # JS逻辑
│       ├── login.js
│       └── app.js
├── assets/             # 静态资源
└── resouces/           # Android SDK等环境资源
```

## 测试项目结构

测试项目的基础目录结构：

```
project_root/
├── cases/              # 测试用例文件夹
│   └── case_001/
│       ├── config.json # 此用例配置
│       ├── result/     # 此用例下脚本运行log结果存放处
│       └── script/     # 测试脚本
│           └── script_001.yaml
├── devices/            # 设备配置
├── locator/            # 元素定位材料
│   ├── element.json    # xml 元素定位信息
│   └── img/            # 裁切截图 图像识别元素定位信息
├── testcase_map.json   # 用例映射表
├── testcase_sheet.csv  # 用例总表
└── workarea/           # 当前工作区
    ├── current_screenshot.png    # 当前设备屏幕截图
    └── current_ui_tree.xml       # 当前设备屏幕xml UI树信息
```

## 快捷键

- `Cmd/Ctrl + N`：新建项目
- `Cmd/Ctrl + O`：打开项目
- `F5`：运行测试
- `Shift + F5`：停止测试
- `Cmd/Ctrl + D`：刷新设备
- `Cmd/Ctrl + S`: 保存
- `Cmd/Ctrl + W`: 关闭Tab页
- `Ctrl+Tab`：下一个tab
- `Ctrl+Shift+Tab`：上一个tab

## Toolkit script

本项目使用自定义自动化测试脚本语言, 对应脚本文件后缀为`.tks`

语法规范可以参考[Toolkit Script语法规范v1.0.0](./docs/The_ToolkitScript_Reference.md)

## 常见问题

### ADB未找到

设置页面, 使用SDK和ADB自检功能检查内建立环境

### 设备未检测到

1. 开启手机开发者选项
2. 开启USB调试
3. USB连接设备
4. 授权调试请求

### 无法获取设备屏幕

1. 检查设备是否正确连接
2. 重新插拔USB线
3. 扫描全部设备
