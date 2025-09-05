// TKS脚本集成模块
// 执行TKS脚本功能

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// TKS脚本运行器
class TKSScriptRunner {
    constructor() {
        this.isRunning = false;
        this.currentScript = null;
        this.initialized = false;
        this.currentExecutor = null; // 保存当前执行器的引用
    }

    /**
     * 初始化脚本运行器
     */
    async init() {
        if (this.initialized) {
            console.log('TKSScriptRunner已经初始化过，跳过重复初始化');
            return;
        }
        
        // 绑定Run Test按钮
        const runTestBtn = document.getElementById('runTestBtn');
        if (runTestBtn) {
            console.log('TKSScriptRunner: 绑定Run Test按钮事件');
            runTestBtn.addEventListener('click', () => {
                console.log('Run Test按钮被点击');
                this.handleRunTest();
            });
        } else {
            console.error('TKSScriptRunner: 找不到runTestBtn元素');
        }
        
        this.initialized = true;
        console.log('TKS脚本运行器已初始化');
    }

    /**
     * 处理运行测试
     */
    async handleRunTest() {
        console.log('handleRunTest: 开始处理测试执行');
        
        // 立即清除编辑器焦点和光标，防止干扰高亮显示
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.contentEl) {
            console.log('handleRunTest: 清除编辑器焦点');
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
        console.log('handleRunTest: 详细调试信息', {
            'window.AppGlobals存在': !!window.AppGlobals,
            'openTabs数量': window.AppGlobals.openTabs ? window.AppGlobals.openTabs.length : '无openTabs',
            'openTabs内容': window.AppGlobals.openTabs ? window.AppGlobals.openTabs.map(t => ({id: t.id, name: t.name, path: t.path})) : '无openTabs',
            'currentTab': currentTab,
            'currentTab类型': typeof currentTab,
            'currentTab是否为null': currentTab === null,
            'currentTab是否为undefined': currentTab === undefined,
            'currentTab完整对象': currentTab
        });
        
        if (!currentTab) {
            console.log('handleRunTest: 没有活动标签页');
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
            await this.executeScript(scriptContent, currentTab.path);
        } catch (error) {
            console.error('脚本执行失败:', error);
            window.NotificationModule.showNotification(`脚本执行失败: ${error.message}`, 'error');
        }
    }

    /**
     * 执行脚本
     */
    async executeScript(scriptContent, scriptPath) {
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
        
        // 切换到控制台标签页
        const consoleTab = document.getElementById('consoleTab');
        if (consoleTab) {
            consoleTab.click();
        }

        this.isRunning = true;
        this.updateRunButton(true);
        
        // 设置屏幕模式管理器为测试运行状态（自动切换到纯屏幕模式并禁用切换）
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(true);
        }
        
