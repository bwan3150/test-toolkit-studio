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
    
    console.log('初始化测试用例页面');
    
    // Run Test按钮事件处理已移至tks-integration.js模块
    // if (runTestBtn) runTestBtn.addEventListener('click', runCurrentTest);
    // 注意：clearConsoleBtn事件已在initializeUIElementsPanel中处理
    if (toggleXmlBtn) toggleXmlBtn.addEventListener('click', toggleXmlOverlay);
    if (refreshDeviceBtn) refreshDeviceBtn.addEventListener('click', () => {
        // 点击按钮时手动刷新
        refreshDeviceScreen();
    });
    
    // 初始化屏幕模式管理器
    setTimeout(() => {
        console.log('延迟初始化 ScreenModeManager');
        initializeScreenModeManager();
    }, 100);
    
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
                const caseItem = createCollapsibleCaseItem(caseName, casePath);
                fileTree.appendChild(caseItem);
                
                // 加载脚本
                await loadCaseScripts(caseItem, casePath);
            }
        }
    } catch (error) {
        console.error('Failed to load file tree:', error);
    }
}

// 存储展开状态
let expandedCases = new Set();

// 创建可折叠的case项目
function createCollapsibleCaseItem(caseName, casePath) {
    const caseContainer = document.createElement('div');
    caseContainer.className = 'case-container';
    caseContainer.dataset.casePath = casePath;
    caseContainer.dataset.caseName = caseName;
    
    const caseHeader = document.createElement('div');
    caseHeader.className = 'case-header';
    
    // Case文件夹图标 - 直接点击切换展开/折叠
    const caseIconContainer = document.createElement('div');
    caseIconContainer.className = 'icon-container case-icon-wrapper';
    const caseIcon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    caseIcon.className = 'case-icon';
    caseIcon.setAttribute('viewBox', '0 0 24 24');
    caseIcon.innerHTML = '<path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
    caseIconContainer.appendChild(caseIcon);
    
    // Case名称
    const caseLabelSpan = document.createElement('span');
    caseLabelSpan.className = 'case-label';
    caseLabelSpan.textContent = caseName;
    
    // 脚本容器
    const scriptsContainer = document.createElement('div');
    scriptsContainer.className = 'scripts-container';
    
    // 设置初始展开状态
    const isExpanded = expandedCases.has(casePath);
    if (isExpanded) {
        scriptsContainer.classList.remove('collapsed');
        // 展开状态使用打开的文件夹图标
        caseIcon.innerHTML = '<path d="M19,20H4C2.89,20 2,19.1 2,18V6C2,4.89 2.89,4 4,4H10L12,6H19A2,2 0 0,1 21,8H21L4,8V18L6.14,10H23.21L20.93,18.5C20.7,19.37 19.92,20 19,20Z"/>';
        // 异步加载脚本文件
        loadCaseScripts(caseContainer, casePath);
    } else {
        scriptsContainer.classList.add('collapsed');
        // 折叠状态使用关闭的文件夹图标 (已在创建时设置)
    }
    
    caseHeader.appendChild(caseIconContainer);
    caseHeader.appendChild(caseLabelSpan);
    
    caseContainer.appendChild(caseHeader);
    caseContainer.appendChild(scriptsContainer);
    
    // 用于处理单击和双击冲突的变量
    let clickTimeout = null;
    
    // 点击切换折叠/展开（整行可点击）
    caseHeader.addEventListener('click', (e) => {
        e.stopPropagation();
        
        // 清除之前的单击延迟
        if (clickTimeout) {
            clearTimeout(clickTimeout);
        }
        
        // 延迟执行单击操作，如果是双击则会被清除
        clickTimeout = setTimeout(() => {
            toggleCaseFolder(caseContainer);
            clickTimeout = null;
        }, 200);
    });
    
    // 双击重命名Case（只在文字区域生效）
    caseLabelSpan.addEventListener('dblclick', (e) => {
        e.stopPropagation();
        
        // 清除单击延迟，防止展开操作
        if (clickTimeout) {
            clearTimeout(clickTimeout);
            clickTimeout = null;
        }
        
        startInlineEdit(caseLabelSpan, 'case', casePath, caseName);
    });
    
    // 添加右键菜单
    caseContainer.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        showContextMenu(e, 'case', casePath, caseName);
    });
    
    return caseContainer;
}

// 加载case的脚本文件
async function loadCaseScripts(caseItem, casePath) {
    const { path, fs } = getGlobals();
    const scriptsContainer = caseItem.querySelector('.scripts-container');
    
    if (!scriptsContainer) {
        console.error('loadCaseScripts: scripts-container not found', caseItem);
        return;
    }
    
    // 清空之前加载的脚本，避免重复
    scriptsContainer.innerHTML = '';
    
    const scriptPath = path.join(casePath, 'script');
    try {
        const scripts = await fs.readdir(scriptPath);
        console.log(`加载case脚本: ${casePath}, 找到${scripts.length}个文件`);
        
        for (const script of scripts) {
            if (script.endsWith('.tks') || script.endsWith('.yaml')) {
                const scriptItem = createScriptItem(
                    script,
                    path.join(scriptPath, script),
                    casePath
                );
                scriptsContainer.appendChild(scriptItem);
            }
        }
    } catch (error) {
        console.error('Failed to load scripts for case:', casePath, error);
        // 如果script目录不存在，显示提示信息
        scriptsContainer.innerHTML = '<div style="padding: 8px; color: #888; font-size: 12px;">暂无脚本文件</div>';
    }
}

// 创建脚本项目
function createScriptItem(scriptName, scriptPath, casePath) {
    const scriptItem = document.createElement('div');
    scriptItem.className = 'script-item';
    scriptItem.dataset.scriptPath = scriptPath;
    scriptItem.dataset.casePath = casePath;
    scriptItem.dataset.scriptName = scriptName;
    
    // 脚本图标 - 使用固定容器
    const scriptContainer = document.createElement('div');
    scriptContainer.className = 'icon-container script-container';
    const scriptIcon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    scriptIcon.className = 'script-icon';
    scriptIcon.setAttribute('viewBox', '0 0 24 24');
    scriptIcon.innerHTML = '<path d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm-1 7V3.5L18.5 9H13z"/>';
    scriptContainer.appendChild(scriptIcon);
    
    // 脚本名称
    const scriptLabel = document.createElement('span');
    scriptLabel.className = 'script-label';
    scriptLabel.textContent = scriptName;
    
    scriptItem.appendChild(scriptContainer);
    scriptItem.appendChild(scriptLabel);
    
    // 点击打开文件
    scriptItem.addEventListener('click', (e) => {
        // 如果正在编辑，不打开文件
        if (scriptLabel.contentEditable === 'true') return;
        e.stopPropagation();
        openFile(scriptPath);
    });
    
    // 双击重命名脚本
    scriptLabel.addEventListener('dblclick', (e) => {
        e.stopPropagation();
        startInlineEdit(scriptLabel, 'script', scriptPath, scriptName, casePath);
    });
    
    // 右键菜单
    scriptItem.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        showContextMenu(e, 'script', scriptPath, scriptName, casePath);
    });
    
    return scriptItem;
}

// 切换case文件夹的折叠状态
function toggleCaseFolder(caseContainer) {
    const scriptsContainer = caseContainer.querySelector('.scripts-container');
    const caseIcon = caseContainer.querySelector('.case-icon');
    const casePath = caseContainer.dataset.casePath;
    
    if (!scriptsContainer) {
        console.error('toggleCaseFolder: 缺少scripts-container元素', caseContainer);
        return;
    }
    
    if (!caseIcon) {
        console.warn('toggleCaseFolder: 缺少case-icon元素，跳过图标更新', caseContainer);
    }
    
    if (scriptsContainer.classList.contains('collapsed')) {
        // 展开
        scriptsContainer.classList.remove('collapsed');
        expandedCases.add(casePath);
        // 切换到打开的文件夹图标（如果图标存在）
        if (caseIcon) {
            caseIcon.innerHTML = '<path d="M19,20H4C2.89,20 2,19.1 2,18V6C2,4.89 2.89,4 4,4H10L12,6H19A2,2 0 0,1 21,8H21L4,8V18L6.14,10H23.21L20.93,18.5C20.7,19.37 19.92,20 19,20Z"/>';
        }
        
        // 自动加载脚本文件
        const caseItem = caseContainer;
        loadCaseScripts(caseItem, casePath);
    } else {
        // 折叠
        scriptsContainer.classList.add('collapsed');
        expandedCases.delete(casePath);
        // 切换到关闭的文件夹图标（如果图标存在）
        if (caseIcon) {
            caseIcon.innerHTML = '<path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
        }
    }
}

