// 控制台面板管理器
// 负责显示命令执行输出和日志

const ConsolePanel = {
    // 最大日志条数
    MAX_LOGS: 1000,
    
    // 日志列表
    logs: [],
    
    // 初始化
    init() {
        this.setupConsoleInterceptor();
        this.clearConsole();
    },
    
    // 设置控制台拦截器
    setupConsoleInterceptor() {
        // 保存原始的console方法
        const originalLog = console.log;
        const originalError = console.error;
        const originalWarn = console.warn;
        const originalInfo = console.info;
        
        // 拦截console.log
        console.log = (...args) => {
            this.addLog('log', args);
            originalLog.apply(console, args);
        };
        
        // 拦截console.error
        console.error = (...args) => {
            this.addLog('error', args);
            originalError.apply(console, args);
        };
        
        // 拦截console.warn
        console.warn = (...args) => {
            this.addLog('warn', args);
            originalWarn.apply(console, args);
        };
        
        // 拦截console.info
        console.info = (...args) => {
            this.addLog('info', args);
            originalInfo.apply(console, args);
        };
    },
    
    // 添加日志
    addLog(type, args) {
        const timestamp = new Date().toLocaleTimeString();
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
        
        const log = {
            type,
            message,
            timestamp,
            id: Date.now() + Math.random()
        };
        
        this.logs.push(log);
        
        // 限制日志数量
        if (this.logs.length > this.MAX_LOGS) {
            this.logs.shift();
        }
        
        // 更新显示
        this.appendLogToUI(log);
    },
    
    // 将日志添加到UI
    appendLogToUI(log) {
        const consoleOutput = document.getElementById('consoleOutput');
        if (!consoleOutput) return;
        
        const logElement = document.createElement('div');
        logElement.className = `console-log console-${log.type}`;
        logElement.innerHTML = `
            <span class="console-timestamp">${log.timestamp}</span>
            <span class="console-type">[${log.type.toUpperCase()}]</span>
            <span class="console-message">${this.escapeHtml(log.message)}</span>
        `;
        
        consoleOutput.appendChild(logElement);
        
        // 自动滚动到底部
        consoleOutput.scrollTop = consoleOutput.scrollHeight;
        
        // 限制UI中的日志数量
        while (consoleOutput.children.length > this.MAX_LOGS) {
            consoleOutput.removeChild(consoleOutput.firstChild);
        }
    },
    
    // 清空控制台
    clearConsole() {
        this.logs = [];
        const consoleOutput = document.getElementById('consoleOutput');
        if (consoleOutput) {
            consoleOutput.innerHTML = '';
        }
        this.addSystemLog('控制台已清空');
    },
    
    // 添加系统日志
    addSystemLog(message) {
        this.addLog('system', [message]);
    },
    
    // 添加命令执行结果
    addCommandResult(command, result, success = true) {
        const type = success ? 'info' : 'error';
        this.addLog(type, [`> ${command}`, result]);
    },
    
    // 执行命令（与TKE集成）
    async executeCommand(command) {
        this.addSystemLog(`执行命令: ${command}`);
        
        try {
            // 这里可以调用TKE执行命令
            const result = await this.runTkeCommand(command);
            this.addCommandResult(command, result, true);
            return result;
        } catch (error) {
            this.addCommandResult(command, error.message, false);
            throw error;
        }
    },
    
    // 运行TKE命令
    async runTkeCommand(command) {
        const { ipcRenderer } = getGlobals();
        
        // 解析命令类型
        if (command.startsWith('click')) {
            // 点击命令
            const match = command.match(/click\((\d+),\s*(\d+)\)/);
            if (match) {
                const [, x, y] = match;
                const result = await ipcRenderer.invoke('execute-tke-click', {
                    x: parseInt(x),
                    y: parseInt(y)
                });
                return result.success ? '点击成功' : `点击失败: ${result.error}`;
            }
        } else if (command.startsWith('swipe')) {
            // 滑动命令
            const match = command.match(/swipe\((\d+),\s*(\d+),\s*(\d+),\s*(\d+)\)/);
            if (match) {
                const [, x1, y1, x2, y2] = match;
                const result = await ipcRenderer.invoke('execute-tke-swipe', {
                    x1: parseInt(x1),
                    y1: parseInt(y1),
                    x2: parseInt(x2),
                    y2: parseInt(y2)
                });
                return result.success ? '滑动成功' : `滑动失败: ${result.error}`;
            }
        } else if (command.startsWith('input')) {
            // 输入命令
            const match = command.match(/input\("([^"]+)"\)/);
            if (match) {
                const [, text] = match;
                const result = await ipcRenderer.invoke('execute-tke-input', {
                    text: text
                });
                return result.success ? '输入成功' : `输入失败: ${result.error}`;
            }
        }
        
        return '未识别的命令';
    },
    
    // HTML转义
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    },
    
    // 导出日志
    exportLogs() {
        const logText = this.logs.map(log => 
            `[${log.timestamp}] [${log.type.toUpperCase()}] ${log.message}`
        ).join('\n');
        
        const blob = new Blob([logText], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `console_logs_${new Date().getTime()}.txt`;
        a.click();
        URL.revokeObjectURL(url);
        
        window.NotificationModule.showNotification('日志已导出', 'success');
    }
};

// 导出到全局
window.ConsolePanel = ConsolePanel;

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    ConsolePanel.init();
});