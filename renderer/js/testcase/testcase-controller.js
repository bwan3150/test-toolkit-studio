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
    
    // 清空控制台按钮事件
    if (clearConsoleBtn) {
        clearConsoleBtn.addEventListener('click', () => {
            if (window.ConsolePanel && window.ConsolePanel.clearConsole) {
                window.ConsolePanel.clearConsole();
            }
        });
    }
    
    // 初始化标签页切换
    initializeTabSwitching();
    
    // 确保底部面板可见并设置正确的高度
    const bottomPanel = document.getElementById('uiElementsBottomPanel');
    if (bottomPanel) {
        bottomPanel.style.display = 'flex';
        bottomPanel.style.height = '300px'; // 设置初始高度
        bottomPanel.classList.remove('collapsed');
        
        // 确保第一个面板内容可见
        const firstPane = document.getElementById('elementsListPane');
        if (firstPane) {
            firstPane.style.display = 'flex';
            firstPane.classList.add('active');
        }
    }
}

// 初始化标签页切换功能
function initializeTabSwitching() {
    const tabBtns = document.querySelectorAll('.ui-elements-bottom-panel .tab-btn');
    const tabPanes = document.querySelectorAll('.ui-elements-bottom-panel .tab-pane');
    
    tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.getAttribute('data-tab');
            
            // 更新标签按钮状态
            tabBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // 切换内容面板
            tabPanes.forEach(pane => {
                const paneId = pane.id;
                // 根据data-tab属性匹配对应的面板ID
                let shouldShow = false;
                switch(tabId) {
                    case 'elements-list':
                        shouldShow = paneId === 'elementsListPane';
                        break;
                    case 'element-props':
                        shouldShow = paneId === 'elementPropsPane';
                        break;
                    case 'locator-lib':
                        shouldShow = paneId === 'locatorLibPane';
                        break;
                    case 'console-output':
                        shouldShow = paneId === 'consoleOutputPane';
                        break;
                }
                
                if (shouldShow) {
                    pane.style.display = 'block';
                    pane.classList.add('active');
                } else {
                    pane.style.display = 'none';
                    pane.classList.remove('active');
                }
            });
            
            window.rLog(`切换到标签: ${tabId}`);
        });
    });
    
    // 默认激活第一个标签
    const firstTabBtn = tabBtns[0];
    const firstTabPane = tabPanes[0];
    if (firstTabBtn && firstTabPane) {
        firstTabBtn.classList.add('active');
        firstTabPane.style.display = 'block';
        firstTabPane.classList.add('active');
    }
}

// 初始化底部面板显示
function initializeBottomPanelDisplay() {
    const testcaseBottomPanel = document.querySelector('#testcasePage .bottom-panel');
    const consoleContent = document.querySelector('#testcasePage #consoleContent');

    if (testcaseBottomPanel && consoleContent) {
        // 确保面板可见
        testcaseBottomPanel.style.display = 'block';

        // 设置初始高度（如果需要）
        if (!testcaseBottomPanel.style.height) {
            testcaseBottomPanel.style.height = '200px';
        }

        // 确保控制台输出区域正确显示
        consoleContent.style.display = 'block';
        
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
        const consoleContent = document.getElementById('consoleContent');
        if (!consoleContent) return;

        const logElement = document.createElement('div');
        logElement.className = `console-log console-${log.type}`;
        logElement.innerHTML = `<span class="timestamp">[${log.timestamp}]</span> <span class="message">${log.message}</span>`;

        consoleContent.appendChild(logElement);

        // 自动滚动到底部
        consoleContent.scrollTop = consoleContent.scrollHeight;
    },

    clearLogs() {
        this.logs = [];
        const consoleContent = document.getElementById('consoleContent');
        if (consoleContent) {
            consoleContent.innerHTML = '';
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
    
    // 运行测试 - 直接调用 IPC handler
    runCurrentTest: async () => {
        try {
            const currentTab = window.AppGlobals.currentTab;
            if (!currentTab || !currentTab.path) {
                window.NotificationModule.showNotification('请先打开一个脚本文件', 'warning');
                return;
            }

            const scriptPath = currentTab.path;
            const deviceId = window.AppGlobals.getCurrentDeviceId();
            const projectPath = window.AppGlobals.getCurrentProjectPath();

            if (!deviceId) {
                window.NotificationModule.showNotification('请先选择一个设备', 'warning');
                return;
            }

            window.rLog(`🚀 开始运行脚本: ${scriptPath}`);
            window.NotificationModule.showNotification('开始执行脚本...', 'info');

            // 调用 IPC handler 执行脚本
            const { ipcRenderer } = require('electron');
            const result = await ipcRenderer.invoke('tke-run-script', deviceId, projectPath, scriptPath);

            if (result.success) {
                window.rLog('✅ 脚本执行完成');
                window.NotificationModule.showNotification('脚本执行完成', 'success');

                // 在控制台输出结果
                if (window.TestcaseController.ConsoleManager) {
                    window.TestcaseController.ConsoleManager.addLog(result.output, 'info');
                }
            } else {
                window.rError('❌ 脚本执行失败:', result.error);
                window.NotificationModule.showNotification(`脚本执行失败: ${result.error}`, 'error');

                // 在控制台输出错误
                if (window.TestcaseController.ConsoleManager) {
                    window.TestcaseController.ConsoleManager.addLog(result.error, 'error');
                    if (result.output) {
                        window.TestcaseController.ConsoleManager.addLog(result.output, 'error');
                    }
                }
            }

            return result;
        } catch (error) {
            window.rError('❌ 运行测试时发生错误:', error);
            window.NotificationModule.showNotification(`运行测试失败: ${error.message}`, 'error');
            return { success: false, error: error.message };
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