// 创建树项
function createTreeItem(name, type, fullPath) {
    const item = document.createElement('div');
    item.className = `tree-item ${type === 'folder' ? 'tree-folder' : ''}`;
    
    const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    icon.className = 'tree-icon';
    icon.setAttribute('viewBox', '0 0 24 24');
    icon.setAttribute('width', '16');
    icon.setAttribute('height', '16');
    icon.style.width = '16px';
    icon.style.height = '16px';
    
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

// 内联编辑功能
function startInlineEdit(labelElement, type, itemPath, itemName, casePath = null) {
    if (labelElement.contentEditable === 'true') return; // 已在编辑状态
    
    const originalText = labelElement.textContent;
    let editableText = originalText;
    let fileExtension = '';
    
    // 如果是脚本文件，分离文件名和扩展名
    if (type === 'script') {
        const lastDotIndex = originalText.lastIndexOf('.');
        if (lastDotIndex > 0) {
            editableText = originalText.substring(0, lastDotIndex);
            fileExtension = originalText.substring(lastDotIndex);
        }
    }
    
    labelElement.contentEditable = 'true';
    labelElement.classList.add('editing');
    
    // 对于脚本文件，只显示可编辑的名称部分
    if (type === 'script' && fileExtension) {
        labelElement.innerHTML = `<span class="editable-name">${editableText}</span><span class="file-extension">${fileExtension}</span>`;
        
        // 选中可编辑部分
        const editableSpan = labelElement.querySelector('.editable-name');
        editableSpan.focus();
        const selection = window.getSelection();
        const range = document.createRange();
        range.selectNodeContents(editableSpan);
        selection.removeAllRanges();
        selection.addRange(range);
    } else {
        labelElement.textContent = editableText;
        labelElement.focus();
        
        // 选中所有文本
        const selection = window.getSelection();
        const range = document.createRange();
        range.selectNodeContents(labelElement);
        selection.removeAllRanges();
        selection.addRange(range);
    }
    
    const finishEdit = async (save = true) => {
        labelElement.contentEditable = 'false';
        labelElement.classList.remove('editing');
        
        if (save) {
            let newName;
            if (type === 'script' && fileExtension) {
                const editableSpan = labelElement.querySelector('.editable-name');
                const nameOnly = editableSpan ? editableSpan.textContent.trim() : editableText;
                newName = nameOnly + fileExtension;
            } else {
                newName = labelElement.textContent.trim();
            }
            
            if (newName && newName !== originalText) {
                if (type === 'case') {
                    await performCaseRename(itemPath, originalText, newName);
                } else if (type === 'script') {
                    await performScriptRename(itemPath, originalText, newName, casePath);
                }
                return;
            }
        }
        
        // 恢复原文本
        labelElement.innerHTML = '';
        labelElement.textContent = originalText;
    };
    
    // 处理键盘事件
    const handleKeydown = (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            finishEdit(true);
        } else if (e.key === 'Escape') {
            e.preventDefault();
            finishEdit(false);
        }
    };
    
    // 处理失焦事件
    const handleBlur = () => {
        finishEdit(true);
    };
    
    labelElement.addEventListener('keydown', handleKeydown);
    labelElement.addEventListener('blur', handleBlur);
    
    // 清理函数
    const cleanup = () => {
        labelElement.removeEventListener('keydown', handleKeydown);
        labelElement.removeEventListener('blur', handleBlur);
    };
    
    // 在编辑完成后清理事件监听器
    setTimeout(() => {
        const observer = new MutationObserver((mutations) => {
            if (labelElement.contentEditable === 'false') {
                cleanup();
                observer.disconnect();
            }
        });
        observer.observe(labelElement, { attributes: true });
    }, 0);
}

// 自定义prompt函数
function customPrompt(title, label, defaultValue = '') {
    return new Promise((resolve) => {
        const dialog = document.getElementById('inputDialog');
        const titleEl = document.getElementById('inputDialogTitle');
        const labelEl = document.getElementById('inputDialogLabel');
        const inputEl = document.getElementById('inputDialogInput');
        const confirmBtn = document.getElementById('confirmInputDialog');
        const cancelBtn = document.getElementById('cancelInputDialog');
        const closeBtn = document.getElementById('closeInputDialog');
        
        titleEl.textContent = title;
        labelEl.textContent = label;
        inputEl.value = defaultValue;
        
        dialog.style.display = 'flex';
        inputEl.focus();
        inputEl.select();
        
        const cleanup = () => {
            dialog.style.display = 'none';
            confirmBtn.removeEventListener('click', handleConfirm);
            cancelBtn.removeEventListener('click', handleCancel);
            closeBtn.removeEventListener('click', handleCancel);
            inputEl.removeEventListener('keydown', handleKeydown);
        };
        
        const handleConfirm = () => {
            const value = inputEl.value.trim();
            cleanup();
            resolve(value || null);
        };
        
        const handleCancel = () => {
            cleanup();
            resolve(null);
        };
        
        const handleKeydown = (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                handleConfirm();
            } else if (e.key === 'Escape') {
                e.preventDefault();
                handleCancel();
            }
        };
        
        confirmBtn.addEventListener('click', handleConfirm);
        cancelBtn.addEventListener('click', handleCancel);
        closeBtn.addEventListener('click', handleCancel);
        inputEl.addEventListener('keydown', handleKeydown);
    });
}

// 显示右键菜单
function showContextMenu(event, type, itemPath, itemName, casePath = null) {
    // 移除现有菜单
    const existingMenu = document.querySelector('.context-menu');
    if (existingMenu) {
        existingMenu.remove();
    }
    
    const menu = document.createElement('div');
    menu.className = 'context-menu';
    menu.style.position = 'fixed';
    menu.style.left = `${event.clientX}px`;
    menu.style.top = `${event.clientY}px`;
    menu.style.zIndex = '10000';
    
    let menuItems = [];
    
    if (type === 'case') {
        menuItems = [
            { text: '新建脚本', action: () => createNewScript(itemPath, itemName) },
            { text: '重命名', action: () => {
                const caseContainer = document.querySelector(`[data-case-path="${itemPath}"]`);
                const labelElement = caseContainer?.querySelector('.case-label');
                if (labelElement) startInlineEdit(labelElement, 'case', itemPath, itemName);
            }},
            { text: '删除Case', action: () => deleteCase(itemPath, itemName) }
        ];
    } else if (type === 'script') {
        menuItems = [
            { text: '重命名', action: () => {
                const scriptItem = document.querySelector(`[data-script-path="${itemPath}"]`);
                const labelElement = scriptItem?.querySelector('.script-label');
                if (labelElement) startInlineEdit(labelElement, 'script', itemPath, itemName, casePath);
            }},
            { text: '复制脚本', action: () => duplicateScriptSimple(itemPath, itemName, casePath) },
            { text: '删除脚本', action: () => deleteScript(itemPath, itemName, casePath) }
        ];
    }
    
    menuItems.forEach(item => {
        const menuItem = document.createElement('div');
        menuItem.className = 'context-menu-item';
        menuItem.textContent = item.text;
        menuItem.addEventListener('click', () => {
            item.action();
            menu.remove();
        });
        menu.appendChild(menuItem);
    });
    
    document.body.appendChild(menu);
    
    // 点击其他地方关闭菜单
    const closeMenu = (e) => {
        if (!menu.contains(e.target)) {
            menu.remove();
            document.removeEventListener('click', closeMenu);
        }
    };
    setTimeout(() => document.addEventListener('click', closeMenu), 0);
}

// 创建新脚本
async function createNewScript(casePath, caseName) {
    const scriptName = await customPrompt('新建脚本', '请输入脚本名称:', `script_${String(Date.now()).slice(-3)}.tks`);
    if (!scriptName) return;
    
    if (!scriptName.endsWith('.tks')) {
        window.NotificationModule.showNotification('脚本文件必须以.tks结尾', 'warning');
        return;
    }
    
    const { path, fs } = getGlobals();
    const scriptPath = path.join(casePath, 'script', scriptName);
    
    try {
        // 检查文件是否已存在
        try {
            await fs.access(scriptPath);
            window.NotificationModule.showNotification('脚本文件已存在', 'warning');
            return;
        } catch {
            // 文件不存在，可以创建
        }
        
        // 创建默认脚本内容
        const defaultContent = `用例: ${caseName}
脚本名: ${scriptName.replace('.tks', '')}
详情: 
    appPackage: com.example.app
    appActivity: .MainActivity
步骤:
    点击 [190,220]
    等待 2s
    断言 [示例元素, 存在]
`;
        
        await fs.writeFile(scriptPath, defaultContent);
        window.NotificationModule.showNotification(`已创建脚本: ${scriptName}`, 'success');
        
        // 重新加载文件树
        await loadFileTree();
        
        // 自动打开新创建的文件
        openFile(scriptPath);
        
    } catch (error) {
        window.NotificationModule.showNotification(`创建脚本失败: ${error.message}`, 'error');
    }
}

// 重命名脚本
async function renameScript(scriptPath, oldName, casePath) {
    const newName = await customPrompt('重命名脚本', '请输入新的脚本名称:', oldName);
    if (!newName || newName === oldName) return;
    
    if (!newName.endsWith('.tks') && !newName.endsWith('.yaml')) {
        window.NotificationModule.showNotification('脚本文件必须以.tks或.yaml结尾', 'warning');
        return;
    }
    
    const { path, fs } = getGlobals();
    const newPath = path.join(path.dirname(scriptPath), newName);
    
    try {
        await fs.rename(scriptPath, newPath);
        window.NotificationModule.showNotification(`脚本重命名成功: ${oldName} -> ${newName}`, 'success');
        
        // 更新打开的标签页
        updateTabAfterRename(scriptPath, newPath, newName);
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`重命名失败: ${error.message}`, 'error');
    }
}

// 简单复制脚本（直接添加_copy后缀）
async function duplicateScriptSimple(scriptPath, scriptName, casePath) {
    const baseName = scriptName.replace(/\.(tks|yaml)$/, '');
    const extension = scriptName.match(/\.(tks|yaml)$/)[0];
    const newName = `${baseName}_copy${extension}`;

    const { path, fs } = getGlobals();
    const newPath = path.join(path.dirname(scriptPath), newName);
    
    try {
        // 检查目标文件是否已存在，如果存在则添加数字后缀
        let finalName = newName;
        let finalPath = newPath;
        let counter = 1;
        
        while (true) {
            try {
                await fs.access(finalPath);
                // 文件存在，尝试下一个名称
                finalName = `${baseName}_copy${counter}${extension}`;
                finalPath = path.join(path.dirname(scriptPath), finalName);
                counter++;
            } catch {
                // 文件不存在，可以使用这个名称
                break;
            }
        }
        
        // 读取原文件内容并写入新文件
        const content = await fs.readFile(scriptPath, 'utf8');
        await fs.writeFile(finalPath, content);
        
        window.NotificationModule.showNotification(`脚本复制成功: ${finalName}`, 'success');
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`复制失败: ${error.message}`, 'error');
    }
}

