// TKS脚本集成模块
// 将XML元素保存到locator，管理元素列表，执行TKS脚本

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// Locator管理器
class LocatorManager {
    constructor() {
        this.currentCase = null;
        this.elements = {};
        this.elementListContainer = null;
        this.initialized = false;
    }

    /**
     * 初始化Locator管理器
     */
    async init() {
        if (this.initialized) return;
        
        // 添加Locator标签页
        this.addLocatorTab();
        
        // 监听元素选择事件
        this.setupElementListeners();
        
        // 延迟加载当前Case的locator，避免初始化时AppGlobals还未完全准备
        setTimeout(() => {
            this.loadCurrentCaseLocator().catch(err => {
                console.log('初始加载locator失败（可能还没有打开项目）:', err.message);
            });
        }, 500);
        
        this.initialized = true;
        console.log('Locator管理器已初始化');
    }

    /**
     * 添加Locator标签页到UI元素面板
     */
    addLocatorTab() {
        const tabHeader = document.querySelector('.elements-tabs .tab-header');
        if (!tabHeader) return;

        // 检查是否已存在
        if (document.querySelector('[data-tab="locator-list"]')) return;

        // 添加新的标签按钮
        const locatorTabBtn = document.createElement('button');
        locatorTabBtn.className = 'tab-btn';
        locatorTabBtn.dataset.tab = 'locator-list';
        locatorTabBtn.textContent = 'Locator库';
        
        // 插入到"元素属性"标签后面
        const elementPropsTab = document.getElementById('elementPropsTab');
        if (elementPropsTab) {
            elementPropsTab.parentNode.insertBefore(locatorTabBtn, elementPropsTab.nextSibling);
        } else {
            tabHeader.appendChild(locatorTabBtn);
        }

        // 添加标签内容面板
        const tabContent = document.getElementById('uiElementsPanelContent');
        if (tabContent) {
            const locatorPane = document.createElement('div');
            locatorPane.className = 'tab-pane';
            locatorPane.id = 'locatorListPane';
            locatorPane.style.display = 'none';
            locatorPane.innerHTML = `
                <div class="locator-container">
                    <div class="locator-header">
                        <input type="text" id="locatorSearch" placeholder="搜索元素..." class="locator-search">
                        <button class="btn btn-icon btn-small" id="refreshLocatorBtn" title="刷新">
                            <svg viewBox="0 0 24 24">
                                <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
                            </svg>
                        </button>
                        <button class="btn btn-icon btn-small" id="clearLocatorBtn" title="清空所有">
                            <svg viewBox="0 0 24 24">
                                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                            </svg>
                        </button>
                    </div>
                    <div class="locator-list" id="locatorList">
                        <div class="empty-state">
                            <div class="empty-state-text">暂无保存的元素</div>
                            <div class="empty-state-hint">从UI元素列表中选择元素并保存到Locator</div>
                        </div>
                    </div>
                </div>
            `;
            tabContent.appendChild(locatorPane);
        }

        // 添加标签切换事件
        locatorTabBtn.addEventListener('click', () => {
            // 切换标签激活状态
            document.querySelectorAll('.elements-tabs .tab-btn').forEach(btn => {
                btn.classList.remove('active');
            });
            locatorTabBtn.classList.add('active');

            // 切换内容面板
            document.querySelectorAll('#uiElementsPanelContent .tab-pane').forEach(pane => {
                pane.style.display = 'none';
            });
            document.getElementById('locatorListPane').style.display = 'block';

            // 刷新locator列表
            this.refreshLocatorList();
        });

        // 绑定按钮事件
        document.getElementById('refreshLocatorBtn')?.addEventListener('click', () => {
            this.loadCurrentCaseLocator();
        });

        document.getElementById('clearLocatorBtn')?.addEventListener('click', () => {
            if (confirm('确定要清空所有保存的元素吗？')) {
                this.clearAllElements();
            }
        });

        document.getElementById('locatorSearch')?.addEventListener('input', (e) => {
            this.filterElements(e.target.value);
        });
    }

