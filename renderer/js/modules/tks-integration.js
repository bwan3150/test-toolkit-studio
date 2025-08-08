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
        
        // 获取当前活动的标签页
        const currentTab = window.AppGlobals.currentTab;
        console.log('handleRunTest: 当前标签页', currentTab);
        
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
        
        try {
            // 获取项目路径
            let projectPath = '';
            if (scriptPath && window.AppGlobals.currentProject) {
                projectPath = window.AppGlobals.currentProject;
            }

            window.TestcaseManagerModule.ConsoleManager.addLog('开始执行TKS脚本...', 'info');
            
            // 使用TKS引擎执行脚本
            if (window.TKSScriptModule && window.TKSScriptModule.Executor) {
                const executor = new window.TKSScriptModule.Executor(projectPath, deviceId);
                
                // 从脚本路径推断case文件夹
                const { path } = window.AppGlobals;
                const scriptPath = currentTab.path;
                const pathParts = scriptPath.split(path.sep);
                const caseIndex = pathParts.indexOf('cases');
                let caseFolder = '';
                
                if (caseIndex !== -1 && caseIndex < pathParts.length - 2) {
                    caseFolder = pathParts[caseIndex + 1]; // case_001, case_002 etc.
                    console.log('推断的case文件夹:', caseFolder);
                    executor.setCurrentCase(caseFolder);
                } else {
                    console.warn('无法推断case文件夹，locator引用可能无法工作');
                }
                
                // 设置脚本物理文件名（去掉.tks扩展名）
                const fileName = path.basename(scriptPath, '.tks');
                executor.setScriptFileName(fileName);
                console.log('设置脚本文件名:', fileName);
                
                // 解析脚本
                const parser = new window.TKSScriptModule.Parser();
                const parsedScript = parser.parse(scriptContent);
                
                if (!parsedScript || !parsedScript.steps || parsedScript.steps.length === 0) {
                    throw new Error('脚本解析失败或没有找到执行步骤');
                }

                // 执行脚本
                const result = await executor.execute(parsedScript);
                
                if (result.success) {
                    const executedSteps = result.steps ? result.steps.length : 0;
                    window.TestcaseManagerModule.ConsoleManager.addLog(`脚本执行完成，共执行 ${executedSteps} 步`, 'success');
                    window.NotificationModule.showNotification('脚本执行成功', 'success');
                } else {
                    window.TestcaseManagerModule.ConsoleManager.addLog(`脚本执行失败: ${result.error}`, 'error');
                    window.NotificationModule.showNotification('脚本执行失败', 'error');
                }
                
            } else {
                throw new Error('TKS脚本引擎未加载');
            }
            
        } catch (error) {
            console.error('执行脚本时出错:', error);
            window.TestcaseManagerModule.ConsoleManager.addLog(`执行出错: ${error.message}`, 'error');
            throw error;
        } finally {
            this.isRunning = false;
        }
    }

    /**
     * 停止执行
     */
    stopExecution() {
        if (this.isRunning) {
            this.isRunning = false;
            window.TestcaseManagerModule.ConsoleManager.addLog('用户停止了脚本执行', 'warning');
            window.NotificationModule.showNotification('已停止执行', 'info');
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