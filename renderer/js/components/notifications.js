// 通知系统
function showNotification(message, type = 'info') {
    console.log(`[${type.toUpperCase()}] ${message}`);
    
    // 创建简单的通知toast
    const toast = document.createElement('div');
    toast.className = `notification notification-${type}`;
    toast.textContent = message;
    toast.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        padding: 12px 20px;
        background: ${type === 'success' ? '#4ec9b0' : type === 'error' ? '#f48771' : type === 'warning' ? '#ce9178' : '#569cd6'};
        color: white;
        border-radius: 6px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        z-index: 10000;
        animation: slideInUp 0.3s ease;
        font-size: 13px;
    `;
    
    document.body.appendChild(toast);
    
    setTimeout(() => {
        toast.style.animation = 'slideOutDown 0.3s ease';
        setTimeout(() => {
            if (toast.parentNode) {
                document.body.removeChild(toast);
            }
        }, 300);
    }, 3000);
    
    // 如果在testcase页面，同时添加到控制台
    if (document.getElementById('testcasePage') && document.getElementById('testcasePage').classList.contains('active')) {
        addConsoleLog(message, type);
    }
}

// 控制台管理
function addConsoleLog(message, type = 'info') {
    const consoleContent = document.getElementById('consoleContent');
    if (!consoleContent) return;
    const line = document.createElement('div');
    line.className = `console-line ${type}`;
    line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    consoleContent.appendChild(line);
    consoleContent.scrollTop = consoleContent.scrollHeight;
}

function clearConsole() {
    const consoleContent = document.getElementById('consoleContent');
    if (consoleContent) {
        consoleContent.innerHTML = '';
    }
}

// 导出函数
window.NotificationModule = {
    showNotification,
    addConsoleLog,
    clearConsole
};