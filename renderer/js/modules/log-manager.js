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
        this.currentFormat = 'threadtime'; // 当前logcat格式
        this.currentBuffer = 'main'; // 当前logcat buffer
        this.filters = {
            filterSpec: '', // logcat风格的过滤器，如 "flutter:V ActivityManager:I *:S"
            search: '',     // 搜索文本
            regex: false,   // 是否使用正则表达式
            caseSensitive: false // 是否区分大小写
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

    // 设置事件监听器 - Logcat风格
    setupEventListeners() {
        // 设备选择
        const deviceSelect = document.getElementById('logDeviceSelect');
        if (deviceSelect) {
            deviceSelect.addEventListener('change', (e) => {
                this.selectDevice(e.target.value);
            });
        }

        // Filter Spec输入框
        const filterSpecInput = document.getElementById('logFilterSpec');
        if (filterSpecInput) {
            filterSpecInput.addEventListener('input', (e) => {
                this.filters.filterSpec = e.target.value;
                this.applyFilters();
            });
        }

        // 应用过滤器按钮
        const applyFilterBtn = document.getElementById('applyFilterBtn');
        if (applyFilterBtn) {
            applyFilterBtn.addEventListener('click', () => {
                this.applyFilters();
            });
        }

        // 搜索输入框
        const searchInput = document.getElementById('logSearchInput');
        if (searchInput) {
            let debounceTimer = null;
            searchInput.addEventListener('input', (e) => {
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    this.filters.search = e.target.value;
                    this.applyFilters();
                }, 300);
            });
        }

        // 正则表达式复选框
        const regexCheckbox = document.getElementById('regexSearchCheckbox');
        if (regexCheckbox) {
            regexCheckbox.addEventListener('change', (e) => {
                this.filters.regex = e.target.checked;
                this.applyFilters();
            });
        }

        // 区分大小写复选框
        const caseSensitiveCheckbox = document.getElementById('caseSensitiveCheckbox');
        if (caseSensitiveCheckbox) {
            caseSensitiveCheckbox.addEventListener('change', (e) => {
                this.filters.caseSensitive = e.target.checked;
                this.applyFilters();
            });
        }

        // Format选择器
        const formatSelect = document.getElementById('logFormatSelect');
        if (formatSelect) {
            formatSelect.addEventListener('change', (e) => {
                this.currentFormat = e.target.value;
                // 重新启动logcat以使用新格式
                if (this.logProcess) {
                    this.stopLogcat();
                    setTimeout(() => this.startLogcat(), 100);
                }
            });
        }

        // Buffer选择器
        const bufferSelect = document.getElementById('logBufferSelect');
        if (bufferSelect) {
            bufferSelect.addEventListener('change', (e) => {
                this.currentBuffer = e.target.value;
                // 重新启动logcat以使用新buffer
                if (this.logProcess) {
                    this.stopLogcat();
                    setTimeout(() => this.startLogcat(), 100);
                }
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
                format: this.currentFormat,
                buffer: this.currentBuffer
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
        const lines = data.split('\n');
        lines.forEach(line => {
            if (line.trim()) {
                const logEntry = this.parseLogLine(line);
                if (logEntry) {
                    this.addLogEntry(logEntry);
                }
            }
        });
    }

    // 简化的日志解析 - 直接模仿adb logcat输出
    parseLogLine(line) {
        // 标准logcat threadtime格式:
        // MM-DD HH:MM:SS.mmm PID TID LEVEL TAG: MESSAGE
        // 例如: 08-15 15:20:21.822 25800 25800 I flutter: 消息内容
        
        const regex = /^(\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+)\s+(\d+)\s+([VDIWEA])\s+([^:]+?):\s+(.*)$/;
        const match = line.match(regex);
        
        if (match) {
            return {
                date: match[1],
                time: match[2], 
                pid: match[3],
                tid: match[4],
                level: match[5],
                tag: match[6].trim(),
                message: match[7],
                raw: line
            };
        }
        
        // 如果不匹配，直接返回原始行
        return {
            raw: line,
            message: line,
            date: '',
            time: '',
            pid: '',
            tid: '',
            level: 'V',
            tag: ''
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

        // 移除了调试代码

        // 如果条目通过过滤器，添加到显示
        if (this.shouldShowEntry(entry)) {
            this.appendLogToView(entry);
        }
    }

    // 解析logcat风格的过滤器
    parseFilterSpec(filterSpec) {
        // 解析如 "flutter:V ActivityManager:I MyApp:D *:S" 格式
        if (!filterSpec.trim()) return null;
        
        const filters = [];
        const parts = filterSpec.split(/\s+/);
        
        for (const part of parts) {
            const match = part.match(/^([^:]+):([VDIWEA])$/);
            if (match) {
                filters.push({
                    tag: match[1] === '*' ? null : match[1],
                    level: match[2],
                    isWildcard: match[1] === '*'
                });
            }
        }
        
        return filters;
    }

    // logcat风格的过滤逻辑
    shouldShowEntry(entry) {
        // 检查filterSpec
        if (this.filters.filterSpec) {
            const parsedFilters = this.parseFilterSpec(this.filters.filterSpec);
            if (parsedFilters && parsedFilters.length > 0) {
                let matched = false;
                
                for (const filter of parsedFilters) {
                    if (filter.isWildcard) {
                        // *:S 表示所有其他tag的级别
                        if (!entry.tag || this.shouldShowLevel(entry.level, filter.level)) {
                            matched = true;
                            break;
                        }
                    } else if (filter.tag && entry.tag === filter.tag) {
                        // 特定tag的级别过滤
                        if (this.shouldShowLevel(entry.level, filter.level)) {
                            matched = true;
                            break;
                        }
                    }
                }
                
                if (!matched) return false;
            }
        }
        
        // 检查搜索文本
        if (this.filters.search) {
            const searchText = this.filters.caseSensitive ? 
                this.filters.search : this.filters.search.toLowerCase();
            const content = this.filters.caseSensitive ? 
                entry.raw : entry.raw.toLowerCase();
                
            if (this.filters.regex) {
                try {
                    const regex = new RegExp(searchText, this.filters.caseSensitive ? '' : 'i');
                    if (!regex.test(entry.raw)) return false;
                } catch (e) {
                    // 如果正则表达式无效，使用普通文本搜索
                    if (!content.includes(searchText)) return false;
                }
            } else {
                if (!content.includes(searchText)) return false;
            }
        }
        
        return true;
    }

    // 检查日志级别是否应该显示
    shouldShowLevel(entryLevel, filterLevel) {
        const levels = ['V', 'D', 'I', 'W', 'E', 'A'];
        const entryIndex = levels.indexOf(entryLevel);
        const filterIndex = levels.indexOf(filterLevel);
        return entryIndex >= filterIndex;
    }

    // 添加日志到视图 - 原始logcat格式
    appendLogToView(entry) {
        const logContainer = document.getElementById('logContainer');
        if (!logContainer) return;

        const logElement = document.createElement('div');
        logElement.className = `log-entry`;
        
        // 直接显示原始日志行，但添加颜色
        let formattedLine = entry.raw;
        
        // 如果有解析出的级别，为级别字符添加颜色
        if (entry.level && entry.level !== 'V') {
            const levelChar = entry.level;
            const colorClass = `log-level-${levelChar}`;
            // 找到级别字符的位置并添加颜色
            formattedLine = formattedLine.replace(
                new RegExp(`\\b${levelChar}\\b`), 
                `<span class="${colorClass}">${levelChar}</span>`
            );
        }
        
        logElement.innerHTML = formattedLine;
        logElement.title = entry.raw; // 悬停显示完整行
        
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

        // 调试输出
        console.log('应用过滤器:', this.filters);
        console.log('缓冲区日志总数:', this.logBuffer.length);
        
        // 统计符合条件的日志
        let matchCount = 0;
        let flutterKonecCount = 0;

        // 清空当前显示
        logContainer.innerHTML = '';

        // 重新显示符合过滤条件的日志
        this.logBuffer.forEach(entry => {
            // 统计flutter+konec的日志
            if (entry.tag === 'flutter' && entry.package && entry.package.includes('konec')) {
                flutterKonecCount++;
            }
            
            if (this.shouldShowEntry(entry)) {
                matchCount++;
                this.appendLogToView(entry);
            }
        });
        
        console.log(`过滤结果: 总共${this.logBuffer.length}条日志，符合过滤条件${matchCount}条`);
        console.log(`其中flutter+konec日志${flutterKonecCount}条`);
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
                
                // 直接导出原始logcat格式
                this.logBuffer.forEach(entry => {
                    if (this.shouldShowEntry(entry)) {
                        logs.push(entry.raw);
                    }
                });

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