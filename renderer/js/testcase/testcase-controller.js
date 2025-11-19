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
    const captureScreenshotBtn = document.getElementById('captureScreenshotBtn');
    const syncClipboardBtn = document.getElementById('syncClipboardBtn');
    const toggleVideoStreamBtn = document.getElementById('toggleVideoStreamBtn');
    const deviceSelect = document.getElementById('deviceSelect');

    window.rLog('初始化测试用例页面');

    // 绑定 Run Test 按钮
    if (runTestBtn) {
        runTestBtn.addEventListener('click', async () => {
            if (window.ScriptRunner) {
                if (window.ScriptRunner.isRunning) {
                    // 当前正在运行，点击停止
                    window.rLog('Stop Test 按钮点击');
                    window.ScriptRunner.stop();
                } else {
                    // 当前空闲，点击运行
                    window.rLog('Run Test 按钮点击');
                    await window.ScriptRunner.runCurrentScript();
                }
            } else {
                window.rError('ScriptRunner 模块未加载');
            }
        });
    }

    // clearConsoleBtn事件在下面的 initializeUIElementsPanel 中处理
    
    // 绑定 XML 覆盖层切换按钮
    if (toggleXmlBtn) {
        toggleXmlBtn.addEventListener('click', () => {
            // 切换到 XML 模式
            if (window.ScreenCoordinator) {
                const currentMode = window.ScreenCoordinator.getCurrentMode();
                if (currentMode === 'xml') {
                    window.ScreenCoordinator.switchTo('normal');
                } else {
                    window.ScreenCoordinator.switchTo('xml');
                }
            }
        });
    }
    
    // 绑定刷新设备屏幕按钮
    if (refreshDeviceBtn) {
        refreshDeviceBtn.addEventListener('click', () => {
            if (window.ScreenCoordinator && window.ScreenCoordinator.refreshDeviceScreen) {
                window.ScreenCoordinator.refreshDeviceScreen();
            }
        });
    }

    // 绑定截图到剪贴板按钮
    if (captureScreenshotBtn) {
        captureScreenshotBtn.addEventListener('click', async () => {
            try {
                const scrcpyVideoCanvas = document.getElementById('scrcpyVideoCanvas');
                const deviceScreenshot = document.getElementById('deviceScreenshot');

                let imageData = null;

                // 如果视频流激活，从视频流截图
                if (window.ScrcpyVideoStream && window.ScrcpyVideoStream.isStreamActive()) {
                    window.rLog('📸 从视频流捕获截图...');
                    // 直接从canvas获取图片
                    if (scrcpyVideoCanvas) {
                        imageData = scrcpyVideoCanvas.toDataURL('image/png');
                    }
                }
                // 否则使用当前显示的截图
                else if (deviceScreenshot && deviceScreenshot.src && deviceScreenshot.style.display !== 'none') {
                    window.rLog('📸 使用当前截图...');
                    // 将图片转换为canvas再转为dataURL
                    const canvas = document.createElement('canvas');
                    canvas.width = deviceScreenshot.naturalWidth;
                    canvas.height = deviceScreenshot.naturalHeight;
                    const ctx = canvas.getContext('2d');
                    ctx.drawImage(deviceScreenshot, 0, 0);
                    imageData = canvas.toDataURL('image/png');
                } else {
                    window.AppNotifications?.warn('请先开启设备投屏');
                    return;
                }

                // 将 dataURL 转换为 Blob
                const response = await fetch(imageData);
                const blob = await response.blob();

                // 复制到剪贴板
                await navigator.clipboard.write([
                    new ClipboardItem({
                        'image/png': blob
                    })
                ]);

                window.rLog('✅ 截图已复制到剪贴板');
                window.AppNotifications?.success('截图已复制到剪贴板');

            } catch (error) {
                window.rError('复制截图到剪贴板失败:', error);
                window.AppNotifications?.error(`复制失败: ${error.message}`);
            }
        });
    }

    // 绑定剪切板同步按钮
    if (syncClipboardBtn) {
        syncClipboardBtn.addEventListener('click', async () => {
            try {
                // 检查视频流是否激活
                if (!window.ScrcpyVideoStream || !window.ScrcpyVideoStream.isStreamActive()) {
                    window.rWarn('⚠️ 请先开启视频流投影');
                    window.AppNotifications?.warning('请先开启视频流投影');
                    return;
                }

                // 检查 WebSocket 连接
                const ws = window.ScrcpyVideoStream.streamReceiver;
                if (!ws || ws.readyState !== WebSocket.OPEN) {
                    window.rError('❌ WebSocket 未连接');
                    window.AppNotifications?.error('WebSocket 未连接');
                    return;
                }

                window.rLog('📋 请求获取设备剪切板...');
                window.AppNotifications?.info('正在获取设备剪切板...');

                // 设置剪切板消息监听器
                const handleClipboardMessage = async (event) => {
                    if (!(event.data instanceof ArrayBuffer)) {
                        return;
                    }

                    const data = new Uint8Array(event.data);

                    // 🔍 调试：显示收到的消息前20字节
                    const preview = Array.from(data.slice(0, Math.min(20, data.length)))
                        .map(b => b.toString(16).padStart(2, '0'))
                        .join(' ');
                    window.rLog(`🔍 [剪切板监听器] 收到消息 ${data.length} 字节，前20字节: ${preview}`);

                    // 检查是否是剪切板消息 (magic bytes: 'scrcpy_message')
                    const MAGIC_BYTES = new Uint8Array([115, 99, 114, 99, 112, 121, 95, 109, 101, 115, 115, 97, 103, 101]);

                    if (data.length >= MAGIC_BYTES.length) {
                        let isMessage = true;
                        for (let i = 0; i < MAGIC_BYTES.length; i++) {
                            if (data[i] !== MAGIC_BYTES[i]) {
                                isMessage = false;
                                break;
                            }
                        }

                        if (isMessage) {
                            // 解析消息类型 (1字节，在 magic bytes 之后)
                            const messageType = data[MAGIC_BYTES.length];

                            // TYPE_CLIPBOARD = 0
                            if (messageType === 0) {
                                window.rLog('📩 收到剪切板消息');

                                // 解析文本长度 (4字节 Big Endian)
                                const view = new DataView(data.buffer, data.byteOffset);
                                const textLength = view.getInt32(MAGIC_BYTES.length + 1, false); // false = Big Endian

                                // 解析文本内容
                                const textBytes = data.slice(MAGIC_BYTES.length + 5, MAGIC_BYTES.length + 5 + textLength);
                                const clipboardText = new TextDecoder().decode(textBytes);

                                window.rLog(`📋 设备剪切板内容: ${clipboardText}`);

                                // 写入电脑剪切板
                                try {
                                    await navigator.clipboard.writeText(clipboardText);
                                    window.rLog('✅ 已同步到电脑剪切板');
                                    window.AppNotifications?.success(`已同步剪切板: ${clipboardText.substring(0, 50)}${clipboardText.length > 50 ? '...' : ''}`);
                                } catch (clipboardError) {
                                    window.rError('写入电脑剪切板失败:', clipboardError);
                                    window.AppNotifications?.error('写入电脑剪切板失败');
                                }

                                // 移除监听器
                                ws.removeEventListener('message', handleClipboardMessage);
                            }
                        }
                    }
                };

                // 添加临时消息监听器
                ws.addEventListener('message', handleClipboardMessage);

                // 3秒后自动移除监听器（超时保护）
                setTimeout(() => {
                    ws.removeEventListener('message', handleClipboardMessage);
                }, 3000);

                // 发送获取剪切板命令
                const ControlMessage = window.ScrcpyControlMessage;
                if (!ControlMessage) {
                    throw new Error('ControlMessage 未加载');
                }

                const getClipboardMsg = ControlMessage.createGetClipboardCommand();

                // 🔍 调试：显示发送的命令内容
                const cmdHex = Array.from(getClipboardMsg)
                    .map(b => b.toString(16).padStart(2, '0'))
                    .join(' ');
                window.rLog(`🔍 [发送命令] GetClipboard: ${getClipboardMsg.length} 字节 = ${cmdHex}`);
                window.rLog(`🔍 [发送命令] TYPE_GET_CLIPBOARD = ${ControlMessage.TYPE_GET_CLIPBOARD}`);

                ws.send(getClipboardMsg);

                window.rLog('📤 已发送获取剪切板命令');

            } catch (error) {
                window.rError('❌ 同步剪切板失败:', error);
                window.AppNotifications?.error(`同步剪切板失败: ${error.message}`);
            }
        });
    }

    // 绑定视频流控制按钮
    if (toggleVideoStreamBtn) {
        toggleVideoStreamBtn.addEventListener('click', async () => {
            const currentState = toggleVideoStreamBtn.getAttribute('data-state');

            if (currentState === 'stopped') {
                // 开始视频流
                const deviceId = deviceSelect?.value;
                if (!deviceId) {
                    window.rError('请先选择设备');
                    window.AppNotifications?.deviceRequired();
                    return;
                }

                if (window.VideoStreamStateManager) {
                    toggleVideoStreamBtn.disabled = true; // 禁用按钮防止重复点击

                    const success = await window.VideoStreamStateManager.startVideoStream(deviceId);

                    if (success) {
                        window.rLog('✅ 视频流已启动（通过按钮）');
                    } else {
                        window.rError('❌ 视频流启动失败');
                    }

                    toggleVideoStreamBtn.disabled = false;
                } else {
                    window.rError('视频流状态管理器未加载');
                }
            } else {
                // 停止视频流
                if (window.VideoStreamStateManager) {
                    toggleVideoStreamBtn.disabled = true; // 禁用按钮防止重复点击

                    await window.VideoStreamStateManager.stopVideoStream();
                    window.rLog('✅ 视频流已停止（通过按钮）');

                    toggleVideoStreamBtn.disabled = false;
                }
            }
        });
    }
    
    // 初始化屏幕协调器
    setTimeout(() => {
        window.rLog('延迟初始化 ScreenCoordinator');
        if (window.initializeScreenCoordinator) {
            window.initializeScreenCoordinator();
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

    toggleCaseFolder: (caseContainer, casePath, autoOpenFirst) => {
        if (window.TestcaseExplorerModule) {
            return window.TestcaseExplorerModule.toggleCaseFolder(caseContainer, casePath, autoOpenFirst);
        }
    },

    // 设备屏幕相关功能 - 委托给 ScreenCoordinator
    refreshDeviceScreen: async () => {
        if (window.ScreenCoordinator && window.ScreenCoordinator.refreshDeviceScreen) {
            return await window.ScreenCoordinator.refreshDeviceScreen();
        }
    },

    toggleXmlOverlay: () => {
        if (window.ScreenCoordinator) {
            const currentMode = window.ScreenCoordinator.getCurrentMode();
            if (currentMode === 'xml') {
                window.ScreenCoordinator.switchTo('normal');
            } else {
                window.ScreenCoordinator.switchTo('xml');
            }
        }
    },

    enableXmlOverlay: async (deviceId) => {
        if (window.ScreenCoordinator) {
            return await window.ScreenCoordinator.switchTo('xml');
        }
    },

    displayUIElementList: (elements) => {
        if (window.UIExtractor && window.UIExtractor.displayUIElementList) {
            return window.UIExtractor.displayUIElementList(elements);
        }
    },

    // 屏幕模式管理器代理（用于兼容性）
    ScreenModeManager: {
        setTestRunning: (running) => {
            if (window.ScreenCoordinator) {
                window.ScreenCoordinator.setTestRunning(running);
            }
        },
        updateZoomControlsVisibility: () => {
            if (window.ScreenCoordinator && window.ScreenCoordinator._updateZoomControlsVisibility) {
                window.ScreenCoordinator._updateZoomControlsVisibility();
            }
        },
        // 代理其他可能用到的方法
        init: () => {
            if (window.ScreenCoordinator) {
                window.ScreenCoordinator.init();
            }
        },
        setMode: (mode) => {
            if (window.ScreenCoordinator) {
                window.ScreenCoordinator.switchTo(mode);
            }
        }
    }
};
