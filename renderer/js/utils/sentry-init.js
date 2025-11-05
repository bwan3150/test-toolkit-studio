// Renderer 进程的 Sentry 初始化
// 必须在所有其他脚本之前加载
(function() {
    'use strict';

    const Sentry = require("@sentry/electron/renderer");

    Sentry.init({
        dsn: "https://cc99833abc12a25305015d39c3bb7adb@o4510309702565888.ingest.de.sentry.io/4510309764497488",
        // 可选：设置环境
        environment: process.env.NODE_ENV || 'production',
        // 可选：设置应用版本
        release: require('../../../package.json').version,
        // 启用实验性日志功能
        _experiments: {
            enableLogs: true,
        },
    });

    // 导出 Sentry 实例供其他模块使用
    window.Sentry = Sentry;

    console.log('Renderer 进程 Sentry 已初始化 (日志功能已启用)');
})();
