// Log管理模块 - 类似Android Studio的Logcat功能

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

class LogManager {
    constructor() {
        this.currentDevice = null;
        this.currentProcess = null;
        this.logBuffer = [];
        this.maxBufferSize = 10000; // 最多保存10000条日志
        this.isFollowing = true; // 是否自动滚动到最新
        this.logProcess = null; // 存储logcat进程
        this.filters = {
            level: 'verbose', // verbose, debug, info, warn, error, assert
            tag: '',
            tags: [], // 支持多个tag过滤
            package: '',
            text: ''
        };
        this.logLevels = {
            'V': 'verbose',
            'D': 'debug', 
            'I': 'info',
            'W': 'warn',
            'E': 'error',
            'A': 'assert'
        };
        this.levelPriority = {
            'verbose': 0,
            'debug': 1,
            'info': 2,
            'warn': 3,
            'error': 4,
            'assert': 5
        };
    }

    // 初始化Log页面
    async initializeLogPage() {
        this.setupEventListeners();
        await this.refreshDeviceList();
    }

    // 解析快速过滤语法
    parseQuickFilter(filterString) {
        const filters = {
            package: '',
            tags: [],
            text: ''
        };
        
        if (!filterString) return filters;
        
        // 解析 package:xxx tag:xxx text:xxx 格式
        const packageMatch = filterString.match(/package:([^\s]+)/i);
        if (packageMatch) {
            filters.package = packageMatch[1];
        }
        
        // 解析多个tag
        const tagMatches = filterString.matchAll(/tag:([^\s]+)/gi);
        for (const match of tagMatches) {
            filters.tags.push(match[1]);
        }
        
        // 解析text
        const textMatch = filterString.match(/text:([^\s]+)/i);
        if (textMatch) {
            filters.text = textMatch[1];
        }
        
        // 如果没有任何关键字，将整个字符串作为text搜索
        if (!packageMatch && !filters.tags.length && !textMatch) {
            filters.text = filterString.trim();
        }
        
        return filters;
    }

