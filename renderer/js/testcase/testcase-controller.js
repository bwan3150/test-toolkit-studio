// 测试用例控制器模块
// 负责测试用例页面的初始化和各子模块的协调

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 初始化测试用例页面
function initializeTestcasePage() {
    const { ipcRenderer } = getGlobals();
    const runTestBtn = document.getElementById('runTestBtn');
    const clearConsoleBtn = document.getElementById('clearConsoleBtn');
    const toggleXmlBtn = document.getElementById('toggleXmlBtn');
    const refreshDeviceBtn = document.getElementById('refreshDeviceBtn');
    const deviceSelect = document.getElementById('deviceSelect');
    
    window.rLog('初始化测试用例页面');
    
    // Run Test按钮事件处理已移至tks-integration.js模块
    // clearConsoleBtn事件在下面的 initializeUIElementsPanel 中处理
    
    // 绑定 XML 覆盖层切换按钮
    if (toggleXmlBtn) {
        toggleXmlBtn.addEventListener('click', () => {
            if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.toggleXmlOverlay) {
                window.DeviceScreenManagerModule.toggleXmlOverlay();
            }
        });
    }
    
    // 绑定刷新设备屏幕按钮
    if (refreshDeviceBtn) {
        refreshDeviceBtn.addEventListener('click', () => {
            if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                window.DeviceScreenManagerModule.refreshDeviceScreen();
            }
        });
    }
    
    // 初始化屏幕模式管理器
    setTimeout(() => {
        window.rLog('延迟初始化 ScreenModeManager');
        if (window.ScreenModeManagerModule && window.ScreenModeManagerModule.initializeScreenModeManager) {
            window.ScreenModeManagerModule.initializeScreenModeManager();
        }
    }, 100);
    
    // 设备选择变化时存储选中设备
    if (deviceSelect) {
        deviceSelect.addEventListener('change', (e) => {
            if (e.target.value) {
                ipcRenderer.invoke('store-set', 'selected_device', e.target.value);
            }
        });
    }
    
    // 加载设备列表
    if (window.DeviceManagerModule) {
        window.DeviceManagerModule.refreshDeviceList();
    }
    
    // 初始化输入焦点保护
    initializeInputFocusProtection();
    
    // 初始化UI元素面板
    initializeUIElementsPanel();
    
    // 初始化文件树资源管理器
    if (window.TestcaseExplorerModule) {
        // loadFileTree 会在项目加载时由 project-manager 调用
        window.rLog('文件树资源管理器已准备就绪');
    }
}

// 初始化输入焦点保护
function initializeInputFocusProtection() {
    window.rLog('初始化输入焦点保护...');
    
    // 需要保护的输入框选择器
    const protectedInputSelectors = [
        '#inputDialogInput',
        '#imageAliasInput', 
        '#locatorSearchInput',
        '#newNameInput',
        '.editing',
        '[contenteditable="true"]:not(#editorContent)',
        '.modal-dialog input',
        '.context-menu input',
        '.form-control'
    ];
    
    // 防止编辑器在这些输入框活动时抢夺焦点
    document.addEventListener('mousedown', (e) => {
        const target = e.target;
        
        // 检查是否点击了受保护的输入框
        const isProtectedInput = protectedInputSelectors.some(selector => {
            try {
                return target.matches(selector) || target.closest(selector);
            } catch (err) {
                return false;
            }
        });
        
        if (isProtectedInput) {
            // 确保编辑器知道有其他输入活动
            const activeEditor = window.EditorManager?.getActiveEditor();
            if (activeEditor) {
                activeEditor.isOtherInputFocused = true;
                activeEditor.suppressCursorRestore = true;
            }
            
            window.rLog('保护输入框焦点:', target);
        }
    }, true);
    
    // 特别处理内联编辑（重命名）的完成事件
    const observer = new MutationObserver((mutations) => {
        mutations.forEach((mutation) => {
            if (mutation.type === 'attributes' && mutation.attributeName === 'contenteditable') {
                const target = mutation.target;
                if (target.contentEditable === 'false' && target.classList.contains('editing')) {
                    // 内联编辑结束
                    setTimeout(() => {
                        const activeEditor = window.EditorManager?.getActiveEditor();
                        if (activeEditor) {
                            activeEditor.isOtherInputFocused = false;
                            activeEditor.suppressCursorRestore = false;
                        }
                    }, 100);
                }
            }
        });
    });
    
    // 监控文档中的contenteditable变化
    observer.observe(document.body, {
        attributes: true,
        attributeFilter: ['contenteditable'],
        subtree: true
    });
}

// 初始化UI元素面板
function initializeUIElementsPanel() {
    const clearConsoleBtn = document.getElementById('clearConsoleBtn');
    const uiElementsPanel = document.getElementById('uiElementsPanel');
    const uiElementsList = document.getElementById('uiElementsList');
    const elementSearchInput = document.getElementById('elementSearchInput');
    
    // 清空控制台按钮事件
    if (clearConsoleBtn) {
        clearConsoleBtn.addEventListener('click', () => {
            ConsoleManager.clearLogs();
        });
    }
    
    // UI元素面板初始化
    if (uiElementsPanel && uiElementsList) {
        // 搜索功能
        if (elementSearchInput) {
            elementSearchInput.addEventListener('input', (e) => {
                const searchText = e.target.value.toLowerCase();
                const items = uiElementsList.querySelectorAll('.ui-element-item');
                
                items.forEach(item => {
                    const text = item.textContent.toLowerCase();
                    item.style.display = text.includes(searchText) ? 'flex' : 'none';
                });
            });
        }
    }
}