// 复制脚本
async function duplicateScript(scriptPath, scriptName, casePath) {
    const baseName = scriptName.replace(/\.(tks|yaml)$/, '');
    const extension = scriptName.match(/\.(tks|yaml)$/)[0];
    const newName = await customPrompt('复制脚本', '请输入复制后的脚本名称:', `${baseName}_copy${extension}`);
    if (!newName) return;
    
    const { path, fs } = getGlobals();
    const newPath = path.join(path.dirname(scriptPath), newName);
    
    try {
        // 检查目标文件是否已存在
        try {
            await fs.access(newPath);
            window.NotificationModule.showNotification('目标文件已存在', 'warning');
            return;
        } catch {
            // 文件不存在，可以复制
        }
        
        // 读取原文件内容并写入新文件
        const content = await fs.readFile(scriptPath, 'utf8');
        await fs.writeFile(newPath, content);
        
        window.NotificationModule.showNotification(`脚本复制成功: ${newName}`, 'success');
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`复制失败: ${error.message}`, 'error');
    }
}

// 删除脚本
async function deleteScript(scriptPath, scriptName, casePath) {
    if (!confirm(`确定要删除脚本 "${scriptName}" 吗？此操作不可撤销。`)) return;
    
    const { fs } = getGlobals();
    
    try {
        await fs.unlink(scriptPath);
        window.NotificationModule.showNotification(`已删除脚本: ${scriptName}`, 'success');
        
        // 关闭相关标签页
        closeTabByPath(scriptPath);
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`删除失败: ${error.message}`, 'error');
    }
}

// 重命名Case
async function renameCase(casePath, oldName) {
    const newName = await customPrompt('重命名Case', '请输入新的Case名称:', oldName);
    if (!newName || newName === oldName) return;
    
    const { path, fs } = getGlobals();
    const parentPath = path.dirname(casePath);
    const newPath = path.join(parentPath, newName);
    
    try {
        await fs.rename(casePath, newPath);
        window.NotificationModule.showNotification(`Case重命名成功: ${oldName} -> ${newName}`, 'success');
        
        // 更新相关标签页
        updateTabsAfterCaseRename(casePath, newPath);
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`重命名失败: ${error.message}`, 'error');
    }
}

// 删除Case
async function deleteCase(casePath, caseName) {
    if (!confirm(`确定要删除Case "${caseName}" 吗？这将删除其中的所有脚本文件，此操作不可撤销。`)) return;
    
    const { fs } = getGlobals();
    
    try {
        await fs.rm(casePath, { recursive: true, force: true });
        window.NotificationModule.showNotification(`已删除Case: ${caseName}`, 'success');
        
        // 关闭相关标签页
        closeTabsByCasePath(casePath);
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`删除失败: ${error.message}`, 'error');
    }
}

// 辅助函数：更新标签页路径
function updateTabAfterRename(oldPath, newPath, newName) {
    const tabs = window.AppGlobals.openTabs;
    const tab = tabs.find(t => t.path === oldPath);
    if (tab) {
        tab.path = newPath;
        tab.name = newName;
        
        // 更新标签页显示
        const tabElement = document.getElementById(tab.id);
        if (tabElement) {
            const nameSpan = tabElement.querySelector('.tab-name');
            if (nameSpan) {
                nameSpan.textContent = newName;
                nameSpan.title = newPath;
            }
        }
    }
}

// 辅助函数：关闭指定路径的标签页
function closeTabByPath(targetPath) {
    const tabs = window.AppGlobals.openTabs;
    const tab = tabs.find(t => t.path === targetPath);
    if (tab) {
        window.EditorModule.closeTab(tab.id);
    }
}

// 辅助函数：关闭Case相关的所有标签页
function closeTabsByCasePath(casePath) {
    const tabs = window.AppGlobals.openTabs.slice(); // 创建副本避免修改时的问题
    tabs.forEach(tab => {
        if (tab.path.includes(casePath)) {
            window.EditorModule.closeTab(tab.id);
        }
    });
}

// 辅助函数：更新Case重命名后的相关标签页
function updateTabsAfterCaseRename(oldCasePath, newCasePath) {
    const tabs = window.AppGlobals.openTabs;
    tabs.forEach(tab => {
        if (tab.path.includes(oldCasePath)) {
            tab.path = tab.path.replace(oldCasePath, newCasePath);
            
            // 更新标签页显示
            const tabElement = document.getElementById(tab.id);
            if (tabElement) {
                const nameSpan = tabElement.querySelector('.tab-name');
                if (nameSpan) {
                    nameSpan.title = tab.path;
                }
            }
        }
    });
}

// 执行Case重命名
async function performCaseRename(casePath, oldName, newName) {
    const { path, fs } = getGlobals();
    const parentPath = path.dirname(casePath);
    const newPath = path.join(parentPath, newName);
    
    try {
        await fs.rename(casePath, newPath);
        window.NotificationModule.showNotification(`Case重命名成功: ${oldName} -> ${newName}`, 'success');
        
        // 更新展开状态
        if (expandedCases.has(casePath)) {
            expandedCases.delete(casePath);
            expandedCases.add(newPath);
        }
        
        // 更新相关标签页
        updateTabsAfterCaseRename(casePath, newPath);
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`重命名失败: ${error.message}`, 'error');
        // 恢复原名称
        const caseContainer = document.querySelector(`[data-case-path="${casePath}"]`);
        const labelElement = caseContainer?.querySelector('.case-label');
        if (labelElement) {
            labelElement.textContent = oldName;
        }
    }
}

