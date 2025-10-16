// 控制台面板管理器
// 负责显示命令执行输出和日志
// 统一管理所有日志输出，避免多个输入源混乱

const ConsolePanel = {
    // 最大日志条数
    MAX_LOGS: 1000,
    
    // 日志列表
    logs: [],
    
    // 是否已初始化
    initialized: false,
    
    // 原始console方法
    originalConsole: {},
    
    // 初始化
    init() {
        if (this.initialized) return;
        this.initialized = true;
        
        // 保存原始console方法
        this.originalConsole = {
            log: console.log,
            error: console.error,
            warn: console.warn,
            info: console.info
        };
        
        this.setupUnifiedLogger();
        this.clearConsole();
    },
    
    // 设置统一的日志系统
    setupUnifiedLogger() {
        // 重写window.rLog等方法，直接输出到控制台面板
        window.rLog = (...args) => this.log(...args);
        window.rInfo = (...args) => this.info(...args);
        window.rWarn = (...args) => this.warn(...args);
        window.rError = (...args) => this.error(...args);
        window.rDebug = (...args) => this.debug(...args);
        
        // 不再拦截console方法，避免重复
        // 让console保持原样用于开发调试
    },
    
    // 统一的日志输出方法
    log(...args) {
        this.addLog('log', args);
    },
    
    info(...args) {
        this.addLog('info', args);
    },
    
    warn(...args) {
        this.addLog('warn', args);
    },
    
    error(...args) {
        this.addLog('error', args);
    },
    
    debug(...args) {
        this.addLog('debug', args);
    },
    
    // 添加日志
    addLog(type, args, source = 'app') {
        // 格式化时间戳 HH:MM:SS.mmm
        const now = new Date();
        const timestamp = `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}.${now.getMilliseconds().toString().padStart(3, '0')}`;
        
        // 格式化消息
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
            source,
            id: Date.now() + Math.random()
        };
        
        this.logs.push(log);
        
        // 限制日志数量
        if (this.logs.length > this.MAX_LOGS) {
            this.logs.shift();
        }
        
        // 更新显示
        this.appendLogToUI(log);
        
        // 同时输出到开发者控制台（用于调试）
        if (this.originalConsole[type]) {
            this.originalConsole[type](`[${timestamp}]`, ...args);
        }
    },
    
    // 将日志添加到UI
    appendLogToUI(log) {
        const consoleOutput = document.getElementById('consoleOutput');
        if (!consoleOutput) return;
        
        const logElement = document.createElement('div');
        logElement.className = `console-log console-${log.type}`;
        
        // 创建时间戳元素
        const timestampElement = document.createElement('span');
        timestampElement.className = 'console-timestamp';
        timestampElement.textContent = log.timestamp;
        
        // 创建类型标签元素
        const typeElement = document.createElement('span');
        typeElement.className = `console-type console-type-${log.type}`;
        typeElement.textContent = log.type.toUpperCase().padEnd(5);
        
        // 创建消息元素
        const messageElement = document.createElement('span');
        messageElement.className = 'console-message';
        messageElement.textContent = log.message;
        
        // 组装日志元素
        logElement.appendChild(timestampElement);
        logElement.appendChild(typeElement);
        logElement.appendChild(messageElement);
        
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
        this.addSystemLog('控制台已初始化');
    },
    
    // 添加系统日志
    addSystemLog(message) {
        this.addLog('system', [message], 'system');
    },
    
    // 添加命令执行结果
    addCommandResult(command, result, success = true) {
        const type = success ? 'info' : 'error';
        this.addLog(type, [`> ${command}`, result], 'command');
    },
    
    // 执行命令（与TKE集成）
    async executeCommand(command) {
        this.addLog('info', [`执行命令: ${command}`], 'command');
        
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

// 导出模块（供BottomPanelManager调用）
window.ConsolePanelModule = {
    clear: () => ConsolePanel.clearConsole(),
    log: (...args) => ConsolePanel.log(...args),
    info: (...args) => ConsolePanel.info(...args),
    warn: (...args) => ConsolePanel.warn(...args),
    error: (...args) => ConsolePanel.error(...args),
    exportLogs: () => ConsolePanel.exportLogs()
};

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    ConsolePanel.init();
});