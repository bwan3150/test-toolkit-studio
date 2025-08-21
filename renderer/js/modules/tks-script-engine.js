// TKS脚本引擎模块
// 用于解析和执行.tks测试脚本

// 使用立即执行函数避免全局变量污染
(function() {
    // 获取全局模块的引用，避免与全局变量冲突
    let tksFs, tksPath;
    
    const initModules = () => {
        if (window.AppGlobals) {
            tksFs = window.AppGlobals.fs;
            tksPath = window.AppGlobals.path;
        } else {
            tksFs = (window.nodeRequire ? window.nodeRequire('fs') : require('fs')).promises;
            tksPath = window.nodeRequire ? window.nodeRequire('path') : require('path');
        }
    };
    
    // 立即初始化
    initModules();

/**
 * TKS脚本解析器类
 * 负责解析.tks脚本文件并转换为可执行的指令序列
 */
class TKSScriptParser {
    constructor() {
        // 支持的命令类型
        this.commandTypes = {
            '启动': 'launch',
            '关闭': 'close',
            '点击': 'click',
            '按压': 'press',
            '滑动': 'swipe',
            '定向滑动': 'directional_swipe',
            '输入': 'input',
            '清理': 'clear',
            '隐藏键盘': 'hide_keyboard',
            '返回': 'back',
            '等待': 'wait',
            '断言': 'assert'
        };

        // 方向映射
        this.directions = {
            '上': 'up',
            '下': 'down',
            '左': 'left',
            '右': 'right'
        };
    }

    /**
     * 解析.tks脚本文件
     * @param {string} scriptPath - 脚本文件路径
     * @returns {Object} 解析后的脚本对象
     */
    async parseFile(scriptPath) {
        const content = await tksFs.readFile(scriptPath, 'utf-8');
        return this.parse(content);
    }

    /**
     * 解析脚本内容
     * @param {string} content - 脚本内容
     * @returns {Object} 解析后的脚本对象
     */
    parse(content) {
        const originalLines = content.split('\n');
        const lines = originalLines.map(line => line.trim());
        const script = {
            caseId: '',
            scriptName: '',
            details: {},
            steps: [],
            metadata: {
                createTime: new Date().toISOString(),
                version: '1.0'
            }
        };

        let currentSection = null;
        let currentStep = null;
        let inDetails = false;

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const originalLine = originalLines[i]; // 保留原始行以便准确计算行号
            
            // 跳过空行和注释（但仍要计算正确的行号）
            if (!line || line.startsWith('#')) continue;

            // 解析用例ID
            if (line.startsWith('用例:')) {
                script.caseId = line.substring(3).trim();
                continue;
            }

            // 解析脚本名
            if (line.startsWith('脚本名:')) {
                script.scriptName = line.substring(4).trim();
                continue;
            }

            // 进入详情部分
            if (line === '详情:') {
                inDetails = true;
                currentSection = 'details';
                continue;
            }

            // 进入步骤部分
            if (line === '步骤:') {
                inDetails = false;
                currentSection = 'steps';
                continue;
            }

            // 解析详情内容
            if (inDetails && line.includes(':')) {
                const [key, value] = line.split(':').map(s => s.trim());
                script.details[key] = value;
                continue;
            }

            // 解析步骤
            if (currentSection === 'steps') {
                const step = this.parseStep(line, i + 1); // i + 1 是正确的1基行号
                if (step) {
                    script.steps.push(step);
                }
            }
        }

        return script;
    }

    /**
     * 解析单个步骤
     * @param {string} line - 步骤行
     * @param {number} lineNumber - 行号
     * @returns {Object|null} 步骤对象
     */
    parseStep(line, lineNumber = 0) {
        // 先尝试匹配方括号格式：命令 [参数1, 参数2]
        let match = line.match(/^(\S+)\s*\[(.*)\]$/);
        let command, paramsStr;
        
        if (match) {
            // 方括号格式
            command = match[1];
            paramsStr = match[2];
        } else {
            // 尝试匹配简单格式：命令 参数1 参数2
            match = line.match(/^(\S+)\s+(.*)$/);
            if (match) {
                command = match[1];
                paramsStr = match[2];
                // 将简单格式转换为方括号格式以便统一处理
                paramsStr = paramsStr.trim();
            } else {
                // 只有命令，没有参数
                match = line.match(/^(\S+)$/);
                if (match) {
                    command = match[1];
                    paramsStr = '';
                } else {
                    return null;
                }
            }
        }

        const commandType = this.commandTypes[command];
        
        if (!commandType) {
            console.warn(`未知命令: ${command}`);
            return null;
        }

        // 解析参数
        const params = this.parseParameters(paramsStr);
        
        return {
            type: commandType,
            command: command,
            params: params,
            raw: line,
            lineNumber: lineNumber
        };
    }

    /**
     * 解析参数字符串
     * @param {string} paramsStr - 参数字符串
     * @returns {Array} 参数数组
     */
    parseParameters(paramsStr) {
        if (!paramsStr.trim()) return [];
        
        // 使用正则表达式来分割参数，支持坐标格式
        const params = [];
        let current = '';
        let inQuotes = false;
        let quoteChar = '';
        
        for (let i = 0; i < paramsStr.length; i++) {
            const char = paramsStr[i];
            
            if ((char === '"' || char === "'") && !inQuotes) {
                inQuotes = true;
                quoteChar = char;
                current += char;
            } else if (char === quoteChar && inQuotes) {
                inQuotes = false;
                current += char;
                quoteChar = '';
            } else if (char === ',' && !inQuotes) {
                // 检查是否是坐标中的逗号
                const trimmed = current.trim();
                const remaining = paramsStr.slice(i + 1).trim();
                
                // 如果当前部分是数字且后面紧跟数字，这是坐标格式
                if (/^\d+$/.test(trimmed) && /^\d+/.test(remaining.split(/[\s,]/)[0])) {
                    current += char;
                } else {
                    // 这是参数分隔符
                    if (current.trim()) {
                        params.push(this.parseParameter(current.trim()));
                    }
                    current = '';
                }
            } else if (char === ' ' && !inQuotes) {
                // 空格分割，但要考虑是否在坐标后面
                const trimmed = current.trim();
                if (trimmed) {
                    // 检查下一个非空字符
                    let nextChar = '';
                    for (let j = i + 1; j < paramsStr.length; j++) {
                        if (paramsStr[j] !== ' ') {
                            nextChar = paramsStr[j];
                            break;
                        }
                    }
                    
                    // 如果当前是坐标格式且下一个是数字，继续添加到当前参数
                    if (trimmed.includes(',') && /^\d/.test(nextChar)) {
                        current += char;
                    } else {
                        params.push(this.parseParameter(trimmed));
                        current = '';
                    }
                }
            } else {
                current += char;
            }
        }
        
        if (current.trim()) {
            params.push(this.parseParameter(current.trim()));
        }
        
        return params;
    }

    /**
     * 解析单个参数
     * @param {string} param - 参数字符串
     * @returns {*} 解析后的参数值
     */
    parseParameter(param) {
        // 移除引号
        if ((param.startsWith('"') && param.endsWith('"')) ||
            (param.startsWith("'") && param.endsWith("'"))) {
            return param.slice(1, -1);
        }
        
        // 解析数字
        if (/^\d+$/.test(param)) {
            return parseInt(param);
        }
        
        // 解析时间（如 10s）
        if (/^\d+s$/.test(param)) {
            return {
                type: 'duration',
                value: parseInt(param.slice(0, -1)),
                unit: 'seconds'
            };
        }
        
        // 解析坐标 {x,y}
        if (/^\{\d+,\d+\}$/.test(param)) {
            const coordStr = param.slice(1, -1); // 去掉花括号
            const [x, y] = coordStr.split(',').map(n => parseInt(n));
            return { type: 'coordinate', x, y };
        }
        
        // 解析图像引用 @{图片名称}
        if (param.startsWith('@{') && param.endsWith('}')) {
            const imageName = param.slice(2, -1);
            return { type: 'image_reference', name: imageName };
        }
        
        // 解析XML元素引用 {元素名称}
        if (param.startsWith('{') && param.endsWith('}')) {
            const elementName = param.slice(1, -1);
            // 直接返回元素名称字符串，不需要特殊类型标记
            // 因为findElement方法会处理字符串类型的选择器
            return elementName;
        }
        
        // 解析方向
        if (this.directions[param]) {
            return { type: 'direction', value: this.directions[param] };
        }
        
        // 解析布尔值
        if (param === '存在' || param === 'true') return true;
        if (param === '不存在' || param === 'false') return false;
        
        // 默认返回字符串
        return param;
    }
}

/**
 * TKS脚本执行器类
 * 负责执行解析后的脚本指令
 */
class TKSScriptExecutor {
    constructor(projectPath, deviceId = null) {
        if (!projectPath || typeof projectPath !== 'string') {
            console.error('TKSScriptExecutor: 无效的项目路径:', projectPath);
            throw new Error('TKS执行器需要有效的项目路径');
        }
        
        this.projectPath = projectPath;
        this.deviceId = deviceId;
        this.isRunning = false;
        this.currentStep = 0;
        this.elementCache = new Map();
        this.currentCommand = null; // 当前执行的命令
        this.shouldStop = false; // 停止标志
    }

    /**
     * 执行脚本
     * @param {Object} script - 解析后的脚本对象
     * @returns {Object} 执行结果
     */
    async execute(script) {
        this.isRunning = true;
        this.shouldStop = false; // 重置停止标志
        this.currentStep = 0;
        
        // 设置编辑器为测试运行状态
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            window.AppGlobals.codeEditor.setTestRunning(true);
        }
        
        // 在脚本开始执行时设置编辑器为测试运行状态
        // 注意：不在这里设置，让 highlightExecutingLine 第一次调用时设置，避免重复调用
        
        const result = {
            success: true,
            caseId: script.caseId,
            scriptName: script.scriptName,
            startTime: new Date().toISOString(),
            endTime: null,
            steps: [],
            error: null
        };

        try {
            // 不再在开始时获取UI，改为每个步骤执行前刷新
            console.log('TKS执行器: 开始执行脚本步骤...');
            
            // 执行每个步骤
            for (const step of script.steps) {
                if (!this.isRunning || this.shouldStop) {
                    result.error = '执行被中止';
                    result.success = false;
                    break;
                }

                this.currentStep++;
                // console.log(`执行步骤 ${this.currentStep}: ${step.raw}`); // 已禁用以减少日志
                
                // 高亮当前执行的行
                if (step.lineNumber && window.AppGlobals.codeEditor) {
                    // console.log(`高亮执行行: ${step.lineNumber}`); // 已禁用以减少日志
                    window.AppGlobals.codeEditor.highlightExecutingLine(step.lineNumber);
                }
                
                const stepResult = await this.executeStep(step, script);
                result.steps.push({
                    index: this.currentStep,
                    ...stepResult
                });

                if (!stepResult.success) {
                    result.success = false;
                    result.error = stepResult.error;
                    // 出错时高亮错误行（保持红色高亮）
                    if (step.lineNumber && window.AppGlobals.codeEditor) {
                        // console.log(`高亮错误行: ${step.lineNumber}`); // 已禁用以减少日志
                        window.AppGlobals.codeEditor.highlightErrorLine(step.lineNumber);
                    }
                    break;
                }
            }
        } catch (error) {
            result.success = false;
            result.error = error.message;
            // 异常时不清除高亮，让用户看到出错的位置
            console.log('TKS执行器: 发生异常，保持当前高亮状态');
        }

        result.endTime = new Date().toISOString();
        this.isRunning = false;
        
        // 脚本执行结束后恢复编辑器状态
        if (window.AppGlobals.codeEditor) {
            if (result.success) {
                // console.log('脚本成功完成，清除编辑器执行行高亮'); // 已禁用以减少日志
                window.AppGlobals.codeEditor.clearExecutionHighlight();
            } else {
                // console.log('脚本执行失败，保持错误行高亮但恢复编辑状态'); // 已禁用以减少日志
                // 失败时也要恢复编辑状态，让用户可以修改代码
                if (window.AppGlobals.codeEditor.setTestRunning) {
                    window.AppGlobals.codeEditor.setTestRunning(false);
                }
            }
        }
        
        // 脚本执行完成后进行最终UI刷新，显示结果状态
        try {
            // console.log('TKS执行器: 执行完成，进行最终UI刷新...'); // 已禁用以减少日志
            await this.refreshUIForStep();
        } catch (error) {
            console.warn('TKS执行器: 最终UI刷新失败:', error);
        }
        
        // 保存执行结果
        await this.saveResult(result);
        
        return result;
    }

    /**
     * 执行单个步骤
     * @param {Object} step - 步骤对象
     * @param {Object} script - 完整脚本对象（用于获取上下文）
     * @returns {Object} 步骤执行结果
     */
    async executeStep(step, script) {
        // 在步骤开始前检查是否需要停止
        if (this.shouldStop) {
            return {
                type: step.type,
                command: step.command,
                params: step.params,
                success: false,
                error: '执行被中止',
                duration: 0
            };
        }
        
        const startTime = Date.now();
        const result = {
            type: step.type,
            command: step.command,
            params: step.params,
            success: true,
            error: null,
            duration: 0
        };

        try {
            // 步骤执行前刷新UI状态
            console.log(`步骤${this.currentStep}: 刷新设备UI状态...`);
            await this.refreshUIForStep();
            
            if (!this.currentElements || this.currentElements.length === 0) {
                console.warn(`步骤${this.currentStep}: 未能获取到UI元素`);
            } else {
                // console.log(`步骤${this.currentStep}: 已获取到 ${this.currentElements.length} 个UI元素`); // 已禁用以减少日志
            }
        } catch (error) {
            console.error(`步骤${this.currentStep}: UI刷新失败:`, error);
            // UI刷新失败不影响步骤执行，继续执行
        }

        // 注释掉原来的captureCurrentState，因为refreshUIForStep已经包含了这个功能
        // try {
        //     // 获取当前UI状态，缓存元素信息供操作使用
        //     await this.captureCurrentState();
        
        try {
            
            switch (step.type) {
                case 'launch':
                    await this.executeLaunch(step.params, script.details);
                    break;
                case 'close':
                    await this.executeClose(step.params);
                    break;
                case 'click':
                    await this.executeClick(step.params);
                    break;
                case 'press':
                    await this.executePress(step.params);
                    break;
                case 'swipe':
                    await this.executeSwipe(step.params);
                    break;
                case 'directional_swipe':
                    await this.executeDirectionalSwipe(step.params);
                    break;
                case 'input':
                    await this.executeInput(step.params);
                    break;
                case 'clear':
                    await this.executeClear(step.params);
                    break;
                case 'hide_keyboard':
                    await this.executeHideKeyboard();
                    break;
                case 'back':
                    await this.executeBack();
                    break;
                case 'wait':
                    await this.executeWait(step.params);
                    break;
                case 'assert':
                    await this.executeAssert(step.params);
                    break;
                default:
                    throw new Error(`未知的命令类型: ${step.type}`);
            }
        } catch (error) {
            result.success = false;
            result.error = error.message;
        }

        result.duration = Date.now() - startTime;
        return result;
    }

    /**
     * 步骤级UI刷新 - 获取截图、UI树并更新前端显示
     */
    async refreshUIForStep() {
        const { ipcRenderer } = window.AppGlobals;
        
        try {
            // 1. 获取设备截图和UI树，自动保存到workarea
            // console.log(`TKS执行器: 获取设备截图和UI树...`); // 已禁用以减少日志
            const projectPath = this.projectPath;
            const screenshotResult = await ipcRenderer.invoke('adb-screenshot', this.deviceId, projectPath);
            
            if (screenshotResult.success) {
                // 2. 更新前端DEVICE SCREEN显示
                const img = document.getElementById('deviceScreenshot');
                if (img) {
                    img.src = `data:image/png;base64,${screenshotResult.data}`;
                    img.style.display = 'block';
                    const placeholder = document.querySelector('.screen-placeholder');
                    if (placeholder) placeholder.style.display = 'none';
                }
                // console.log(`TKS执行器: 截图已更新到前端显示`); // 已禁用以减少日志
            }
            
            // 3. 获取UI树并解析元素
            const uiDumpResult = await ipcRenderer.invoke('adb-ui-dump-enhanced', this.deviceId);
            if (uiDumpResult.success && uiDumpResult.xml) {
                // 解析UI树并缓存元素
                const parser = new window.XMLParser();
                if (uiDumpResult.screenSize) {
                    parser.setScreenSize(uiDumpResult.screenSize.width, uiDumpResult.screenSize.height);
                }
                
                // console.log(`TKS执行器: XML长度: ${uiDumpResult.xml.length}`); // 已禁用以减少日志  
                // console.log(`TKS执行器: XML前200字符:`, uiDumpResult.xml.substring(0, 200)); // 已禁用以减少日志
                
                const optimizedTree = parser.optimizeUITree(uiDumpResult.xml);
                // console.log(`TKS执行器: 优化后的树:`, optimizedTree ? '成功' : '失败'); // 已禁用以减少日志
                
                // 如果优化失败，传递原始XML作为回退
                const elements = parser.extractUIElements(optimizedTree, uiDumpResult.xml);
                // console.log(`TKS执行器: 提取到 ${elements.length} 个元素`); // 已禁用以减少日志
                
                // 4. 更新实例状态
                this.currentElements = elements;
                this.currentUITree = uiDumpResult.xml;
                
                // console.log(`TKS执行器: UI树已更新，共解析 ${elements.length} 个元素`); // 已禁用以减少日志
                
                // 5. 保存UI树到workarea（如果没有被截图接口保存的话）
                if (projectPath && window.AppGlobals.fs && window.AppGlobals.path) {
                    try {
                        const { fs, path } = window.AppGlobals;
                        const workareaDir = path.join(projectPath, 'workarea');
                        const xmlPath = path.join(workareaDir, 'current_ui_tree.xml');
                        
                        // 确保workarea目录存在
                        await fs.mkdir(workareaDir, { recursive: true });
                        // 保存UI树
                        await fs.writeFile(xmlPath, uiDumpResult.xml, 'utf8');
                        // console.log(`TKS执行器: UI树已保存到 ${xmlPath}`); // 已禁用以减少日志
                    } catch (error) {
                        console.warn(`TKS执行器: 保存UI树到workarea失败:`, error);
                    }
                }
            } else {
                console.warn(`TKS执行器: UI树获取失败`);
                this.currentElements = [];
                this.currentUITree = null;
            }
            
        } catch (error) {
            console.error(`TKS执行器: refreshUIForStep失败:`, error);
            this.currentElements = [];
            this.currentUITree = null;
        }
    }

    /**
     * 获取当前UI状态（仅用于缓存元素，不更新屏幕显示）
     */
    async captureCurrentState() {
        const { ipcRenderer } = window.AppGlobals;
        
        // 仅获取UI树用于元素查找，屏幕更新在执行前已完成
        const uiDumpResult = await ipcRenderer.invoke('adb-ui-dump-enhanced', this.deviceId);
        if (uiDumpResult.success) {
            // 解析UI树并缓存元素供后续查找使用
            const parser = new window.XMLParser();
            if (uiDumpResult.screenSize) {
                parser.setScreenSize(uiDumpResult.screenSize.width, uiDumpResult.screenSize.height);
            }
            
            const optimizedTree = parser.optimizeUITree(uiDumpResult.xml);
            const elements = parser.extractUIElements(optimizedTree || uiDumpResult.xml);
            
            // 缓存元素供后续使用
            this.currentElements = elements;
            this.currentUITree = uiDumpResult.xml;
        }
    }

    /**
     * 获取当前UI元素数据
     * @returns {Array} UI元素数组
     */
    async getCurrentUIElements() {
        const { ipcRenderer } = window.AppGlobals;
        
        try {
            console.log('TKS执行器: 获取当前UI元素数据...');
            const uiDumpResult = await ipcRenderer.invoke('adb-ui-dump-enhanced', this.deviceId);
            
            if (uiDumpResult.success && uiDumpResult.xml) {
                // 解析UI树并缓存元素
                const parser = new window.XMLParser();
                if (uiDumpResult.screenSize) {
                    parser.setScreenSize(uiDumpResult.screenSize.width, uiDumpResult.screenSize.height);
                }
                
                console.log(`TKS执行器: XML长度: ${uiDumpResult.xml.length}`);
                
                const optimizedTree = parser.optimizeUITree(uiDumpResult.xml);
                // 如果优化失败，传递原始XML作为回退
                const elements = parser.extractUIElements(optimizedTree, uiDumpResult.xml);
                
                console.log(`TKS执行器: 解析得到 ${elements.length} 个UI元素`);
                
                // 更新实例状态
                this.currentElements = elements;
                this.currentUITree = uiDumpResult.xml;
                
                // 保存UI树到workarea（确保始终保存）
                if (this.projectPath && window.AppGlobals.fs && window.AppGlobals.path) {
                    try {
                        const { fs, path } = window.AppGlobals;
                        const workareaDir = path.join(this.projectPath, 'workarea');
                        const xmlPath = path.join(workareaDir, 'current_ui_tree.xml');
                        
                        // 确保workarea目录存在
                        await fs.mkdir(workareaDir, { recursive: true });
                        // 保存UI树
                        await fs.writeFile(xmlPath, uiDumpResult.xml, 'utf8');
                        console.log(`TKS执行器: UI树已保存到 ${xmlPath}`);
                    } catch (error) {
                        console.warn(`TKS执行器: 保存UI树到workarea失败:`, error);
                    }
                }
                
                return elements;
            } else {
                console.warn('TKS执行器: UI树获取失败');
                this.currentElements = [];
                this.currentUITree = null;
                return [];
            }
        } catch (error) {
            console.error('TKS执行器: getCurrentUIElements失败:', error);
            this.currentElements = [];
            this.currentUITree = null;
            return [];
        }
    }
    

    /**
     * 启动应用
     */
    async executeLaunch(params, details) {
        const { ipcRenderer } = window.AppGlobals;
        const appPackage = params[0];
        const appActivity = params[1];
        
        if (!appPackage) {
            throw new Error('未指定应用包名');
        }
        
        if (!appActivity) {
            throw new Error('未指定应用Activity');
        }

        const command = `am start -n ${appPackage}/${appActivity}`;
        
        // console.log(`启动命令: ${command}`); // 已禁用以减少日志
        // console.log(`包名: ${appPackage}, Activity: ${appActivity}`); // 已禁用以减少日志
        
        const result = await this.runAdbCommand(command);
        if (!result.success) {
            throw new Error(`启动应用失败: ${result.error}`);
        }
        
        // 等待应用启动
        await this.sleep(2000);
    }

    /**
     * 关闭应用
     */
    async executeClose(params) {
        const appPackage = params[0];
        if (!appPackage) {
            throw new Error('未指定应用包名');
        }

        const result = await this.runAdbCommand(`am force-stop ${appPackage}`);
        if (!result.success) {
            throw new Error(`关闭应用失败: ${result.error}`);
        }
    }

    /**
     * 点击操作
     */
    async executeClick(params) {
        const target = params[0];
        
        if (!target) {
            throw new Error('点击目标未指定');
        }

        let x, y;

        // 判断目标类型
        if (typeof target === 'object' && target.type === 'coordinate') {
            // 直接使用坐标
            x = target.x;
            y = target.y;
        } else if (typeof target === 'object' && target.type === 'image_reference') {
            // 图像引用 - 查找图像元素
            console.log('处理图像引用:', target);
            const element = await this.findElement(target);
            if (!element) {
                throw new Error(`未找到图像元素: ${target.name}`);
            }
            x = element.centerX;
            y = element.centerY;
            console.log(`图像元素坐标: (${x}, ${y})`);
        } else if (typeof target === 'string') {
            // 查找元素
            const element = await this.findElement(target);
            if (!element) {
                throw new Error(`未找到元素: ${target}`);
            }
            x = element.centerX;
            y = element.centerY;
        } else {
            throw new Error(`无效的点击目标: ${JSON.stringify(target)}`);
        }

        // 检查坐标有效性
        if (x === undefined || y === undefined) {
            throw new Error(`坐标解析失败: (${x},${y})`);
        }

        const result = await this.runAdbCommand(`input tap ${x} ${y}`);
        if (!result.success) {
            throw new Error(`点击失败: ${result.error}`);
        }
    }

    /**
     * 长按操作
     */
    async executePress(params) {
        const target = params[0];
        const duration = params[1] || 1000; // 默认1秒
        
        let x, y;

        if (typeof target === 'object' && target.type === 'coordinate') {
            x = target.x;
            y = target.y;
        } else if (typeof target === 'object' && target.type === 'image_reference') {
            // 图像引用 - 查找图像元素
            const element = await this.findElement(target);
            if (!element) {
                throw new Error(`未找到图像元素: ${target.name}`);
            }
            x = element.centerX;
            y = element.centerY;
        } else if (typeof target === 'string') {
            const element = await this.findElement(target);
            if (!element) {
                throw new Error(`未找到元素: ${target}`);
            }
            x = element.centerX;
            y = element.centerY;
        }

        // 使用swipe模拟长按
        const result = await this.runAdbCommand(`input swipe ${x} ${y} ${x} ${y} ${duration}`);
        if (!result.success) {
            throw new Error(`长按失败: ${result.error}`);
        }
    }

    /**
     * 滑动操作
     */
    async executeSwipe(params) {
        const from = params[0];
        const to = params[1];
        const duration = params[2] || 300; // 默认300ms
        
        let x1, y1, x2, y2;

        // 解析起始点
        if (typeof from === 'object' && from.type === 'coordinate') {
            x1 = from.x;
            y1 = from.y;
        } else if (typeof from === 'object' && from.type === 'image_reference') {
            const element = await this.findElement(from);
            if (!element) {
                throw new Error(`未找到起始图像元素: ${from.name}`);
            }
            x1 = element.centerX;
            y1 = element.centerY;
        } else if (typeof from === 'string') {
            const element = await this.findElement(from);
            if (!element) {
                throw new Error(`未找到起始元素: ${from}`);
            }
            x1 = element.centerX;
            y1 = element.centerY;
        } else {
            throw new Error('无效的起始点参数');
        }

        // 解析终点
        if (typeof to === 'object' && to.type === 'coordinate') {
            x2 = to.x;
            y2 = to.y;
        } else if (typeof to === 'object' && to.type === 'image_reference') {
            const element = await this.findElement(to);
            if (!element) {
                throw new Error(`未找到目标图像元素: ${to.name}`);
            }
            x2 = element.centerX;
            y2 = element.centerY;
        } else if (typeof to === 'string') {
            const element = await this.findElement(to);
            if (!element) {
                throw new Error(`未找到目标元素: ${to}`);
            }
            x2 = element.centerX;
            y2 = element.centerY;
        } else {
            throw new Error('无效的终点参数');
        }

        // 检查坐标有效性
        if (x1 === undefined || y1 === undefined || x2 === undefined || y2 === undefined) {
            throw new Error(`坐标解析失败: from(${x1},${y1}) to(${x2},${y2})`);
        }

        const result = await this.runAdbCommand(`input swipe ${x1} ${y1} ${x2} ${y2} ${duration}`);
        if (!result.success) {
            throw new Error(`滑动失败: ${result.error}`);
        }
    }

    /**
     * 定向滑动操作
     */
    async executeDirectionalSwipe(params) {
        const target = params[0];
        const direction = params[1];
        const distance = params[2] || 300; // 默认300像素
        
        let x, y;

        if (typeof target === 'object' && target.type === 'coordinate') {
            x = target.x;
            y = target.y;
        } else if (typeof target === 'object' && target.type === 'image_reference') {
            const element = await this.findElement(target);
            if (!element) {
                throw new Error(`未找到图像元素: ${target.name}`);
            }
            x = element.centerX;
            y = element.centerY;
        } else if (typeof target === 'string') {
            const element = await this.findElement(target);
            if (!element) {
                throw new Error(`未找到元素: ${target}`);
            }
            x = element.centerX;
            y = element.centerY;
        }

        let x2 = x, y2 = y;
        const dir = direction.type === 'direction' ? direction.value : direction;

        switch (dir) {
            case 'up':
                y2 = y - distance;
                break;
            case 'down':
                y2 = y + distance;
                break;
            case 'left':
                x2 = x - distance;
                break;
            case 'right':
                x2 = x + distance;
                break;
            default:
                throw new Error(`无效的方向: ${dir}`);
        }

        const result = await this.runAdbCommand(`input swipe ${x} ${y} ${x2} ${y2} 300`);
        if (!result.success) {
            throw new Error(`定向滑动失败: ${result.error}`);
        }
    }

    /**
     * 输入文本
     */
    async executeInput(params) {
        const target = params[0];
        const text = params[1] || '';
        
        // 先点击输入框
        if (target) {
            await this.executeClick([target]);
            await this.sleep(500); // 等待键盘弹出
        }

        // 输入文本
        const escapedText = text.replace(/"/g, '\\"').replace(/'/g, "\\'");
        const result = await this.runAdbCommand(`input text "${escapedText}"`);
        if (!result.success) {
            throw new Error(`输入文本失败: ${result.error}`);
        }
    }

    /**
     * 清理输入框
     */
    async executeClear(params) {
        const target = params[0];
        
        // 先点击输入框
        if (target) {
            await this.executeClick([target]);
            await this.sleep(500);
        }

        // 全选并删除
        await this.runAdbCommand('input keyevent KEYCODE_MOVE_END');
        await this.runAdbCommand('input keyevent --longpress $(printf "KEYCODE_DEL %.0s" {1..50})');
    }

    /**
     * 隐藏键盘
     */
    async executeHideKeyboard() {
        const result = await this.runAdbCommand('input keyevent KEYCODE_BACK');
        if (!result.success) {
            // 尝试其他方法
            await this.runAdbCommand('input keyevent 111'); // KEYCODE_ESCAPE
        }
    }

    /**
     * 返回操作
     */
    async executeBack() {
        const result = await this.runAdbCommand('input keyevent KEYCODE_BACK');
        if (!result.success) {
            throw new Error(`返回操作失败: ${result.error}`);
        }
    }

    /**
     * 等待操作
     */
    async executeWait(params) {
        const target = params[0];
        
        if (!target) {
            throw new Error('等待目标未指定');
        }

        // 判断是时间等待还是元素等待
        if (typeof target === 'object' && target.type === 'duration') {
            // 时间等待
            await this.sleep(target.value * 1000);
        } else if (typeof target === 'string') {
            // 元素等待
            const timeout = 30000; // 最多等待30秒
            const startTime = Date.now();
            
            while (Date.now() - startTime < timeout) {
                // 检查是否需要停止
                if (this.shouldStop) {
                    throw new Error('执行被中止');
                }
                
                await this.captureCurrentState();
                const element = await this.findElement(target);
                if (element) {
                    return; // 找到元素，结束等待
                }
                await this.sleep(1000); // 每秒检查一次
            }
            
            throw new Error(`等待元素超时: ${target}`);
        }
    }

    /**
     * 断言操作
     */
    async executeAssert(params) {
        const target = params[0];
        const condition = params[1];
        const area = params[2]; // 可选的区域限制
        
        if (!target || condition === undefined) {
            throw new Error('断言参数不完整');
        }

        await this.captureCurrentState();
        const element = await this.findElement(target);
        
        // 根据条件类型进行断言
        if (condition === true || condition === '存在') {
            if (!element) {
                throw new Error(`断言失败: 元素 "${target}" 不存在`);
            }
        } else if (condition === false || condition === '不存在') {
            if (element) {
                throw new Error(`断言失败: 元素 "${target}" 存在`);
            }
        } else if (typeof condition === 'string') {
            // 文本断言
            if (!element) {
                throw new Error(`断言失败: 元素 "${target}" 不存在`);
            }
            if (element.text !== condition && element.contentDesc !== condition) {
                throw new Error(`断言失败: 元素文本不匹配，期望 "${condition}"，实际 "${element.text || element.contentDesc}"`);
            }
        }
    }

    /**
     * 查找元素
     * @param {string|object} selector - 元素选择器（可以是别名、实际属性或图像引用对象）
     * @returns {Object|null} 找到的元素
     */
    async findElement(selector) {
        // 如果是图像引用对象，直接查找图像元素
        if (typeof selector === 'object' && selector.type === 'image_reference') {
            const elementDef = await this.getElementDefinition(selector.name);
            if (elementDef) {
                return await this.findElementByDefinition(elementDef);
            }
            return null;
        }
        
        // 如果是字符串，检查是否是元素别名
        if (typeof selector === 'string') {
            const elementDef = await this.getElementDefinition(selector);
            
            if (elementDef) {
                // 使用定义的选择器查找
                return await this.findElementByDefinition(elementDef);
            }
            
            // 否则直接在当前元素中查找
            if (!this.currentElements) {
                return null;
            }

            // 通过文本查找
            for (const element of this.currentElements) {
                if (element.text === selector || 
                    element.contentDesc === selector || 
                    element.hint === selector ||
                    (element.resourceId && element.resourceId.includes(selector))) {
                    return element;
                }
            }
        }

        return null;
    }

    /**
     * 获取元素定义
     * @param {string} alias - 元素别名
     * @returns {Object|null} 元素定义
     */
    async getElementDefinition(alias) {
        // 从项目级别的element.json中读取元素定义
        const elementFile = tksPath.join(
            this.projectPath,
            'locator',
            'element.json'
        );

        console.log('查找locator元素:', alias);
        console.log('当前case文件夹:', this.currentCaseFolder);
        console.log('Locator文件路径:', elementFile);

        try {
            const content = await tksFs.readFile(elementFile, 'utf-8');
            const elements = JSON.parse(content);
            console.log('已加载locator文件，包含元素:', Object.keys(elements));
            
            const elementDef = elements[alias] || null;
            console.log(`查找元素"${alias}":`, elementDef ? '找到' : '未找到');
            
            return elementDef;
        } catch (error) {
            console.error('读取元素定义失败:', error);
            return null;
        }
    }

    /**
     * 根据元素定义查找元素
     * @param {Object} definition - 元素定义
     * @returns {Object|null} 找到的元素
     */
    async findElementByDefinition(definition) {
        // 如果是图像类型，使用图像匹配
        if (definition.type === 'image') {
            return await this.findElementByImage(definition);
        }
        
        // 确保有当前UI元素数据，如果没有则刷新获取
        if (!this.currentElements) {
            console.log('当前无UI元素数据，正在刷新获取...');
            await this.getCurrentUIElements();
            if (!this.currentElements) {
                console.error('无法获取当前UI元素数据');
                return null;
            }
        }

        // XML元素匹配 - 使用多重策略，从最精确到最宽松
        console.log(`查找XML元素，共${this.currentElements.length}个候选元素`);
        
        // 策略1: 精确匹配 (最高优先级)
        const exactMatch = this.findByExactMatch(definition);
        if (exactMatch) {
            console.log('通过精确匹配找到元素');
            return exactMatch;
        }
        
        // 策略2: 基于resourceId的匹配 (高优先级)
        if (definition.resourceId) {
            const idMatch = this.findByResourceId(definition.resourceId);
            if (idMatch) {
                console.log('通过resourceId匹配找到元素');
                return idMatch;
            }
        }
        
        // 策略3: 基于内容描述的匹配 (中优先级)
        if (definition.contentDesc) {
            const descMatch = this.findByContentDesc(definition.contentDesc);
            if (descMatch) {
                console.log('通过contentDesc匹配找到元素');
                return descMatch;
            }
        }
        
        // 策略4: 基于文本内容的匹配 (中优先级)
        if (definition.text) {
            const textMatch = this.findByText(definition.text);
            if (textMatch) {
                console.log('通过text匹配找到元素');
                return textMatch;
            }
        }
        
        // 策略5: 基于类名和位置的模糊匹配 (低优先级)
        if (definition.className && definition.bounds) {
            const fuzzyMatch = this.findByClassAndPosition(definition.className, definition.bounds);
            if (fuzzyMatch) {
                console.log('通过类名和位置模糊匹配找到元素');
                return fuzzyMatch;
            }
        }
        
        // 策略6: 仅基于类名的匹配 (最低优先级)
        if (definition.className) {
            const classMatch = this.findByClassName(definition.className);
            if (classMatch) {
                console.log('通过className匹配找到元素');
                return classMatch;
            }
        }

        console.log('所有匹配策略都未找到元素');
        return null;
    }

    /**
     * 通过图像匹配查找元素
     * @param {Object} definition - 图像元素定义
     * @returns {Object|null} 找到的元素
     */
    async findElementByImage(definition) {
        try {
            console.log('开始图像匹配:', definition);
            
            // 构建模板图片路径
            const templatePath = tksPath.join(this.projectPath, definition.path);
            console.log('模板图片路径:', templatePath);
            
            // 构建当前截图路径
            const screenshotPath = tksPath.join(this.projectPath, 'workarea', 'current_screenshot.png');
            console.log('当前截图路径:', screenshotPath);
            
            // 检查文件是否存在
            try {
                await tksFs.access(templatePath);
                await tksFs.access(screenshotPath);
            } catch (error) {
                console.error('图像文件不存在:', error.message);
                return null;
            }
            
            // 创建图像匹配器实例
            const imageMatcher = new window.ImageMatcher();
            
            // 执行图像匹配
            const matchResult = await imageMatcher.templateMatch(screenshotPath, templatePath);
            
            console.log('图像匹配结果:', matchResult);
            
            if (matchResult.success) {
                // 简化逻辑：直接使用截图坐标，因为截图通常就是设备屏幕的实际尺寸
                console.log(`图像匹配成功，使用坐标: (${matchResult.center_x}, ${matchResult.center_y}), 置信度: ${matchResult.confidence.toFixed(3)}`);
                
                // 返回符合Element格式的对象
                return {
                    centerX: matchResult.center_x,
                    centerY: matchResult.center_y,
                    x: matchResult.bounding_box.x,
                    y: matchResult.bounding_box.y,
                    width: matchResult.bounding_box.width,
                    height: matchResult.bounding_box.height,
                    confidence: matchResult.confidence,
                    type: 'image',
                    source: 'image_matching',
                    templatePath: templatePath
                };
            } else {
                console.error('图像匹配失败:', matchResult.error);
                return null;
            }
            
        } catch (error) {
            console.error('图像匹配出错:', error);
            return null;
        }
    }

    /**
     * 执行ADB命令
     */
    async runAdbCommand(command) {
        const { ipcRenderer } = window.AppGlobals;
        
        // 如果已经要求停止，直接返回
        if (this.shouldStop) {
            return { success: false, error: '执行已被中止' };
        }
        
        // 保存当前命令
        this.currentCommand = command;
        
        // 使用新的IPC处理器执行ADB shell命令
        const result = await ipcRenderer.invoke('adb-shell-command', command, this.deviceId);
        
        // 清除当前命令
        this.currentCommand = null;
        
        if (!result.success) {
            console.error(`ADB命令执行失败: ${command}`, result.error);
        } else {
            // console.log(`ADB命令执行成功: ${command}`); // 已禁用以减少日志
        }
        
        return result;
    }

    /**
     * 保存执行结果
     */
    async saveResult(result) {
        // 保存到当前case的result文件夹
        const resultDir = tksPath.join(this.projectPath, 'cases', this.currentCaseFolder, 'result');
        
        // 确保result目录存在
        try {
            await tksFs.mkdir(resultDir, { recursive: true });
        } catch (error) {
            // 目录可能已存在，忽略错误
        }
        
        // 生成结果文件名：物理文件名 + 时间 + 结果
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5);
        const status = result.success ? 'PASS' : 'FAIL';
        // 直接使用物理文件名，如果没有设置则使用默认值
        const fileName = this.scriptFileName || 'unknown_script';
        const resultFileName = `${fileName}_${timestamp}_${status}.json`;
        const resultPath = tksPath.join(resultDir, resultFileName);

        try {
            await tksFs.writeFile(resultPath, JSON.stringify(result, null, 2));
            // console.log(`执行结果已保存到: ${resultPath}`); // 已禁用以减少日志
        } catch (error) {
            console.error('保存执行结果失败:', error);
        }
    }

    /**
     * 停止执行
     */
    stop() {
        console.log('TKSScriptExecutor: 收到停止请求');
        
        this.isRunning = false;
        this.shouldStop = true; // 设置停止标志
        
        // 如果有正在执行的ADB命令，尝试中断它
        if (this.currentCommand) {
            console.log(`TKSScriptExecutor: 尝试中断当前命令: ${this.currentCommand}`);
            // TODO: 可以通过IPC发送中断信号给主进程
        }
        
        // 停止测试时恢复编辑器交互状态
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            window.AppGlobals.codeEditor.setTestRunning(false);
        }
        
        // 清除高亮状态
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.clearExecutionHighlight) {
            window.AppGlobals.codeEditor.clearExecutionHighlight();
        }
    }

    /**
     * 辅助函数：睡眠
     */
    sleep(ms) {
        return new Promise((resolve, reject) => {
            // 如果已经要求停止，立即拒绝
            if (this.shouldStop) {
                reject(new Error('执行被中止'));
                return;
            }
            
            const checkInterval = 100; // 每100ms检查一次
            let elapsed = 0;
            
            const interval = setInterval(() => {
                elapsed += checkInterval;
                
                if (this.shouldStop) {
                    clearInterval(interval);
                    reject(new Error('执行被中止'));
                } else if (elapsed >= ms) {
                    clearInterval(interval);
                    resolve();
                }
            }, checkInterval);
        });
    }

    /**
     * 设置当前用例文件夹
     */
    setCurrentCase(caseFolder) {
        this.currentCaseFolder = caseFolder;
    }
    
    /**
     * 设置脚本物理文件名
     */
    setScriptFileName(fileName) {
        this.scriptFileName = fileName;
    }
    
    // ===== 新增的多重元素匹配策略方法 =====
    
    /**
     * 精确匹配 - 所有保存的属性必须完全匹配
     */
    findByExactMatch(definition) {
        return this.currentElements.find(element => {
            return (
                (!definition.text || element.text === definition.text) &&
                (!definition.resourceId || element.resourceId === definition.resourceId) &&
                (!definition.className || element.className === definition.className) &&
                (!definition.contentDesc || element.contentDesc === definition.contentDesc) &&
                (!definition.xpath || element.xpath === definition.xpath)
            );
        });
    }
    
    /**
     * 基于resourceId匹配 - 最稳定的定位方式
     */
    findByResourceId(resourceId) {
        return this.currentElements.find(element => {
            return element.resourceId && (
                element.resourceId === resourceId ||
                element.resourceId.includes(resourceId) ||
                resourceId.includes(element.resourceId)
            );
        });
    }
    
    /**
     * 基于内容描述匹配
     */
    findByContentDesc(contentDesc) {
        return this.currentElements.find(element => {
            return element.contentDesc && (
                element.contentDesc === contentDesc ||
                element.contentDesc.includes(contentDesc) ||
                contentDesc.includes(element.contentDesc)
            );
        });
    }
    
    /**
     * 基于文本内容匹配
     */
    findByText(text) {
        return this.currentElements.find(element => {
            return element.text && (
                element.text === text ||
                element.text.includes(text) ||
                text.includes(element.text)
            );
        });
    }
    
    /**
     * 基于类名匹配
     */
    findByClassName(className) {
        // 先尝试完全匹配
        let match = this.currentElements.find(element => element.className === className);
        if (match) return match;
        
        // 再尝试包含匹配
        return this.currentElements.find(element => {
            return element.className && (
                element.className.includes(className) ||
                className.includes(element.className)
            );
        });
    }
    
    /**
     * 基于类名和位置的模糊匹配 - 用于处理页面滑动后的元素定位
     */
    findByClassAndPosition(className, originalBounds, tolerance = 100) {
        // 计算原始元素的中心点
        const originalCenterX = (originalBounds[0] + originalBounds[2]) / 2;
        const originalCenterY = (originalBounds[1] + originalBounds[3]) / 2;
        
        // 查找同类型的元素
        const sameClassElements = this.currentElements.filter(element => {
            return element.className && (
                element.className === className ||
                element.className.includes(className) ||
                className.includes(element.className)
            );
        });
        
        if (sameClassElements.length === 0) return null;
        
        // 如果只有一个同类型元素，直接返回
        if (sameClassElements.length === 1) {
            return sameClassElements[0];
        }
        
        // 多个同类型元素时，选择位置最接近的
        let bestMatch = null;
        let minDistance = Infinity;
        
        for (const element of sameClassElements) {
            const centerX = (element.bounds[0] + element.bounds[2]) / 2;
            const centerY = (element.bounds[1] + element.bounds[3]) / 2;
            
            const distance = Math.sqrt(
                Math.pow(centerX - originalCenterX, 2) + 
                Math.pow(centerY - originalCenterY, 2)
            );
            
            // 在容差范围内选择最近的
            if (distance < minDistance && distance <= tolerance) {
                minDistance = distance;
                bestMatch = element;
            }
        }
        
        return bestMatch;
    }
}

