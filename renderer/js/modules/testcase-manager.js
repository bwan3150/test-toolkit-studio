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
    // 注意：clearConsoleBtn事件已在initializeUIElementsPanel中处理
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
    } else {
        scriptsContainer.classList.add('collapsed');
        // 折叠状态使用关闭的文件夹图标 (已在创建时设置)
    }
    
    caseHeader.appendChild(caseIconContainer);
    caseHeader.appendChild(caseLabelSpan);
    
    caseContainer.appendChild(caseHeader);
    caseContainer.appendChild(scriptsContainer);
    
    // 点击切换折叠/展开（只在点击图标和空白区域时触发）
    caseHeader.addEventListener('click', (e) => {
        // 如果点击的是标签本身，不触发折叠
        if (e.target === caseLabelSpan) return;
        e.stopPropagation();
        toggleCaseFolder(caseContainer);
    });
    
    // 双击重命名Case
    caseLabelSpan.addEventListener('dblclick', (e) => {
        e.stopPropagation();
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
    
    const scriptPath = path.join(casePath, 'script');
    try {
        const scripts = await fs.readdir(scriptPath);
        
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
    
    if (scriptsContainer.classList.contains('collapsed')) {
        // 展开
        scriptsContainer.classList.remove('collapsed');
        expandedCases.add(casePath);
        // 切换到打开的文件夹图标
        caseIcon.innerHTML = '<path d="M19,20H4C2.89,20 2,19.1 2,18V6C2,4.89 2.89,4 4,4H10L12,6H19A2,2 0 0,1 21,8H21L4,8V18L6.14,10H23.21L20.93,18.5C20.7,19.37 19.92,20 19,20Z"/>';
    } else {
        // 折叠
        scriptsContainer.classList.add('collapsed');
        expandedCases.delete(casePath);
        // 切换到关闭的文件夹图标
        caseIcon.innerHTML = '<path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
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
        
        currentUIElements = xmlParser.extractUIElements(optimizedTree);
        console.log('提取的UI元素:', currentUIElements);
        
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
    
    // 创建新的叠层容器
    const overlay = document.createElement('div');
    overlay.className = 'ui-overlay';
    overlay.style.cssText = `
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        pointer-events: none;
        z-index: 10;
    `;
    
    // 获取图片实际显示尺寸
    const imageRect = deviceImage.getBoundingClientRect();
    const containerRect = screenContent.getBoundingClientRect();
    
    console.log('图片尺寸:', imageRect);
    console.log('屏幕尺寸:', screenSize);
    
    // 为每个UI元素创建可视化标记
    elements.forEach((element, index) => {
        const marker = createElementMarker(element, screenSize, imageRect, containerRect);
        overlay.appendChild(marker);
    });
    
    // 设置容器为相对定位
    screenContent.style.position = 'relative';
    screenContent.appendChild(overlay);
}

function createElementMarker(element, screenSize, imageRect, containerRect) {
    const marker = document.createElement('div');
    marker.className = 'ui-element-marker';
    marker.dataset.elementIndex = element.index;
    
    // 计算缩放比例
    const scaleX = imageRect.width / screenSize.width;
    const scaleY = imageRect.height / screenSize.height;
    
    // 计算在容器中的位置
    const left = element.bounds[0] * scaleX;
    const top = element.bounds[1] * scaleY;
    const width = element.width * scaleX;
    const height = element.height * scaleY;
    
    marker.style.cssText = `
        position: absolute;
        left: ${left}px;
        top: ${top}px;
        width: ${width}px;
        height: ${height}px;
        border: 2px solid #00ff00;
        background: rgba(0, 255, 0, 0.1);
        pointer-events: all;
        cursor: pointer;
        box-sizing: border-box;
        transition: all 0.2s ease;
    `;
    
    // 添加元素索引标签
    const label = document.createElement('div');
    label.className = 'element-label';
    label.textContent = element.index;
    label.style.cssText = `
        position: absolute;
        top: -20px;
        left: 0;
        background: #00ff00;
        color: black;
        padding: 2px 4px;
        font-size: 12px;
        font-weight: bold;
        border-radius: 3px;
        min-width: 16px;
        text-align: center;
    `;
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
    
    // 高亮当前元素
    marker.style.borderColor = '#ffff00';
    marker.style.background = 'rgba(255, 255, 0, 0.2)';
}

function hideElementTooltip() {
    const tooltip = document.querySelector('.element-tooltip');
    if (tooltip) tooltip.remove();
    
    // 恢复所有元素的样式（除了选中的）
    const markers = document.querySelectorAll('.ui-element-marker');
    markers.forEach(marker => {
        if (!marker.classList.contains('selected')) {
            marker.style.borderColor = '#00ff00';
            marker.style.background = 'rgba(0, 255, 0, 0.1)';
        }
    });
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
        <div class="element-item" data-index="${el.index}" onclick="selectElementByIndex(${el.index})">
            <div class="element-header">
                <span class="element-index">[${el.index}]</span>
                <span class="element-type">${el.className.split('.').pop()}</span>
            </div>
            ${el.text ? `<div class="element-text">文本: ${el.text}</div>` : ''}
            ${el.contentDesc ? `<div class="element-desc">描述: ${el.contentDesc}</div>` : ''}
            ${el.hint ? `<div class="element-hint">提示: ${el.hint}</div>` : ''}
            <div class="element-size">${el.width}×${el.height} @ (${el.centerX},${el.centerY})</div>
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
                    <button class="btn btn-action btn-click" onclick="insertElementReference(${element.index}, 'click')">插入点击</button>
                    <button class="btn btn-action btn-input" onclick="insertElementReference(${element.index}, 'input')">插入输入</button>
                    <button class="btn btn-action btn-assert" onclick="insertElementReference(${element.index}, 'assert')">插入断言</button>
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
    
    // 获取当前图片和容器的尺寸
    const imageRect = deviceImage.getBoundingClientRect();
    const containerRect = screenContent.getBoundingClientRect();
    
    console.log('重新计算XML标记位置 - 图片尺寸:', imageRect);
    
    // 获取原始屏幕尺寸（如果可用的话）
    let screenSize = currentScreenSize;
    if (!screenSize) {
        // 尝试从第一个元素推断屏幕尺寸
        const maxBounds = currentUIElements.reduce((max, el) => ({
            width: Math.max(max.width, el.bounds[2]),
            height: Math.max(max.height, el.bounds[3])
        }), { width: 0, height: 0 });
        screenSize = { width: maxBounds.width, height: maxBounds.height };
    }
    
    // 重新计算每个标记的位置
    const markers = overlay.querySelectorAll('.ui-element-marker');
    markers.forEach((marker, index) => {
        const elementIndex = parseInt(marker.dataset.elementIndex);
        const element = currentUIElements.find(el => el.index === elementIndex);
        
        if (element) {
            // 计算新的缩放比例和位置
            const scaleX = imageRect.width / screenSize.width;
            const scaleY = imageRect.height / screenSize.height;
            
            const left = element.bounds[0] * scaleX;
            const top = element.bounds[1] * scaleY;
            const width = element.width * scaleX;
            const height = element.height * scaleY;
            
            // 更新标记位置
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
    // 防抖延迟重新计算
    if (handleWindowResize.timeout) {
        clearTimeout(handleWindowResize.timeout);
    }
    
    handleWindowResize.timeout = setTimeout(() => {
        // 重新计算XML标记位置
        if (xmlOverlayEnabled && currentUIElements.length > 0) {
            recalculateXmlMarkersPosition();
        }
    }, 200);
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
    ConsoleManager
};