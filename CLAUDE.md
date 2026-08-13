# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目说明
Electron 桌面应用，用于自动化测试。项目禁止使用开发者模式调试，frontend层的调试只允许使用 `./frontend/js/utils/renderer-logger.js` 提供的能力将日志传输到 CLI 中进行 debug。

## 目录结构

```
studio/
├── main.js              # Electron 主进程入口
├── frontend/            # Electron 内嵌前端（HTML + CSS + JS），只与 handlers 交互
├── handlers/            # Electron IPC handlers：系统操作、TKE/tester-ai 调用、本地文件等
├── backend/             # 与远端服务器 HTTP API 交互（登录/注册/找回密码/bug分析/release notes）
├── bin/                 # 二进制文件（不上传 GitHub，由 bin-proj 构建输出或从 S3 下载）
├── bin-proj/            # 内嵌二进制工具的源码（子项目）
│   ├── toolkit-engine/  # Rust，核心 CLI 工具 tke
│   ├── tester-ai/       # Rust，AI 自动化测试执行器
│   ├── opencv-matcher/  # Python，图像匹配工具 tke-opencv
│   └── scrcpy-server/   # Rust，设备屏幕投屏 WebSocket 服务
├── assets/              # 图片、图标、字体等静态资源
├── build/               # 打包配置（entitlements、release notes）
├── docs/                # 所有文档，含 tks-language-support VSCode 扩展
├── test/                # 测试文件
└── website/             # 独立静态宣传网站
```

## 错误追踪
- 项目集成了 Sentry (@sentry/electron) 用于错误追踪和性能监控
- 主进程在 main.js 开头初始化 Sentry
- Frontend（Renderer）进程通过 `./frontend/js/utils/sentry-init.js` 初始化
- `renderer-logger.js` 已集成 Sentry，自动将错误和警告发送到 Sentry
- DSN: https://cc99833abc12a25305015d39c3bb7adb@o4510309702565888.ingest.de.sentry.io/4510309764497488
- Sentry 项目: test-toolkitapp/electron

## 逻辑分层(极其重要!)
- `./frontend` — HTML/CSS/JS UI 逻辑，只允许通过 IPC 与 handlers 交互，禁止直接调用外部 API 或二进制工具
- `./handlers` — Electron 主进程 IPC handlers：与系统、tke、tester-ai 等可执行文件交互，以及本地文件/数据库操作
- `./backend` — 与远端服务器 HTTP API 交互：登录、注册、找回密码、bug 分析、release notes 获取等
- `./bin` — 运行时二进制文件（不上传 GitHub）；开发阶段由 `bin-proj` 各子项目构建后输出到此，生产用户按需从 S3 下载
- `./bin-proj` — 内嵌二进制工具的项目源码，构建脚本统一输出到 `../bin/[platform]/`

## Code Style Guidelines
- 这是 Electron 桌面 App，不是网页，frontend 进程中的所有 log 必须通过 `./frontend/js/utils/renderer-logger.js` 的方法输出
- 核心处理逻辑交给 tke（toolkit engine），App 的 JS 前端只是 tke 的外围封装
- 额外的 ai tester 模块：根据测试用例实时逐轮操作手机探索，操作记录为 .tks 脚本可直接回放

## 代码规范补充
- **必须**：
  - Frontend 进程：使用 `window.rLog()`, `window.rError()` 等（自动集成 Sentry）
  - 主进程：使用 `logger.log()`, `logger.error()` 等（定义在 main.js，同时输出到控制台和 Sentry）
  - 不要直接使用 `console.log()`
- 编辑 bin-proj 子项目后需运行对应构建脚本重新构建，不要使用 `cargo build`：
  - `./bin-proj/toolkit-engine/build-mac.sh`（Linux 用 `build-linux.sh`，CI 加 `--no-ocr --quiet`）
  - `./bin-proj/tester-ai/build-mac.sh`
  - `./bin-proj/scrcpy-server/build-mac.sh`
  - `./bin-proj/opencv-matcher/build-mac.sh`
- 代码单元化，不要耦合，大文件立刻拆分为多个小文件并分类放入不同文件夹
- 注释用中文
- 修改路径后记得更新所有引用
- 所有文档放在 `./docs` 文件夹下
