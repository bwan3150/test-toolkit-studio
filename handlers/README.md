# Handlers 目录结构

本目录包含所有 IPC 处理器，按功能模块组织。

## 目录结构

### 📁 `api-proxy/`
与外部 API 服务交互的代理处理器

- **`toolkit-gateway.js`** - Toolkit Gateway 网关服务代理
  - 用户认证 (login, logout, refresh-token)
  - 用户信息获取 (get-user-info)
  - Token 自动刷新管理

- **`bug-analysis.js`** - Bug 分析服务代理
  - Bug 分析 API 代理

### 📁 `electron-core/`
Electron 应用核心功能模块

- **`window-handlers.js`** - 窗口控制
  - 窗口最小化/最大化/关闭
  - 窗口状态管理

- **`store-handlers.js`** - 本地存储
  - electron-store 配置读写
  - 应用配置持久化

- **`log-handlers.js`** - 日志输出
  - 渲染进程日志转发到 CLI
  - 日志文件写入

- **`filesystem-handlers.js`** - 文件系统操作
  - 文件/目录存在性检查 (file-exists)
  - 目录内容读取 (read-directory)
  - 用户数据路径获取 (get-user-data-path)

- **`system-handlers.js`** - 系统工具检查
  - 应用版本查询 (get-app-version)
  - TKE 状态检查 (check-tke-status)
  - ADB 版本检查 (check-adb-version)
  - AAPT 状态检查 (check-aapt-status)

### 📁 `tke-integration/`
与 TKE (Toolkit Engine) 交互的底层功能模块

- **`adb-handlers.js`** - ADB 核心功能
  - ADB 命令执行封装
  - Scrcpy/STB 路径管理
  - TKE 路径管理

- **`device-handlers.js`** - 设备管理
  - 设备列表获取 (get-connected-devices, adb-devices)
  - 设备信息查询 (get-device-info)
  - 配对状态检查 (check-pairing-status)
  - 应用列表获取 (get-app-list, get-third-party-apps)
  - 设备控制 (reboot-device)
  - 设备日志 (get-device-log, clear-device-log)

- **`logcat-handlers.js`** - Android Logcat
  - Logcat 实时流式传输
  - Logcat 过滤和搜索

- **`ios-handlers.js`** - iOS 设备支持
  - iOS 设备发现和管理
  - iOS 设备控制

### 📁 `project/`
项目管理模块

- **`project-handlers.js`** - 项目管理
  - 项目创建/打开/保存
  - 项目配置管理

## 设计原则

1. **模块化**: 每个文件专注于单一职责
2. **分层清晰**:
   - `tke-integration`: 底层硬件交互
   - `electron-core`: Electron 应用层
   - `api-proxy`: 外部服务集成
   - `project`: 业务逻辑层
3. **易于维护**: 功能明确，文件命名见名知意
4. **未来扩展**: 所有底层设备交互逐步迁移到 TKE 统一管理

## 添加新的处理器

根据功能类型选择合适的目录：

- **设备相关** → `tke-integration/device-handlers.js`
- **文件操作** → `electron-core/filesystem-handlers.js`
- **外部 API** → `api-proxy/` 新建文件
- **项目功能** → `project/project-handlers.js`

记得在 `main.js` 的 `registerAllHandlers()` 函数中注册新的处理器。
