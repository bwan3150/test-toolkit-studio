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
    
    // 折叠/展开箭头
    const toggleIcon = document.createElement('svg');
    toggleIcon.className = 'case-toggle-icon';
    toggleIcon.setAttribute('viewBox', '0 0 24 24');
    toggleIcon.innerHTML = '<path fill="currentColor" d="M8.59 16.59L13.17 12 8.59 7.41 10 6l6 6-6 6-1.41-1.41z"/>';
    
    // Case图标
    const caseIcon = document.createElement('svg');
    caseIcon.className = 'case-icon';
    caseIcon.setAttribute('viewBox', '0 0 24 24');
    caseIcon.innerHTML = '<path fill="currentColor" d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
    
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
        toggleIcon.style.transform = 'rotate(90deg)';
    } else {
        scriptsContainer.classList.add('collapsed');
        toggleIcon.style.transform = 'rotate(0deg)';
    }
    
    caseHeader.appendChild(toggleIcon);
    caseHeader.appendChild(caseIcon);
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
    
    // 脚本图标
    const scriptIcon = document.createElement('svg');
    scriptIcon.className = 'script-icon';
    scriptIcon.setAttribute('viewBox', '0 0 24 24');
    scriptIcon.innerHTML = '<path fill="currentColor" d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm-1 7V3.5L18.5 9H13z"/>';
    
    // 脚本名称
    const scriptLabel = document.createElement('span');
    scriptLabel.className = 'script-label';
    scriptLabel.textContent = scriptName;
    
    scriptItem.appendChild(scriptIcon);
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
    const toggleIcon = caseContainer.querySelector('.case-toggle-icon');
    const casePath = caseContainer.dataset.casePath;
    
    if (scriptsContainer.classList.contains('collapsed')) {
        scriptsContainer.classList.remove('collapsed');
        toggleIcon.style.transform = 'rotate(90deg)';
        expandedCases.add(casePath);
    } else {
        scriptsContainer.classList.add('collapsed');
        toggleIcon.style.transform = 'rotate(0deg)';
        expandedCases.delete(casePath);
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