// 执行脚本重命名
async function performScriptRename(scriptPath, oldName, newName, casePath) {
    const { path, fs } = getGlobals();
    const newPath = path.join(path.dirname(scriptPath), newName);
    
    try {
        await fs.rename(scriptPath, newPath);
        window.NotificationModule.showNotification(`脚本重命名成功: ${oldName} -> ${newName}`, 'success');
        
        // 更新打开的标签页
        updateTabAfterRename(scriptPath, newPath, newName);
        
        // 重新加载文件树
        await loadFileTree();
        
    } catch (error) {
        window.NotificationModule.showNotification(`重命名失败: ${error.message}`, 'error');
        // 恢复原名称
        const scriptItem = document.querySelector(`[data-script-path="${scriptPath}"]`);
        const labelElement = scriptItem?.querySelector('.script-label');
        if (labelElement) {
            labelElement.innerHTML = '';
            labelElement.textContent = oldName;
        }
    }
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
    
    ConsoleManager.addLog('开始运行测试...', 'info');
    ConsoleManager.switchToConsole(); // 自动切换到控制台标签页
    
    // TODO: 实现实际的测试执行
    setTimeout(() => {
        ConsoleManager.addLog('测试执行功能尚未实现', 'warning');
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
    
    // 传递当前项目路径，以便自动保存截图和XML到工作区
    const projectPath = window.AppGlobals.currentProject;
    const result = await ipcRenderer.invoke('adb-screenshot', deviceSelect.value, projectPath);
    
    if (result.success) {
        const img = document.getElementById('deviceScreenshot');
        img.src = `data:image/png;base64,${result.data}`;
        img.style.display = 'block';
        document.querySelector('.screen-placeholder').style.display = 'none';
        
        // 显示缩放控制
        if (window.TestcaseManagerModule && window.TestcaseManagerModule.ScreenModeManager) {
            window.TestcaseManagerModule.ScreenModeManager.updateZoomControlsVisibility();
        }
        
        // 如果有项目路径，显示保存成功的提示
        if (projectPath) {
            console.log('已自动保存截图和UI树到工作区');
        }
    } else {
        window.NotificationModule.showNotification(`Failed to capture screen: ${result.error}`, 'error');
    }
}

// XML Overlay 相关变量
let xmlOverlayEnabled = false;
let currentUIElements = [];
let currentScreenSize = null;
let xmlParser = null;
let selectedElement = null;

function toggleXmlOverlay() {
    const { ipcRenderer } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    
    if (!deviceSelect?.value) {
        window.NotificationModule.showNotification('请先选择设备', 'warning');
        return;
    }
    
    xmlOverlayEnabled = !xmlOverlayEnabled;
    
    if (xmlOverlayEnabled) {
        enableXmlOverlay(deviceSelect.value);
    } else {
        disableXmlOverlay();
    }
}

async function enableXmlOverlay(deviceId) {
    try {
        window.NotificationModule.showNotification('正在加载UI树结构...', 'info');
        
        let result;
        const projectPath = window.AppGlobals.currentProject;
        
        // 1. 优先尝试从工作区读取UI树
        if (projectPath) {
            try {
                const { fs, path } = getGlobals();
                const xmlPath = path.join(projectPath, 'workarea', 'current_ui_tree.xml');
                const xmlContent = await fs.readFile(xmlPath, 'utf8');
                
                // 从工作区成功读取
                result = {
                    success: true,
                    xml: xmlContent,
                    screenSize: null, // 将从获取的XML中推断
                    source: 'workarea'
                };
                
                console.log('从工作区读取UI树成功');
            } catch (workareaError) {
                console.log('工作区UI树不存在或读取失败，将重新获取:', workareaError.message);
                result = null;
            }
        }
        
        // 2. 如果工作区没有，则重新获取
        if (!result) {
            result = await getGlobals().ipcRenderer.invoke('adb-ui-dump-enhanced', deviceId);
            if (!result.success) {
                throw new Error(result.error);
            }
            result.source = 'adb';
        }
        
        console.log('=== UI数据获取成功 ===');
        console.log('数据来源:', result.source);
        console.log('XML长度:', result.xml ? result.xml.length : 0);
        console.log('屏幕尺寸:', result.screenSize);
        console.log('XML内容前200字符:', result.xml ? result.xml.substring(0, 200) : 'XML为空');
        
        // 保存XML到sessionStorage用于调试
        if (result.xml) {
            sessionStorage.setItem('debug_xml', result.xml);
            console.log('XML已保存到 sessionStorage.debug_xml，可在控制台中查看完整内容');
        }
        
        // 2. 初始化XML解析器
        if (!xmlParser) {
            xmlParser = window.XMLParserModule.createParser();
        }
        
        // 设置屏幕尺寸
        let screenSize = result.screenSize;
        if (!screenSize && result.xml) {
            // 从XML推断屏幕尺寸
            screenSize = xmlParser.inferScreenSizeFromXML(result.xml);
        }
        
        if (screenSize) {
            xmlParser.setScreenSize(screenSize.width, screenSize.height);
            result.screenSize = screenSize; // 确保后续使用
        } else {
            // 使用默认尺寸
            screenSize = { width: 1080, height: 1920 };
            xmlParser.setScreenSize(screenSize.width, screenSize.height);
            result.screenSize = screenSize;
        }
        
        // 3. 解析XML并提取UI元素
        let optimizedTree = xmlParser.optimizeUITree(result.xml);
        if (!optimizedTree) {
            console.warn('UI树优化失败，尝试使用原始XML');
            // 备用方案：直接解析原始XML
            try {
                const parser = new DOMParser();
                const doc = parser.parseFromString(result.xml, 'text/xml');
                optimizedTree = doc.documentElement;
                if (!optimizedTree || optimizedTree.nodeName === 'parsererror') {
                    throw new Error('XML格式不正确');
                }
            } catch (parseError) {
                throw new Error(`XML解析完全失败: ${parseError.message}`);
            }
        }
        
        // 如果优化失败，传递原始XML作为回退
        currentUIElements = xmlParser.extractUIElements(optimizedTree, result.xml);
        console.log('提取的UI元素:', currentUIElements);
        
        // 暴露到全局，供TKS集成模块使用
        window.currentUIElements = currentUIElements;
        
        if (currentUIElements.length === 0) {
            console.warn('未提取到任何UI元素，可能需要检查XML格式或优化算法');
        }
        
        // 存储当前屏幕尺寸
        currentScreenSize = result.screenSize;
        
        // 4. 在屏幕截图上创建可交互叠层
        await createUIOverlay(currentUIElements, result.screenSize);
        
        // 5. 显示元素列表
        displayUIElementList(currentUIElements);
        
        // 更新按钮状态
        const toggleBtn = document.getElementById('toggleXmlBtn');
        if (toggleBtn) {
            toggleBtn.style.background = '#4CAF50';
            toggleBtn.setAttribute('title', '关闭XML Overlay');
        }
        
        
        window.NotificationModule.showNotification(
            `XML Overlay已启用，识别到${currentUIElements.length}个元素`, 
            'success'
        );
        
        // 在控制台输出日志
        ConsoleManager.addLog(`XML Overlay已启用，成功识别到${currentUIElements.length}个UI元素`, 'success');
        
    } catch (error) {
        console.error('启用XML Overlay失败:', error);
        window.NotificationModule.showNotification(`启用XML Overlay失败: ${error.message}`, 'error');
        xmlOverlayEnabled = false;
    }
}

function disableXmlOverlay() {
    // 移除UI叠层
    const screenContent = document.getElementById('screenContent');
    if (screenContent) {
        const overlay = screenContent.querySelector('.ui-overlay');
        if (overlay) overlay.remove();
    }
    
    // 清空UI元素列表（但保持面板可见）
    displayUIElementList([]);
    
    // 重置状态
    currentUIElements = [];
    selectedElement = null;
    
    // 更新按钮状态
    const toggleBtn = document.getElementById('toggleXmlBtn');
    if (toggleBtn) {
        toggleBtn.style.background = '';
        toggleBtn.setAttribute('title', '显示XML Overlay');
    }
    
    // 在控制台输出日志
    ConsoleManager.addLog('XML Overlay已关闭', 'info');
    
    window.NotificationModule.showNotification('XML Overlay已关闭', 'info');
}

async function createUIOverlay(elements, screenSize) {
    const screenContent = document.getElementById('screenContent');
    const deviceImage = document.getElementById('deviceScreenshot');
    
    if (!deviceImage || !screenContent) {
        throw new Error('找不到设备截图容器');
    }
    
    // 确保图片已加载
    if (!deviceImage.complete || deviceImage.naturalHeight === 0) {
        // 先刷新屏幕截图
        await refreshDeviceScreen();
        // 等待图片加载
        await new Promise(resolve => {
            if (deviceImage.complete) {
                resolve();
            } else {
                deviceImage.onload = resolve;
                setTimeout(resolve, 3000); // 3秒超时
            }
        });
    }
    
    // 移除旧的叠层
    const oldOverlay = screenContent.querySelector('.ui-overlay');
    if (oldOverlay) oldOverlay.remove();
    
    // 计算图片在容器中的实际显示区域
    const imageDisplayInfo = calculateImageDisplayArea(deviceImage, screenContent);
    if (!imageDisplayInfo) {
        console.error('无法计算图片显示区域');
        return;
    }
    
    console.log('图片显示信息:', imageDisplayInfo);
    console.log('设备屏幕尺寸:', screenSize);
    
    // 创建新的叠层容器，只覆盖图片显示区域
    const overlay = document.createElement('div');
    overlay.className = 'ui-overlay';
    overlay.style.cssText = `
        position: absolute;
        left: ${imageDisplayInfo.left}px;
        top: ${imageDisplayInfo.top}px;
        width: ${imageDisplayInfo.width}px;
        height: ${imageDisplayInfo.height}px;
        pointer-events: none;
        z-index: 10;
    `;
    
    // 为每个UI元素创建可视化标记
    elements.forEach((element, index) => {
        const marker = createElementMarker(element, screenSize, imageDisplayInfo);
        overlay.appendChild(marker);
    });
    
    // 设置容器为相对定位
    screenContent.style.position = 'relative';
    screenContent.appendChild(overlay);
}

// 计算图片在容器中的实际显示区域
function calculateImageDisplayArea(image, container) {
    if (!image.complete || !image.naturalWidth || !image.naturalHeight) {
        return null;
    }
    
    const containerStyle = getComputedStyle(container);
    const containerPadding = {
        left: parseFloat(containerStyle.paddingLeft) || 0,
        top: parseFloat(containerStyle.paddingTop) || 0,
        right: parseFloat(containerStyle.paddingRight) || 0,
        bottom: parseFloat(containerStyle.paddingBottom) || 0
    };
    
    // 容器可用空间
    const availableWidth = container.clientWidth - containerPadding.left - containerPadding.right;
    const availableHeight = container.clientHeight - containerPadding.top - containerPadding.bottom;
    
    // 图片原始尺寸
    const imageNaturalRatio = image.naturalWidth / image.naturalHeight;
    const availableRatio = availableWidth / availableHeight;
    
    let displayWidth, displayHeight;
    
    // 模拟 object-fit: contain 的行为
    if (imageNaturalRatio > availableRatio) {
        // 图片更宽，以宽度为准
        displayWidth = availableWidth;
        displayHeight = availableWidth / imageNaturalRatio;
    } else {
        // 图片更高，以高度为准
        displayHeight = availableHeight;
        displayWidth = availableHeight * imageNaturalRatio;
    }
    
    // 计算图片在容器中的位置（居中显示）
    const left = containerPadding.left + (availableWidth - displayWidth) / 2;
    const top = containerPadding.top + (availableHeight - displayHeight) / 2;
    
    return {
        left,
        top,
        width: displayWidth,
        height: displayHeight,
        scaleX: displayWidth / image.naturalWidth,
        scaleY: displayHeight / image.naturalHeight
    };
}

function createElementMarker(element, screenSize, imageDisplayInfo) {
    const marker = document.createElement('div');
    marker.className = 'ui-element-marker';
    marker.dataset.elementIndex = element.index;
    
    // 使用设备屏幕尺寸和图片显示比例计算位置
    const scaleX = imageDisplayInfo.width / screenSize.width;
    const scaleY = imageDisplayInfo.height / screenSize.height;
    
    // 计算在overlay中的相对位置
    const left = element.bounds[0] * scaleX;
    const top = element.bounds[1] * scaleY;
    const width = element.width * scaleX;
    const height = element.height * scaleY;
    
    // 只设置位置和尺寸，样式交给CSS处理
    marker.style.cssText = `
        position: absolute;
        left: ${left}px;
        top: ${top}px;
        width: ${width}px;
        height: ${height}px;
    `;
    
    // 添加元素索引标签
    const label = document.createElement('div');
    label.className = 'element-label';
    label.textContent = element.index;
    // 样式由CSS类处理，只设置文本内容
    marker.appendChild(label);
    
    // 添加交互事件
    marker.addEventListener('mouseenter', (e) => showElementTooltip(element, marker, e));
    marker.addEventListener('mouseleave', hideElementTooltip);
    marker.addEventListener('click', (e) => {
        e.stopPropagation();
        selectUIElement(element);
    });
    
    return marker;
}

function showElementTooltip(element, marker, event) {
    // 移除旧的提示框
    const oldTooltip = document.querySelector('.element-tooltip');
    if (oldTooltip) oldTooltip.remove();
    
    // 创建提示框
    const tooltip = document.createElement('div');
    tooltip.className = 'element-tooltip';
    tooltip.innerHTML = `
        <div class="tooltip-header">[${element.index}] ${element.className.split('.').pop()}</div>
        <div class="tooltip-content">
            ${element.text ? `<div><strong>文本:</strong> ${element.text}</div>` : ''}
            ${element.contentDesc ? `<div><strong>描述:</strong> ${element.contentDesc}</div>` : ''}
            ${element.hint ? `<div><strong>提示:</strong> ${element.hint}</div>` : ''}
            ${element.resourceId ? `<div><strong>ID:</strong> ${element.resourceId.split('/').pop()}</div>` : ''}
            <div><strong>位置:</strong> (${element.centerX}, ${element.centerY})</div>
            <div><strong>尺寸:</strong> ${element.width}x${element.height}</div>
        </div>
    `;
    
    tooltip.style.cssText = `
        position: fixed;
        background: rgba(0, 0, 0, 0.9);
        color: white;
        padding: 10px;
        border-radius: 5px;
        font-size: 12px;
        max-width: 300px;
        z-index: 1000;
        pointer-events: none;
        box-shadow: 0 2px 10px rgba(0,0,0,0.5);
    `;
    
    document.body.appendChild(tooltip);
    
    // 定位提示框
    const rect = marker.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    
    let left = rect.right + 10;
    let top = rect.top;
    
    // 防止超出屏幕
    if (left + tooltipRect.width > window.innerWidth) {
        left = rect.left - tooltipRect.width - 10;
    }
    if (top + tooltipRect.height > window.innerHeight) {
        top = window.innerHeight - tooltipRect.height - 10;
    }
    
    tooltip.style.left = Math.max(10, left) + 'px';
    tooltip.style.top = Math.max(10, top) + 'px';
    
    // CSS hover状态会自动处理高亮效果
}

function hideElementTooltip() {
    const tooltip = document.querySelector('.element-tooltip');
    if (tooltip) tooltip.remove();
    
    // CSS会自动处理样式状态，无需手动重置
}

function displayUIElementList(elements) {
    // 使用嵌入式UI面板
    const bottomPanel = document.getElementById('uiElementsBottomPanel');
    const elementsContainer = document.getElementById('elementsListContainer');
    
    if (!bottomPanel || !elementsContainer) {
        console.error('嵌入式UI面板元素未找到');
        return;
    }
    
    // 生成元素列表HTML
    const elementsHTML = elements.map(el => `
        <div class="element-item" data-index="${el.index}">
            <div class="element-main" onclick="selectElementByIndex(${el.index})">
                <div class="element-header">
                    <span class="element-index">[${el.index}]</span>
                    <span class="element-type">${el.className.split('.').pop()}</span>
                </div>
                ${el.text ? `<div class="element-text">文本: ${el.text}</div>` : ''}
                ${el.contentDesc ? `<div class="element-desc">描述: ${el.contentDesc}</div>` : ''}
                ${el.hint ? `<div class="element-hint">提示: ${el.hint}</div>` : ''}
                <div class="element-size">${el.width}×${el.height} @ (${el.centerX},${el.centerY})</div>
            </div>
            <div class="element-actions">
                <button class="btn-icon-small save-to-locator-btn" 
                        onclick="event.stopPropagation(); saveElementToLocatorFromList(${el.index})" 
                        title="入库"
                        style="background: transparent; border: none; padding: 4px;">
                    <svg viewBox="0 0 24 24" width="20" height="20">
                        <path fill="#FF9800" d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
                    </svg>
                </button>
            </div>
        </div>
    `).join('');
    
    // 更新容器内容
    if (elements.length > 0) {
        elementsContainer.innerHTML = elementsHTML;
    } else {
        // 显示空状态信息
        let emptyStateHTML;
        if (!xmlOverlayEnabled) {
            emptyStateHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">XML Overlay未启用</div>
                </div>
            `;
        } else {
            emptyStateHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">暂无UI元素</div>
                </div>
            `;
        }
        
        elementsContainer.innerHTML = emptyStateHTML;
    }
}

function selectUIElement(element) {
    console.log('选择UI元素:', element);
    
    // 取消之前的选择
    const previousSelected = document.querySelector('.ui-element-marker.selected');
    if (previousSelected) {
        previousSelected.classList.remove('selected');
        previousSelected.style.borderColor = '#00ff00';
        previousSelected.style.background = 'rgba(0, 255, 0, 0.1)';
    }
    
    // 高亮当前选择的元素
    const currentMarker = document.querySelector(`[data-element-index="${element.index}"]`);
    if (currentMarker) {
        currentMarker.classList.add('selected');
        currentMarker.style.borderColor = '#ff0000';
        currentMarker.style.background = 'rgba(255, 0, 0, 0.2)';
        currentMarker.style.borderWidth = '3px';
    }
    
    selectedElement = element;
    
    // 显示元素详情
    showElementProperties(element);
    
    window.NotificationModule.showNotification(`已选择元素 [${element.index}]: ${element.toAiText()}`, 'info');
}

function showElementProperties(element) {
    // 使用嵌入式面板的元素属性标签页
    const elementPropsTab = document.getElementById('elementPropsTab');
    const elementPropsPane = document.getElementById('elementPropsPane');
    const elementPropsContainer = document.getElementById('elementPropsContainer');
    const elementsListTab = document.querySelector('.tab-btn[data-tab="elements-list"]');
    const elementsListPane = document.getElementById('elementsListPane');
    
    if (!elementPropsTab || !elementPropsPane || !elementPropsContainer) {
        console.error('元素属性面板组件未找到');
        return;
    }
    
    // 生成属性面板HTML
    const propertiesHTML = `
        <div class="element-details">
            <div class="prop-group">
                <h4 class="prop-title">基本信息</h4>
                <div class="prop-item"><strong>索引:</strong> [${element.index}]</div>
                <div class="prop-item"><strong>类型:</strong> ${element.className}</div>
                <div class="prop-item"><strong>AI描述:</strong> ${element.toAiText()}</div>
            </div>
            
            <div class="prop-group">
                <h4 class="prop-title">位置信息</h4>
                <div class="prop-item"><strong>中心点:</strong> (${element.centerX}, ${element.centerY})</div>
                <div class="prop-item"><strong>边界:</strong> [${element.bounds.join(', ')}]</div>
                <div class="prop-item"><strong>尺寸:</strong> ${element.width} × ${element.height}</div>
            </div>
            
            ${element.text || element.contentDesc || element.hint ? `
                <div class="prop-group">
                    <h4 class="prop-title">文本信息</h4>
                    ${element.text ? `<div class="prop-item"><strong>文本:</strong> ${element.text}</div>` : ''}
                    ${element.contentDesc ? `<div class="prop-item"><strong>描述:</strong> ${element.contentDesc}</div>` : ''}
                    ${element.hint ? `<div class="prop-item"><strong>提示:</strong> ${element.hint}</div>` : ''}
                </div>
            ` : ''}
            
            <div class="prop-group">
                <h4 class="prop-title">状态</h4>
                <div class="prop-item">
                    <span class="status-item ${element.clickable ? 'status-true' : 'status-false'}">
                        ${element.clickable ? '✓' : '✗'} 可点击
                    </span>
                    <span class="status-item ${element.enabled ? 'status-true' : 'status-false'}">
                        ${element.enabled ? '✓' : '✗'} 已启用
                    </span>
                </div>
                <div class="prop-item">
                    <span class="status-item ${element.focusable ? 'status-true' : 'status-false'}">
                        ${element.focusable ? '✓' : '✗'} 可获焦点
                    </span>
                    <span class="status-item ${element.scrollable ? 'status-true' : 'status-false'}">
                        ${element.scrollable ? '✓' : '✗'} 可滚动
                    </span>
                </div>
            </div>
            
            <div class="prop-group">
                <h4 class="prop-title">操作</h4>
                <div class="action-buttons">
                    <button class="btn btn-primary" onclick="saveElementToLocator(${element.index})">
                        <svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;">
                            <path fill="currentColor" d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
                        </svg>
                        入库
                    </button>
                </div>
            </div>
        </div>
    `;
    
    // 更新属性容器内容
    elementPropsContainer.innerHTML = propertiesHTML;
    
    // 切换到属性标签页
    elementsListTab.classList.remove('active');
    elementPropsTab.classList.add('active');
    elementsListPane.style.display = 'none';
    elementsListPane.classList.remove('active');
    elementPropsPane.style.display = 'block';
    elementPropsPane.classList.add('active');
    
    // 触发标签页变化事件
    document.dispatchEvent(new CustomEvent('tabChanged', { detail: { tabId: 'element-props' } }));
}

// 全局函数供HTML调用
window.selectElementByIndex = function(index) {
    const element = currentUIElements.find(el => el.index === index);
    if (element) {
        selectUIElement(element);
    }
};

// 从列表直接入库元素
window.saveElementToLocatorFromList = async function(index) {
    const element = currentUIElements.find(el => el.index === index);
    if (element) {
        if (window.LocatorManager && window.LocatorManager.instance) {
            await window.LocatorManager.instance.saveElement(element);
        } else {
            console.error('Locator管理器未初始化');
            window.NotificationModule.showNotification('Locator管理器未初始化', 'error');
        }
    } else {
        console.error('未找到指定索引的元素:', index);
        window.NotificationModule.showNotification('元素未找到', 'error');
    }
};

window.insertElementReference = function(index, action) {
    const element = currentUIElements.find(el => el.index === index);
    if (!element) return;
    
    let scriptText = '';
    switch (action) {
        case 'click':
            scriptText = `    点击 [${index}]`;
            break;
        case 'input':
            scriptText = `    输入 [${index}] "文本内容"`;
            break;
        case 'assert':
            scriptText = `    断言 [${index}, 存在]`;
            break;
    }
    
    // 获取当前活动的编辑器并插入文本
    const activeTab = document.querySelector('.tab.active');
    if (activeTab && window.EditorModule) {
        const tabId = activeTab.id;
        const editor = window.EditorModule.getEditor(tabId);
        if (editor) {
            editor.insertText(scriptText + '\n');
            window.NotificationModule.showNotification(`已插入: ${scriptText}`, 'success');
        } else {
            // 如果没有编辑器，复制到剪贴板
            navigator.clipboard.writeText(scriptText).then(() => {
                window.NotificationModule.showNotification(`已复制到剪贴板: ${scriptText}`, 'success');
            });
        }
    } else {
        // 复制到剪贴板作为备用
        navigator.clipboard.writeText(scriptText).then(() => {
            window.NotificationModule.showNotification(`已复制到剪贴板: ${scriptText}`, 'success');
        });
    }
};

// 初始化嵌入式UI元素面板事件处理器
function initializeUIElementsPanel() {
    // 收起/展开按钮
    const toggleBtn = document.getElementById('toggleElementsPanelBtn');
    const panelContent = document.getElementById('uiElementsPanelContent');
    const toggleIcon = document.getElementById('toggleElementsPanelIcon');
    const bottomPanel = document.getElementById('uiElementsBottomPanel');
    
    if (toggleBtn && panelContent && toggleIcon && bottomPanel) {
        // 从localStorage恢复面板状态
        const savedState = localStorage.getItem('uiElementsPanelCollapsed');
        if (savedState === 'true') {
            bottomPanel.classList.add('collapsed');
            toggleIcon.innerHTML = '<path d="M7.41 15.41L12 10.83l4.59 4.58L18 14l-6-6-6 6z"/>';
            toggleBtn.title = '展开';
        }
        
        toggleBtn.addEventListener('click', () => {
            const isCollapsed = bottomPanel.classList.contains('collapsed');
            
            if (isCollapsed) {
                // 展开面板
                bottomPanel.classList.remove('collapsed');
                toggleIcon.innerHTML = '<path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z"/>';
                toggleBtn.title = '收起';
                localStorage.setItem('uiElementsPanelCollapsed', 'false');
            } else {
                // 收起面板
                bottomPanel.classList.add('collapsed');
                toggleIcon.innerHTML = '<path d="M7.41 15.41L12 10.83l4.59 4.58L18 14l-6-6-6 6z"/>';
                toggleBtn.title = '展开';
                localStorage.setItem('uiElementsPanelCollapsed', 'true');
            }
        });
        
        // 键盘快捷键支持 (Ctrl+U 切换面板)
        document.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.key === 'u') {
                e.preventDefault();
                toggleBtn.click();
            }
        });
    }
    
    // 清空控制台按钮事件和显示控制
    const clearConsoleBtn = document.getElementById('clearConsoleBtn');
    if (clearConsoleBtn) {
        clearConsoleBtn.addEventListener('click', () => {
            ConsoleManager.clear();
        });
        
        // 根据当前标签页控制清空按钮的显示
        const updateClearButtonVisibility = () => {
            const consoleTab = document.getElementById('consoleTab');
            if (consoleTab && consoleTab.classList.contains('active')) {
                clearConsoleBtn.style.display = 'block';
            } else {
                clearConsoleBtn.style.display = 'none';
            }
        };
        
        // 初始检查
        updateClearButtonVisibility();
        
        // 标签页切换时更新按钮显示
        document.addEventListener('tabChanged', updateClearButtonVisibility);
    }
    
    // 标签页切换
    const tabBtns = document.querySelectorAll('.ui-elements-bottom-panel .tab-btn');
    const tabPanes = document.querySelectorAll('.ui-elements-bottom-panel .tab-pane');
    
    tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.getAttribute('data-tab');
            
            // 更新标签按钮状态
            tabBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // 更新标签页内容显示
            tabPanes.forEach(pane => {
                pane.style.display = 'none';
                pane.classList.remove('active');
            });
            
            // 根据标签ID显示对应面板
            let targetPane;
            switch (tabId) {
                case 'elements-list':
                    targetPane = document.getElementById('elementsListPane');
                    break;
                case 'element-props':
                    targetPane = document.getElementById('elementPropsPane');
                    break;
                case 'locator-lib':
                    targetPane = document.getElementById('locatorLibPane');
                    // 加载Locator库
                    if (window.LocatorManager && window.LocatorManager.instance) {
                        window.LocatorManager.instance.loadLocators();
                    }
                    break;
                case 'console-output':
                    targetPane = document.getElementById('consoleOutputPane');
                    break;
            }
            
            if (targetPane) {
                targetPane.style.display = 'block';
                targetPane.classList.add('active');
            }
            
            // 触发标签页变化事件，用于更新UI
            document.dispatchEvent(new CustomEvent('tabChanged', { detail: { tabId } }));
        });
    });
}

