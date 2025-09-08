// 渲染进程日志模块
// 将渲染进程的日志发送到主进程，在CLI中显示
(function() {
    'use strict';
    
    const { ipcRenderer } = require('electron');
    
    class RendererLogger {
        constructor() {
            this.enabled = true;
            this.prefix = '[Renderer]';
            this.logBuffer = []; // 缓存日志
            this.maxBufferSize = 10000; // 最大缓存10000条日志
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
            
            const logEntry = {
                level: level,
                message: message,
                timestamp: new Date().toISOString()
            };
            
            // 添加到缓存
            this.logBuffer.push(logEntry);
            
            // 限制缓存大小
            if (this.logBuffer.length > this.maxBufferSize) {
                this.logBuffer.shift(); // 移除最旧的日志
            }
            
            // 发送到主进程
            try {
                ipcRenderer.send('renderer-log', logEntry);
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
        
        // 导出日志到文件
        async exportLogs() {
            try {
                // 准备日志数据
                const logData = {
                    timestamp: new Date().toISOString(),
                    platform: process.platform,
                    nodeVersion: process.versions.node,
                    electronVersion: process.versions.electron,
                    resourcesPath: process.resourcesPath || 'N/A',
                    dirname: __dirname,
                    logs: this.logBuffer
                };
                
                // 通过 IPC 调用主进程导出日志
                const result = await ipcRenderer.invoke('export-logs', logData);
                
                return result;
            } catch (error) {
                console.error('导出日志失败:', error);
                return { 
                    success: false, 
                    message: `导出失败: ${error.message}` 
                };
            }
        }
        
        // 获取日志缓存
        getLogs() {
            return this.logBuffer;
        }
        
        // 清空日志缓存
        clearLogs() {
            this.logBuffer = [];
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