    /**
     * 设置元素监听器
     */
    setupElementListeners() {
        // 监听UI元素列表的变化
        const observer = new MutationObserver((mutations) => {
            mutations.forEach((mutation) => {
                if (mutation.type === 'childList') {
                    this.enhanceElementItems();
                }
            });
        });

        const elementsContainer = document.getElementById('elementsListContainer');
        if (elementsContainer) {
            observer.observe(elementsContainer, { childList: true, subtree: true });
        }
    }

    /**
     * 增强元素项，添加保存按钮
     */
    enhanceElementItems() {
        const elementItems = document.querySelectorAll('.element-item:not(.enhanced)');
        
        elementItems.forEach(item => {
            item.classList.add('enhanced');
            
            // 添加保存按钮
            const saveBtn = document.createElement('button');
            saveBtn.className = 'element-save-btn';
            saveBtn.title = '保存到Locator';
            saveBtn.innerHTML = `
                <svg viewBox="0 0 24 24" width="16" height="16">
                    <path d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
                </svg>
            `;
            
            saveBtn.addEventListener('click', (e) => {
                e.stopPropagation();
                const elementData = this.extractElementData(item);
                this.saveElementToLocator(elementData);
            });
            
            item.appendChild(saveBtn);
        });
    }

    /**
     * 从元素项提取数据
     */
    extractElementData(elementItem) {
        // 获取元素索引
        const indexMatch = elementItem.textContent.match(/\[(\d+)\]/);
        const index = indexMatch ? parseInt(indexMatch[1]) : 0;
        
        // 从当前的UI元素列表中获取完整数据
        if (window.currentUIElements && window.currentUIElements[index]) {
            return window.currentUIElements[index];
        }
        
        // 备用方案：从显示文本解析
        const text = elementItem.textContent;
        const classMatch = text.match(/(\w+)\(/);
        const className = classMatch ? classMatch[1] : '';
        
        // 解析属性
        const attributes = {};
        const attrRegex = /(\w+)=([^,)]+)/g;
        let match;
        while ((match = attrRegex.exec(text)) !== null) {
            attributes[match[1]] = match[2];
        }
        
        return {
            index,
            className,
            text: attributes.text || '',
            contentDesc: attributes['content-desc'] || '',
            resourceId: attributes.id || '',
            hint: attributes.hintText || '',
            ...attributes
        };
    }

    /**
     * 保存元素到Locator
     */
    async saveElementToLocator(elementData) {
        // 弹出输入框让用户命名
        const name = await this.promptElementName(elementData);
        if (!name) return;
        
        // 构建元素定义
        const elementDef = {
            text: elementData.text || undefined,
            contentDesc: elementData.contentDesc || undefined,
            resourceId: elementData.resourceId || undefined,
            className: elementData.className || undefined,
            hint: elementData.hint || undefined,
            xpath: elementData.xpath || undefined,
            bounds: elementData.bounds || undefined
        };
        
        // 移除undefined值
        Object.keys(elementDef).forEach(key => {
            if (elementDef[key] === undefined) {
                delete elementDef[key];
            }
        });
        
        // 保存到内存
        this.elements[name] = elementDef;
        
        // 保存到文件
        await this.saveToFile();
        
        // 刷新显示
        this.refreshLocatorList();
        
        window.NotificationModule.showNotification(`元素 "${name}" 已保存到Locator`, 'success');
    }

    /**
     * 提示用户输入元素名称
     */
    async promptElementName(elementData) {
        // 生成建议的名称
        let suggestedName = '';
        if (elementData.text) {
            suggestedName = elementData.text.replace(/\s+/g, '_').toLowerCase();
        } else if (elementData.contentDesc) {
            suggestedName = elementData.contentDesc.replace(/\s+/g, '_').toLowerCase();
        } else if (elementData.resourceId) {
            const idParts = elementData.resourceId.split(/[/:]/);
            suggestedName = idParts[idParts.length - 1];
        } else {
            suggestedName = `element_${Object.keys(this.elements).length + 1}`;
        }
        
        const name = prompt('请输入元素名称（用于在脚本中引用）:', suggestedName);
        
        if (name && this.elements[name]) {
            if (!confirm(`元素 "${name}" 已存在，是否覆盖？`)) {
                return null;
            }
        }
        
        return name;
    }