// 初始化底部面板显示
function initializeBottomPanelDisplay() {
    const testcaseBottomPanel = document.querySelector('#testcasePage .bottom-panel');
    const consoleOutput = document.querySelector('#testcasePage #consoleOutput');
    
    if (testcaseBottomPanel && consoleOutput) {
        // 确保面板可见
        testcaseBottomPanel.style.display = 'block';
        
        // 设置初始高度（如果需要）
        if (!testcaseBottomPanel.style.height) {
            testcaseBottomPanel.style.height = '200px';
        }
        
        // 确保控制台输出区域正确显示
        consoleOutput.style.display = 'block';
        
        // 触发重新计算布局
        window.dispatchEvent(new Event('resize'));
        
        window.rLog('底部面板已初始化并显示');
    }
}

// 控制台管理器
const ConsoleManager = {
    logs: [],
    maxLogs: 1000,
    
    addLog(message, type = 'info') {
        const timestamp = new Date().toLocaleTimeString();
        const log = { message, type, timestamp };
        this.logs.push(log);
        
        // 限制日志数量
        if (this.logs.length > this.maxLogs) {
            this.logs.shift();
        }
        
        // 更新UI
        this.updateConsoleUI(log);
    },
    
    updateConsoleUI(log) {
        const consoleOutput = document.getElementById('consoleOutput');
        if (!consoleOutput) return;
        
        const logElement = document.createElement('div');
        logElement.className = `console-log console-${log.type}`;
        logElement.innerHTML = `<span class="timestamp">[${log.timestamp}]</span> <span class="message">${log.message}</span>`;
        
        consoleOutput.appendChild(logElement);
        
        // 自动滚动到底部
        consoleOutput.scrollTop = consoleOutput.scrollHeight;
    },
    
    clearLogs() {
        this.logs = [];
        const consoleOutput = document.getElementById('consoleOutput');
        if (consoleOutput) {
            consoleOutput.innerHTML = '';
        }
        window.rLog('控制台已清空');
    }
};

// 重新计算 XML 标记位置（由 device-screen-manager 处理）
function recalculateXmlMarkersPosition() {
    if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.recalculateXmlMarkersPosition) {
        window.DeviceScreenManagerModule.recalculateXmlMarkersPosition();
    }
}

// 导出函数
window.TestcaseController = {
    initializeTestcasePage,
    initializeBottomPanelDisplay,
    initializeUIElementsPanel,
    recalculateXmlMarkersPosition,
    ConsoleManager,
    
    // ===== 以下函数委托给已拆分的模块 =====
    
    // 文件树相关功能 - 委托给 TestcaseExplorerModule
    loadFileTree: async () => {
        if (window.TestcaseExplorerModule) {
            return await window.TestcaseExplorerModule.loadFileTree();
        }
    },
    
    createTreeItem: (name, type, fullPath) => {
        if (window.TestcaseExplorerModule) {
            return window.TestcaseExplorerModule.createTreeItem(name, type, fullPath);
        }
    },
    
    openFile: (filePath) => {
        if (window.TestcaseExplorerModule) {
            return window.TestcaseExplorerModule.openFile(filePath);
        }
    },
    
    toggleCaseFolder: (caseContainer) => {
        if (window.TestcaseExplorerModule) {
            return window.TestcaseExplorerModule.toggleCaseFolder(caseContainer);
        }
    },
    
    // 设备屏幕相关功能 - 委托给 DeviceScreenManagerModule
    refreshDeviceScreen: async () => {
        if (window.DeviceScreenManagerModule) {
            return await window.DeviceScreenManagerModule.refreshDeviceScreen();
        }
    },
    
    toggleXmlOverlay: () => {
        if (window.DeviceScreenManagerModule) {
            return window.DeviceScreenManagerModule.toggleXmlOverlay();
        }
    },
    
    enableXmlOverlay: async (deviceId) => {
        if (window.DeviceScreenManagerModule) {
            return await window.DeviceScreenManagerModule.enableXmlOverlay(deviceId);
        }
    },
    
    displayUIElementList: (elements) => {
        if (window.DeviceScreenManagerModule) {
            return window.DeviceScreenManagerModule.displayUIElementList(elements);
        }
    },
    
    // 运行测试 - 委托给 TKS 集成模块
    runCurrentTest: async () => {
        if (window.TKSIntegrationTKEModule) {
            return await window.TKSIntegrationTKEModule.runCurrentTest();
        } else if (window.TKSIntegrationModule) {
            return await window.TKSIntegrationModule.runCurrentTest();
        }
    },
    
    // 屏幕模式管理器代理（用于兼容性）
    ScreenModeManager: {
        setTestRunning: (running) => {
            if (window.ScreenModeManagerModule && window.ScreenModeManagerModule.ScreenModeManager) {
                window.ScreenModeManagerModule.ScreenModeManager.setTestRunning(running);
            }
        },
        updateZoomControlsVisibility: () => {
            if (window.ScreenModeManagerModule && window.ScreenModeManagerModule.ScreenModeManager) {
                window.ScreenModeManagerModule.ScreenModeManager.updateZoomControlsVisibility();
            }
        },
        // 代理其他可能用到的方法
        init: () => {
            if (window.ScreenModeManagerModule && window.ScreenModeManagerModule.ScreenModeManager) {
                window.ScreenModeManagerModule.ScreenModeManager.init();
            }
        },
        setMode: (mode) => {
            if (window.ScreenModeManagerModule && window.ScreenModeManagerModule.ScreenModeManager) {
                window.ScreenModeManagerModule.ScreenModeManager.setMode(mode);
            }
        }
    }
};