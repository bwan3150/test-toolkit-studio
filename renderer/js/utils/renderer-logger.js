// 渲染进程日志模块
// 将渲染进程的日志发送到主进程，在CLI中显示
(function() {
    'use strict';
    
    const { ipcRenderer } = require('electron');
    
    class RendererLogger {
        constructor() {
            this.enabled = true;
            this.prefix = '[Renderer]';
        }
        
        // 基础日志方法
        _log(level, ...args) {
            if (!this.enabled) return;
            
            // 将参数转换为字符串
            const message = args.map(arg => {
                if (typeof arg === 'object') {
                    try {
                        return JSON.stringify(arg, null, 2);
                    } catch (e) {
                        return String(arg);
                    }
                }
                return String(arg);
            }).join(' ');
            
            // 发送到主进程
            try {
                ipcRenderer.send('renderer-log', {
                    level: level,
                    message: message,
                    timestamp: new Date().toISOString()
                });
            } catch (error) {
                // 如果IPC失败，至少在控制台输出
                console.error('Failed to send log to main process:', error);
            }
            
            // 同时在控制台输出（如果开发者工具开启）
            const consoleMethod = console[level] || console.log;
            consoleMethod(`${this.prefix}`, ...args);
        }
        
        // 各种日志级别
        log(...args) {
            this._log('log', ...args);
        }
        
        info(...args) {
            this._log('info', ...args);
        }
        
        warn(...args) {
            this._log('warn', ...args);
        }
        
        error(...args) {
            this._log('error', ...args);
        }
        
        debug(...args) {
            this._log('debug', ...args);
        }
        
        // 带标签的日志
        logWithTag(tag, ...args) {
            this._log('log', `[${tag}]`, ...args);
        }
        
        // 启用/禁用日志
        enable() {
            this.enabled = true;
        }
        
        disable() {
            this.enabled = false;
        }
    }
    
    // 创建全局实例
    const logger = new RendererLogger();
    
    // 导出到全局
    window.RendererLogger = logger;
    
    // 为了方便使用，也创建全局快捷方法
    window.rLog = (...args) => logger.log(...args);
    window.rInfo = (...args) => logger.info(...args);
    window.rWarn = (...args) => logger.warn(...args);
    window.rError = (...args) => logger.error(...args);
    window.rDebug = (...args) => logger.debug(...args);
    
    console.log('渲染进程日志模块已加载');
})();