/**
 * TKS脚本管理器
 * 提供脚本的高级管理功能
 */
class TKSScriptManager {
    constructor(projectPath) {
        this.projectPath = projectPath;
        this.parser = new TKSScriptParser();
        this.executor = null;
    }

    /**
     * 加载并验证脚本
     */
    async loadScript(scriptPath) {
        try {
            const script = await this.parser.parseFile(scriptPath);
            
            // 验证脚本
            this.validateScript(script);
            
            return script;
        } catch (error) {
            throw new Error(`加载脚本失败: ${error.message}`);
        }
    }

    /**
     * 验证脚本
     */
    validateScript(script) {
        if (!script.caseId) {
            throw new Error('脚本缺少用例ID');
        }
        
        if (!script.steps || script.steps.length === 0) {
            throw new Error('脚本没有定义任何步骤');
        }

        // 验证每个步骤
        for (let i = 0; i < script.steps.length; i++) {
            const step = script.steps[i];
            if (!step.type) {
                throw new Error(`步骤 ${i + 1} 无效`);
            }
        }
    }

    /**
     * 运行脚本
     */
    async runScript(scriptPath, deviceId = null) {
        const script = await this.loadScript(scriptPath);
        
        // 创建执行器
        this.executor = new TKSScriptExecutor(this.projectPath, deviceId);
        
        // 设置用例文件夹
        const caseFolder = tksPath.dirname(scriptPath).split(tksPath.sep).pop();
        this.executor.setCurrentCase(caseFolder);
        
        // 获取物理文件名（去掉扩展名）并设置
        const fileName = tksPath.basename(scriptPath, '.tks');
        this.executor.setScriptFileName(fileName);
        
        // 执行脚本
        const result = await this.executor.execute(script);
        
        return result;
    }