// 重新计算XML标记位置（当面板大小改变时调用）
function recalculateXmlMarkersPosition() {
    const overlay = document.querySelector('.ui-overlay');
    const deviceImage = document.getElementById('deviceScreenshot');
    const screenContent = document.getElementById('screenContent');
    
    if (!overlay || !deviceImage || !screenContent || !currentUIElements || currentUIElements.length === 0) {
        return; // 如果没有overlay或元素，不需要重新计算
    }
    
    // 重新计算图片显示区域
    const imageDisplayInfo = calculateImageDisplayArea(deviceImage, screenContent);
    if (!imageDisplayInfo) {
        console.error('重新计算时无法获取图片显示区域');
        return;
    }
    
    console.log('重新计算XML标记位置 - 新的显示区域:', imageDisplayInfo);
    
    // 获取原始屏幕尺寸
    let screenSize = currentScreenSize;
    if (!screenSize) {
        // 尝试从第一个元素推断屏幕尺寸
        const maxBounds = currentUIElements.reduce((max, el) => ({
            width: Math.max(max.width, el.bounds[2]),
            height: Math.max(max.height, el.bounds[3])
        }), { width: 0, height: 0 });
        screenSize = { width: maxBounds.width, height: maxBounds.height };
    }
    
    // 更新overlay本身的位置和大小
    overlay.style.left = imageDisplayInfo.left + 'px';
    overlay.style.top = imageDisplayInfo.top + 'px';
    overlay.style.width = imageDisplayInfo.width + 'px';
    overlay.style.height = imageDisplayInfo.height + 'px';
    
    // 重新计算每个标记的位置
    const markers = overlay.querySelectorAll('.ui-element-marker');
    markers.forEach((marker) => {
        const elementIndex = parseInt(marker.dataset.elementIndex);
        const element = currentUIElements.find(el => el.index === elementIndex);
        
        if (element) {
            // 使用新的缩放比例计算位置
            const scaleX = imageDisplayInfo.width / screenSize.width;
            const scaleY = imageDisplayInfo.height / screenSize.height;
            
            const left = element.bounds[0] * scaleX;
            const top = element.bounds[1] * scaleY;
            const width = element.width * scaleX;
            const height = element.height * scaleY;
            
            // 更新标记位置（相对于overlay）
            marker.style.left = left + 'px';
            marker.style.top = top + 'px';
            marker.style.width = width + 'px';
            marker.style.height = height + 'px';
        }
    });
    
    console.log('XML标记位置重新计算完成');
}