    /**
     * 加载当前Case的locator
     */
    async loadCurrentCaseLocator() {
        const { path, fs } = getGlobals();
        
        // 获取当前打开的文件路径
        const currentTab = window.AppGlobals.currentTab;
        if (!currentTab || !currentTab.path) {
            this.currentCase = null;
            this.elements = {};
            return;
        }
        
        // 查找case文件夹
        const filePath = currentTab.path;
        const casesIndex = filePath.indexOf('cases');
        if (casesIndex === -1) {
            this.currentCase = null;
            this.elements = {};
            return;
        }
        
        // 提取case文件夹路径
        const pathParts = filePath.substring(casesIndex).split(path.sep);
        if (pathParts.length < 2) {
            this.currentCase = null;
            this.elements = {};
            return;
        }
        
        const caseFolder = pathParts[1];
        const casePath = filePath.substring(0, casesIndex) + path.join('cases', caseFolder);
        this.currentCase = casePath;
        
        // 加载element.json
        const elementFile = path.join(casePath, 'locator', 'element.json');
        
        try {
            const content = await fs.readFile(elementFile, 'utf-8');
            this.elements = JSON.parse(content);
        } catch (error) {
            // 文件不存在或解析失败
            this.elements = {};
        }
        
        this.refreshLocatorList();
    }

    /**
     * 保存到文件
     */
    async saveToFile() {
        if (!this.currentCase) {
            window.NotificationModule.showNotification('请先打开一个测试用例', 'warning');
            return;
        }
        
        const { path, fs } = getGlobals();
        const locatorDir = path.join(this.currentCase, 'locator');
        const elementFile = path.join(locatorDir, 'element.json');
        
        try {
            // 确保目录存在
            await fs.mkdir(locatorDir, { recursive: true });
            
            // 保存文件
            await fs.writeFile(elementFile, JSON.stringify(this.elements, null, 4));
        } catch (error) {
            console.error('保存locator失败:', error);
            window.NotificationModule.showNotification('保存失败: ' + error.message, 'error');
        }
    }