    /**
     * 停止当前执行
     */
    stopExecution() {
        if (this.executor) {
            this.executor.stop();
        }
    }

    /**
     * 创建新脚本模板
     */
    async createScriptTemplate(caseId, scriptName) {
        const template = `用例: ${caseId}
脚本名: ${scriptName}
详情: 
    appPackage: com.example.app
    appActivity: com.example.app.MainActivity
步骤:
    启动 [com.example.app, .MainActivity]
    等待 [3000]
    点击 [{登录按钮}]
    输入 [{用户名输入框}, testuser]
    输入 [{密码输入框}, password123]
    点击 [{确认按钮}]
    等待 [2000]
    断言 [{欢迎文本}, 存在]
    返回
    关闭 [com.example.app]
`;

        const scriptPath = tksPath.join(
            this.projectPath,
            'cases',
            caseId,
            `${scriptName}.tks`
        );

        // 确保目录存在
        const dir = tksPath.dirname(scriptPath);
        await tksFs.mkdir(dir, { recursive: true });

        // 确保项目级别的locator目录和element.json存在
        const locatorDir = tksPath.join(this.projectPath, 'locator');
        await tksFs.mkdir(locatorDir, { recursive: true });
        
        // 确保当前case的result目录存在
        const resultDir = tksPath.join(this.projectPath, 'cases', caseId, 'result');
        await tksFs.mkdir(resultDir, { recursive: true });
        
        const elementFile = tksPath.join(locatorDir, 'element.json');
        const elementTemplate = {
            "登录按钮": {
                "resourceId": "com.example.app:id/login_button",
                "text": "登录"
            },
            "用户名输入框": {
                "resourceId": "com.example.app:id/username_input",
                "className": "android.widget.EditText"
            },
            "密码输入框": {
                "resourceId": "com.example.app:id/password_input",
                "className": "android.widget.EditText"
            },
            "确认按钮": {
                "resourceId": "com.example.app:id/confirm_button",
                "text": "确认"
            },
            "主页面": {
                "resourceId": "com.example.app:id/main_container"
            },
            "欢迎文本": {
                "contentDesc": "欢迎",
                "className": "android.widget.TextView"
            }
        };

        await tksFs.writeFile(elementFile, JSON.stringify(elementTemplate, null, 4));
        await tksFs.writeFile(scriptPath, template);

        return scriptPath;
    }
}

    // 导出到全局
    window.TKSScriptParser = TKSScriptParser;
    window.TKSScriptExecutor = TKSScriptExecutor;
    window.TKSScriptManager = TKSScriptManager;

    // 导出模块
    window.TKSScriptModule = {
        Parser: TKSScriptParser,
        Executor: TKSScriptExecutor,
        Manager: TKSScriptManager,
        
        // 便捷方法
        createManager: (projectPath) => new TKSScriptManager(projectPath),
        parseScript: async (content) => {
            const parser = new TKSScriptParser();
            return parser.parse(content);
        },
        runScript: async (scriptPath, projectPath, deviceId) => {
            const manager = new TKSScriptManager(projectPath);
            return await manager.runScript(scriptPath, deviceId);
        }
    };

    console.log('TKS脚本引擎模块已加载');

})(); // 结束立即执行函数