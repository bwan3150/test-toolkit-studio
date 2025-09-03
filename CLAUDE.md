# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目说明
Electron 桌面应用，用于自动化测试。

## Commands

## Code Style Guidelines
- 这是一个Electron的桌面App应用, 不是网页, 所以Renderer进程中的所有log都应该通过./renderer/js/utils/renderer-logger.js提供的方法输出

## 关键路径（重构后）
- HTML 文件在 `renderer/html/` 
- 从 HTML 引用 JS：`../js/xxx.js`
- 从 HTML 引用 CSS：`../styles/xxx.css`
- 从 HTML 引用图片：`../../assets/xxx`

## 代码规范补充
- **必须**：使用 `window.rLog()`, `window.rError()` 等，不用 `console.log()`
- 注释用中文
- 修改路径后记得更新所有引用

## 调试白屏问题
检查：`app.js` 模块路径、`main.js` 和 `window-handlers.js` 的 HTML 路径