    /**
     * 刷新Locator列表显示
     */
    refreshLocatorList() {
        const locatorList = document.getElementById('locatorList');
        if (!locatorList) return;
        
        if (Object.keys(this.elements).length === 0) {
            locatorList.innerHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">暂无保存的元素</div>
                    <div class="empty-state-hint">从UI元素列表中选择元素并保存到Locator</div>
                </div>
            `;
            return;
        }
        
        locatorList.innerHTML = '';
        
        Object.entries(this.elements).forEach(([name, def]) => {
            const item = document.createElement('div');
            item.className = 'locator-item';
            item.dataset.name = name;
            
            // 生成描述
            let description = '';
            if (def.text) description = def.text;
            else if (def.contentDesc) description = def.contentDesc;
            else if (def.resourceId) description = def.resourceId.split(/[/:]/);
            else if (def.className) description = def.className.split('.').pop();
            
            item.innerHTML = `
                <div class="locator-item-header">
                    <span class="locator-name">${name}</span>
                    <div class="locator-actions">
                        <button class="btn-icon-mini" title="复制名称" data-action="copy">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                            </svg>
                        </button>
                        <button class="btn-icon-mini" title="编辑" data-action="edit">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                            </svg>
                        </button>
                        <button class="btn-icon-mini" title="删除" data-action="delete">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                            </svg>
                        </button>
                    </div>
                </div>
                <div class="locator-item-desc">${description}</div>
                <div class="locator-item-props">
                    ${Object.entries(def).map(([key, value]) => 
                        `<span class="prop-tag">${key}: ${value}</span>`
                    ).join('')}
                </div>
            `;
            
            // 绑定事件
            item.querySelector('[data-action="copy"]').addEventListener('click', () => {
                navigator.clipboard.writeText(name);
                window.NotificationModule.showNotification(`已复制: ${name}`, 'success');
            });
            
            item.querySelector('[data-action="edit"]').addEventListener('click', () => {
                this.editElement(name);
            });
            
            item.querySelector('[data-action="delete"]').addEventListener('click', () => {
                this.deleteElement(name);
            });
            
            locatorList.appendChild(item);
        });
    }

    /**
     * 编辑元素
     */
    async editElement(name) {
        const def = this.elements[name];
        if (!def) return;
        
        // 简单的编辑对话框（可以后续改进为更好的UI）
        const newDef = prompt(`编辑元素 "${name}" 的定义（JSON格式）:`, JSON.stringify(def, null, 2));
        
        if (newDef) {
            try {
                this.elements[name] = JSON.parse(newDef);
                await this.saveToFile();
                this.refreshLocatorList();
                window.NotificationModule.showNotification(`元素 "${name}" 已更新`, 'success');
            } catch (error) {
                window.NotificationModule.showNotification('JSON格式错误', 'error');
            }
        }
    }

    /**
     * 删除元素
     */
    async deleteElement(name) {
        if (!confirm(`确定要删除元素 "${name}" 吗？`)) return;
        
        delete this.elements[name];
        await this.saveToFile();
        this.refreshLocatorList();
        window.NotificationModule.showNotification(`元素 "${name}" 已删除`, 'success');
    }

    /**
     * 清空所有元素
     */
    async clearAllElements() {
        this.elements = {};
        await this.saveToFile();
        this.refreshLocatorList();
        window.NotificationModule.showNotification('已清空所有元素', 'success');
    }

    /**
     * 过滤元素
     */
    filterElements(keyword) {
        const items = document.querySelectorAll('.locator-item');
        const lowerKeyword = keyword.toLowerCase();
        
        items.forEach(item => {
            const name = item.dataset.name.toLowerCase();
            const desc = item.querySelector('.locator-item-desc').textContent.toLowerCase();
            
            if (name.includes(lowerKeyword) || desc.includes(lowerKeyword)) {
                item.style.display = 'block';
            } else {
                item.style.display = 'none';
            }
        });
    }
}

// TKS脚本运行器
class TKSScriptRunner {
    constructor() {
        this.isRunning = false;
        this.currentExecutor = null;
    }

    /**
     * 初始化运行器
     */
    init() {
        // 绑定Run Test按钮
        const runTestBtn = document.getElementById('runTestBtn');
        if (runTestBtn) {
            // 移除所有现有的事件监听器
            const newRunTestBtn = runTestBtn.cloneNode(true);
            runTestBtn.parentNode.replaceChild(newRunTestBtn, runTestBtn);
            
            // 绑定新的事件处理器
            newRunTestBtn.addEventListener('click', () => {
                this.handleRunTest();
            });
        }
        
        console.log('TKS脚本运行器已初始化');
    }

    /**
     * 处理运行测试
     */
    async handleRunTest() {
        if (this.isRunning) {
            // 停止当前执行
            this.stopExecution();
            return;
        }
        
        // 获取当前打开的脚本
        const currentTab = window.AppGlobals.currentTab;
        if (!currentTab || !currentTab.path) {
            window.NotificationModule.showNotification('请先打开一个.tks脚本文件', 'warning');
            return;
        }
        
        // 检查是否是.tks文件
        if (!currentTab.path.endsWith('.tks')) {
            window.NotificationModule.showNotification('当前文件不是.tks脚本', 'warning');
            return;
        }
        
        // 获取选中的设备
        const deviceSelect = document.getElementById('deviceSelect');
        const deviceId = deviceSelect ? deviceSelect.value : null;
        
        if (!deviceId) {
            window.NotificationModule.showNotification('请先选择一个设备', 'warning');
            return;
        }
        
        // 开始执行
        this.runScript(currentTab.path, deviceId);
    }

    /**
     * 运行脚本
     */
    async runScript(scriptPath, deviceId) {
        const { path, fs } = getGlobals();
        
        try {
            // 更新UI状态
            this.setRunningState(true);
            
            // 切换到控制台标签
            this.switchToConsole();
            
            // 清空控制台
            this.clearConsole();
            
            // 输出开始信息
            this.logToConsole('info', `开始执行脚本: ${path.basename(scriptPath)}`);
            this.logToConsole('info', `目标设备: ${deviceId}`);
            this.logToConsole('info', '-----------------------------------');
            
            // 加载TKS脚本引擎（如果还没加载）
            if (!window.TKSScriptModule) {
                const scriptTag = document.createElement('script');
                scriptTag.src = 'js/modules/tks-script-engine.js';
                document.head.appendChild(scriptTag);
                
                // 等待加载完成
                await new Promise(resolve => {
                    scriptTag.onload = resolve;
                });
            }
            
            // 创建执行器
            const projectPath = window.AppGlobals.currentProject;
            const manager = new window.TKSScriptManager(projectPath);
            
            // 解析脚本
            this.logToConsole('info', '正在解析脚本...');
            const script = await manager.loadScript(scriptPath);
            
            this.logToConsole('success', `脚本解析成功，共 ${script.steps.length} 个步骤`);
            
            // 创建执行器并设置事件监听
            const executor = new window.TKSScriptExecutor(projectPath, deviceId);
            this.currentExecutor = executor;
            
            // 获取case文件夹
            const casesIndex = scriptPath.indexOf('cases');
            if (casesIndex !== -1) {
                const pathParts = scriptPath.substring(casesIndex).split(path.sep);
                if (pathParts.length >= 2) {
                    executor.setCurrentCase(pathParts[1]);
                }
            }
            
            // 执行脚本
            this.logToConsole('info', '开始执行步骤...');
            
            for (let i = 0; i < script.steps.length; i++) {
                if (!this.isRunning) {
                    this.logToConsole('warning', '执行已中止');
                    break;
                }
                
                const step = script.steps[i];
                this.logToConsole('info', `[步骤 ${i + 1}/${script.steps.length}] ${step.raw}`);
                
                try {
                    const result = await executor.executeStep(step, script);
                    
                    if (result.success) {
                        this.logToConsole('success', `  ✓ 执行成功 (${result.duration}ms)`);
                    } else {
                        this.logToConsole('error', `  ✗ 执行失败: ${result.error}`);
                        break;
                    }
                } catch (error) {
                    this.logToConsole('error', `  ✗ 异常: ${error.message}`);
                    break;
                }
            }
            
            this.logToConsole('info', '-----------------------------------');
            this.logToConsole('success', '脚本执行完成');
            
            // 提示结果已保存
            const resultDir = path.join(projectPath, 'result');
            this.logToConsole('info', `执行结果已保存到: ${resultDir}`);
            
        } catch (error) {
            this.logToConsole('error', `执行失败: ${error.message}`);
        } finally {
            this.setRunningState(false);
            this.currentExecutor = null;
        }
    }

    /**
     * 停止执行
     */
    stopExecution() {
        if (this.currentExecutor) {
            this.currentExecutor.stop();
        }
        this.isRunning = false;
        this.setRunningState(false);
        this.logToConsole('warning', '正在停止执行...');
    }

    /**
     * 设置运行状态
     */
    setRunningState(running) {
        this.isRunning = running;
        const runTestBtn = document.getElementById('runTestBtn');
        
        if (runTestBtn) {
            if (running) {
                runTestBtn.innerHTML = `
                    <svg class="btn-icon" viewBox="0 0 24 24">
                        <path d="M6 6h12v12H6z"/>
                    </svg>
                    Stop Test
                `;
                runTestBtn.classList.add('btn-danger');
                runTestBtn.classList.remove('btn-primary');
            } else {
                runTestBtn.innerHTML = `
                    <svg class="btn-icon" viewBox="0 0 24 24">
                        <path d="M8 5v14l11-7z"/>
                    </svg>
                    Run Test
                `;
                runTestBtn.classList.remove('btn-danger');
                runTestBtn.classList.add('btn-primary');
            }
        }
    }

    /**
     * 切换到控制台
     */
    switchToConsole() {
        const consoleTab = document.getElementById('consoleTab');
        if (consoleTab) {
            consoleTab.click();
        }
    }

    /**
     * 清空控制台
     */
    clearConsole() {
        const consoleOutput = document.getElementById('consoleOutput');
        if (consoleOutput) {
            consoleOutput.innerHTML = '';
        }
    }

    /**
     * 输出到控制台
     */
    logToConsole(type, message) {
        const consoleOutput = document.getElementById('consoleOutput');
        if (!consoleOutput) return;
        
        const timestamp = new Date().toLocaleTimeString();
        const logEntry = document.createElement('div');
        logEntry.className = `console-entry console-${type}`;
        logEntry.innerHTML = `
            <span class="console-time">[${timestamp}]</span>
            <span class="console-message">${this.escapeHtml(message)}</span>
        `;
        
        consoleOutput.appendChild(logEntry);
        
        // 自动滚动到底部
        consoleOutput.scrollTop = consoleOutput.scrollHeight;
    }

    /**
     * 转义HTML
     */
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// 初始化模块
let locatorManager = null;
let scriptRunner = null;

function initializeTKSIntegration() {
    // 创建管理器实例
    locatorManager = new LocatorManager();
    scriptRunner = new TKSScriptRunner();
    
    // 初始化
    locatorManager.init();
    scriptRunner.init();
    
    // 导出到全局
    window.TKSIntegration = {
        locatorManager,
        scriptRunner,
        
        // 公开的API
        saveElementToLocator: (elementData) => locatorManager.saveElementToLocator(elementData),
        runCurrentScript: () => scriptRunner.handleRunTest(),
        stopExecution: () => scriptRunner.stopExecution(),
        refreshLocator: () => locatorManager.loadCurrentCaseLocator()
    };
    
    console.log('TKS集成模块已初始化');
}

// 延迟初始化，等待 AppGlobals 加载完成
function waitForAppGlobals() {
    if (window.AppGlobals && window.AppGlobals.path && window.AppGlobals.fs) {
        initializeTKSIntegration();
    } else {
        // 如果 AppGlobals 还未准备好，100ms 后重试
        setTimeout(waitForAppGlobals, 100);
    }
}

// 页面加载完成后开始等待
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', waitForAppGlobals);
} else {
    waitForAppGlobals();
}

// 添加样式
const style = document.createElement('style');
style.textContent = `
/* Locator样式 */
.locator-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 8px;
}

.locator-header {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
}

.locator-search {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--input-bg);
    color: var(--text-primary);
    font-size: 13px;
}

.locator-list {
    flex: 1;
    overflow-y: auto;
}

.locator-item {
    background: var(--card-bg);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 10px;
    margin-bottom: 8px;
    transition: all 0.2s;
}

.locator-item:hover {
    border-color: var(--primary-color);
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.locator-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
}

.locator-name {
    font-weight: 600;
    color: var(--primary-color);
    font-size: 14px;
}

.locator-actions {
    display: flex;
    gap: 4px;
}

.btn-icon-mini {
    background: none;
    border: none;
    padding: 2px;
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.2s;
}

.btn-icon-mini:hover {
    opacity: 1;
}

.btn-icon-mini svg {
    fill: var(--text-secondary);
}

.locator-item-desc {
    color: var(--text-secondary);
    font-size: 12px;
    margin-bottom: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.locator-item-props {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
}

.prop-tag {
    background: var(--bg-secondary);
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
    color: var(--text-secondary);
}

/* 元素项增强样式 */
.element-item {
    position: relative;
    padding-right: 30px;
}

.element-save-btn {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    background: var(--primary-color);
    border: none;
    border-radius: 4px;
    padding: 4px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.2s;
}

.element-item:hover .element-save-btn {
    opacity: 1;
}

.element-save-btn:hover {
    background: var(--primary-hover);
}

.element-save-btn svg {
    fill: white;
    display: block;
}

/* 控制台样式增强 */
.console-entry {
    padding: 4px 8px;
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 12px;
    line-height: 1.4;
    border-bottom: 1px solid var(--border-color);
}

.console-entry.console-info {
    color: var(--text-primary);
}

.console-entry.console-success {
    color: #4caf50;
}

.console-entry.console-warning {
    color: #ff9800;
}

.console-entry.console-error {
    color: #f44336;
}

.console-time {
    color: var(--text-secondary);
    margin-right: 8px;
}

.console-message {
    word-break: break-word;
}

/* 运行按钮状态 */
.btn-danger {
    background: #f44336 !important;
}

.btn-danger:hover {
    background: #da190b !important;
}
`;
document.head.appendChild(style);

// 导出模块
window.TKSIntegrationModule = {
    LocatorManager,
    TKSScriptRunner,
    initializeTKSIntegration
};