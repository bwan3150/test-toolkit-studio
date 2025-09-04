// TKE适配器 - 封装对Rust TKE可执行文件的调用
// 将原有的JavaScript模块调用转换为TKE CLI调用

const { spawn } = require('child_process');
const path = require('path');

/**
 * TKE适配器类 - 提供与原有JS模块兼容的接口
 */
class TKEAdapter {
    constructor() {
        this.tkeExecutable = null;
        this.isInitialized = false;
        this.logCallback = null;
    }

    /**
     * 初始化TKE适配器
     */
    async initialize() {
        if (this.isInitialized) return;
        
        try {
            // 查找TKE可执行文件路径
            this.tkeExecutable = this.findTKEExecutable();
            console.log('TKE可执行文件路径:', this.tkeExecutable);
            
            // 测试TKE是否可用
            await this.testTKEConnection();
            
            this.isInitialized = true;
            console.log('TKE适配器初始化完成');
        } catch (error) {
            console.error('TKE适配器初始化失败:', error);
            throw error;
        }
    }

    /**
     * 查找TKE可执行文件 - 参考ADB的路径处理方式
     */
    findTKEExecutable() {
        const fs = require('fs');
        
        try {
            // 获取平台信息，参考ADB的做法
            const platform = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'win32' : 'linux';
            const tkeBinaryName = process.platform === 'win32' ? 'tke.exe' : 'tke';
            
            let tkePath;
            let app;
            
            // 尝试获取Electron app实例
            try {
                const { remote } = require('electron');
                app = remote.app;
            } catch (e) {
                try {
                    const remote = require('@electron/remote');
                    app = remote.app;
                } catch (e2) {
                    // 无法获取app实例的情况，使用开发模式路径
                }
            }
            
            if (app) {
                // 按照ADB的路径模式：resources/[platform]/toolkit-engine/tke
                if (app.isPackaged) {
                    // 生产模式：process.resourcesPath/[platform]/toolkit-engine/tke
                    tkePath = path.join(process.resourcesPath, platform, 'toolkit-engine', tkeBinaryName);
                } else {
                    // 开发模式：resources/[platform]/toolkit-engine/tke
                    tkePath = path.join(__dirname, '..', '..', '..', '..', '..', 'resources', platform, 'toolkit-engine', tkeBinaryName);
                }
                
                if (fs.existsSync(tkePath)) {
                    window.rLog(`找到TKE可执行文件: ${tkePath}`);
                    return tkePath;
                }
            }
            
            // 开发模式的备用路径
            const fallbackPaths = [
                // 当前构建的路径
                path.join(__dirname, '..', '..', '..', '..', '..', 'toolkit-engine', 'target', 'release', tkeBinaryName),
                path.join(__dirname, '..', '..', '..', '..', '..', 'toolkit-engine', 'target', 'debug', tkeBinaryName),
                // 相对于工作目录的路径
                path.join(process.cwd(), 'toolkit-engine', 'target', 'release', tkeBinaryName),
                path.join(process.cwd(), 'toolkit-engine', 'target', 'debug', tkeBinaryName)
            ];

            for (const fallbackPath of fallbackPaths) {
                if (fs.existsSync(fallbackPath)) {
                    window.rLog(`找到TKE可执行文件(备用路径): ${fallbackPath}`);
                    return fallbackPath;
                }
            }

            throw new Error(`找不到TKE可执行文件。需要的路径: ${tkePath || 'unknown'}`);
            
        } catch (error) {
            window.rError('查找TKE可执行文件时出错:', error);
            throw new Error('找不到TKE可执行文件。请确保TKE二进制文件已正确打包到应用中');
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
     * 运行脚本文件
     */
    async runScriptFile(scriptPath) {
        if (this.isRunning) {
            throw new Error('脚本正在运行中');
        }

        const args = ['--project', this.projectPath, 'run', 'script', scriptPath];
        if (this.deviceId) {
            args.unshift('--device', this.deviceId);
        }

        this.isRunning = true;
        
        try {
            const result = await this.tkeAdapter.executeTKECommand(args);
            return this.parseExecutionResult(result.stdout);
        } finally {
            this.isRunning = false;
            this.currentProcess = null;
        }
    }

    /**
     * 运行脚本内容
     */
    async runScriptContent(content) {
        if (this.isRunning) {
            throw new Error('脚本正在运行中');
        }

        const args = ['--project', this.projectPath, 'run', 'content', content];
        if (this.deviceId) {
            args.unshift('--device', this.deviceId);
        }

        this.isRunning = true;
        
        try {
            const result = await this.tkeAdapter.executeTKECommand(args);
            return this.parseExecutionResult(result.stdout);
        } finally {
            this.isRunning = false;
            this.currentProcess = null;
        }
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