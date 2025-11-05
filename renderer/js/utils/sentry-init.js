// Renderer 进程的 Sentry 初始化
// 必须在所有其他脚本之前加载
(function() {
    'use strict';

    const Sentry = require("@sentry/electron/renderer");

    // 检测是否为开发环境
    const isDev = process.env.ELECTRON_DEV_MODE === 'true';
    const environment = isDev ? 'development' : 'production';

    // 只在生产环境启用 Sentry
    if (!isDev) {
        Sentry.init({
            dsn: "https://cc99833abc12a25305015d39c3bb7adb@o4510309702565888.ingest.de.sentry.io/4510309764497488",
            environment: environment,
            release: require('../../../package.json').version,
            // 启用日志功能
            enableLogs: true,
            // 设置采样率
            tracesSampleRate: 1.0,
            // 过滤日志：只发送 warn 和 error 到 Logs
            beforeSendLog: (log) => {
                // 过滤掉 info 和 debug 级别的日志
                if (log.level === 'info' || log.level === 'debug' || log.level === 'trace') {
                    return null;
                }
                return log;
            },
        });
        console.log('✅ Renderer 进程 Sentry 已启用 (生产环境)');
    } else {
        console.log('ℹ️  Renderer 进程 Sentry 已禁用 (开发环境)');
    }

    // 导出 Sentry 实例供其他模块使用（即使在开发环境也导出，以避免错误）
    window.Sentry = Sentry;
    window.isSentryEnabled = !isDev;
})();