    // 设置事件监听器
    setupEventListeners() {
        // 设备选择
        const deviceSelect = document.getElementById('logDeviceSelect');
        if (deviceSelect) {
            deviceSelect.addEventListener('change', (e) => {
                this.selectDevice(e.target.value);
            });
        }

        // Package输入框
        const packageInput = document.getElementById('logPackageInput');
        if (packageInput) {
            packageInput.addEventListener('input', (e) => {
                this.filters.package = e.target.value;
                // 清空Quick Filter，因为用户正在使用独立输入框
                const quickFilterInput = document.getElementById('logQuickFilterInput');
                if (quickFilterInput) quickFilterInput.value = '';
                this.applyFilters();
            });
        }

        // Tag输入框
        const tagInput = document.getElementById('logTagInput');
        if (tagInput) {
            tagInput.addEventListener('input', (e) => {
                this.filters.tag = e.target.value;
                // 处理多个tag（用逗号分隔）
                const tags = e.target.value.split(',').map(t => t.trim()).filter(t => t);
                this.filters.tags = tags;
                // 清空Quick Filter，因为用户正在使用独立输入框
                const quickFilterInput = document.getElementById('logQuickFilterInput');
                if (quickFilterInput) quickFilterInput.value = '';
                this.applyFilters();
            });
        }

        // Text输入框
        const textInput = document.getElementById('logTextInput');
        if (textInput) {
            textInput.addEventListener('input', (e) => {
                this.filters.text = e.target.value;
                // 清空Quick Filter，因为用户正在使用独立输入框
                const quickFilterInput = document.getElementById('logQuickFilterInput');
                if (quickFilterInput) quickFilterInput.value = '';
                this.applyFilters();
            });
        }

        // 日志级别选择
        const levelSelect = document.getElementById('logLevelSelect');
        if (levelSelect) {
            levelSelect.addEventListener('change', (e) => {
                this.filters.level = e.target.value;
                this.applyFilters();
            });
        }

        // 快速过滤输入框 - 输入即生效
        const quickFilterInput = document.getElementById('logQuickFilterInput');
        if (quickFilterInput) {
            // 使用防抖处理输入事件
            let debounceTimer = null;
            quickFilterInput.addEventListener('input', (e) => {
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    this.applyQuickFilter(e.target.value, false); // false表示不更新其他输入框
                }, 300); // 300ms防抖
            });
        }

        // 清空按钮
        const clearBtn = document.getElementById('clearLogBtn');
        if (clearBtn) {
            clearBtn.addEventListener('click', () => this.clearLogs());
        }

        // 导出按钮
        const exportBtn = document.getElementById('exportLogBtn');
        if (exportBtn) {
            exportBtn.addEventListener('click', () => this.exportLogs());
        }

        // 自动滚动切换
        const followBtn = document.getElementById('followLogBtn');
        if (followBtn) {
            followBtn.addEventListener('click', () => {
                this.isFollowing = !this.isFollowing;
                followBtn.classList.toggle('active', this.isFollowing);
                if (this.isFollowing) {
                    this.scrollToBottom();
                }
            });
        }

        // 开始/停止按钮
        const toggleBtn = document.getElementById('toggleLogBtn');
        if (toggleBtn) {
            toggleBtn.addEventListener('click', () => {
                if (this.logProcess) {
                    this.stopLogcat();
                } else {
                    this.startLogcat();
                }
            });
        }
    }

    // 刷新设备列表
    async refreshDeviceList() {
        try {
            const { ipcRenderer } = getGlobals();
            const devices = await ipcRenderer.invoke('get-connected-devices');
            const deviceSelect = document.getElementById('logDeviceSelect');
            
            if (deviceSelect) {
                deviceSelect.innerHTML = '<option value="">Select a device</option>';
                devices.forEach(device => {
                    const option = document.createElement('option');
                    option.value = device.id;
                    option.textContent = `${device.model || 'Unknown'} (${device.id})`;
                    deviceSelect.appendChild(option);
                });
            }
        } catch (error) {
            console.error('Failed to refresh device list:', error);
            this.showNotification('Failed to get device list', 'error');
        }
    }

    // 选择设备
    async selectDevice(deviceId) {
        this.currentDevice = deviceId;
        this.stopLogcat();
        // 不再自动开始抓取日志，需要用户手动点击Start
    }

    // 开始抓取日志
    async startLogcat() {
        if (!this.currentDevice) {
            this.showNotification('Please select a device first', 'warning');
            return;
        }

        try {
            // 停止之前的logcat进程
            this.stopLogcat();

            // 启动新的logcat进程
            const result = await getGlobals().ipcRenderer.invoke('start-logcat', {
                device: this.currentDevice,
                format: 'threadtime' // 使用threadtime格式以获取完整信息
            });

            if (result.success) {
                this.logProcess = result.pid;
                this.updateToggleButton(true);
                
                // 监听日志数据
                getGlobals().ipcRenderer.on('logcat-data', (event, data) => {
                    this.handleLogData(data);
                });

                this.showNotification('Logcat started', 'success');
            } else {
                this.showNotification('Failed to start logcat', 'error');
            }
        } catch (error) {
            console.error('Failed to start logcat:', error);
            this.showNotification('Failed to start logcat', 'error');
        }
    }

    // 停止抓取日志
    stopLogcat() {
        if (this.logProcess) {
            getGlobals().ipcRenderer.invoke('stop-logcat', this.logProcess);
            getGlobals().ipcRenderer.removeAllListeners('logcat-data');
            this.logProcess = null;
            this.updateToggleButton(false);
        }
    }

    // 更新开始/停止按钮状态
    updateToggleButton(isRunning) {
        const toggleBtn = document.getElementById('toggleLogBtn');
        if (toggleBtn) {
            if (isRunning) {
                toggleBtn.textContent = 'Stop';
                toggleBtn.classList.add('btn-danger');
                toggleBtn.classList.remove('btn-primary');
            } else {
                toggleBtn.textContent = 'Start';
                toggleBtn.classList.add('btn-primary');
                toggleBtn.classList.remove('btn-danger');
            }
        }
    }

    // 处理日志数据
    handleLogData(data) {
        console.log('接收到日志数据:', data); // 调试信息
        const lines = data.split('\n');
        lines.forEach(line => {
            if (line.trim()) {
                const logEntry = this.parseLogLine(line);
                if (logEntry) {
                    console.log('解析的日志条目:', logEntry); // 调试信息
                    this.addLogEntry(logEntry);
                }
            }
        });
    }

    // 解析日志行
    parseLogLine(line) {
        // Android logcat threadtime格式:
        // 日期 时间 PID TID 优先级 标签: 消息
        // 例如: 08-09 14:30:45.123  1234  5678 I MyApp    : This is a log message
        const regex = /^(\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+)\s+(\d+)\s+([VDIWEA])\s+([^:]+?):\s+(.*)$/;
        const match = line.match(regex);
        
        if (match) {
            return {
                date: match[1],
                time: match[2],
                pid: match[3],
                tid: match[4],
                level: this.logLevels[match[5]] || 'verbose',
                tag: match[6].trim(),
                message: match[7],
                raw: line
            };
        }
        
        // 如果不匹配标准格式，尝试作为普通文本处理
        return {
            date: '',
            time: new Date().toLocaleTimeString(),
            pid: '',
            tid: '',
            level: 'verbose',
            tag: '',
            message: line,
            raw: line
        };
    }

    // 添加日志条目
    addLogEntry(entry) {
        // 添加到缓冲区
        this.logBuffer.push(entry);
        
        // 限制缓冲区大小
        if (this.logBuffer.length > this.maxBufferSize) {
            this.logBuffer.shift();
        }

        // 如果条目通过过滤器，添加到显示
        if (this.shouldShowEntry(entry)) {
            console.log('添加日志到视图:', entry); // 调试信息
            this.appendLogToView(entry);
        } else {
            console.log('日志被过滤:', entry); // 调试信息
        }
    }

    // 判断是否应该显示日志条目
    shouldShowEntry(entry) {
        // 检查日志级别
        if (this.levelPriority[entry.level] < this.levelPriority[this.filters.level]) {
            return false;
        }

        // 检查标签过滤（支持单个tag或多个tags）
        if (this.filters.tag && !entry.tag.toLowerCase().includes(this.filters.tag.toLowerCase())) {
            return false;
        }
        
        // 检查多个tags（所有tags都必须匹配）
        if (this.filters.tags && this.filters.tags.length > 0) {
            for (const tag of this.filters.tags) {
                if (!entry.tag.toLowerCase().includes(tag.toLowerCase())) {
                    return false;
                }
            }
        }

        // 检查包名过滤（在tag或message中搜索）
        if (this.filters.package && 
            !entry.tag.toLowerCase().includes(this.filters.package.toLowerCase()) &&
            !entry.message.toLowerCase().includes(this.filters.package.toLowerCase())) {
            return false;
        }

        // 检查文本过滤
        if (this.filters.text && !entry.message.toLowerCase().includes(this.filters.text.toLowerCase())) {
            return false;
        }

        return true;
    }

    // 添加日志到视图
    appendLogToView(entry) {
        const logContainer = document.getElementById('logContainer');
        if (!logContainer) {
            console.error('找不到logContainer元素！');
            return;
        }
        console.log('logContainer找到，准备添加日志');

        const logElement = document.createElement('div');
        logElement.className = `log-entry log-${entry.level}`;
        
        const timestamp = document.createElement('span');
        timestamp.className = 'log-timestamp';
        timestamp.textContent = `${entry.date} ${entry.time}`;
        
        const pid = document.createElement('span');
        pid.className = 'log-pid';
        pid.textContent = entry.pid;
        
        const tag = document.createElement('span');
        tag.className = 'log-tag';
        tag.textContent = entry.tag;
        
        const level = document.createElement('span');
        level.className = `log-level log-level-${entry.level}`;
        level.textContent = entry.level.charAt(0).toUpperCase();
        
        const message = document.createElement('span');
        message.className = 'log-message';
        message.textContent = entry.message;
        
        logElement.appendChild(timestamp);
        logElement.appendChild(pid);
        logElement.appendChild(level);
        logElement.appendChild(tag);
        logElement.appendChild(message);
        
        logContainer.appendChild(logElement);

        // 如果开启自动滚动，滚动到底部
        if (this.isFollowing) {
            this.scrollToBottom();
        }

        // 限制显示的日志数量
        while (logContainer.children.length > 1000) {
            logContainer.removeChild(logContainer.firstChild);
        }
    }

    // 滚动到底部
    scrollToBottom() {
        const logContainer = document.getElementById('logContainer');
        if (logContainer) {
            logContainer.scrollTop = logContainer.scrollHeight;
        }
    }

    // 应用快速过滤
    applyQuickFilter(filterString, updateOtherInputs = true) {
        const parsed = this.parseQuickFilter(filterString);
        
        // 如果Quick Filter为空，不覆盖独立输入框的值
        if (!filterString.trim()) {
            // Quick Filter清空时，保持使用独立输入框的值
            return;
        }
        
        // 更新过滤器
        this.filters.package = parsed.package;
        this.filters.tags = parsed.tags;
        this.filters.text = parsed.text;
        
        // 可选：同时更新独立输入框的值（保持同步）
        if (updateOtherInputs) {
            const packageInput = document.getElementById('logPackageInput');
            const tagInput = document.getElementById('logTagInput');
            const textInput = document.getElementById('logTextInput');
            
            if (packageInput) packageInput.value = parsed.package;
            if (tagInput) tagInput.value = parsed.tags.join(', ');
            if (textInput) textInput.value = parsed.text;
        }
        
        // 应用过滤
        this.applyFilters();
    }

    // 应用过滤器
    applyFilters() {
        const logContainer = document.getElementById('logContainer');
        if (!logContainer) return;

        // 清空当前显示
        logContainer.innerHTML = '';

        // 重新显示符合过滤条件的日志
        this.logBuffer.forEach(entry => {
            if (this.shouldShowEntry(entry)) {
                this.appendLogToView(entry);
            }
        });
    }

    // 清空日志
    clearLogs() {
        this.logBuffer = [];
        const logContainer = document.getElementById('logContainer');
        if (logContainer) {
            logContainer.innerHTML = '';
        }
    }

    // 导出日志
    async exportLogs() {
        try {
            const result = await getGlobals().ipcRenderer.invoke('show-save-dialog', {
                defaultPath: `logcat_${new Date().toISOString().replace(/[:.]/g, '-')}.log`,
                filters: [
                    { name: 'Log Files', extensions: ['log', 'txt'] },
                    { name: 'All Files', extensions: ['*'] }
                ]
            });

            if (result.filePath) {
                // 收集当前显示的日志
                const logContainer = document.getElementById('logContainer');
                const logs = [];
                
                if (logContainer) {
                    const entries = logContainer.querySelectorAll('.log-entry');
                    entries.forEach(entry => {
                        const timestamp = entry.querySelector('.log-timestamp')?.textContent || '';
                        const pid = entry.querySelector('.log-pid')?.textContent || '';
                        const level = entry.querySelector('.log-level')?.textContent || '';
                        const tag = entry.querySelector('.log-tag')?.textContent || '';
                        const message = entry.querySelector('.log-message')?.textContent || '';
                        
                        logs.push(`${timestamp} ${pid} ${level} ${tag}: ${message}`);
                    });
                }

                // 写入文件
                await getGlobals().fs.writeFile(result.filePath, logs.join('\n'), 'utf8');
                this.showNotification('Logs exported successfully', 'success');
            }
        } catch (error) {
            console.error('Failed to export logs:', error);
            this.showNotification('Failed to export logs', 'error');
        }
    }

    // 显示通知
    showNotification(message, type = 'info') {
        if (window.NotificationModule) {
            window.NotificationModule.showNotification(message, type);
        }
    }
}

// 创建并导出实例
const logManager = new LogManager();

window.LogManagerModule = {
    initializeLogPage: () => logManager.initializeLogPage(),
    refreshDeviceList: () => logManager.refreshDeviceList(),
    startLogcat: () => logManager.startLogcat(),
    stopLogcat: () => logManager.stopLogcat(),
    clearLogs: () => logManager.clearLogs(),
    exportLogs: () => logManager.exportLogs()
};