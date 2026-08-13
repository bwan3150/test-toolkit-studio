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

        // 不再重写 rLog 等方法
        // 控制台面板现在只接收来自 ExecutionOutput 的日志

        this.clearConsole();
    },
    
    // 所有日志现在通过 execution-log 事件接收
    // 不再提供 log/info/warn/error 等方法
    // 参见文件底部的事件监听器
    
    // 清空控制台
    clearConsole() {
        this.logs = [];
        const consoleContent = document.getElementById('consoleContent');
        if (consoleContent) {
            // 显示统一的空状态提示
            consoleContent.innerHTML = `
                <div class="console-empty-state">
                    <div class="empty-icon">
                        <svg viewBox="0 0 48 48" width="48" height="48">
                            <rect x="6" y="8" width="36" height="32" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
                            <line x1="10" y1="16" x2="18" y2="16" stroke="currentColor" stroke-width="1.5"/>
                            <line x1="10" y1="22" x2="26" y2="22" stroke="currentColor" stroke-width="1.5"/>
                            <line x1="10" y1="28" x2="20" y2="28" stroke="currentColor" stroke-width="1.5"/>
                            <circle cx="38" cy="32" r="2" fill="currentColor"/>
                        </svg>
                    </div>
                    <div class="empty-title">No console output</div>
                </div>
            `;
        }
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
        
        window.AppNotifications?.exported('console_logs');
    }
};

// 导出到全局
window.ConsolePanel = ConsolePanel;

// 导出模块（供BottomPanelManager调用）
window.ConsolePanelModule = {
    clear: () => ConsolePanel.clearConsole(),
    exportLogs: () => ConsolePanel.exportLogs()
};

// 监听执行输出事件
document.addEventListener('execution-log', (event) => {
    const { type, message, timestamp } = event.detail;

    // 格式化时间戳
    const time = timestamp instanceof Date ? timestamp : new Date(timestamp);
    const timeStr = `${time.getHours().toString().padStart(2, '0')}:${time.getMinutes().toString().padStart(2, '0')}:${time.getSeconds().toString().padStart(2, '0')}.${time.getMilliseconds().toString().padStart(3, '0')}`;

    // 添加到UI
    appendExecutionLogToUI(type, message, timeStr);
});

// 渲染执行日志到UI
function appendExecutionLogToUI(type, message, timestamp) {
    const consoleContent = document.getElementById('consoleContent');
    if (!consoleContent) return;

    // 如果存在空状态，先清除
    const emptyState = consoleContent.querySelector('.console-empty-state');
    if (emptyState) {
        emptyState.remove();
    }

    const logElement = document.createElement('div');
    logElement.className = `console-log console-${type}`;

    // 创建时间戳元素
    const timestampElement = document.createElement('span');
    timestampElement.className = 'console-timestamp';
    timestampElement.textContent = timestamp;

    // 创建类型标签元素
    const typeElement = document.createElement('span');
    typeElement.className = `console-type console-type-${type}`;
    const typeLabels = {
        'info': 'INFO ',
        'success': 'DONE ',
        'warn': 'WARN ',
        'error': 'ERROR'
    };
    typeElement.textContent = typeLabels[type] || type.toUpperCase().padEnd(5);

    // 创建消息元素
    const messageElement = document.createElement('span');
    messageElement.className = 'console-message';
    messageElement.textContent = message;

    // 组装日志元素
    logElement.appendChild(timestampElement);
    logElement.appendChild(typeElement);
    logElement.appendChild(messageElement);

    consoleContent.appendChild(logElement);

    // 自动滚动到底部
    consoleContent.scrollTop = consoleContent.scrollHeight;

    // 限制UI中的日志数量
    const logElements = consoleContent.querySelectorAll('.console-log');
    const MAX_LOGS = 1000;
    while (logElements.length > MAX_LOGS) {
        logElements[0].remove();
    }
}

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    ConsolePanel.init();
});