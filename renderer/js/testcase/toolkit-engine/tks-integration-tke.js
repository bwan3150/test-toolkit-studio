// TKS脚本集成模块 (TKE版本)
// 使用Rust TKE执行TKS脚本功能

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// TKS脚本运行器 (TKE版本)
class TKSScriptRunnerTKE {
    constructor() {
        this.isRunning = false;
        this.currentScript = null;
        this.initialized = false;
        this.tkeAdapter = null;
        this.scriptRunner = null;
    }

    /**
     * 初始化脚本运行器
     */
    async init() {
        if (this.initialized) {
            window.rLog('TKSScriptRunnerTKE已经初始化过，跳过重复初始化');
            return;
        }
        
        try {
            // 初始化TKE适配器
            this.tkeAdapter = await window.TKEAdapterModule.getTKEAdapter();
            
            // 设置日志回调
            this.tkeAdapter.setLogCallback((message, level) => {
                if (window.TestcaseController && window.TestcaseController.ConsoleManager) {
                    const logLevel = level === 'error' ? 'error' : 'info';
                    window.TestcaseController.ConsoleManager.addLog(message, logLevel);
                }
            });
            
            // 绑定Run Test按钮
            const runTestBtn = document.getElementById('runTestBtn');
            if (runTestBtn) {
                window.rLog('TKSScriptRunnerTKE: 绑定Run Test按钮事件');
                runTestBtn.addEventListener('click', () => {
                    window.rLog('Run Test按钮被点击');
                    this.handleRunTest();
                });
            } else {
                window.rError('TKSScriptRunnerTKE: 找不到runTestBtn元素');
            }
            
            this.initialized = true;
            window.rLog('TKS脚本运行器(TKE版本)已初始化');
        } catch (error) {
            window.rError('TKS脚本运行器(TKE版本)初始化失败:', error);
            throw error;
        }
    }

    /**
     * 处理运行测试
     */
    async handleRunTest() {
        window.rLog('handleRunTest: 开始处理测试执行 (TKE版本)');
        
        // 立即清除编辑器焦点和光标，防止干扰高亮显示
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.contentEl) {
            window.rLog('handleRunTest: 清除编辑器焦点');
            window.AppGlobals.codeEditor.contentEl.blur();
            // 清除选择区域
            if (window.getSelection) {
                window.getSelection().removeAllRanges();
            }
            // 移除焦点到一个不可见的元素
            if (document.body) {
                document.body.focus();
            }
        }
        
        // 获取当前活动的标签页
        const currentTab = window.AppGlobals.currentTab;
        window.rLog('handleRunTest: 详细调试信息', {
            'window.AppGlobals存在': !!window.AppGlobals,
            'openTabs数量': window.AppGlobals.openTabs ? window.AppGlobals.openTabs.length : '无openTabs',
            'currentTab': currentTab,
            'currentTab类型': typeof currentTab
        });
        
        if (!currentTab) {
            window.rLog('handleRunTest: 没有活动标签页');
            window.NotificationModule.showNotification('请先打开一个测试脚本', 'warning');
            return;
        }

        // 检查文件扩展名
        const fileName = currentTab.name;
        if (!fileName.endsWith('.tks') && !fileName.endsWith('.yaml')) {
            window.NotificationModule.showNotification('请打开一个 .tks 或 .yaml 脚本文件', 'warning');
            return;
        }

        // 获取脚本内容
        const scriptContent = currentTab.content;
        if (!scriptContent || scriptContent.trim().length === 0) {
            window.NotificationModule.showNotification('脚本内容为空', 'warning');
            return;
        }

