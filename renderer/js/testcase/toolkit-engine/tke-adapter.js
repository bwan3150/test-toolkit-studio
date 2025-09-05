// TKE适配器 - 封装对Rust TKE可执行文件的调用
// 将原有的JavaScript模块调用转换为TKE CLI调用

(function() {
    'use strict';
    
    // 延迟require，避免模块加载时的问题
    let spawn, path;
    
    /**
     * TKE适配器类 - 提供与原有JS模块兼容的接口
     */
    class TKEAdapter {
        constructor() {
            this.tkeExecutable = null;
            this.isInitialized = false;
            this.logCallback = null;
            
            // 在构造函数中才加载依赖
            if (!spawn) {
                const cp = require('child_process');
                spawn = cp.spawn;
            }
            if (!path) {
                path = require('path');
            }
        }

        /**
         * 初始化TKE适配器
         */
        async initialize() {
            if (this.isInitialized) return;
            
            try {
                // 查找TKE可执行文件路径
                this.tkeExecutable = this.findTKEExecutable();
                
                if (window.rLog) {
                    window.rLog('TKE可执行文件路径:', this.tkeExecutable);
                }
                
                // 测试TKE是否可用
                await this.testTKEConnection();
                
                this.isInitialized = true;
                
                if (window.rLog) {
                    window.rLog('TKE适配器初始化完成');
                }
            } catch (error) {
                if (window.rError) {
                    window.rError('TKE适配器初始化失败:', error);
                }
                // 不要抛出错误，让模块加载继续
                this.isInitialized = false;
            }
        }

        /**
         * 查找TKE可执行文件 - 参考ADB的路径处理方式
         */
        findTKEExecutable() {
            // 获取平台信息，参考ADB的做法
            const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
            const tkeBinaryName = process.platform === 'win32' ? 'tke.exe' : 'tke';
            const fs = require('fs');
            
            let app;
            
            // 尝试获取Electron app实例
            try {
                const { remote } = require('electron');
                app = remote && remote.app ? remote.app : null;
            } catch (e) {
                try {
                    const remote = require('@electron/remote');
                    app = remote && remote.app ? remote.app : null;
                } catch (e2) {
                    // 无法获取app实例的情况，使用开发模式路径
                    app = null;
                }
            }
            
            // 构建可能的路径列表
            const possiblePaths = [];
            
            if (app) {
                // 按照ADB的路径模式：resources/[platform]/toolkit-engine/tke
                if (app.isPackaged) {
                    // 生产模式：process.resourcesPath/[platform]/toolkit-engine/tke
                    possiblePaths.push(path.join(process.resourcesPath, platform, 'toolkit-engine', tkeBinaryName));
                } else {
                    // 开发模式：resources/[platform]/toolkit-engine/tke
                    possiblePaths.push(path.join(__dirname, '..', '..', '..', '..', '..', 'resources', platform, 'toolkit-engine', tkeBinaryName));
                }
            }
            
            // 开发模式的备用路径
            possiblePaths.push(
                // 首先尝试已部署的资源路径
                path.join(__dirname, '..', '..', '..', '..', '..', 'resources', platform, 'toolkit-engine', tkeBinaryName),
                // 当前构建的路径
                path.join(__dirname, '..', '..', '..', '..', '..', 'toolkit-engine', 'target', 'release', tkeBinaryName),
                path.join(__dirname, '..', '..', '..', '..', '..', 'toolkit-engine', 'target', 'debug', tkeBinaryName),
                // 相对于工作目录的路径
                path.join(process.cwd(), 'toolkit-engine', 'target', 'release', tkeBinaryName),
                path.join(process.cwd(), 'toolkit-engine', 'target', 'debug', tkeBinaryName)
            );
            
            // 遍历所有路径，找到第一个存在的文件
            for (const possiblePath of possiblePaths) {
                try {
                    if (fs.existsSync(possiblePath)) {
                        if (window.rLog) {
                            window.rLog('找到TKE可执行文件:', possiblePath);
                        }
                        return possiblePath;
                    }
                } catch (error) {
                    // 忽略访问错误，继续尝试下一个路径
                }
            }
            
            // 如果没找到存在的文件，返回第一个路径作为默认值
            if (window.rLog) {
                window.rLog('TKE可执行文件候选路径:', possiblePaths[0]);
                window.rLog('警告：没有找到存在的TKE文件，使用默认路径');
            }
            return possiblePaths[0];
        }

        /**
         * 测试TKE连接
         */
        async testTKEConnection() {
            return new Promise((resolve, reject) => {
                const child = spawn(this.tkeExecutable, ['--version']);
                
                child.on('close', (code) => {
                    if (code === 0) {
                        resolve();
                    } else {
                        reject(new Error(`TKE测试失败，退出码: ${code}`));
                    }
                });

                child.on('error', (error) => {
                    reject(new Error(`启动TKE失败: ${error.message}`));
                });
            });
        }

        /**
         * 执行TKE命令的通用方法
         */
        async executeTKECommand(args, options = {}) {
            if (!this.isInitialized) {
                throw new Error('TKE适配器未初始化');
            }

            return new Promise((resolve, reject) => {
                const child = spawn(this.tkeExecutable, args, { 
                    stdio: ['pipe', 'pipe', 'pipe'],
                    ...options 
                });

                let stdout = '';
                let stderr = '';

                child.stdout.on('data', (data) => {
                    const text = data.toString();
                    stdout += text;
                    
                    // 实时日志输出
                    if (this.logCallback) {
                        this.logCallback(text.trim(), 'info');
                    }
                });

                child.stderr.on('data', (data) => {
                    const text = data.toString();
                    stderr += text;
                    
                    // 实时错误输出
                    if (this.logCallback) {
                        this.logCallback(text.trim(), 'error');
                    }
                });

                child.on('close', (code) => {
                    if (code === 0) {
                        resolve({
                            success: true,
                            stdout: stdout.trim(),
                            stderr: stderr.trim()
                        });
                    } else {
                        reject(new Error(`TKE命令失败 (退出码 ${code}): ${stderr || stdout}`));
                    }
                });

                child.on('error', (error) => {
                    reject(new Error(`执行TKE命令失败: ${error.message}`));
                });
            });
        }

        /**
         * 设置日志回调函数
         */
        setLogCallback(callback) {
            this.logCallback = callback;
        }

        /**
         * 执行带stdin输入的TKE命令
         */
        async executeTKECommandWithStdin(args, stdinInput, options = {}) {
            if (!this.isInitialized) {
                throw new Error('TKE适配器未初始化');
            }

            return new Promise((resolve, reject) => {
                const child = spawn(this.tkeExecutable, args, { 
                    stdio: ['pipe', 'pipe', 'pipe'],
                    ...options 
                });

                let stdout = '';
                let stderr = '';

                child.stdout.on('data', (data) => {
                    const text = data.toString();
                    stdout += text;
                    
                    // 实时日志输出
                    if (this.logCallback) {
                        this.logCallback(text.trim(), 'info');
                    }
                });

                child.stderr.on('data', (data) => {
                    const text = data.toString();
                    stderr += text;
                    
                    // 实时错误输出
                    if (this.logCallback) {
                        this.logCallback(text.trim(), 'error');
                    }
                });

                child.on('close', (code) => {
                    if (code === 0) {
                        resolve({
                            success: true,
                            stdout: stdout.trim(),
                            stderr: stderr.trim()
                        });
                    } else {
                        reject(new Error(`TKE命令失败 (退出码 ${code}): ${stderr || stdout}`));
                    }
                });

                child.on('error', (error) => {
                    reject(new Error(`执行TKE命令失败: ${error.message}`));
                });

                // 写入stdin内容并关闭stdin
                if (stdinInput) {
                    child.stdin.write(stdinInput, 'utf8');
                }
                child.stdin.end();
            });
        }
    }

    /**
     * Controller适配器 - ADB控制功能
     */
    class TKEControllerAdapter {
        constructor(tkeAdapter, projectPath, deviceId = null) {
            this.tkeAdapter = tkeAdapter;
            this.projectPath = projectPath;
            this.deviceId = deviceId;
        }

        /**
         * 获取连接的设备列表
         */
        async getDevices() {
            const result = await this.tkeAdapter.executeTKECommand(['controller', 'devices']);
            
            // 解析设备列表输出
            const devices = [];
            const lines = result.stdout.split('\n');
            
            for (const line of lines) {
                const trimmed = line.trim();
                if (trimmed.startsWith('- ')) {
                    devices.push(trimmed.substring(2));
                }
            }
            
            return devices;
        }

        /**
         * 捕获UI状态(截图和XML)
         */
        async captureUIState() {
            const args = ['--project', this.projectPath, 'controller', 'capture'];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 点击坐标
         */
        async tap(x, y) {
            const args = ['controller', 'tap', x.toString(), y.toString()];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 滑动操作
         */
        async swipe(x1, y1, x2, y2, duration = 300) {
            const args = ['controller', 'swipe', 
                         x1.toString(), y1.toString(), 
                         x2.toString(), y2.toString(), 
                         '--duration', duration.toString()];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 启动应用
         */
        async launchApp(packageName, activityName) {
            const args = ['controller', 'launch', packageName, activityName];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 停止应用
         */
        async stopApp(packageName) {
            const args = ['controller', 'stop', packageName];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 输入文本
         */
        async inputText(text) {
            const args = ['controller', 'input', text];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 返回键
         */
        async back() {
            const args = ['controller', 'back'];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }

        /**
         * 主页键
         */
        async home() {
            const args = ['controller', 'home'];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            await this.tkeAdapter.executeTKECommand(args);
        }
    }

    /**
     * LocatorFetcher适配器 - XML元素获取
     */
    class TKELocatorFetcherAdapter {
        constructor(tkeAdapter, projectPath) {
            this.tkeAdapter = tkeAdapter;
            this.projectPath = projectPath;
        }

        /**
         * 获取当前UI元素
         */
        async getCurrentElements() {
            const args = ['--project', this.projectPath, 'fetcher', 'current'];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseElementsOutput(result.stdout);
        }

        /**
         * 获取可交互元素
         */
        async getInteractiveElements() {
            const args = ['--project', this.projectPath, 'fetcher', 'interactive'];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseElementsOutput(result.stdout);
        }

        /**
         * 获取有文本的元素
         */
        async getTextElements() {
            const args = ['--project', this.projectPath, 'fetcher', 'text'];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseElementsOutput(result.stdout);
        }

        /**
         * 从XML文件提取元素
         */
        async extractElementsFromFile(xmlPath) {
            const args = ['fetcher', 'extract', xmlPath];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseElementsOutput(result.stdout);
        }

        /**
         * 解析元素输出
         */
        parseElementsOutput(output) {
            const elements = [];
            const lines = output.split('\n');
            
            for (const line of lines) {
                const trimmed = line.trim();
                // 匹配格式: [index] element_description
                const match = trimmed.match(/^\[(\d+)\]\s+(.+)$/);
                if (match) {
                    elements.push({
                        index: parseInt(match[1]),
                        description: match[2],
                        raw: trimmed
                    });
                }
            }
            
            return elements;
        }

        /**
         * 从XML推断屏幕尺寸
         */
        async inferScreenSizeFromXml(xmlContent) {
            const result = await this.tkeAdapter.executeTKECommandWithStdin(
                ['fetcher', 'infer-screen-size'], 
                xmlContent
            );
            
            try {
                const parsed = JSON.parse(result.stdout.trim());
                return { success: true, data: parsed };
            } catch (error) {
                if (result.stdout.trim() === 'null') {
                    return { success: true, data: null };
                }
                return { success: false, error: `解析输出失败: ${error.message}` };
            }
        }

        /**
         * 优化UI树结构
         */
        async optimizeUITree(xmlContent) {
            const result = await this.tkeAdapter.executeTKECommandWithStdin(
                ['fetcher', 'optimize-ui-tree'], 
                xmlContent
            );
            
            return { success: true, data: result.stdout };
        }

        /**
         * 从XML内容提取UI元素
         */
        async extractUIElements(xmlContent, screenWidth = null, screenHeight = null) {
            const args = ['fetcher', 'extract-ui-elements'];
            if (screenWidth && screenHeight) {
                args.push('--width', screenWidth.toString(), '--height', screenHeight.toString());
            }
            
            const result = await this.tkeAdapter.executeTKECommandWithStdin(args, xmlContent);
            
            try {
                const rawElements = JSON.parse(result.stdout.trim());
                
                // 转换TKE返回的数据格式到前端期望的格式
                const elements = rawElements.map((tkeElement, index) => {
                    return {
                        index: index,
                        className: tkeElement.class_name || '',
                        bounds: [
                            tkeElement.bounds.x1,
                            tkeElement.bounds.y1,
                            tkeElement.bounds.x2,
                            tkeElement.bounds.y2
                        ],
                        text: tkeElement.text || '',
                        contentDesc: tkeElement.content_desc || '',
                        resourceId: tkeElement.resource_id || '',
                        hint: tkeElement.hint || '',
                        clickable: tkeElement.clickable || false,
                        checkable: tkeElement.checkable || false,
                        checked: tkeElement.checked || false,
                        focusable: tkeElement.focusable || false,
                        focused: tkeElement.focused || false,
                        scrollable: tkeElement.scrollable || false,
                        selected: tkeElement.selected || false,
                        enabled: tkeElement.enabled !== false,
                        xpath: tkeElement.xpath || ''
                    };
                });
                
                return { success: true, data: { elements } };
            } catch (error) {
                return { success: false, error: `解析UI元素失败: ${error.message}` };
            }
        }

        /**
         * 生成UI树的字符串表示
         */
        async generateTreeString(xmlContent) {
            const result = await this.tkeAdapter.executeTKECommandWithStdin(
                ['fetcher', 'generate-tree-string'], 
                xmlContent
            );
            
            return { success: true, data: { treeString: result.stdout } };
        }
    }

    /**
     * Recognizer适配器 - 元素识别
     */
    class TKERecognizerAdapter {
        constructor(tkeAdapter, projectPath) {
            this.tkeAdapter = tkeAdapter;
            this.projectPath = projectPath;
        }

        /**
         * 根据XML locator查找元素
         */
        async findXmlElement(locatorName) {
            const args = ['--project', this.projectPath, 'recognizer', 'find-xml', locatorName];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseCoordinateOutput(result.stdout);
        }

        /**
         * 根据图像locator查找元素
         */
        async findImageElement(locatorName) {
            const args = ['--project', this.projectPath, 'recognizer', 'find-image', locatorName];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseCoordinateOutput(result.stdout);
        }

        /**
         * 根据文本查找元素
         */
        async findElementByText(text) {
            const args = ['--project', this.projectPath, 'recognizer', 'find-text', text];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseCoordinateOutput(result.stdout);
        }

        /**
         * 解析坐标输出
         */
        parseCoordinateOutput(output) {
            // 匹配格式: 找到xxx的位置: (x, y)
            const match = output.match(/位置:\s*\((\d+),\s*(\d+)\)/);
            if (match) {
                return {
                    x: parseInt(match[1]),
                    y: parseInt(match[2])
                };
            }
            
            throw new Error('无法解析坐标信息');
        }
    }

    /**
     * ScriptParser适配器 - 脚本解析
     */
    class TKEScriptParserAdapter {
        constructor(tkeAdapter) {
            this.tkeAdapter = tkeAdapter;
        }

        /**
         * 解析脚本文件
         */
        async parseScriptFile(scriptPath) {
            const args = ['parser', 'parse', scriptPath];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return this.parseScriptOutput(result.stdout);
        }

        /**
         * 验证脚本
         */
        async validateScript(scriptPath) {
            const args = ['parser', 'validate', scriptPath];
            const result = await this.tkeAdapter.executeTKECommand(args);
            
            return {
                valid: true,
                output: result.stdout
            };
        }

        /**
         * 解析脚本输出信息
         */
        parseScriptOutput(output) {
            const lines = output.split('\n');
            const result = {
                caseId: '',
                scriptName: '',
                detailsCount: 0,
                stepsCount: 0,
                steps: []
            };

            let inStepsList = false;
            
            for (const line of lines) {
                const trimmed = line.trim();
                
                if (trimmed.startsWith('用例ID:')) {
                    result.caseId = trimmed.substring(trimmed.indexOf(':') + 1).trim();
                } else if (trimmed.startsWith('脚本名:')) {
                    result.scriptName = trimmed.substring(trimmed.indexOf(':') + 1).trim();
                } else if (trimmed.startsWith('详情数:')) {
                    result.detailsCount = parseInt(trimmed.split(':')[1].trim());
                } else if (trimmed.startsWith('步骤数:')) {
                    result.stepsCount = parseInt(trimmed.split(':')[1].trim());
                } else if (trimmed === '步骤列表:') {
                    inStepsList = true;
                } else if (inStepsList && trimmed.match(/^\d+\./)) {
                    // 解析步骤: "1. 启动 [com.example.app, .MainActivity] (行号: 7)"
                    const stepMatch = trimmed.match(/^(\d+)\.\s+(.+?)\s+\(行号:\s*(\d+)\)$/);
                    if (stepMatch) {
                        result.steps.push({
                            index: parseInt(stepMatch[1]) - 1,
                            command: stepMatch[2],
                            lineNumber: parseInt(stepMatch[3])
                        });
                    }
                }
            }
            
            return result;
        }
    }

    /**
     * ScriptRunner适配器 - 脚本执行
     */
    class TKEScriptRunnerAdapter {
        constructor(tkeAdapter, projectPath, deviceId = null) {
            this.tkeAdapter = tkeAdapter;
            this.projectPath = projectPath;
            this.deviceId = deviceId;
            this.isRunning = false;
            this.currentProcess = null;
        }

        /**
         * 运行脚本文件 (实时版本)
         */
        async runScriptFile(scriptPath, callbacks = {}) {
            if (this.isRunning) {
                throw new Error('脚本正在运行中');
            }

            const args = ['-v', '--project', this.projectPath, 'run', 'script', scriptPath];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            this.isRunning = true;
            
            try {
                return await this.executeScriptWithCallbacks(args, callbacks);
            } finally {
                this.isRunning = false;
                this.currentProcess = null;
            }
        }

        /**
         * 运行脚本文件 (简单版本，兼容现有代码)
         */
        async runScriptFileSimple(scriptPath) {
            const args = ['--project', this.projectPath, 'run', 'script', scriptPath];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }
            
            const result = await this.tkeAdapter.executeTKECommand(args);
            return this.parseExecutionResult(result.stdout);
        }

        /**
         * 运行脚本内容 (实时版本)
         */
        async runScriptContent(content, callbacks = {}) {
            if (this.isRunning) {
                throw new Error('脚本正在运行中');
            }

            const args = ['-v', '--project', this.projectPath, 'run', 'content', content];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            this.isRunning = true;
            
            try {
                return await this.executeScriptWithCallbacks(args, callbacks);
            } finally {
                this.isRunning = false;
                this.currentProcess = null;
            }
        }

        /**
         * 实时执行脚本
         */
        async executeScriptWithCallbacks(args, callbacks) {
            return new Promise((resolve, reject) => {
                // 确保spawn已加载
                if (!spawn) {
                    const cp = require('child_process');
                    spawn = cp.spawn;
                }
                
                console.log('TKE执行命令:', this.tkeAdapter.tkeExecutable, args);
                const child = spawn(this.tkeAdapter.tkeExecutable, args);
                this.currentProcess = child;
                
                let stdout = '';
                let stderr = '';
                let currentStep = 0;
                
                // 处理标准输出
                child.stdout.on('data', (data) => {
                    const output = data.toString();
                    stdout += output;
                    
                    // 解析实时输出
                    const lines = output.split('\n');
                    for (const line of lines) {
                        const trimmed = line.trim();
                        if (!trimmed) continue;
                        
                        // 日志输出回调
                        if (callbacks.onLog) {
                            callbacks.onLog(trimmed);
                        } else {
                            console.log('TKE输出:', trimmed);
                        }
                        
                        // 步骤执行检测
                        const stepMatch = trimmed.match(/执行步骤\s+(\d+)\/(\d+):\s*(.+)/);
                        if (stepMatch) {
                            const stepNum = parseInt(stepMatch[1]);
                            const totalSteps = parseInt(stepMatch[2]);
                            const stepDesc = stepMatch[3];
                            
                            if (callbacks.onStepStart) {
                                callbacks.onStepStart(stepNum - 1, stepDesc, totalSteps);
                            }
                        }
                        
                        // 步骤完成检测 (通过下一步开始或成功日志推断)
                        if (stepMatch && currentStep < parseInt(stepMatch[1]) - 1) {
                            if (callbacks.onStepComplete) {
                                callbacks.onStepComplete(currentStep, true);
                            }
                            currentStep = parseInt(stepMatch[1]) - 1;
                        }
                        
                        // UI状态已捕获 - 刷新截图
                        if (trimmed.includes('UI状态已捕获并保存到workarea')) {
                            if (callbacks.onScreenshotUpdated) {
                                callbacks.onScreenshotUpdated();
                            }
                        }
                        
                        // 错误检测
                        if (trimmed.includes('ERROR') || trimmed.includes('失败')) {
                            if (callbacks.onStepComplete && currentStep >= 0) {
                                callbacks.onStepComplete(currentStep, false, trimmed);
                            }
                        }
                    }
                });
                
                // 处理标准错误
                child.stderr.on('data', (data) => {
                    const output = data.toString();
                    stderr += output;
                    
                    if (callbacks.onLog) {
                        callbacks.onLog(output, 'error');
                    }
                });
                
                // 处理进程退出
                child.on('close', (code) => {
                    this.currentProcess = null;
                    
                    if (code === 0) {
                        const result = this.parseExecutionResult(stdout);
                        if (callbacks.onComplete) {
                            callbacks.onComplete(result);
                        }
                        resolve(result);
                    } else {
                        const error = new Error(`TKE执行失败，退出码: ${code}\n${stderr}`);
                        if (callbacks.onComplete) {
                            callbacks.onComplete(null, error);
                        }
                        reject(error);
                    }
                });
                
                // 处理启动错误
                child.on('error', (error) => {
                    this.currentProcess = null;
                    if (callbacks.onComplete) {
                        callbacks.onComplete(null, error);
                    }
                    reject(new Error(`启动TKE失败: ${error.message}`));
                });
            });
        }

        /**
         * 停止执行
         */
        async stopExecution() {
            if (this.currentProcess) {
                this.currentProcess.kill('SIGTERM');
                this.currentProcess = null;
            }
            this.isRunning = false;
        }

        /**
         * 解析执行结果
         */
        parseExecutionResult(output) {
            const lines = output.split('\n');
            const result = {
                success: false,
                caseId: '',
                scriptName: '',
                startTime: '',
                endTime: '',
                totalSteps: 0,
                successfulSteps: 0,
                error: null
            };

            for (const line of lines) {
                const trimmed = line.trim();
                
                if (trimmed.includes('成功 ✓')) {
                    result.success = true;
                } else if (trimmed.includes('失败 ✗')) {
                    result.success = false;
                } else if (trimmed.startsWith('用例ID:')) {
                    result.caseId = trimmed.split(':')[1].trim();
                } else if (trimmed.startsWith('脚本名:')) {
                    result.scriptName = trimmed.split(':')[1].trim();
                } else if (trimmed.startsWith('开始时间:')) {
                    result.startTime = trimmed.split(':', 2)[1].trim();
                } else if (trimmed.startsWith('结束时间:')) {
                    result.endTime = trimmed.split(':', 2)[1].trim();
                } else if (trimmed.startsWith('总步骤数:')) {
                    result.totalSteps = parseInt(trimmed.split(':')[1].trim());
                } else if (trimmed.startsWith('成功步骤:')) {
                    result.successfulSteps = parseInt(trimmed.split(':')[1].trim());
                } else if (trimmed.startsWith('错误信息:')) {
                    result.error = trimmed.split(':')[1].trim();
                }
            }

            return result;
        }
    }

    // 创建全局TKE适配器实例
    let globalTKEAdapter = null;

    /**
     * 获取或创建TKE适配器实例
     */
    async function getTKEAdapter() {
        if (!globalTKEAdapter) {
            globalTKEAdapter = new TKEAdapter();
            await globalTKEAdapter.initialize();
        }
        return globalTKEAdapter;
    }

    // 导出模块
    window.TKEAdapterModule = {
        TKEAdapter,
        TKEControllerAdapter,
        TKELocatorFetcherAdapter,
        TKERecognizerAdapter,
        TKEScriptParserAdapter,
        TKEScriptRunnerAdapter,
        getTKEAdapter
    };

    // 通过renderer-logger发送日志
    if (window.rLog) {
        window.rLog('TKE适配器模块已成功加载');
    }
    
})();