        try {
            // 获取项目路径
            let projectPath = window.AppGlobals.getCurrentProjectPath();
            
            console.log('项目路径调试信息:', {
                'window.AppGlobals.currentProject': window.AppGlobals.currentProject,
                'projectPath': projectPath,
                'scriptPath': scriptPath,
                'typeof projectPath': typeof projectPath
            });
            
            if (!projectPath) {
                // 详细调试信息，找出为什么获取不到项目路径
                console.error('❌ 无法获取项目路径 - 详细调试信息:');
                console.error('- window.AppGlobals:', !!window.AppGlobals);
                console.error('- window.AppGlobals.currentProject:', window.AppGlobals?.currentProject);
                console.error('- typeof window.AppGlobals.currentProject:', typeof window.AppGlobals?.currentProject);
                
                // 检查状态栏显示的项目路径作为参考
                const statusBarElement = document.getElementById('statusBarProjectPath');
                if (statusBarElement) {
                    console.error('- 状态栏显示内容:', statusBarElement.textContent);
                    console.error('- 状态栏存储的完整路径:', statusBarElement.parentElement.dataset.fullPath);
                } else {
                    console.error('- 状态栏元素不存在');
                }
                
                throw new Error('无法获取项目路径。请确保已打开一个项目（Project页面 -> Open Project或Create Project）');
            }

            window.TestcaseController.ConsoleManager.addLog('开始执行TKS脚本...', 'info');
            
            // 使用TKS引擎执行脚本
            if (window.TKSScriptModule && window.TKSScriptModule.Executor) {
                this.currentExecutor = new window.TKSScriptModule.Executor(projectPath, deviceId);
                
                // 从脚本路径推断case文件夹
                console.log('调试信息:', {
                    'window.AppGlobals': window.AppGlobals,
                    'window.AppGlobals.path': window.AppGlobals ? window.AppGlobals.path : 'undefined',
                    'scriptPath': scriptPath,
                    'typeof scriptPath': typeof scriptPath
                });
                
                const pathModule = window.AppGlobals ? window.AppGlobals.path : null;
                let currentScriptPath = scriptPath; // 使用传递进来的scriptPath参数
                
                // 如果没有路径，这可能是一个临时脚本，创建一个临时路径
                if (!currentScriptPath) {
                    console.log('检测到临时脚本或未保存的脚本，创建临时路径');
                    // 获取当前项目路径
                    const currentProject = window.AppGlobals.currentProject;
                    if (currentProject && pathModule) {
                        // 创建临时路径：项目/cases/temp/脚本名
                        // 从当前标签页获取文件名（如果有的话）
                        const currentTab = window.AppGlobals.currentTab;
                        const fileName = currentTab && currentTab.name ? currentTab.name : 'temp_script.tks';
                        currentScriptPath = pathModule.join(currentProject, 'cases', 'temp', fileName);
                        console.log('生成临时脚本路径:', currentScriptPath);
                    }
                }
                
                if (pathModule && currentScriptPath) {
                    const pathParts = currentScriptPath.split(pathModule.sep);
                    const caseIndex = pathParts.indexOf('cases');
                    let caseFolder = '';
                    
                    if (caseIndex !== -1 && caseIndex < pathParts.length - 2) {
                        caseFolder = pathParts[caseIndex + 1]; // case_001, case_002 etc.
                        console.log('推断的case文件夹:', caseFolder);
                        this.currentExecutor.setCurrentCase(caseFolder);
                    } else {
                        console.warn('无法推断case文件夹，locator引用可能无法工作');
                    }
                    
                    // 设置脚本物理文件名（去掉.tks扩展名）
                    const fileName = pathModule.basename(currentScriptPath, '.tks');
                    this.currentExecutor.setScriptFileName(fileName);
                    console.log('设置脚本文件名:', fileName);
                } else {
                    console.error('缺少必要的路径信息:', { pathModule, currentScriptPath });
                    // 使用默认值继续执行
                    if (currentScriptPath) {
                        // 简单的文件名提取，不依赖path模块
                        const fileName = currentScriptPath.split(/[/\\]/).pop().replace('.tks', '');
                        this.currentExecutor.setScriptFileName(fileName);
                        console.log('使用简单方式设置脚本文件名:', fileName);
                    } else {
                        // 如果仍然没有路径，使用默认值
                        this.currentExecutor.setCurrentCase('temp');
                        this.currentExecutor.setScriptFileName('temp_script');
                        console.log('使用默认的临时case和文件名');
                    }
                }
                
                // 解析脚本
                const parser = new window.TKSScriptModule.Parser();
                const parsedScript = parser.parse(scriptContent);
                
                if (!parsedScript || !parsedScript.steps || parsedScript.steps.length === 0) {
                    throw new Error('脚本解析失败或没有找到执行步骤');
                }

                // 执行脚本
                const result = await this.currentExecutor.execute(parsedScript);
                
                if (result.success) {
                    const executedSteps = result.steps ? result.steps.length : 0;
                    window.TestcaseController.ConsoleManager.addLog(`脚本执行完成，共执行 ${executedSteps} 步`, 'success');
                    window.NotificationModule.showNotification('脚本执行成功', 'success');
                } else {
                    window.TestcaseController.ConsoleManager.addLog(`脚本执行失败: ${result.error}`, 'error');
                    window.NotificationModule.showNotification('脚本执行失败', 'error');
                }
                
            } else {
                throw new Error('TKS脚本引擎未加载');
            }
            
        } catch (error) {
            console.error('执行脚本时出错:', error);
            window.TestcaseController.ConsoleManager.addLog(`执行出错: ${error.message}`, 'error');
            throw error;
        } finally {
            this.isRunning = false;
            this.currentExecutor = null; // 清除执行器引用
            this.updateRunButton(false);
            
            // 恢复编辑器交互状态
            if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
                console.log('TKS集成: 恢复编辑器交互状态');
                window.AppGlobals.codeEditor.setTestRunning(false);
            }
            
            // 恢复屏幕模式切换功能
            if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
                window.TestcaseController.ScreenModeManager.setTestRunning(false);
            }
            
            // 更新状态栏显示
            if (window.StatusBarModule && window.StatusBarModule.updateRunStatus) {
                console.log('TKS集成: 更新状态栏为空闲状态');
                window.StatusBarModule.updateRunStatus(false);
            }
        }
    }

    /**
     * 停止执行
     */
    stopExecution() {
        console.log('TKSScriptRunner: 用户请求停止脚本执行');
        
        if (this.isRunning && this.currentExecutor) {
            // 停止TKS执行器
            this.currentExecutor.stop();
        }
        
        // 更新状态
        this.isRunning = false;
        this.currentExecutor = null;
        this.updateRunButton(false);
        
        // 恢复编辑器交互状态
        if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.setTestRunning) {
            console.log('TKSScriptRunner: 恢复编辑器交互状态');
            window.AppGlobals.codeEditor.setTestRunning(false);
        }
        
        // 恢复屏幕模式切换功能
        if (window.TestcaseController && window.TestcaseController.ScreenModeManager) {
            window.TestcaseController.ScreenModeManager.setTestRunning(false);
        }
        
        window.TestcaseController.ConsoleManager.addLog('用户停止了脚本执行', 'warning');
        window.NotificationModule.showNotification('已停止执行', 'info');
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
                console.log('Stop Test按钮被点击');
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
                console.log('Run Test按钮被点击');
                this.handleRunTest();
            });
        }
    }
}

// 模块变量
let scriptRunner = null;
let moduleInitialized = false;

// 初始化模块
function initializeTKSIntegration() {
    if (moduleInitialized) {
        console.log('TKS集成模块已经初始化过，跳过重复初始化');
        return;
    }
    
    // 创建脚本运行器实例
    scriptRunner = new TKSScriptRunner();
    
    // 初始化
    scriptRunner.init();
    
    // 导出到全局
    window.TKSIntegration = {
        scriptRunner,
        
        // 公开的API
        runCurrentScript: () => scriptRunner.handleRunTest(),
        stopExecution: () => scriptRunner.stopExecution()
    };
    
    moduleInitialized = true;
    console.log('TKS集成模块已初始化');
}

// 自动初始化已移除，改为由app.js统一管理初始化

// 导出模块
window.TKSIntegrationModule = {
    initializeTKSIntegration
};