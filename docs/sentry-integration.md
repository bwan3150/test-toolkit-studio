# Sentry 集成文档

## 概述

本项目已集成 Sentry 进行错误追踪和性能监控，包括主进程和渲染进程。

## 配置信息

- **Sentry 组织**: test-toolkitapp
- **Sentry 项目**: electron
- **DSN**: `https://cc99833abc12a25305015d39c3bb7adb@o4510309702565888.ingest.de.sentry.io/4510309764497488`

## 架构

### 主进程 (main.js)

- 在 `main.js` 文件的最开始初始化 Sentry
- 使用 `@sentry/electron` 包
- 提供了 `logger` 对象用于日志记录：
  - `logger.log(message, context)` - 普通日志，记录为 Breadcrumb
  - `logger.info(message, context)` - 信息日志，记录为 Breadcrumb
  - `logger.warn(message, context)` - 警告日志，发送到 Sentry
  - `logger.error(message, error, context)` - 错误日志，发送异常到 Sentry

### 渲染进程 (Renderer)

#### 初始化
- 在 `renderer/js/utils/sentry-init.js` 中初始化
- 在 `renderer/html/index.html` 的 `<head>` 部分第一个加载
- 使用 `@sentry/electron/renderer` 包

#### 日志系统
- `renderer/js/utils/renderer-logger.js` 已集成 Sentry
- 提供全局方法：
  - `window.rLog(...)` - 普通日志，记录为 Breadcrumb
  - `window.rInfo(...)` - 信息日志，记录为 Breadcrumb
  - `window.rWarn(...)` - 警告日志，发送到 Sentry
  - `window.rError(...)` - 错误日志，发送异常到 Sentry
  - `window.rDebug(...)` - 调试日志，记录为 Breadcrumb

## 使用方法

### 主进程中记录日志

```javascript
// 普通日志（只作为 Breadcrumb，不单独发送）
logger.log('应用启动成功');
logger.info('设备已连接', { deviceId: '12345' });
logger.debug('调试信息');

// 警告（会发送到 Sentry Logs）⚠️
logger.warn('配置文件缺少某些字段', { missingFields: ['api_key'] });

// 错误（会发送到 Sentry Issues 和 Logs）❌
logger.error('配置加载失败', null, { configPath: '/path/to/config' });

// 错误（带 Error 对象，发送到 Issues）
try {
  // 某些操作
} catch (error) {
  logger.error('操作失败', error, { operationType: 'device_scan' });
}
```

### 渲染进程中记录日志

```javascript
// 普通日志（只作为 Breadcrumb，不单独发送）
window.rLog('用户点击了按钮');
window.rInfo('项目已加载', projectPath);
window.rDebug('当前状态:', state);

// 警告（会发送到 Sentry Logs）⚠️
window.rWarn('连接速度较慢');

// 错误（会发送到 Sentry Issues 和 Logs）❌
window.rError('无法加载文件', error);
```

### 日志级别说明

| 级别 | 发送位置 | 用途 | 建议使用场景 |
|------|---------|------|-------------|
| `log/info/debug` | Breadcrumb（附加在错误上下文中） | 记录操作轨迹 | 正常流程、状态变化 |
| `warn` | Sentry Logs | 需要注意的问题 | 配置问题、性能警告 |
| `error` | Sentry Issues + Logs | 错误和异常 | 操作失败、异常情况 |

## 验证集成

### 方法 1: 触发测试错误

在主进程中添加测试代码：
```javascript
// 测试 undefined 函数调用
// myUndefinedFunction();

// 测试原生崩溃（谨慎使用）
// process.crash();
```

在渲染进程中添加测试代码：
```javascript
// 测试错误
window.rError('这是一个测试错误', new Error('Test Error'));

// 测试 undefined 函数调用
// myUndefinedFunction();
```

### 方法 2: 查看 Sentry 控制台

访问: https://test-toolkitapp.sentry.io/issues/

## Source Maps

### 配置文件
- `.sentryclirc` - Sentry CLI 配置文件

### 上传 Source Maps
如需上传 source maps，需要：
1. 在 https://sentry.io/settings/account/api/auth-tokens/ 创建认证令牌
2. 设置环境变量：`export SENTRY_AUTH_TOKEN=your_token_here`
3. 在构建脚本中添加上传步骤

## 环境分离

本项目通过 `ELECTRON_DEV_MODE` 环境变量区分开发和生产环境：

### 开发环境
- 启动方式: 使用 `./dev.sh` 脚本
- 环境变量: `ELECTRON_DEV_MODE=true`
- Sentry 行为: **完全禁用**，不发送任何数据
- 控制台输出: 所有日志正常输出到控制台

### 生产环境
- 启动方式: `npm start` 或打包后的应用
- 环境变量: 未设置 `ELECTRON_DEV_MODE`（或为其他值）
- Sentry 行为: **完全启用**，发送错误和警告到 Sentry
- 控制台输出: 所有日志正常输出到控制台

### 代码中的实现
```javascript
// 主进程和渲染进程都使用相同的检测逻辑
const isDev = process.env.ELECTRON_DEV_MODE === 'true';

// 只在生产环境初始化 Sentry
if (!isDev) {
  Sentry.init({ /* ... */ });
}

// 日志方法会检查环境
if (!isDev && Sentry.logger) {
  Sentry.logger.warn(message);
}
```

## 环境变量

- `ELECTRON_DEV_MODE` - 开发模式标志（`true` = 开发环境，禁用 Sentry）
- `SENTRY_AUTH_TOKEN` - 用于上传 source maps 的认证令牌（可选）

## 用户上下文

Sentry 会自动关联用户信息，帮助追踪特定用户遇到的问题。

### 自动设置时机

用户上下文会在以下时机自动设置：

1. **用户登录时** - 登录成功后自动设置
2. **应用启动时** - 如果用户已登录，启动时自动设置
3. **用户登出时** - 自动清除用户上下文

### 用户信息包含

- `id`: 用户唯一标识
- `email`: 用户邮箱
- `username`: 用户名

### 在 Sentry 中查看

在 Sentry Issues 页面，每个错误都会显示关联的用户信息，方便：
- 识别哪些用户遇到了问题
- 统计受影响的用户数量
- 联系用户获取更多信息

## 注意事项

1. **不要**在代码中直接使用 `console.log()` 或 `console.error()`
2. **必须**使用提供的日志方法 (`logger.*` 或 `window.r*`)
3. 所有 error 级别的日志都会自动发送到 Sentry
4. warn 级别的日志也会发送到 Sentry
5. log/info/debug 级别的日志只记录为 Breadcrumb，用于提供错误发生时的上下文
6. 用户上下文自动管理，无需手动调用（除非特殊情况）

## 相关文件

- `main.js` - 主进程 Sentry 初始化和 logger 定义
- `handlers/api-proxy/toolkit-gateway.js` - 用户认证和 Sentry 用户上下文管理
- `renderer/js/utils/sentry-init.js` - 渲染进程 Sentry 初始化
- `renderer/js/utils/renderer-logger.js` - 渲染进程日志模块（集成 Sentry 和用户上下文管理）
- `renderer/js/login/login.js` - 登录页面（登录时设置用户上下文）
- `renderer/js/settings/settings.js` - 设置页面（登出时清除用户上下文）
- `renderer/js/app.js` - 主应用初始化（启动时设置用户上下文）
- `renderer/html/index.html` - 加载 Sentry 和日志模块
- `.sentryclirc` - Sentry CLI 配置
- `CLAUDE.md` - 项目代码规范（包含日志使用说明）
