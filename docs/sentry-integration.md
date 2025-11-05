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
// 普通日志
logger.log('应用启动成功');

// 带上下文的日志
logger.info('设备已连接', { deviceId: '12345' });

// 警告
logger.warn('配置文件缺少某些字段', { missingFields: ['api_key'] });

// 错误（不带 Error 对象）
logger.error('配置加载失败', null, { configPath: '/path/to/config' });

// 错误（带 Error 对象）
try {
  // 某些操作
} catch (error) {
  logger.error('操作失败', error, { operationType: 'device_scan' });
}
```

### 渲染进程中记录日志

```javascript
// 普通日志
window.rLog('用户点击了按钮');

// 信息日志
window.rInfo('项目已加载', projectPath);

// 警告
window.rWarn('连接速度较慢');

// 错误
window.rError('无法加载文件', error);

// 调试日志
window.rDebug('当前状态:', state);
```

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

## 环境变量

- `NODE_ENV` - 设置 Sentry 环境（production/development）
- `SENTRY_AUTH_TOKEN` - 用于上传 source maps 的认证令牌（可选）

## 注意事项

1. **不要**在代码中直接使用 `console.log()` 或 `console.error()`
2. **必须**使用提供的日志方法 (`logger.*` 或 `window.r*`)
3. 所有 error 级别的日志都会自动发送到 Sentry
4. warn 级别的日志也会发送到 Sentry
5. log/info/debug 级别的日志只记录为 Breadcrumb，用于提供错误发生时的上下文

## 相关文件

- `main.js` - 主进程 Sentry 初始化和 logger 定义
- `renderer/js/utils/sentry-init.js` - 渲染进程 Sentry 初始化
- `renderer/js/utils/renderer-logger.js` - 渲染进程日志模块（集成 Sentry）
- `renderer/html/index.html` - 加载 Sentry 和日志模块
- `.sentryclirc` - Sentry CLI 配置
- `CLAUDE.md` - 项目代码规范（包含日志使用说明）
