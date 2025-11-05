# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目说明
Electron 桌面应用，用于自动化测试。项目禁止使用开发者模式调试, renderer层的调试只允许使用./renderer/js/utils/renderer-logger.js提供的能力将日志传输到cli中进行debug。

## 错误追踪
- 项目集成了 Sentry (@sentry/electron) 用于错误追踪和性能监控
- 主进程在 main.js 开头初始化 Sentry
- Renderer 进程通过 ./renderer/js/utils/sentry-init.js 初始化
- renderer-logger.js 已集成 Sentry，自动将错误和警告发送到 Sentry
- DSN: https://cc99833abc12a25305015d39c3bb7adb@o4510309702565888.ingest.de.sentry.io/4510309764497488
- Sentry 项目: test-toolkitapp/electron

## Commands

## Code Style Guidelines
- 这是一个Electron的桌面App应用, 不是网页, 所以Renderer进程中的所有log都应该通过./renderer/js/utils/renderer-logger.js提供的方法输出
- 核心的代码处理逻辑都要交给CLI工具toolkit engine(tke), App的electron js前端只是tke核心逻辑的外围封装
- 额外加入独立的ai tester模块, 用于根据一条测试用例, 开始实时逐轮次操作手机, 在手机中探索, 直到完成测试要求, 途中AI的所有操作将被记录为.tks脚本, 可以用于直接运行

## 逻辑分层(极其重要!)
- 本项目为electron桌面端App, 内部附带着rust打包成的二进制可执行文件tke, tester-ai等(项目源代码作为子项目就在./toolkit-engine和./tester-ai下)
- ./handlers文件夹下放着和系统交互的js代码, 和tke, tester-ai等可执行文件交互的js代码, 请求其他api服务器的代理js代码
- ./renderer文件夹下放着html+css+js专注于UI逻辑, 只允许与handlers交互进行处理, 不能直接请求外部api和与tke, tester-ai等可执行文件交互, 保持项目分层逻辑的统一

## 代码规范补充
- **必须**：
  - Renderer进程：使用 `window.rLog()`, `window.rError()` 等方法进行日志记录，这些方法会自动集成 Sentry
  - 主进程：使用 `logger.log()`, `logger.error()` 等方法（定义在 main.js 中），会同时输出到控制台和 Sentry
  - 不要直接使用 `console.log()`
- 编辑toolkit-engine后, 需要运行 ./toolkit-engine/build-mac.sh 来重新构建, 不要使用cargo build; ./tester-ah也是同理
- 一定要注意代码单元化, 不要任何耦合, 如果需要则立刻拆分大文件为多个小文件, 并且分类放进不同文件夹
- 注释用中文
- 修改路径后记得更新所有引用
- 所有文档都放在`./docs`文件夹下
