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
            if (this.isInitialized) {
                if (window.rLog) {
                    window.rLog('TKE适配器已初始化，跳过重复初始化');
                }
                return;
            }
            
            try {
                if (window.rLog) {
                    window.rLog('🚀 开始初始化TKE适配器...');
                }
                
                // 查找TKE可执行文件路径
                this.tkeExecutable = this.findTKEExecutable();
                
                if (window.rLog) {
                    window.rLog('📍 TKE可执行文件路径:', this.tkeExecutable);
                }
                
                // 测试TKE是否可用
                if (window.rLog) {
                    window.rLog('🧪 测试TKE连接...');
                }
                await this.testTKEConnection();
                
                this.isInitialized = true;
                
                if (window.rLog) {
                    window.rLog('✅ TKE适配器初始化成功');
                }
            } catch (error) {
                if (window.rError) {
                    window.rError('❌ TKE适配器初始化失败:', error);
                    window.rError('错误详情:', error.message);
                    window.rError('错误堆栈:', error.stack);
                }
                // 不要抛出错误，让模块加载继续
                this.isInitialized = false;
            }
        }

        /**
         * 查找TKE可执行文件 - 使用环境变量判断开发模式
         */
        findTKEExecutable() {
            // 获取平台信息
            const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
            const tkeBinaryName = process.platform === 'win32' ? 'tke.exe' : 'tke';
            const fs = require('fs');

            // 通过环境变量判断是否是开发模式
            const isDevMode = process.env.ELECTRON_DEV_MODE === 'true';
            const projectRoot = process.env.ELECTRON_PROJECT_ROOT;

            let tkePath;

            if (isDevMode && projectRoot) {
                // 开发模式: 直接使用dev.sh传入的项目根目录
                tkePath = path.join(projectRoot, 'resources', platform, 'toolkit-engine', tkeBinaryName);
                if (window.rLog) {
                    window.rLog('🔧 开发模式 TKE路径:', tkePath);
                }
            } else {
                // 打包模式: 包内的 Contents/Resources/平台/toolkit-engine/tke
                tkePath = path.join(process.resourcesPath, platform, 'toolkit-engine', tkeBinaryName);
                if (window.rLog) {
                    window.rLog('📦 打包模式 TKE路径:', tkePath);
                }
            }

            // 检查文件是否存在
            if (fs.existsSync(tkePath)) {
                if (window.rLog) {
                    window.rLog('✅ TKE可执行文件存在:', tkePath);
                }
                return tkePath;
            } else {
                if (window.rError) {
                    window.rError('❌ TKE可执行文件不存在:', tkePath);
                }
                return tkePath; // 即使不存在也返回,让后续错误处理
            }
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
         * 返回JSON格式: {"devices":["device1", "device2"]}
         */
        async getDevices() {
            const result = await this.tkeAdapter.executeTKECommand(['controller', 'devices']);

            // 解析JSON输出
            try {
                const jsonResult = JSON.parse(result.stdout.trim());
                return jsonResult.devices || [];
            } catch (error) {
                if (window.rError) {
                    window.rError('解析devices JSON失败:', error);
                }
                return [];
            }
        }

        /**
         * 捕获UI状态(截图和XML)
         * 返回JSON格式: {"screenshot":"path","success":true,"xml":"path"}
         */
        async captureUIState() {
            const args = ['--project', this.projectPath, 'controller', 'capture'];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                const jsonResult = JSON.parse(result.stdout.trim());
                return jsonResult;
            } catch (error) {
                if (window.rError) {
                    window.rError('解析capture JSON失败:', error);
                }
                throw error;
            }
        }

        /**
         * 点击坐标
         * 返回JSON格式: {"success":true,"x":400,"y":2000}
         */
        async tap(x, y) {
            const args = ['controller', 'tap', x.toString(), y.toString()];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析tap JSON失败:', error);
                }
                return { success: false };
            }
        }

        /**
         * 滑动操作
         * 返回JSON格式: {"duration":300,"from":{"x":500,"y":1500},"success":true,"to":{"x":500,"y":500}}
         */
        async swipe(x1, y1, x2, y2, duration = 300) {
            const args = ['controller', 'swipe',
                         x1.toString(), y1.toString(),
                         x2.toString(), y2.toString(),
                         '--duration', duration.toString()];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析swipe JSON失败:', error);
                }
                return { success: false };
            }
        }

        /**
         * 启动应用
         * 返回JSON格式: {"activity":".Settings","package":"com.android.settings","success":true}
         */
        async launchApp(packageName, activityName) {
            const args = ['controller', 'launch', packageName, activityName];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析launch JSON失败:', error);
                }
                return { success: false };
            }
        }

        /**
         * 停止应用
         * 返回JSON格式: {"package":"com.android.settings","success":true}
         */
        async stopApp(packageName) {
            const args = ['controller', 'stop', packageName];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析stop JSON失败:', error);
                }
                return { success: false };
            }
        }

        /**
         * 输入文本
         * 返回JSON格式: {"success":true,"text":"Hello World"}
         */
        async inputText(text) {
            const args = ['controller', 'input', text];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析input JSON失败:', error);
                }
                return { success: false };
            }
        }

        /**
         * 返回键
         * 返回JSON格式: {"success":true}
         */
        async back() {
            const args = ['controller', 'back'];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析back JSON失败:', error);
                }
                return { success: false };
            }
        }

        /**
         * 主页键
         * 返回JSON格式: {"success":true}
         */
        async home() {
            const args = ['controller', 'home'];
            if (this.deviceId) {
                args.unshift('--device', this.deviceId);
            }

            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                return JSON.parse(result.stdout.trim());
            } catch (error) {
                if (window.rError) {
                    window.rError('解析home JSON失败:', error);
                }
                return { success: false };
            }
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
         * 返回JSON格式: {"success":true,"x":728,"y":360}
         */
        async findXmlElement(locatorName) {
            const args = ['--project', this.projectPath, 'recognizer', 'find-xml', locatorName];
            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                const jsonResult = JSON.parse(result.stdout.trim());
                if (jsonResult.success) {
                    return { x: jsonResult.x, y: jsonResult.y, success: true };
                } else {
                    throw new Error(jsonResult.error || '元素未找到');
                }
            } catch (error) {
                if (window.rError) {
                    window.rError('解析find-xml JSON失败:', error);
                }
                throw error;
            }
        }

        /**
         * 根据图像locator查找元素
         * 返回JSON格式: {"success":true,"x":725,"y":910,"width":490,"height":105,"matches_count":1}
         */
        async findImageElement(locatorName, threshold = 0.5) {
            const args = ['--project', this.projectPath, 'recognizer', 'find-image', locatorName];
            if (threshold !== 0.5) {
                args.push('--threshold', threshold.toString());
            }
            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                const jsonResult = JSON.parse(result.stdout.trim());
                if (jsonResult.success) {
                    return jsonResult;
                } else {
                    throw new Error(jsonResult.error || '图像未找到');
                }
            } catch (error) {
                if (window.rError) {
                    window.rError('解析find-image JSON失败:', error);
                }
                throw error;
            }
        }

        /**
         * 根据文本查找元素
         * 返回JSON格式: {"success":true,"x":394,"y":186}
         */
        async findElementByText(text) {
            const args = ['--project', this.projectPath, 'recognizer', 'find-text', text];
            const result = await this.tkeAdapter.executeTKECommand(args);

            // 解析JSON输出
            try {
                const jsonResult = JSON.parse(result.stdout.trim());
                if (jsonResult.success) {
                    return { x: jsonResult.x, y: jsonResult.y, success: true };
                } else {
                    throw new Error(jsonResult.error || '文本未找到');
                }
            } catch (error) {
                if (window.rError) {
                    window.rError('解析find-text JSON失败:', error);
                }
                throw error;
            }
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
         * 解析脚本输出信息 - 纯JSON格式
         */
        parseScriptOutput(output) {
            try {
                // 直接解析纯JSON输出
                const jsonResult = JSON.parse(output.trim());
                
                // 转换为兼容的格式
                const result = {
                    success: jsonResult.success,
                    caseId: jsonResult.case_id,
                    scriptName: jsonResult.script_name,
                    detailsCount: Object.keys(jsonResult.details || {}).length,
                    stepsCount: jsonResult.steps ? jsonResult.steps.length : 0,
                    steps: jsonResult.steps ? jsonResult.steps.map((step, index) => ({
                        index: index,
                        command: step.command,
                        lineNumber: step.line_number,
                        commandType: step.command_type,
                        params: step.params
                    })) : []
                };
                
                window.rLog('🎯 JSON解析成功:', result);
                return result;
                
            } catch (error) {
                window.rWarn('JSON解析失败，尝试使用旧格式解析:', error);
                // 回退到旧格式解析（兼容性）
                return this.parseScriptOutputLegacy(output);
            }
        }

        /**
         * 解析旧格式的脚本输出信息（兼容性保留）
         */
        parseScriptOutputLegacy(output) {
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
                
                window.rLog('TKE执行命令:', this.tkeAdapter.tkeExecutable, args);
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
                        // 移除ANSI颜色代码
                        const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '');
                        const trimmed = cleanLine.trim();
                        if (!trimmed) continue;
                        
                        // 日志输出回调
                        if (callbacks.onLog) {
                            callbacks.onLog(trimmed);
                        } else {
                            window.rLog('TKE输出:', trimmed);
                        }
                        
                        // 步骤执行检测 - 匹配TKE实际输出格式
                        const stepMatch = trimmed.match(/执行步骤\s+(\d+)\/(\d+):\s*(.+)/);
                        if (stepMatch) {
                            const stepNum = parseInt(stepMatch[1]);
                            const totalSteps = parseInt(stepMatch[2]);
                            const stepDesc = stepMatch[3];
                            
                            window.rLog(`TKE步骤检测: 步骤${stepNum}/${totalSteps} - ${stepDesc}`);
                            
                            // 先标记上一个步骤完成（如果有的话）
                            if (currentStep >= 0 && currentStep < stepNum - 1) {
                                if (callbacks.onStepComplete) {
                                    callbacks.onStepComplete(currentStep, true);
                                }
                            }
                            currentStep = stepNum - 1;
                            
                            if (callbacks.onStepStart) {
                                callbacks.onStepStart(stepNum - 1, stepDesc, totalSteps);
                            }
                        }
                        
                        // 已经在上面的stepMatch中处理步骤完成检测
                        
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
                    
                    // 标记最后一个步骤完成
                    if (currentStep >= 0) {
                        const success = code === 0;
                        if (callbacks.onStepComplete) {
                            callbacks.onStepComplete(currentStep, success, success ? null : stderr);
                        }
                    }
                    
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