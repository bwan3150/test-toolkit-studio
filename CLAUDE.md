# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目说明
Electron 桌面应用，用于自动化测试。

## Commands

## Code Style Guidelines
- 这是一个Electron的桌面App应用, 不是网页, 所以Renderer进程中的所有log都应该通过./renderer/js/utils/renderer-logger.js提供的方法输出
- 核心的代码处理逻辑都要交给CLI工具toolkit engine(tke), App的electron js前端只是tke核心逻辑的外围封装
- 额外加入独立的ai tester模块, 用于根据一条测试用例, 开始实时逐轮次操作手机, 在手机中探索, 直到完成测试要求, 途中AI的所有操作将被记录为.tks脚本, 可以用于直接运行

## 关键路径（重构后）
- HTML 文件在 `renderer/html/` 
- 从 HTML 引用 JS：`renderer/js/xxx.js`
- 从 HTML 引用 CSS：`renderer/styles/xxx.css`
- 从 HTML 引用图片：`../../assets/xxx`

## 代码规范补充
- **必须**：使用 `window.rLog()`, `window.rError()` 等，不用 `console.log()`
- 编辑toolkit-engine后, 需要运行 ./toolkit-engine/build.sh 来重新构建, 不要使用cargo build
- 一定要注意代码单元化, 不要任何耦合, 如果需要则立刻拆分大文件为多个小文件, 并且分类放进不同文件夹
- 注释用中文
- 修改路径后记得更新所有引用
- 所有文档都放在`./docs`文件夹下
