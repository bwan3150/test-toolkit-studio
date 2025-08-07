// 测试用例管理模块

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
    
    if (runTestBtn) runTestBtn.addEventListener('click', runCurrentTest);
    if (clearConsoleBtn) clearConsoleBtn.addEventListener('click', window.NotificationModule.clearConsole);
    if (toggleXmlBtn) toggleXmlBtn.addEventListener('click', toggleXmlOverlay);
    if (refreshDeviceBtn) refreshDeviceBtn.addEventListener('click', () => {
        // 点击按钮时手动刷新
        refreshDeviceScreen();
    });
    
    if (deviceSelect) {
        deviceSelect.addEventListener('change', (e) => {
            // 存储选中的设备
            if (e.target.value) {
                ipcRenderer.invoke('store-set', 'selected_device', e.target.value);
            }
            // 不再启动自动刷新
        });
    }
    
    // 加载设备
    window.DeviceManagerModule.refreshDeviceList();
}

// 加载文件树
async function loadFileTree() {
    const { path, fs } = getGlobals();
    if (!window.AppGlobals.currentProject) return;
    
    const fileTree = document.getElementById('fileTree');
    fileTree.innerHTML = '';
    
    const casesPath = path.join(window.AppGlobals.currentProject, 'cases');
    
    try {
        const cases = await fs.readdir(casesPath);
        
        for (const caseName of cases) {
            const casePath = path.join(casesPath, caseName);
            const stat = await fs.stat(casePath);
            
            if (stat.isDirectory()) {
                const caseItem = createTreeItem(caseName, 'folder', casePath);
                fileTree.appendChild(caseItem);
                
                // 加载脚本
                const scriptPath = path.join(casePath, 'script');
                try {
                    const scripts = await fs.readdir(scriptPath);
                    const scriptContainer = document.createElement('div');
                    scriptContainer.className = 'tree-children';
                    
                    for (const script of scripts) {
                        if (script.endsWith('.yaml')) {
                            const scriptItem = createTreeItem(
                                script,
                                'file',
                                path.join(scriptPath, script)
                            );
                            scriptContainer.appendChild(scriptItem);
                        }
                    }
                    
                    caseItem.appendChild(scriptContainer);
                } catch (error) {
                    console.error('Failed to load scripts:', error);
                }
            }
        }
    } catch (error) {
        console.error('Failed to load file tree:', error);
    }
}

// 创建树项
function createTreeItem(name, type, fullPath) {
    const item = document.createElement('div');
    item.className = `tree-item ${type === 'folder' ? 'tree-folder' : ''}`;
    
    const icon = document.createElement('svg');
    icon.className = 'tree-icon';
    icon.setAttribute('viewBox', '0 0 24 24');
    
    if (type === 'folder') {
        icon.innerHTML = '<path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
    } else {
        icon.innerHTML = '<path d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm-1 7V3.5L18.5 9H13z"/>';
    }
    
    const label = document.createElement('span');
    label.textContent = name;
    
    item.appendChild(icon);
    item.appendChild(label);
    
    if (type === 'file') {
        item.addEventListener('click', () => openFile(fullPath));
    }
    
    return item;
}

// 在编辑器中打开文件
async function openFile(filePath) {
    const { ipcRenderer, path } = getGlobals();
    const result = await ipcRenderer.invoke('read-file', filePath);
    if (result.success) {
        const fileName = path.basename(filePath);
        
        // 检查是否已经打开
        const existingTab = window.AppGlobals.openTabs.find(tab => tab.path === filePath);
        if (existingTab) {
            window.EditorModule.selectTab(existingTab.id);
            return;
        }
        
        // 创建新标签
        const tabId = `tab-${Date.now()}`;
        const tab = {
            id: tabId,
            path: filePath,
            name: fileName,
            content: result.content
        };
        
        window.AppGlobals.openTabs.push(tab);
        window.EditorModule.createTab(tab);
        window.EditorModule.selectTab(tabId);
        
        console.log('Opening file:', fileName, 'Content length:', result.content.length);
    } else {
        window.NotificationModule.showNotification(`Failed to open file: ${result.error}`, 'error');
    }
}

// 运行当前测试
async function runCurrentTest() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) {
        window.NotificationModule.showNotification('No test script open', 'warning');
        return;
    }
    
    const deviceSelect = document.getElementById('deviceSelect');
    if (!deviceSelect.value) {
        window.NotificationModule.showNotification('Please select a device', 'warning');
        return;
    }
    
    window.NotificationModule.addConsoleLog('Running test...', 'info');
    
    // TODO: 实现实际的测试执行
    setTimeout(() => {
        window.NotificationModule.addConsoleLog('Test execution not yet implemented', 'warning');
    }, 1000);
}

// 设备屏幕管理
async function refreshDeviceScreen() {
    const { ipcRenderer } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    if (!deviceSelect || !deviceSelect.value) {
        window.NotificationModule.showNotification('Please select a device', 'warning');
        return;
    }
    
    const result = await ipcRenderer.invoke('adb-screenshot', deviceSelect.value);
    if (result.success) {
        const img = document.getElementById('deviceScreenshot');
        img.src = `data:image/png;base64,${result.data}`;
        img.style.display = 'block';
        document.querySelector('.screen-placeholder').style.display = 'none';
    } else {
        window.NotificationModule.showNotification(`Failed to capture screen: ${result.error}`, 'error');
    }
}

function toggleXmlOverlay() {
    window.NotificationModule.showNotification('XML overlay not yet implemented', 'info');
}

// 导出函数
window.TestcaseManagerModule = {
    initializeTestcasePage,
    loadFileTree,
    createTreeItem,
    openFile,
    runCurrentTest,
    refreshDeviceScreen,
    toggleXmlOverlay
};