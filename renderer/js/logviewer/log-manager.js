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
        this.isFiltering = false; // 是否正在过滤
        this.filterTimeout = null; // 过滤延时器
        this.filterMode = 'simple'; // 'simple' 或 'expert'
        this.filters = {
            // Expert模式
            filterSpec: '', // logcat风格的过滤器，如 "flutter:V ActivityManager:I *:S"
            
            // Simple模式
            level: 'V',     // 日志级别 - 改为V显示所有日志
            tag: '',        // 标签过滤
            package: '',    // 包名过滤
            
            // 通用
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

    // 设置事件监听器 - 支持双模式
    setupEventListeners() {
        // 设备选择
        const deviceSelect = document.getElementById('logDeviceSelect');
        if (deviceSelect) {
            deviceSelect.addEventListener('change', (e) => {
                this.selectDevice(e.target.value);
            });
        }

        // 模式切换
        const simpleModeRadio = document.getElementById('simpleModeRadio');
        const expertModeRadio = document.getElementById('expertModeRadio');
        
        if (simpleModeRadio) {
            simpleModeRadio.addEventListener('change', () => {
                if (simpleModeRadio.checked) {
                    this.switchToMode('simple');
                }
            });
        }
        
        if (expertModeRadio) {
            expertModeRadio.addEventListener('change', () => {
                if (expertModeRadio.checked) {
                    this.switchToMode('expert');
                }
            });
        }

        // Expert模式：Filter Spec输入框
        const filterSpecInput = document.getElementById('logFilterSpec');
        if (filterSpecInput) {
            let debounceTimer = null;
            filterSpecInput.addEventListener('input', (e) => {
                // 显示输入框filtering状态
                this.setInputFilteringState(filterSpecInput, true);
                
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    this.filters.filterSpec = e.target.value;
                    this.applyFilters();
                    
                    // 移除输入框filtering状态
                    setTimeout(() => {
                        this.setInputFilteringState(filterSpecInput, false);
                    }, 500);
                }, 300);
            });
        }

        // Expert模式：应用过滤器按钮
        const applyFilterBtn = document.getElementById('applyFilterBtn');
        if (applyFilterBtn) {
            applyFilterBtn.addEventListener('click', () => {
                this.applyFilters();
            });
        }

        // Simple模式：日志级别选择
        const simpleLogLevel = document.getElementById('simpleLogLevel');
        if (simpleLogLevel) {
            simpleLogLevel.addEventListener('change', (e) => {
                this.filters.level = e.target.value;
                this.applyFilters();
            });
        }

        // Simple模式：Tag过滤
        const simpleTagFilter = document.getElementById('simpleTagFilter');
        if (simpleTagFilter) {
            let debounceTimer = null;
            simpleTagFilter.addEventListener('input', (e) => {
                // 显示输入框filtering状态
                this.setInputFilteringState(simpleTagFilter, true);
                
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    this.filters.tag = e.target.value;
                    this.applyFilters();
                    
                    // 移除输入框filtering状态
                    setTimeout(() => {
                        this.setInputFilteringState(simpleTagFilter, false);
                    }, 500);
                }, 300);
            });
        }

        // Simple模式：Package过滤
        const simplePackageFilter = document.getElementById('simplePackageFilter');
        if (simplePackageFilter) {
            let debounceTimer = null;
            simplePackageFilter.addEventListener('input', (e) => {
                // 显示输入框filtering状态
                this.setInputFilteringState(simplePackageFilter, true);
                
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    this.filters.package = e.target.value;
                    this.applyFilters();
                    
                    // 移除输入框filtering状态
                    setTimeout(() => {
                        this.setInputFilteringState(simplePackageFilter, false);
                    }, 500);
                }, 300);
            });
        }

        // 搜索输入框
        const searchInput = document.getElementById('logSearchInput');
        if (searchInput) {
            let debounceTimer = null;
            searchInput.addEventListener('input', (e) => {
                // 显示输入框filtering状态
                this.setInputFilteringState(searchInput, true);
                
                clearTimeout(debounceTimer);
                debounceTimer = setTimeout(() => {
                    this.filters.search = e.target.value;
                    this.applyFilters();
                    
                    // 移除输入框filtering状态
                    setTimeout(() => {
                        this.setInputFilteringState(searchInput, false);
                    }, 500);
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

    // 切换过滤模式
    switchToMode(mode) {
        this.filterMode = mode;
        
        const expertFilterRow = document.getElementById('expertFilterRow');
        const simpleFilterRows = document.getElementById('simpleFilterRows');
        
        if (mode === 'expert') {
            expertFilterRow.style.display = 'block';
            simpleFilterRows.style.display = 'none';
            // 清空simple模式的过滤器
            this.filters.level = 'I';
            this.filters.tag = '';
            this.filters.package = '';
        } else {
            expertFilterRow.style.display = 'none';
            simpleFilterRows.style.display = 'block';
            // 清空expert模式的过滤器
            this.filters.filterSpec = '';
            const filterSpecInput = document.getElementById('logFilterSpec');
            if (filterSpecInput) filterSpecInput.value = '';
        }
        
        this.applyFilters();
    }

    // 刷新设备列表 - 优先显示用户保存的设备配置名称
    async refreshDeviceList() {
        try {
            const { ipcRenderer } = getGlobals();
            const result = await ipcRenderer.invoke('get-connected-devices');
            const deviceSelect = document.getElementById('logDeviceSelect');
            
            if (deviceSelect) {
                deviceSelect.innerHTML = '<option value="">Select a device</option>';
                
                // 检查返回结果格式
                let devices = [];
                if (result && result.success && Array.isArray(result.devices)) {
                    devices = result.devices;
                } else if (Array.isArray(result)) {
                    devices = result;
                } else {
                    console.warn('Unexpected device list format:', result);
                    return;
                }
                
                // 尝试获取用户保存的设备配置
                let savedDevices = [];
                try {
                    const savedResult = await ipcRenderer.invoke('get-saved-devices');
                    if (savedResult && savedResult.success && Array.isArray(savedResult.devices)) {
                        savedDevices = savedResult.devices;
                    } else if (Array.isArray(savedResult)) {
                        savedDevices = savedResult;
                    }
                } catch (e) {
                    console.log('无法获取保存的设备配置，使用默认显示方式');
                    savedDevices = [];
                }
                
                devices.forEach(device => {
                    const option = document.createElement('option');
                    option.value = device.id;
                    
                    // 简化的设备匹配逻辑
                    let displayName = device.model || 'Unknown Device';
                    let foundSavedDevice = false;
                    
                    // 查找匹配的保存配置
                    for (const saved of savedDevices) {
                        // 直接匹配设备ID，或者匹配IP地址（无线设备）
                        if (saved.deviceId === device.id || 
                            (saved.ipAddress && device.id.includes(saved.ipAddress))) {
                            displayName = saved.deviceName;
                            foundSavedDevice = true;
                            break;
                        }
                    }
                    
                    option.textContent = `${displayName} (${device.id})`;
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
            // 统一使用long格式以获取最完整的信息
            const result = await getGlobals().ipcRenderer.invoke('start-logcat', {
                device: this.currentDevice,
                format: 'long', // 统一使用long格式
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
        console.log('[LogManager] Received logcat data, length:', data.length);
        // 确保数据是字符串格式
        const dataStr = typeof data === 'string' ? data : data.toString('utf8');
        
        // Windows平台编码验证和修复
        let processedData = dataStr;
        if (navigator.platform.indexOf('Win') === 0) {
            // 检测常见的编码问题
            const hasReplacementChar = processedData.includes('\ufffd');
            const hasHighBytePattern = /[\x80-\xFF]{2,}/.test(processedData);
            
            if (hasReplacementChar || hasHighBytePattern) {
                console.warn('检测到Windows端编码问题，尝试修复...');
                
                // 尝试清理和修复编码问题
                // 1. 替换常见的乱码字符
                processedData = processedData
                    .replace(/\ufffd/g, '?') // 替换Unicode替换字符
                    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '') // 移除控制字符
                    .replace(/[\u0080-\u009F]/g, ''); // 移除扩展控制字符
                    
                // 2. 尝试修复中文乱码（GBK/GB2312转换问题）
                // 注意：由于主进程已经处理了编码转换，这里主要是清理残留问题
                processedData = this.cleanupEncodingIssues(processedData);
                
                console.log('编码修复完成');
            } else {
                console.log('Windows端logcat数据编码正常');
            }
        }
        
        const lines = processedData.split('\n');
        console.log('[LogManager] Processing', lines.length, 'lines');
        
        // 处理多行日志
        this.processMultilineLogs(lines);
    }
    
    // 处理多行日志
    processMultilineLogs(lines) {
        let pendingLogEntry = null;
        
        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            
            // 空行处理
            if (!line.trim()) {
                // 空行可能是日志内容的一部分
                if (pendingLogEntry) {
                    pendingLogEntry.multilineMessage.push('');
                    pendingLogEntry.raw += '\n';
                }
                continue;
            }
            
            // 检查是否是日志头部（新日志的开始）
            if (this.isLogHeader(line)) {
                // 如果有待处理的日志，先保存它
                if (pendingLogEntry) {
                    this.addLogEntry(pendingLogEntry);
                }
                
                // 解析新的日志头
                pendingLogEntry = this.parseLogLine(line);
                
                // 初始化消息数组（用于收集多行内容）
                if (pendingLogEntry) {
                    pendingLogEntry.multilineMessage = [];
                    // 如果当前行已经包含消息，添加到数组
                    if (pendingLogEntry.message) {
                        pendingLogEntry.multilineMessage.push(pendingLogEntry.message);
                    }
                }
            } else if (pendingLogEntry) {
                // 这是当前日志的续行，添加到消息数组
                pendingLogEntry.multilineMessage.push(line);
                // 更新原始内容
                pendingLogEntry.raw += '\n' + line;
            } else {
                // 没有待处理的日志头，可能是独立的消息行
                const standaloneEntry = {
                    date: '',
                    time: '',
                    pid: '',
                    tid: '',
                    level: 'V',
                    tag: 'unknown',
                    message: line,
                    multilineMessage: [line],
                    package: '',
                    raw: line
                };
                
                // 尝试从行中提取包名
                const pkgMatch = line.match(/\b([a-zA-Z][a-zA-Z0-9_]*(\.[a-zA-Z][a-zA-Z0-9_]*)+)\b/);
                if (pkgMatch) {
                    standaloneEntry.package = pkgMatch[1];
                }
                
                this.addLogEntry(standaloneEntry);
            }
        }
        
        // 不要忘记最后一个待处理的日志
        if (pendingLogEntry) {
            this.addLogEntry(pendingLogEntry);
        }
    }
    
    // 检查是否是日志头部
    isLogHeader(line) {
        // 检查各种日志头格式
        
        // 格式1: 标准long格式 [ MM-DD HH:MM:SS.mmm  PID: TID L/TAG ]
        // 注意：PID前可能有多个空格
        if (/^\[\s*\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3}\s+\d+:\s*\d+\s+[VDIWEFA]\//.test(line)) {
            return true;
        }
        
        // 格式2: 标准Android logcat格式 MM-DD HH:MM:SS.mmm PID TID LEVEL TAG
        if (/^\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3}\s+\d+\s+\d+\s+[VDIWEFA]\s+/.test(line)) {
            return true;
        }
        
        // 格式3: 简化格式 MM-DD HH:MM:SS.mmm LEVEL/TAG(PID)
        if (/^\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3}\s+[VDIWEFA]\//.test(line)) {
            return true;
        }
        
        return false;
    }

    // 清理编码问题
    cleanupEncodingIssues(text) {
        // 常见的Windows编码问题模式替换
        return text
            // 清理常见的GBK/UTF-8混淆字符
            .replace(/Â|Ã|Ä|Å|À|Á|â|ã|ä|å|à|á/g, '')
            // 清理其他非ASCII可打印字符（保留中文）
            .replace(/[^\x20-\x7E\u4e00-\u9fa5\n\r\t]/g, function(char) {
                // 如果是中文字符，保留
                if (/[\u4e00-\u9fa5]/.test(char)) {
                    return char;
                }
                // 否则替换为空格或问号
                return '?';
            });
    }

    // 增强的日志解析 - 支持多种格式
    parseLogLine(line) {
        // 跳过空行
        if (!line || !line.trim()) {
            return null;
        }
        
        // 调试输出（采样）
        if (Math.random() < 0.02) {
            console.log('[DEBUG] Parsing line:', line.substring(0, 150));
        }
        
        // 格式1: 标准Android logcat格式
        // MM-DD HH:MM:SS.mmm PID TID LEVEL TAG : MESSAGE
        // 例如: 08-15 17:55:07.544 26702 26702 I flutter : TAG LOG_SERVICE 开始执行初始维护任务...
        let match = line.match(/^(\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+)\s+(\d+)\s+([VDIWEFA])\s+([^\s:]+)\s*:\s*(.*)$/);
        if (match) {
            const result = {
                date: match[1],
                time: match[2],
                pid: match[3],
                tid: match[4],
                level: match[5],
                tag: match[6].trim(),
                message: match[7] || '',
                package: '',
                raw: line
            };
            
            // 尝试从消息中提取包名（如果有的话）
            // 例如: pn=com.konec.smarthome
            const packageMatch = line.match(/\bpn=([a-zA-Z0-9._]+)/);
            if (packageMatch) {
                result.package = packageMatch[1];
            }
            // 或者从TAG中提取包名格式
            // 例如: package:konec_smart_home/
            const packageFromTag = line.match(/package:([^/\s]+)/);
            if (packageFromTag) {
                result.package = packageFromTag[1].replace(/_/g, '.');
            }
            
            return result;
        }
        
        // 格式2: long格式
        // [ MM-DD HH:MM:SS.mmm  PID: TID L/TAG ]
        // 注意：PID前可能有多个空格，PID和TID之间是冒号
        match = line.match(/^\[\s*(\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+):\s*(\d+)\s+([VDIWEFA])\/([^\]]+)\s*\]\s*(.*)$/);
        if (match) {
            return {
                date: match[1],
                time: match[2],
                pid: match[3],
                tid: match[4],
                level: match[5],
                tag: match[6].trim(),
                message: match[7] || '',
                package: '',
                raw: line
            };
        }
        
        // 格式3: 简化格式（可能出现在某些设备上）
        // MM-DD HH:MM:SS.mmm LEVEL/TAG(PID): MESSAGE
        match = line.match(/^(\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2}\.\d{3})\s+([VDIWEFA])\/([^\(]+)\((\d+)\):\s+(.*)$/);
        if (match) {
            return {
                date: match[1],
                time: match[2],
                pid: match[5],
                tid: match[5],
                level: match[3],
                tag: match[4].trim(),
                message: match[6],
                package: '',
                raw: line
            };
        }
        
        // 格式4: 特殊格式（如你提供的日志中的一些行）
        // scene=COMMON pn=com.konec.smarthome
        // 或者其他非标准格式
        if (line.includes('pn=')) {
            const packageMatch = line.match(/pn=([a-zA-Z0-9._]+)/);
            if (packageMatch) {
                return {
                    date: '',
                    time: '',
                    pid: '',
                    tid: '',
                    level: 'V',
                    tag: 'system',
                    message: line,
                    package: packageMatch[1],
                    raw: line
                };
            }
        }
        
        // 格式5: 只有消息内容的行（可能是多行日志的续行）
        // 例如: pre-Filtered:com.konec.smarthome # com.android.settings # ...
        if (line.includes('pre-Filtered:') || line.includes('ThermalControllHandler:') || 
            line.includes('setSmartCoolDown:') || line.includes('setFrameRateTarget')) {
            // 尝试提取包名
            let extractedPackage = '';
            const pkgMatch = line.match(/\b([a-zA-Z][a-zA-Z0-9_]*(\.[a-zA-Z][a-zA-Z0-9_]*)+)\b/);
            if (pkgMatch) {
                extractedPackage = pkgMatch[1];
            }
            
            return {
                date: '',
                time: '',
                pid: '',
                tid: '',
                level: 'V',
                tag: 'system',
                message: line,
                package: extractedPackage,
                raw: line
            };
        }
        
        // 如果都不匹配，返回原始行
        const result = {
            raw: line,
            message: line,
            date: '',
            time: '',
            pid: '',
            tid: '',
            level: 'V',
            tag: '',
            package: ''
        };
        
        // 尝试提取任何可能的包名
        const genericPackageMatch = line.match(/\b([a-zA-Z][a-zA-Z0-9_]*(\.[a-zA-Z][a-zA-Z0-9_]*)+)\b/);
        if (genericPackageMatch) {
            result.package = genericPackageMatch[1];
        }
        
        // 调试输出（采样）
        if (Math.random() < 0.02) {
            console.log('[DEBUG] Parsed result:', result);
        }
        
        return result;
    }

    // 添加日志条目
    addLogEntry(entry) {
        // 跳过空条目
        if (!entry) {
            return;
        }
        
        // 合并多行消息
        if (entry.multilineMessage && entry.multilineMessage.length > 0) {
            // 将多行消息合并为单个消息字符串
            entry.message = entry.multilineMessage.join('\n');
        }
        
        // 添加到缓冲区
        this.logBuffer.push(entry);
        
        // 限制缓冲区大小
        if (this.logBuffer.length > this.maxBufferSize) {
            this.logBuffer.shift();
        }

        // 调试输出
        if (this.logBuffer.length % 100 === 0) {
            console.log('[LogManager] Buffer size:', this.logBuffer.length, 'entries');
        }

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

    // 双模式过滤逻辑
    shouldShowEntry(entry) {
        // 对于多行日志，需要检查所有内容
        const fullContent = entry.raw || '';
        if (this.filterMode === 'expert') {
            // Expert模式：使用filterSpec
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
        } else {
            // Simple模式：使用分离的过滤器
            
            // 检查日志级别
            if (!this.shouldShowLevel(entry.level, this.filters.level)) {
                return false;
            }
            
            // 检查Tag过滤 - 改进匹配逻辑，支持多个tag
            if (this.filters.tag && this.filters.tag.trim()) {
                const filterTag = this.filters.tag.trim().toLowerCase();
                const entryTag = (entry.tag || '').toLowerCase();
                const entryMessage = (entry.message || '').toLowerCase();
                
                // 支持多个tag（逗号分隔）
                const filterTags = filterTag.split(',').map(t => t.trim()).filter(t => t);
                let tagMatched = false;
                
                for (const ft of filterTags) {
                    // 检查tag和消息内容
                    if (entryTag.includes(ft) || entryMessage.includes(ft)) {
                        tagMatched = true;
                        break;
                    }
                }
                
                if (!tagMatched && filterTags.length > 0) {
                    return false;
                }
            }
            
            // 检查Package过滤 - 优先使用解析出的package字段
            if (this.filters.package && this.filters.package.trim()) {
                const filterPackage = this.filters.package.trim().toLowerCase();
                
                // 先检查解析出的package字段
                const entryPackage = (entry.package || '').toLowerCase();
                let packageMatched = false;
                
                if (entryPackage && entryPackage.includes(filterPackage)) {
                    packageMatched = true;
                } else {
                    // 如果没有package字段或不匹配，在整个日志中搜索
                    const rawLog = (entry.raw || '').toLowerCase();
                    const message = (entry.message || '').toLowerCase();
                    
                    if (rawLog.includes(filterPackage) || message.includes(filterPackage)) {
                        packageMatched = true;
                    }
                }
                
                if (!packageMatched) {
                    return false;
                }
            }
        }
        
        // 通用搜索（两种模式都支持）
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

    // 添加日志到视图 - 增强版支持Windows平台
    appendLogToView(entry) {
        const logContainer = document.getElementById('logContainer');
        if (!logContainer) return;

        const logElement = document.createElement('div');
        logElement.className = `log-entry`;
        
        // 根据日志级别添加类名用于颜色标记
        if (entry.level) {
            logElement.className += ` log-level-${entry.level}`;
        }
        
        // 格式化日志行
        let formattedLine = this.formatLogLine(entry);
        
        // Windows平台特殊处理：确保显示正确
        if (navigator.platform.indexOf('Win') === 0) {
            // 清理可能残留的编码问题
            formattedLine = this.sanitizeForDisplay(formattedLine);
        }
        
        logElement.innerHTML = formattedLine;
        logElement.title = entry.raw; // 悬停显示完整行
        
        // 添加数据属性，便于过滤和搜索
        logElement.dataset.level = entry.level || 'V';
        logElement.dataset.tag = entry.tag || '';
        logElement.dataset.pid = entry.pid || '';
        logElement.dataset.tid = entry.tid || '';
        
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

    // 格式化日志行显示 - 标准adb logcat格式
    formatLogLine(entry) {
        // 严格按照adb logcat格式：MM-DD HH:MM:SS.mmm  PID  TID L TAG: MESSAGE
        let formatted = '';
        
        if (entry.level && entry.tag && entry.date && entry.time) {
            // 标准logcat格式的各个部分
            const timestamp = `${entry.date} ${entry.time}`;
            const pid = entry.pid || '';
            const tid = entry.tid || entry.pid || '';
            const level = entry.level;
            const tag = entry.tag;
            
            // 消息内容处理
            let message = '';
            if (entry.message) {
                // 将多行消息合并为单行，保持原始内容
                message = entry.message
                    .split('\n')
                    .map(line => line.trim())
                    .filter(line => line)
                    .join(' ');
            }
            
            // 构建固定宽度的格式化字符串
            formatted = `<span class="log-timestamp">${this.padRight(timestamp, 18)}</span>` +
                       `<span class="log-pid">${this.padLeft(pid, 5)}</span>` +
                       `<span class="log-tid">${this.padLeft(tid, 5)}</span>` +
                       ` <span class="log-level log-level-${level}">${level}</span>` +
                       ` <span class="log-tag">${this.padRight(tag, 25)}</span>` +
                       `: <span class="log-message">${this.escapeHtml(message)}</span>`;
        } else {
            // 如果解析失败，显示原始内容
            const rawText = entry.raw
                .split('\n')
                .map(line => line.trim())
                .filter(line => line)
                .join(' ');
            formatted = this.escapeHtml(rawText);
        }
        
        // 高亮搜索关键词
        if (this.filters.search && !this.filters.regex) {
            const searchTerm = this.filters.caseSensitive ? 
                this.filters.search : this.filters.search.toLowerCase();
            const regex = new RegExp(`(${this.escapeRegex(searchTerm)})`, 
                this.filters.caseSensitive ? 'g' : 'gi');
            formatted = formatted.replace(regex, '<mark>$1</mark>');
        }
        
        return formatted;
    }

    // 辅助函数：右填充字符串到指定长度
    padRight(str, length) {
        str = str.toString();
        while (str.length < length) {
            str += ' ';
        }
        return str.substring(0, length);
    }

    // 辅助函数：左填充字符串到指定长度
    padLeft(str, length) {
        str = str.toString();
        while (str.length < length) {
            str = ' ' + str;
        }
        return str.substring(0, length);
    }

    // 清理显示内容（Windows平台）
    sanitizeForDisplay(text) {
        return text
            // 移除不可见字符
            .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
            // 替换常见的编码错误字符
            .replace(/[\u0080-\u009F]/g, '')
            // 确保没有破坏HTML标签
            .replace(/<([^>]+)>/g, function(match, p1) {
                // 保护HTML标签不被破坏
                return match;
            });
    }

    // HTML转义
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // 正则表达式转义
    escapeRegex(string) {
        return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
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

    // 应用过滤器 - 异步版本
    async applyFilters() {
        // 清除之前的延时器
        if (this.filterTimeout) {
            clearTimeout(this.filterTimeout);
        }
        
        // 设置新的延时器，避免频繁过滤
        this.filterTimeout = setTimeout(async () => {
            await this.performAsyncFiltering();
        }, 200); // 200ms延迟
    }
    
    // 执行异步过滤
    async performAsyncFiltering() {
        const logContainer = document.getElementById('logContainer');
        if (!logContainer) return;

        // 显示loading状态
        this.showLoadingState();
        
        try {
            // 使用setTimeout将任务分解，避免阻塞UI
            await new Promise(resolve => {
                setTimeout(() => {
                    // 调试输出
                    console.log('应用过滤器:', this.filters);
                    console.log('缓冲区日志总数:', this.logBuffer.length);
                    
                    // 统计符合条件的日志
                    let matchCount = 0;

                    // 清空当前显示
                    logContainer.innerHTML = '';

                    // 分批处理日志，避免一次性处理太多导致卡顿
                    this.processBatchedLogs(0, 0).then((finalMatchCount) => {
                        console.log(`过滤完成: 总共${this.logBuffer.length}条日志，符合过滤条件${finalMatchCount}条`);
                        resolve();
                    });
                }, 0);
            });
        } finally {
            // 隐藏loading状态
            this.hideLoadingState();
        }
    }
    
    // 分批处理日志
    async processBatchedLogs(startIndex, matchCount) {
        const batchSize = 100; // 每批处理100条
        const endIndex = Math.min(startIndex + batchSize, this.logBuffer.length);
        
        for (let i = startIndex; i < endIndex; i++) {
            const entry = this.logBuffer[i];
            if (this.shouldShowEntry(entry)) {
                matchCount++;
                this.appendLogToView(entry);
            }
        }
        
        // 如果还有更多日志需要处理，继续下一批
        if (endIndex < this.logBuffer.length) {
            // 使用setTimeout让出控制权，避免阻塞UI
            await new Promise(resolve => {
                setTimeout(() => {
                    this.processBatchedLogs(endIndex, matchCount).then(resolve);
                }, 1);
            });
        }
        
        return matchCount;
    }

    // 清空日志
    clearLogs() {
        this.logBuffer = [];
        const logContainer = document.getElementById('logContainer');
        if (logContainer) {
            logContainer.innerHTML = '';
        }
    }

    // 导出日志 - 导出和Log Viewer显示完全一致的内容
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
                const logs = [];
                
                // 导出和Log Viewer显示完全一致的格式化内容
                this.logBuffer.forEach(entry => {
                    if (this.shouldShowEntry(entry)) {
                        // 使用相同的格式化函数，但生成纯文本版本
                        const formattedLine = this.formatLogLineForExport(entry);
                        logs.push(formattedLine);
                    }
                });

                // 写入文件
                await getGlobals().fs.writeFile(result.filePath, logs.join('\n'), 'utf8');
                this.showNotification(`已导出 ${logs.length} 条日志`, 'success');
            }
        } catch (error) {
            console.error('Failed to export logs:', error);
            this.showNotification('Failed to export logs', 'error');
        }
    }

    // 为导出生成纯文本格式的日志行
    formatLogLineForExport(entry) {
        // 和formatLogLine使用相同的逻辑，但生成纯文本
        if (entry.level && entry.tag && entry.date && entry.time) {
            // 标准logcat格式的各个部分
            const timestamp = `${entry.date} ${entry.time}`;
            const pid = entry.pid || '';
            const tid = entry.tid || entry.pid || '';
            const level = entry.level;
            const tag = entry.tag;
            
            // 消息内容处理
            let message = '';
            if (entry.message) {
                // 将多行消息合并为单行，保持原始内容
                message = entry.message
                    .split('\n')
                    .map(line => line.trim())
                    .filter(line => line)
                    .join(' ');
                
                // 清理可能的编码问题
                message = this.cleanMessageForExport(message);
            }
            
            // 构建固定宽度的格式化字符串（纯文本版本）
            return `${this.padRight(timestamp, 18)}${this.padLeft(pid, 5)}${this.padLeft(tid, 5)} ${level} ${this.padRight(tag, 25)}: ${message}`;
        } else {
            // 如果解析失败，清理原始内容
            const rawText = entry.raw
                .split('\n')
                .map(line => line.trim())
                .filter(line => line)
                .join(' ');
            return this.cleanMessageForExport(rawText);
        }
    }

    // 清理消息内容用于导出
    cleanMessageForExport(message) {
        return message
            // 移除不可见字符和控制字符
            .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '')
            // 替换常见的编码问题字符
            .replace(/\ufffd/g, '?')
            // 清理其他可能导致问题的字符
            .replace(/[\u0080-\u009F]/g, '')
            // 确保没有多余的空白字符
            .replace(/\s+/g, ' ')
            .trim();
    }

    // 显示loading状态
    showLoadingState() {
        const logContainer = document.getElementById('logContainer');
        if (!logContainer || this.isFiltering) return;
        
        this.isFiltering = true;
        logContainer.classList.add('loading');
        
        // 创建loading overlay
        const overlay = document.createElement('div');
        overlay.className = 'log-loading-overlay';
        overlay.id = 'logLoadingOverlay';
        
        const spinner = document.createElement('div');
        spinner.className = 'log-loading-spinner';
        
        const text = document.createElement('div');
        text.className = 'log-loading-text';
        text.textContent = `正在过滤 ${this.logBuffer.length} 条日志...`;
        
        overlay.appendChild(spinner);
        overlay.appendChild(text);
        logContainer.appendChild(overlay);
    }
    
    // 隐藏loading状态
    hideLoadingState() {
        const logContainer = document.getElementById('logContainer');
        const overlay = document.getElementById('logLoadingOverlay');
        
        if (logContainer) {
            logContainer.classList.remove('loading');
        }
        
        if (overlay) {
            overlay.remove();
        }
        
        this.isFiltering = false;
    }
    
    // 为输入框添加filtering状态
    setInputFilteringState(inputElement, isFiltering) {
        if (!inputElement) return;
        
        if (isFiltering) {
            inputElement.classList.add('filtering');
        } else {
            inputElement.classList.remove('filtering');
        }
    }

    // 显示通知
    showNotification(message, type = 'info') {
        if (window.AppNotifications) {
            const methods = {
                'success': 'success',
                'error': 'error',
                'warning': 'warn',
                'warn': 'warn',
                'info': 'info'
            };
            const method = methods[type] || 'info';
            window.AppNotifications[method](message);
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