// 控制台管理功能
const ConsoleManager = {
    // 添加控制台日志
    addLog: function(message, type = 'info') {
        const consoleOutput = document.getElementById('consoleOutput');
        if (!consoleOutput) return;
        
        // 清除欢迎信息（如果存在）
        const welcome = consoleOutput.querySelector('.console-welcome');
        if (welcome) {
            welcome.remove();
        }
        
        // 创建日志条目
        const logEntry = document.createElement('div');
        logEntry.className = `console-log ${type}`;
        
        // 添加时间戳
        const timestamp = new Date().toLocaleTimeString();
        const timestampSpan = document.createElement('span');
        timestampSpan.className = 'console-timestamp';
        timestampSpan.textContent = `[${timestamp}]`;
        
        // 添加消息内容
        const messageSpan = document.createElement('span');
        messageSpan.textContent = message;
        
        logEntry.appendChild(timestampSpan);
        logEntry.appendChild(messageSpan);
        consoleOutput.appendChild(logEntry);
        
        // 自动滚动到底部
        consoleOutput.scrollTop = consoleOutput.scrollHeight;
    },
    
    // 清空控制台
    clear: function() {
        const consoleOutput = document.getElementById('consoleOutput');
        if (!consoleOutput) return;
        
        consoleOutput.innerHTML = `
            <div class="console-welcome">
                <div class="console-welcome-text">控制台已清空</div>
                <div class="console-welcome-hint">新的输出将在这里显示</div>
            </div>
        `;
    },
    
    // 切换到控制台标签页
    switchToConsole: function() {
        const consoleTab = document.getElementById('consoleTab');
        if (consoleTab) {
            consoleTab.click();
        }
    }
};

// 窗口大小变化时的响应式处理
function handleWindowResize() {
    // 防抖延迟重新计算，缩短延迟时间以提高响应性
    if (handleWindowResize.timeout) {
        clearTimeout(handleWindowResize.timeout);
    }
    
    handleWindowResize.timeout = setTimeout(() => {
        // 重新计算XML标记位置
        if (xmlOverlayEnabled && currentUIElements.length > 0) {
            recalculateXmlMarkersPosition();
        }
    }, 100);
}

// 在页面加载时初始化
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        initializeUIElementsPanel();
        initializeBottomPanelDisplay();
        // 添加窗口大小变化监听器
        window.addEventListener('resize', handleWindowResize);
    });
} else {
    initializeUIElementsPanel();
    initializeBottomPanelDisplay();
    window.addEventListener('resize', handleWindowResize);
}

// 初始化底部面板显示
function initializeBottomPanelDisplay() {
    // 默认显示底部面板，并显示初始状态
    displayUIElementList([]); // 传入空数组会显示适当的空状态
}