        try {
            window.rLog('🚀 准备调用 executeScriptWithProperFlow 方法');
            await this.executeScriptWithProperFlow(scriptContent, currentTab.path);
            window.rLog('✅ executeScriptWithProperFlow 方法执行完成');
        } catch (error) {
            window.rError('脚本执行失败:', error);
            window.NotificationModule.showNotification(`脚本执行失败: ${error.message}`, 'error');
        }
    }

    /**
     * 按照用户要求的流程执行脚本
     */
    async executeScriptWithProperFlow(scriptContent, scriptPath) {
        window.rLog('=== 📋 执行新的流程方法 executeScriptWithProperFlow ===');
        
        if (this.isRunning) {
            window.NotificationModule.showNotification('脚本正在运行中...', 'warning');
            return;
        }

        // 检查是否有选中的设备
        const deviceSelect = document.getElementById('deviceSelect');
        if (!deviceSelect || !deviceSelect.value) {
            window.NotificationModule.showNotification('请先选择一个设备', 'warning');
            return;
        }

        const deviceId = deviceSelect.value;
        
        // 1. 切换到控制台标签页
        const consoleTab = document.getElementById('consoleTab');
        if (consoleTab) {
            consoleTab.click();
        }

        // 2. 设置运行中状态
        this.isRunning = true;
        this.updateRunButton(true);
        
        // 3. 更新状态栏为运行中
        if (window.AppGlobals.codeEditor) {
            if (typeof window.AppGlobals.codeEditor.setTestRunning === 'function') {
                window.AppGlobals.codeEditor.setTestRunning(true);
                window.rLog('✓ 状态栏已设置为运行中');
            } else {
                window.rWarn('编辑器存在但 setTestRunning 方法不可用', {
                    editorType: window.AppGlobals.codeEditor.constructor.name,
                    hasMethod: typeof window.AppGlobals.codeEditor.setTestRunning
                });
                // 尝试直接获取活动编辑器
                if (window.EditorManager) {
                    const activeEditor = window.EditorManager.getActiveEditor();
                    if (activeEditor && typeof activeEditor.setTestRunning === 'function') {
                        activeEditor.setTestRunning(true);
                        window.rLog('✓ 通过直接访问活动编辑器设置状态栏');
                    }
                }
            }
        } else {
            window.rWarn('编辑器不可用，无法设置状态栏');
        }
        
        // 4. 设置屏幕模式管理器为测试运行状态
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(true);
        }
        
        let executionSuccess = false;
        try {
            window.rLog('开始执行脚本流程...');
            
            // 获取项目路径
            let projectPath = window.AppGlobals.getCurrentProjectPath();
            window.rLog('项目路径:', projectPath);
            
            if (!projectPath) {
                throw new Error('无法获取项目路径。请确保已打开一个项目');
            }

            // 步骤1: 清空控制台并显示开始消息
            window.TestcaseController.ConsoleManager.addLog('=== 开始执行TKS脚本 ===', 'info');
            
            // 步骤2: 先获取当前屏幕截图到项目目录展示
            window.TestcaseController.ConsoleManager.addLog('获取当前设备屏幕...', 'info');
            
            try {
                await this.captureInitialScreen(deviceId, projectPath);
            } catch (screenError) {
                window.rError('获取初始截图失败，继续执行:', screenError);
            }
            
            // 步骤3: 逐行执行脚本
            window.rLog('准备执行逐行脚本, scriptPath:', scriptPath);
            await this.executeScriptLineByLine(scriptPath, projectPath, deviceId);
            
            // 执行成功
            executionSuccess = true;
            
        } catch (error) {
            window.rError('执行脚本时出错:', error);
            window.rError('错误类型:', typeof error);
            window.rError('错误详情:', JSON.stringify(error));
            window.rError('错误堆栈:', error.stack);
            
            const errorMessage = error.message || '未知错误';
            window.TestcaseController.ConsoleManager.addLog(`执行出错: ${errorMessage}`, 'error');
            throw error;
        } finally {
            // 步骤4: 恢复正常模式
            this.isRunning = false;
            this.scriptRunner = null;
            this.updateRunButton(false);
            
            // 恢复编辑器交互状态和状态栏
            if (window.AppGlobals.codeEditor) {
                window.rLog('TKS集成: 恢复编辑器交互状态和状态栏');
                // 成功执行则清除高亮，失败则保持错误高亮
                const clearHighlight = executionSuccess;
                window.rLog('执行结果 - executionSuccess:', executionSuccess, 'clearHighlight:', clearHighlight);
                
                if (typeof window.AppGlobals.codeEditor.setTestRunning === 'function') {
                    window.AppGlobals.codeEditor.setTestRunning(false, clearHighlight);
                } else {
                    // 尝试直接获取活动编辑器
                    if (window.EditorManager) {
                        const activeEditor = window.EditorManager.getActiveEditor();
                        if (activeEditor && typeof activeEditor.setTestRunning === 'function') {
                            window.rLog('通过EditorManager设置 - clearHighlight:', clearHighlight);
                            activeEditor.setTestRunning(false, clearHighlight);
                            window.rLog('✓ 通过直接访问活动编辑器恢复状态');
                        }
                    }
                }
            }
            
            // 恢复屏幕模式切换功能
            if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
                window.TestcaseController.ScreenModeManager.setTestRunning(false);
            }
        }
    }
    
    /**
     * 逐行执行脚本 - 新的执行方式
     */
    async executeScriptLineByLine(scriptPath, projectPath, deviceId) {
        window.rLog('📋 开始逐行执行脚本');
        
        // 读取原始脚本内容
        const fs = window.nodeRequire('fs').promises;
        this.rawContent = await fs.readFile(scriptPath, 'utf8');
        
        // 先解析脚本获取所有步骤
        const parseResult = await this.parseScript(scriptPath);
        if (!parseResult || !parseResult.success || !parseResult.steps || parseResult.steps.length === 0) {
            throw new Error('脚本解析失败或没有可执行的步骤');
        }
        
        const steps = parseResult.steps;
        window.TestcaseController.ConsoleManager.addLog(`准备执行 ${steps.length} 个步骤`, 'info');
        
        // 逐个执行每个步骤
        for (let i = 0; i < steps.length; i++) {
            const step = steps[i];
            const stepNum = i + 1;
            
            try {
                // 1. 高亮当前要执行的行
                this.highlightExecutingStep(i);
                
                // 2. 在控制台显示正在执行的步骤
                window.TestcaseController.ConsoleManager.addLog(
                    `🚀 [步骤 ${stepNum}/${steps.length}] ${step.command}`, 
                    'info'
                );
                
                // 3. 执行单个步骤
                await this.executeSingleStep(step, deviceId, projectPath, i);
                
                // 4. 步骤执行成功，更新截图
                await this.refreshDeviceScreenAfterStep(stepNum);
                
                // 5. 记录步骤完成
                window.TestcaseController.ConsoleManager.addLog(
                    `✅ [步骤 ${stepNum}] 执行成功`, 
                    'success'
                );
                
                // 6. 等待一下让用户看到执行效果
                await this.delay(500);
                
            } catch (error) {
                // 步骤执行失败
                window.rError(`步骤 ${stepNum} 执行失败:`, error);
                window.TestcaseController.ConsoleManager.addLog(
                    `❌ [步骤 ${stepNum}] 执行失败: ${error.message}`, 
                    'error'
                );
                
                // 高亮错误行
                this.highlightErrorStep(i);
                
                // 抛出错误停止执行
                throw error;
            }
        }
        
        // 所有步骤执行完成
        window.TestcaseController.ConsoleManager.addLog('=== 脚本执行完成 ===', 'success');
        window.NotificationModule.showNotification('脚本执行成功', 'success');
    }
    
    /**
     * 解析脚本文件
     */
    async parseScript(scriptPath) {
        const { spawn } = require('child_process');
        
        return new Promise((resolve, reject) => {
            const child = spawn(this.tkeAdapter.tkeExecutable, ['parser', 'parse', scriptPath]);
            
            let stdout = '';
            let stderr = '';
            
            child.stdout.on('data', (data) => {
                stdout += data.toString();
            });
            
            child.stderr.on('data', (data) => {
                stderr += data.toString();
            });
            
            child.on('close', (code) => {
                if (code === 0) {
                    try {
                        const result = JSON.parse(stdout.trim());
                        window.rLog('📋 脚本解析成功:', result);
                        resolve(result);
                    } catch (e) {
                        window.rError('JSON解析错误:', e);
                        window.rError('原始输出:', stdout);
                        reject(new Error(`解析结果失败: ${e.message}`));
                    }
                } else {
                    reject(new Error(stderr || '脚本解析失败'));
                }
            });
        });
    }
    
    /**
     * 执行单个步骤
     */
    async executeSingleStep(step, deviceId, projectPath, stepIndex) {
        const { spawn } = require('child_process');
        
        window.rLog(`执行单步骤 ${stepIndex + 1}:`, step.command);
        
        // 使用 TKE 的 run step 命令来执行单个步骤 - 返回JSON结果
        const args = [
            '--device', deviceId,
            '--project', projectPath,
            'run', 'step', this.rawContent, stepIndex.toString()
        ];
        
        window.rLog('TKE执行命令:', this.tkeAdapter.tkeExecutable, args);
        
        return new Promise((resolve, reject) => {
            const child = spawn(this.tkeAdapter.tkeExecutable, args);
            
            let stdout = '';
            let stderr = '';
            
            child.stdout.on('data', (data) => {
                const output = data.toString();
                stdout += output;
                
                // 输出执行日志
                const lines = output.split('\n');
                for (const line of lines) {
                    const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '').trim();
                    if (cleanLine) {
                        window.TestcaseController.ConsoleManager.addLog(cleanLine, 'info');
                    }
                }
            });
            
            child.stderr.on('data', (data) => {
                stderr += data.toString();
                window.rError('TKE stderr:', data.toString());
            });
            
            child.on('error', (error) => {
                window.rError('TKE进程错误:', error);
                reject(error);
            });
            
            child.on('close', (code) => {
                window.rLog(`TKE进程退出，退出码: ${code}`);
                window.rLog('stdout:', stdout);
                window.rLog('stderr:', stderr);
                
                if (code === 0) {
                    // 尝试解析 JSON 输出
                    try {
                        const result = JSON.parse(stdout.trim());
                        if (result.success) {
                            resolve(result);
                        } else {
                            const errorMsg = result.error || '步骤执行失败';
                            reject(new Error(errorMsg));
                        }
                    } catch (parseError) {
                        // 如果无法解析JSON，回退到原始逻辑
                        window.rError('无法解析TKE JSON输出:', parseError);
                        
                        // 检查输出中是否包含失败信息
                        const outputText = stdout + stderr;
                        const hasError = outputText.includes('脚本执行失败') || 
                                        outputText.includes('步骤执行失败') ||
                                        outputText.includes('ERROR') ||
                                        outputText.includes('元素未找到') ||
                                        outputText.includes('_FAIL.json');
                        
                        if (hasError) {
                            // 从输出中提取错误信息
                            let errorMsg = '步骤执行失败';
                            
                            const errorMatch = outputText.match(/元素未找到[：:]\s*([^\n]+)/);
                            if (errorMatch) {
                                errorMsg = errorMatch[1].trim();
                            } else if (outputText.includes('脚本执行失败')) {
                                const failMatch = outputText.match(/脚本执行失败[：:]\s*[^-\n]*[-]\s*([^\n]+)/);
                                if (failMatch) {
                                    errorMsg = failMatch[1].trim();
                                }
                            }
                            
                            reject(new Error(errorMsg));
                        } else {
                            resolve();
                        }
                    }
                } else {
                    const errorMsg = stderr || stdout || `步骤执行失败，退出码: ${code}`;
                    reject(new Error(errorMsg));
                }
            });
        });
    }
    
    /**
     * 延迟函数
     */
    async delay(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
    
    /**
     * 执行脚本前先截图
     */
    async captureInitialScreen(deviceId, projectPath) {
        try {
            // 使用设备管理器获取初始截图
            if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.captureScreenshot) {
                window.TestcaseController.ConsoleManager.addLog('正在获取设备屏幕截图...', 'info');
                await window.DeviceScreenManagerModule.captureScreenshot(deviceId, projectPath);
                window.TestcaseController.ConsoleManager.addLog('✓ 设备屏幕截图获取成功', 'success');
            }
        } catch (error) {
            window.rError('获取初始屏幕截图失败:', error);
            window.TestcaseController.ConsoleManager.addLog('⚠ 获取初始截图失败，继续执行...', 'warning');
        }
    }
    
    
    
    /**
     * 高亮正在执行的步骤
     */
    highlightExecutingStep(stepIndex) {
        try {
            if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.highlightExecutingLine) {
                const lineNumber = this.getLineNumberFromStepIndex(stepIndex);
                if (lineNumber > 0) {
                    window.rLog('🎯 高亮执行步骤:', stepIndex, '行号:', lineNumber);
                    window.AppGlobals.codeEditor.highlightExecutingLine(lineNumber);
                } else {
                    window.rError('获取行号失败，无法高亮步骤:', stepIndex);
                }
            } else {
                window.rError('编辑器不可用或缺少高亮方法');
            }
        } catch (highlightError) {
            window.rError('高亮执行步骤时出错:', highlightError);
            // 高亮失败不应该阻止脚本执行流程
        }
    }
    
    /**
     * 高亮出错的步骤（红色）
     */
    highlightErrorStep(stepIndex) {
        try {
            if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.highlightErrorLine) {
                const lineNumber = this.getLineNumberFromStepIndex(stepIndex);
                if (lineNumber > 0) {
                    window.rLog('❌ 高亮错误步骤:', stepIndex, '行号:', lineNumber);
                    window.AppGlobals.codeEditor.highlightErrorLine(lineNumber);
                } else {
                    window.rError('获取错误步骤行号失败:', stepIndex);
                }
            } else {
                window.rError('编辑器不可用或缺少错误高亮方法');
            }
        } catch (highlightError) {
            window.rError('高亮错误步骤时出错:', highlightError);
            // 高亮失败不应该阻止脚本执行流程
        }
    }
    
    /**
     * 清除代码高亮
     */
    clearExecutionHighlight() {
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.clearExecutionHighlight) {
            window.rLog('清除执行高亮');
            window.AppGlobals.codeEditor.clearExecutionHighlight();
        }
    }
    
    /**
     * 步骤完成后刷新设备截图 (同步版本)
     */
    async refreshDeviceScreenAfterStep(stepNum = null) {
        const message = stepNum ? `📸 步骤${stepNum}完成，刷新设备截图...` : '📸 刷新设备截图...';
        window.rLog(message);
        
        // 等待一点时间确保设备状态稳定后再截图
        await this.delay(800);
        
        try {
            if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                await window.DeviceScreenManagerModule.refreshDeviceScreen();
                const successMsg = stepNum ? `✓ 设备截图已更新 (步骤 ${stepNum} 完成)` : '✓ 设备截图已更新';
                window.TestcaseController.ConsoleManager.addLog(successMsg, 'info');
            } else {
                window.rWarn('设备屏幕管理器不可用');
            }
        } catch (error) {
            window.rError('刷新设备截图失败:', error);
            window.TestcaseController.ConsoleManager.addLog(`⚠ 刷新截图失败: ${error.message}`, 'warning');
        }
    }


    /**
     * 停止执行
     */
    async stopExecution() {
        window.rLog('TKSScriptRunnerTKE: 用户请求停止脚本执行');
        
        if (this.isRunning && this.scriptRunner) {
            try {
                // 停止TKE执行器
                await this.scriptRunner.stopExecution();
            } catch (error) {
                window.rError('停止执行时出错:', error);
            }
        }
        
        // 更新状态
        this.isRunning = false;
        this.scriptRunner = null;
        this.updateRunButton(false);
        
        // 恢复编辑器交互状态和状态栏
        if (window.AppGlobals.codeEditor) {
            window.rLog('TKSScriptRunnerTKE: 恢复编辑器交互状态和状态栏');
            // 停止执行时保持错误高亮显示，不清除高亮
            if (typeof window.AppGlobals.codeEditor.setTestRunning === 'function') {
                window.AppGlobals.codeEditor.setTestRunning(false, false);
            } else {
                // 尝试直接获取活动编辑器
                if (window.EditorManager) {
                    const activeEditor = window.EditorManager.getActiveEditor();
                    if (activeEditor && typeof activeEditor.setTestRunning === 'function') {
                        activeEditor.setTestRunning(false, false);
                        window.rLog('✓ 通过直接访问活动编辑器恢复状态（停止执行）');
                    }
                }
            }
        }
        
        // 恢复屏幕模式切换功能
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(false);
        }
        
        window.TestcaseController.ConsoleManager.addLog('用户停止了脚本执行', 'warning');
        window.NotificationModule.showNotification('已停止执行', 'info');
    }
    
    /**
     * 根据步骤索引获取行号
     */
    getLineNumberFromStepIndex(stepIndex) {
        // 获取当前打开的文件内容
        const currentTab = window.AppGlobals.currentTab;
        if (!currentTab || !currentTab.content) {
            window.rError('无法获取当前文件内容');
            return 0;
        }

        const content = currentTab.content;
        const lines = content.split('\n');
        
        window.rLog(`🔍 解析TKS脚本行号, 总行数: ${lines.length}, 查找步骤索引: ${stepIndex}`);
        
        // 找到"步骤:"行
        let stepsStartLine = -1;
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].trim() === '步骤:') {
                stepsStartLine = i;
                window.rLog(`找到"步骤:"行: ${i + 1}`);
                break;
            }
        }
        
        if (stepsStartLine === -1) {
            window.rError('未找到"步骤:"行');
            return 0;
        }
        
        // 从"步骤:"之后找到第stepIndex个步骤
        let stepCount = 0;
        for (let i = stepsStartLine + 1; i < lines.length; i++) {
            const line = lines[i].trim();
            if (line && !line.startsWith('#')) { // 非空行且非注释行
                window.rLog(`步骤 ${stepCount}: 行 ${i + 1} = "${line}"`);
                if (stepCount === stepIndex) {
                    window.rLog(`✓ 找到步骤 ${stepIndex} 对应行号: ${i + 1}`);
                    return i + 1; // 编辑器行号从1开始
                }
                stepCount++;
            }
        }
        
        window.rError(`步骤索引 ${stepIndex} 超出范围，总步骤数: ${stepCount}`);
        return 0;
    }

    /**
     * 更新运行按钮状态
     */
    updateRunButton(isRunning) {
        const runTestBtn = document.getElementById('runTestBtn');
        if (!runTestBtn) return;
        
        // 移除所有旧的事件监听器
        const newBtn = runTestBtn.cloneNode(true);
        runTestBtn.parentNode.replaceChild(newBtn, runTestBtn);
        
        if (isRunning) {
            // 运行中状态 - 变成停止按钮
            newBtn.innerHTML = `
                <svg class="btn-icon" viewBox="0 0 24 24">
                    <path d="M6 6h12v12H6z"/>
                </svg>
                Stop Test
            `;
            newBtn.className = 'btn btn-danger btn-block';
            newBtn.addEventListener('click', () => {
                window.rLog('Stop Test按钮被点击');
                this.stopExecution();
            });
        } else {
            // 空闲状态 - 变成运行按钮
            newBtn.innerHTML = `
                <svg class="btn-icon" viewBox="0 0 24 24">
                    <path d="M8 5v14l11-7z"/>
                </svg>
                Run Test
            `;
            newBtn.className = 'btn btn-primary btn-block';
            newBtn.addEventListener('click', () => {
                window.rLog('Run Test按钮被点击');
                this.handleRunTest();
            });
        }
    }
}

// 模块变量
let scriptRunnerTKE = null;
let moduleInitialized = false;

// 初始化模块
async function initializeTKSIntegrationTKE() {
    if (moduleInitialized) {
        window.rLog('TKS集成模块(TKE版本)已经初始化过，跳过重复初始化');
        return;
    }
    
    try {
        // 创建脚本运行器实例
        scriptRunnerTKE = new TKSScriptRunnerTKE();
        
        // 初始化
        await scriptRunnerTKE.init();
        
        // 导出到全局
        window.TKSIntegrationTKE = {
            scriptRunner: scriptRunnerTKE,
            
            // 公开的API
            runCurrentScript: () => scriptRunnerTKE.handleRunTest(),
            stopExecution: () => scriptRunnerTKE.stopExecution()
        };
        
        moduleInitialized = true;
        window.rLog('TKS集成模块(TKE版本)已初始化');
    } catch (error) {
        window.rError('TKS集成模块(TKE版本)初始化失败:', error);
        throw error;
    }
}

// 导出模块
window.TKSIntegrationTKEModule = {
    initializeTKSIntegrationTKE,
    TKSScriptRunnerTKE
};