// 设备屏幕模式管理
const ScreenModeManager = {
    initialized: false,
    currentMode: 'normal', // 'normal', 'xml', 'screenshot', 'coordinate'
    screenshotSelector: null,
    isSelecting: false,
    startX: 0,
    startY: 0,
    endX: 0,
    endY: 0,
    
    // 统一的坐标转换系统
    getImageDisplayInfo() {
        const deviceImage = document.getElementById('deviceScreenshot');
        const screenContent = document.getElementById('screenContent');
        if (!deviceImage || !screenContent) return null;
        
        return calculateImageDisplayArea(deviceImage, screenContent);
    },
    
    // 将屏幕坐标转换为图片内坐标
    screenToImageCoords(screenX, screenY) {
        const imageInfo = this.getImageDisplayInfo();
        if (!imageInfo) return { x: screenX, y: screenY };
        
        // 转换为图片内相对坐标
        const imageX = screenX - imageInfo.left;
        const imageY = screenY - imageInfo.top;
        
        // 确保坐标在图片范围内
        const clampedX = Math.max(0, Math.min(imageX, imageInfo.width));
        const clampedY = Math.max(0, Math.min(imageY, imageInfo.height));
        
        return { x: clampedX, y: clampedY };
    },
    
    // 将图片内坐标转换为实际设备坐标
    imageToDeviceCoords(imageX, imageY) {
        const deviceImage = document.getElementById('deviceScreenshot');
        const imageInfo = this.getImageDisplayInfo();
        if (!deviceImage || !imageInfo) return { x: imageX, y: imageY };
        
        // 计算缩放比例
        const scaleX = deviceImage.naturalWidth / imageInfo.width;
        const scaleY = deviceImage.naturalHeight / imageInfo.height;
        
        return {
            x: Math.round(imageX * scaleX),
            y: Math.round(imageY * scaleY)
        };
    },
    
    // 检查点是否在图片区域内
    isPointInImage(screenX, screenY) {
        const imageCoords = this.screenToImageCoords(screenX, screenY);
        const imageInfo = this.getImageDisplayInfo();
        if (!imageInfo) return false;
        
        return imageCoords.x >= 0 && imageCoords.x <= imageInfo.width &&
               imageCoords.y >= 0 && imageCoords.y <= imageInfo.height;
    },
    
    // 初始化模式管理器
    init() {
        // 防止重复初始化
        if (this.initialized) {
            console.log('ScreenModeManager 已经初始化过了，跳过重复初始化');
            return;
        }
        
        this.setupModeButtons();
        this.setupScreenshotMode();
        this.setupCoordinateMode();
        
        this.initialized = true;
        console.log('ScreenModeManager 初始化完成');
    },
    
    // 设置模式切换按钮
    setupModeButtons() {
        const normalModeBtn = document.getElementById('normalModeBtn');
        const xmlModeBtn = document.getElementById('xmlModeBtn');
        const screenshotModeBtn = document.getElementById('screenshotModeBtn');
        const coordinateModeBtn = document.getElementById('coordinateModeBtn');
        
        if (normalModeBtn) {
            normalModeBtn.addEventListener('click', () => this.switchMode('normal'));
        }
        if (xmlModeBtn) {
            xmlModeBtn.addEventListener('click', () => this.switchMode('xml'));
        }
        if (screenshotModeBtn) {
            screenshotModeBtn.addEventListener('click', () => this.switchMode('screenshot'));
        }
        if (coordinateModeBtn) {
            coordinateModeBtn.addEventListener('click', () => this.switchMode('coordinate'));
        }
    },
    
    // 切换模式
    switchMode(mode) {
        this.currentMode = mode;
        const screenContent = document.getElementById('screenContent');
        const screenshotSelector = document.getElementById('screenshotSelector');
        const coordinateMarker = document.getElementById('coordinateMarker');
        
        // 重置所有模式按钮状态
        document.querySelectorAll('.mode-btn').forEach(btn => btn.classList.remove('active'));
        
        // 清理之前的模式状态
        screenContent.classList.remove('screenshot-mode', 'coordinate-mode');
        if (screenshotSelector) screenshotSelector.style.display = 'none';
        if (coordinateMarker) coordinateMarker.style.display = 'none';
        
        // 先禁用任何活动的 XML overlay
        const existingOverlay = document.querySelector('.ui-overlay');
        if (existingOverlay) {
            existingOverlay.remove();
        }
        
        switch(mode) {
            case 'normal':
                document.getElementById('normalModeBtn').classList.add('active');
                // 纯屏幕模式，不显示任何覆盖层
                xmlOverlayEnabled = false;
                break;
                
            case 'xml':
                document.getElementById('xmlModeBtn').classList.add('active');
                // 启用XML overlay
                xmlOverlayEnabled = true;
                const deviceSelect = document.getElementById('deviceSelect');
                if (deviceSelect?.value) {
                    enableXmlOverlay(deviceSelect.value);
                }
                break;
                
            case 'screenshot':
                document.getElementById('screenshotModeBtn').classList.add('active');
                screenContent.classList.add('screenshot-mode');
                xmlOverlayEnabled = false;
                window.NotificationModule.showNotification('截图模式：拖动鼠标框选要截取的区域', 'info');
                break;
                
            case 'coordinate':
                document.getElementById('coordinateModeBtn').classList.add('active');
                screenContent.classList.add('coordinate-mode');
                xmlOverlayEnabled = false;
                window.NotificationModule.showNotification('坐标模式：点击屏幕获取坐标', 'info');
                break;
        }
        
        // 显示或隐藏缩放控制
        this.updateZoomControlsVisibility();
    },
    
    // 更新缩放控制的可见性
    updateZoomControlsVisibility() {
        const screenZoomControls = document.getElementById('screenZoomControls');
        const deviceImage = document.getElementById('deviceScreenshot');
        
        if (screenZoomControls && deviceImage && deviceImage.style.display !== 'none') {
            screenZoomControls.style.display = 'flex';
        } else if (screenZoomControls) {
            screenZoomControls.style.display = 'none';
        }
    },
    
    
    // 设置截图模式
    setupScreenshotMode() {
        const screenContent = document.getElementById('screenContent');
        const screenshotSelector = document.getElementById('screenshotSelector');
        
        if (!screenshotSelector) {
            console.error('截图选择器未找到');
            return;
        }
        
        const selectorBox = screenshotSelector.querySelector('.selector-box');
        const confirmBtn = document.getElementById('confirmScreenshotBtn');
        const cancelBtn = document.getElementById('cancelScreenshotBtn');
        
        // 备用查找方式
        if (!confirmBtn) {
            const altConfirmBtn = screenshotSelector.querySelector('#confirmScreenshotBtn');
            console.log('使用备用方式查找确认按钮:', !!altConfirmBtn);
        }
        
        if (!cancelBtn) {
            const altCancelBtn = screenshotSelector.querySelector('#cancelScreenshotBtn');
            console.log('使用备用方式查找取消按钮:', !!altCancelBtn);
        }
        
        console.log('截图模式设置：', {
            screenContent: !!screenContent,
            screenshotSelector: !!screenshotSelector,
            selectorBox: !!selectorBox,
            confirmBtn: !!confirmBtn,
            cancelBtn: !!cancelBtn
        });
        
        let isSelecting = false;
        let startX = 0, startY = 0;
        
        // 鼠标按下开始选择
        screenContent.addEventListener('mousedown', (e) => {
            if (this.currentMode !== 'screenshot') return;
            
            // 检查是否点击在控制按钮上，如果是则不处理选择
            if (e.target.closest('.selector-controls')) {
                return;
            }
            
            // 获取相对于 screenContent 的坐标
            const rect = screenContent.getBoundingClientRect();
            const screenX = e.clientX - rect.left;
            const screenY = e.clientY - rect.top;
            
            // 检查是否在图片区域内
            if (!this.isPointInImage(screenX, screenY)) return;
            
            isSelecting = true;
            startX = screenX;
            startY = screenY;
            
            screenshotSelector.style.display = 'block';
            selectorBox.style.left = startX + 'px';
            selectorBox.style.top = startY + 'px';
            selectorBox.style.width = '0px';
            selectorBox.style.height = '0px';
            
            // 隐藏控制按钮
            screenshotSelector.querySelector('.selector-controls').style.display = 'none';
            
            e.preventDefault();
        });
        
        // 鼠标移动更新选择框
        screenContent.addEventListener('mousemove', (e) => {
            if (!isSelecting || this.currentMode !== 'screenshot') return;
            
            const rect = screenContent.getBoundingClientRect();
            const currentX = e.clientX - rect.left;
            const currentY = e.clientY - rect.top;
            
            // 限制在图片区域内
            const imageInfo = this.getImageDisplayInfo();
            if (imageInfo) {
                const clampedX = Math.max(imageInfo.left, Math.min(currentX, imageInfo.left + imageInfo.width));
                const clampedY = Math.max(imageInfo.top, Math.min(currentY, imageInfo.top + imageInfo.height));
                
                const left = Math.min(startX, clampedX);
                const top = Math.min(startY, clampedY);
                const width = Math.abs(clampedX - startX);
                const height = Math.abs(clampedY - startY);
                
                selectorBox.style.left = left + 'px';
                selectorBox.style.top = top + 'px';
                selectorBox.style.width = width + 'px';
                selectorBox.style.height = height + 'px';
            }
        });
        
        // 鼠标释放结束选择
        screenContent.addEventListener('mouseup', (e) => {
            if (!isSelecting || this.currentMode !== 'screenshot') return;
            
            isSelecting = false;
            const rect = screenContent.getBoundingClientRect();
            const endX = e.clientX - rect.left;
            const endY = e.clientY - rect.top;
            
            // 限制在图片区域内
            const imageInfo = this.getImageDisplayInfo();
            if (imageInfo) {
                const clampedEndX = Math.max(imageInfo.left, Math.min(endX, imageInfo.left + imageInfo.width));
                const clampedEndY = Math.max(imageInfo.top, Math.min(endY, imageInfo.top + imageInfo.height));
                
                // 转换为图片内坐标
                const startCoords = this.screenToImageCoords(startX, startY);
                const endCoords = this.screenToImageCoords(clampedEndX, clampedEndY);
                
                this.startX = Math.min(startCoords.x, endCoords.x);
                this.startY = Math.min(startCoords.y, endCoords.y);
                this.endX = Math.max(startCoords.x, endCoords.x);
                this.endY = Math.max(startCoords.y, endCoords.y);
                
                // 如果选择区域太小，忽略
                if ((this.endX - this.startX) < 10 || (this.endY - this.startY) < 10) {
                    screenshotSelector.style.display = 'none';
                    return;
                }
                
                // 显示控制按钮
                const controls = screenshotSelector.querySelector('.selector-controls');
                controls.style.display = 'flex';
                controls.style.left = Math.max(startX, clampedEndX) + 'px';
                controls.style.top = Math.min(startY, clampedEndY) + 'px';
            }
        });
        
        // 确认按钮
        if (confirmBtn) {
            console.log('截图确认按钮找到，绑定事件');
            
            // 移除已存在的事件监听器避免重复绑定
            if (confirmBtn._screenshotClickHandler) {
                confirmBtn.removeEventListener('click', confirmBtn._screenshotClickHandler);
            }
            
            // 防止重复处理
            let isProcessing = false;
            
            const clickHandler = (e) => {
                e.preventDefault();
                e.stopPropagation();
                
                if (isProcessing) {
                    console.log('正在处理中，跳过重复点击');
                    return;
                }
                
                isProcessing = true;
                this.captureSelectedArea().finally(() => {
                    isProcessing = false;
                });
            };
            
            // 保存处理器引用并添加监听器
            confirmBtn._screenshotClickHandler = clickHandler;
            confirmBtn.addEventListener('click', clickHandler);
            
            
        } else {
            console.error('截图确认按钮未找到');
        }
        
        // 取消按钮
        if (cancelBtn) {
            // 移除已存在的事件监听器避免重复绑定
            if (cancelBtn._screenshotCancelHandler) {
                cancelBtn.removeEventListener('click', cancelBtn._screenshotCancelHandler);
            }
            
            const cancelHandler = () => {
                console.log('截图取消按钮被点击');
                screenshotSelector.style.display = 'none';
            };
            
            // 保存处理器引用并添加监听器
            cancelBtn._screenshotCancelHandler = cancelHandler;
            cancelBtn.addEventListener('click', cancelHandler);
        } else {
            console.error('截图取消按钮未找到');
        }
    },
    
    // 截取选中区域
    async captureSelectedArea() {
        const { fs, fsSync, path } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;
        
        if (!projectPath) {
            window.NotificationModule.showNotification('请先打开项目', 'error');
            return;
        }
        
        // 使用 workarea 中的截图文件
        const screenshotPath = path.join(projectPath, 'workarea', 'current_screenshot.png');
        
        try {
            // 检查截图文件是否存在（fs 已经是 fs.promises）
            const screenshotExists = await fs.access(screenshotPath).then(() => true).catch(() => false);
            if (!screenshotExists) {
                window.NotificationModule.showNotification('请先刷新设备屏幕截图', 'error');
                document.getElementById('screenshotSelector').style.display = 'none';
                return;
            }
            
            // 读取截图文件
            const imageBuffer = await fs.readFile(screenshotPath);
            const base64Full = `data:image/png;base64,${imageBuffer.toString('base64')}`;
            
            // 创建临时图片对象来获取原始尺寸
            const tempImg = new Image();
            tempImg.src = base64Full;
            
            await new Promise((resolve) => {
                tempImg.onload = resolve;
            });
            
            // 使用统一的坐标转换系统
            const deviceStartCoords = this.imageToDeviceCoords(this.startX, this.startY);
            const deviceEndCoords = this.imageToDeviceCoords(this.endX, this.endY);
            
            const realStartX = deviceStartCoords.x;
            const realStartY = deviceStartCoords.y;
            const realEndX = deviceEndCoords.x;
            const realEndY = deviceEndCoords.y;
            
            // 创建Canvas来裁切图片
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            const width = realEndX - realStartX;
            const height = realEndY - realStartY;
            
            canvas.width = width;
            canvas.height = height;
            
            // 从完整图片绘制裁切区域
            ctx.drawImage(tempImg, realStartX, realStartY, width, height, 0, 0, width, height);
            
            // 转换为Base64
            const base64Image = canvas.toDataURL('image/png');
            
            // 弹出对话框让用户输入别名
            let alias;
            do {
                alias = await this.promptForAlias();
                if (!alias) {
                    document.getElementById('screenshotSelector').style.display = 'none';
                    return;
                }
            } while (!await this.saveImageLocator(alias, base64Image)); // 如果保存失败（重名）则重新输入
            
            // 隐藏选择器
            document.getElementById('screenshotSelector').style.display = 'none';
            
        } catch (error) {
            console.error('截取图片失败:', error);
            window.NotificationModule.showNotification('截取失败: ' + error.message, 'error');
            document.getElementById('screenshotSelector').style.display = 'none';
        }
    },
    
    // 提示用户输入别名
    async promptForAlias() {
        return new Promise((resolve) => {
            const modal = document.createElement('div');
            modal.className = 'modal-overlay';
            modal.innerHTML = `
                <div class="modal-dialog">
                    <h3>保存截图定位器</h3>
                    <input type="text" id="imageAliasInput" placeholder="请输入截图别名" autofocus>
                    <div style="display: flex; gap: 10px; justify-content: flex-end;">
                        <button class="btn btn-secondary" id="cancelAliasBtn">取消</button>
                        <button class="btn btn-primary" id="confirmAliasBtn">确定</button>
                    </div>
                </div>
            `;
            document.body.appendChild(modal);
            
            const input = modal.querySelector('#imageAliasInput');
            const confirmBtn = modal.querySelector('#confirmAliasBtn');
            const cancelBtn = modal.querySelector('#cancelAliasBtn');
            
            const confirm = () => {
                const value = input.value.trim();
                if (value) {
                    document.body.removeChild(modal);
                    resolve(value);
                } else {
                    window.NotificationModule.showNotification('请输入别名', 'warning');
                }
            };
            
            const cancel = () => {
                document.body.removeChild(modal);
                resolve(null);
            };
            
            confirmBtn.addEventListener('click', confirm);
            cancelBtn.addEventListener('click', cancel);
            input.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') confirm();
                if (e.key === 'Escape') cancel();
            });
            
            input.focus();
        });
    },
    
    // 保存图片定位器
    async saveImageLocator(alias, base64Image) {
        const { fs, fsSync, path, ipcRenderer } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;
        
        if (!projectPath) {
            window.NotificationModule.showNotification('请先打开项目', 'error');
            return;
        }
        
        try {
            // 确保目录存在
            const locatorDir = path.join(projectPath, 'locator');
            const imgDir = path.join(locatorDir, 'img');
            
            // 使用 fs API（fs 已经是 fs.promises）
            try {
                await fs.mkdir(locatorDir, { recursive: true });
            } catch (err) {
                // 目录可能已存在，忽略错误
            }
            
            try {
                await fs.mkdir(imgDir, { recursive: true });
            } catch (err) {
                // 目录可能已存在，忽略错误
            }
            
            // 保存图片文件
            const fileName = `${alias.replace(/[^a-zA-Z0-9_\u4e00-\u9fa5]/g, '_')}.png`;
            const filePath = path.join(imgDir, fileName);
            const base64Data = base64Image.replace(/^data:image\/png;base64,/, '');
            await fs.writeFile(filePath, Buffer.from(base64Data, 'base64'));
            
            // 更新element.json
            const elementsPath = path.join(locatorDir, 'element.json');
            let elements = {};
            
            // 尝试读取现有的element.json
            try {
                const content = await fs.readFile(elementsPath, 'utf-8');
                elements = JSON.parse(content);
            } catch (err) {
                // 文件不存在或解析失败，使用空对象
                console.log('element.json not found or invalid, creating new one');
            }
            
            // 检查名称是否已存在
            if (elements[alias]) {
                window.NotificationModule.showNotification(`定位器名称 "${alias}" 已存在，请使用其他名称`, 'warning');
                return false;
            }
            
            // 添加新的图片定位器（使用相对路径）
            elements[alias] = {
                type: 'image',
                path: `locator/img/${fileName}`,
                addedAt: new Date().toISOString()
            };
            
            // 保存更新后的element.json
            await fs.writeFile(elementsPath, JSON.stringify(elements, null, 2));
            
            window.NotificationModule.showNotification(`图片定位器 "${alias}" 已保存`, 'success');
            
            // 刷新Locator库显示
            if (window.LocatorManagerModule) {
                window.LocatorManagerModule.loadLocators();
            }
            
            // 刷新编辑器中的图片定位器显示
            if (window.AppGlobals.codeEditor && typeof window.AppGlobals.codeEditor.refreshImageLocators === 'function') {
                window.AppGlobals.codeEditor.refreshImageLocators();
            }
            
            return true; // 保存成功
            
        } catch (error) {
            console.error('保存图片定位器失败:', error);
            window.NotificationModule.showNotification('保存失败: ' + error.message, 'error');
            return false; // 保存失败
        }
    },
    
    // 设置坐标模式
    setupCoordinateMode() {
        const screenContent = document.getElementById('screenContent');
        const coordinateMarker = document.getElementById('coordinateMarker');
        const coordinateDot = coordinateMarker.querySelector('.coordinate-dot');
        const coordinateLabel = coordinateMarker.querySelector('.coordinate-label');
        
        screenContent.addEventListener('click', async (e) => {
            if (this.currentMode !== 'coordinate') return;
            
            // 获取相对于 screenContent 的坐标
            const rect = screenContent.getBoundingClientRect();
            const screenX = e.clientX - rect.left;
            const screenY = e.clientY - rect.top;
            
            // 检查是否在图片区域内
            if (!this.isPointInImage(screenX, screenY)) return;
            
            // 转换为图片内坐标，然后转换为设备坐标
            const imageCoords = this.screenToImageCoords(screenX, screenY);
            const deviceCoords = this.imageToDeviceCoords(imageCoords.x, imageCoords.y);
            
            // 显示坐标标记
            coordinateMarker.style.display = 'block';
            coordinateMarker.style.left = screenX + 'px';
            coordinateMarker.style.top = screenY + 'px';
            
            // 更新坐标标签
            const coordText = `(${deviceCoords.x}, ${deviceCoords.y})`;
            coordinateLabel.textContent = coordText;
            
            // 复制到剪贴板
            try {
                await navigator.clipboard.writeText(coordText);
                window.NotificationModule.showNotification(`坐标 ${coordText} 已复制到剪贴板`, 'success');
            } catch (err) {
                console.error('Failed to copy coordinates:', err);
                window.NotificationModule.showNotification('复制坐标失败', 'error');
            }
            
            // 3秒后自动隐藏标记
            setTimeout(() => {
                coordinateMarker.style.display = 'none';
            }, 3000);
        });
    }
};

// 延迟初始化模式管理器，确保在 testcase 页面显示后再初始化
function initializeScreenModeManager() {
    console.log('开始初始化 ScreenModeManager');
    
    // 检查必要的 DOM 元素是否存在
    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');
    
    if (screenContent && screenshotSelector) {
        console.log('DOM 元素已准备好，初始化 ScreenModeManager');
        ScreenModeManager.init();
    } else {
        console.log('DOM 元素未准备好，延迟初始化', {
            screenContent: !!screenContent,
            screenshotSelector: !!screenshotSelector
        });
        // 延迟重试
        setTimeout(initializeScreenModeManager, 500);
    }
}

// 在页面加载时初始化模式管理器
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        initializeScreenModeManager();
    });
} else {
    initializeScreenModeManager();
}

// 导出函数
window.TestcaseManagerModule = {
    initializeTestcasePage,
    loadFileTree,
    createTreeItem,
    openFile,
    runCurrentTest,
    refreshDeviceScreen,
    toggleXmlOverlay,
    initializeUIElementsPanel,
    initializeBottomPanelDisplay,
    recalculateXmlMarkersPosition,
    ConsoleManager,
    ScreenModeManager
};