// 测试用例文件浏览器模块
// 负责文件树的显示、管理和操作

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 存储展开状态
let expandedCases = new Set();

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
        window.rError('Failed to load file tree:', error);
    }
}

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
    
    // 单击文件夹切换展开/折叠状态
    caseHeader.addEventListener('click', async (e) => {
        e.stopPropagation();
        e.preventDefault();
        try {
            // 用户手动点击时，不自动打开第一个脚本（autoOpenFirst=false）
            await toggleCaseFolder(caseContainer, casePath, false);
        } catch (error) {
            window.rError('切换文件夹状态失败:', error);
        }
    });
    
    // 右键菜单
    caseHeader.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        showCaseContextMenu(e, caseName, casePath);
    });
    
    return caseContainer;
}

// 加载case下的脚本
async function loadCaseScripts(caseItem, casePath) {
    const { path, fs } = getGlobals();
    const scriptsContainer = caseItem.querySelector('.scripts-container');

    if (!scriptsContainer) {
        window.rError('loadCaseScripts: scripts-container not found', caseItem);
        return [];
    }

    // 清空现有内容
    scriptsContainer.innerHTML = '';

    const scriptPath = path.join(casePath, 'script');
    const scriptPaths = []; // 收集脚本路径

    try {
        const scripts = await fs.readdir(scriptPath);
        window.rLog(`加载case脚本: ${casePath}, 找到${scripts.length}个文件`);

        for (const script of scripts) {
            if (script.endsWith('.tks') || script.endsWith('.yaml')) {
                const fullScriptPath = path.join(scriptPath, script);
                const scriptItem = createScriptItem(
                    script,
                    fullScriptPath,
                    casePath
                );
                scriptsContainer.appendChild(scriptItem);
                scriptPaths.push(fullScriptPath); // 添加到路径列表
            }
        }
    } catch (error) {
        window.rError('Failed to load scripts for case:', casePath, error);
        // 如果script目录不存在，显示提示信息
        const emptyMsg = document.createElement('div');
        emptyMsg.className = 'empty-scripts-message';
        emptyMsg.textContent = '暂无脚本文件';
        emptyMsg.style.padding = '8px';
        emptyMsg.style.color = 'var(--text-secondary)';
        emptyMsg.style.fontSize = '12px';
        scriptsContainer.appendChild(emptyMsg);
    }

    return scriptPaths; // 返回脚本路径列表
}

// 切换case文件夹的展开状态
async function toggleCaseFolder(caseItem, casePath, autoOpenFirst = false) {
    window.rLog(`📂 toggleCaseFolder 调用: casePath=${casePath}, autoOpenFirst=${autoOpenFirst}`);

    const scriptsContainer = caseItem.querySelector('.scripts-container');
    const caseIcon = caseItem.querySelector('.case-icon');

    if (!scriptsContainer) {
        window.rError('未找到 scripts-container 元素');
        return [];
    }

    const isCurrentlyCollapsed = scriptsContainer.classList.contains('collapsed');
    window.rLog(`📂 当前折叠状态: ${isCurrentlyCollapsed}`);

    if (isCurrentlyCollapsed) {
        // 展开
        scriptsContainer.classList.remove('collapsed');
        expandedCases.add(casePath);
        window.rLog(`📂 展开文件夹`);

        // 更改图标为打开的文件夹（如果图标存在）
        if (caseIcon) {
            caseIcon.innerHTML = '<path d="M19,20H4C2.89,20 2,19.1 2,18V6C2,4.89 2.89,4 4,4H10L12,6H19A2,2 0 0,1 21,8H21L4,8V18L6.14,10H23.21L20.93,18.5C20.7,19.37 19.92,20 19,20Z"/>';
        }

        // 异步加载脚本文件
        window.rLog(`📂 开始加载脚本文件`);
        const scriptPaths = await loadCaseScripts(caseItem, casePath);
        window.rLog(`📂 加载完成，共 ${scriptPaths.length} 个脚本`);

        // 如果需要自动打开第一个脚本
        if (autoOpenFirst && scriptPaths.length > 0) {
            window.rLog(`📂 自动打开第一个脚本: ${scriptPaths[0]}`);
            await openFile(scriptPaths[0]);
            window.rLog(`📂 脚本已打开`);
        } else {
            window.rLog(`📂 不自动打开脚本 (autoOpenFirst=${autoOpenFirst}, 脚本数=${scriptPaths.length})`);
        }

        return scriptPaths;
    } else {
        // 折叠
        scriptsContainer.classList.add('collapsed');
        expandedCases.delete(casePath);
        window.rLog(`📂 折叠文件夹`);

        // 更改图标为关闭的文件夹（如果图标存在）
        if (caseIcon) {
            caseIcon.innerHTML = '<path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
        }

        return [];
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
    
    // 右键菜单
    scriptItem.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        showFileContextMenu(e, scriptName, scriptPath);
    });
    
    return scriptItem;
}

// 创建文件项目 (保留原有函数作为备用)
function createTreeItem(name, type, fullPath) {
    const item = document.createElement('div');
    item.className = 'tree-item';
    item.dataset.path = fullPath;
    item.dataset.type = type;
    
    // 创建图标容器
    const iconContainer = document.createElement('div');
    iconContainer.className = 'icon-container';
    
    // 创建SVG图标
    const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    icon.setAttribute('viewBox', '0 0 24 24');
    
    if (type === 'file') {
        icon.innerHTML = '<path d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20Z"/>';
        icon.className = 'file-icon';
    }
    
    iconContainer.appendChild(icon);
    
    // 创建标签
    const label = document.createElement('span');
    label.textContent = name;
    label.className = 'tree-label';
    
    item.appendChild(iconContainer);
    item.appendChild(label);
    
    // 双击打开文件
    item.addEventListener('dblclick', () => {
        if (type === 'file') {
            openFile(fullPath);
        }
    });
    
    // 右键菜单
    item.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation(); // 阻止事件冒泡到父元素
        if (type === 'file') {
            showFileContextMenu(e, name, fullPath);
        }
    });
    
    return item;
}

// 打开文件
async function openFile(filePath) {
    const { ipcRenderer, path } = getGlobals();
    
    try {
        window.rLog(`点击打开文件: ${filePath}`);
        
        const result = await ipcRenderer.invoke('read-file', filePath);
        if (result.success) {
            const fileName = path.basename(filePath);
            
            // 检查是否已经打开
            const existingTab = window.AppGlobals.openTabs.find(tab => tab.path === filePath);
            if (existingTab) {
                window.EditorManager.selectTab(existingTab.id);
                window.rLog(`文件已打开，切换到标签页: ${fileName}`);
                return;
            }
            
            // 创建新标签
            const tabId = `tab-${Date.now()}`;
            const tab = {
                id: tabId,
                path: filePath,
                filePath: filePath, // 兼容EditorManager的期望
                name: fileName,
                content: result.content
            };
            
            window.AppGlobals.openTabs.push(tab);
            window.EditorManager.createTab(tab);
            window.EditorManager.selectTab(tabId);
            
            window.rLog(`文件打开成功: ${fileName}, 内容长度: ${result.content.length}`);
            
            // 立即刷新定位器列表
            if (window.LocatorManagerModule && window.LocatorManagerModule.refreshLocatorList) {
                window.LocatorManagerModule.refreshLocatorList();
            }
        } else {
            window.rError(`打开文件失败: ${result.error}`);
            window.AppNotifications?.error(`Failed to open file: ${result.error}`);
        }
    } catch (error) {
        window.rError('打开文件时发生错误:', error);
        window.AppNotifications?.error(`Error opening file: ${error.message}`);
    }
}

// 显示case右键菜单
function showCaseContextMenu(event, caseName, casePath) {
    const contextMenu = document.createElement('div');
    contextMenu.className = 'context-menu';

    const menuItems = [
        {
            text: '新建脚本',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.createNewScript(caseName, casePath);
            }
        },
        {
            text: '重命名',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.renameCase(caseName, casePath);
            }
        },
        {
            text: '删除',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.deleteCase(caseName, casePath);
            }
        },
        {
            text: '在文件管理器中显示',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.showInFileManager(casePath);
            }
        }
    ];

    menuItems.forEach(item => {
        const menuItem = document.createElement('div');
        menuItem.className = 'context-menu-item';
        menuItem.textContent = item.text;
        menuItem.addEventListener('click', () => {
            try {
                item.action();
            } catch (error) {
                window.rError('执行菜单操作失败:', error);
                window.AppNotifications?.error(`操作失败: ${error.message}`);
            }
            if (document.body.contains(contextMenu)) {
                document.body.removeChild(contextMenu);
            }
        });
        contextMenu.appendChild(menuItem);
    });

    contextMenu.style.left = event.pageX + 'px';
    contextMenu.style.top = event.pageY + 'px';

    document.body.appendChild(contextMenu);

    // 点击其他地方关闭菜单
    const closeMenu = (e) => {
        if (!contextMenu.contains(e.target)) {
            document.body.removeChild(contextMenu);
            document.removeEventListener('click', closeMenu);
        }
    };
    setTimeout(() => document.addEventListener('click', closeMenu), 10);
}

// 显示文件右键菜单
function showFileContextMenu(event, fileName, filePath) {
    const contextMenu = document.createElement('div');
    contextMenu.className = 'context-menu';

    const menuItems = [
        {
            text: '打开',
            action: () => openFile(filePath)
        },
        {
            text: '重命名',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.renameFile(fileName, filePath);
            }
        },
        {
            text: '删除',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.deleteFile(fileName, filePath);
            }
        },
        {
            text: '复制',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.copyFile(fileName, filePath);
            }
        },
        {
            text: '在文件管理器中显示',
            action: () => {
                if (!window.ContextMenuActions) {
                    window.rError('ContextMenuActions 模块未加载');
                    window.AppNotifications?.error('功能模块未加载，请刷新页面');
                    return;
                }
                window.ContextMenuActions.showInFileManager(filePath);
            }
        }
    ];

    menuItems.forEach(item => {
        const menuItem = document.createElement('div');
        menuItem.className = 'context-menu-item';
        menuItem.textContent = item.text;
        menuItem.addEventListener('click', () => {
            try {
                item.action();
            } catch (error) {
                window.rError('执行菜单操作失败:', error);
                window.AppNotifications?.error(`操作失败: ${error.message}`);
            }
            if (document.body.contains(contextMenu)) {
                document.body.removeChild(contextMenu);
            }
        });
        contextMenu.appendChild(menuItem);
    });

    contextMenu.style.left = event.pageX + 'px';
    contextMenu.style.top = event.pageY + 'px';

    document.body.appendChild(contextMenu);

    // 点击其他地方关闭菜单
    const closeMenu = (e) => {
        if (!contextMenu.contains(e.target)) {
            document.body.removeChild(contextMenu);
            document.removeEventListener('click', closeMenu);
        }
    };
    setTimeout(() => document.addEventListener('click', closeMenu), 10);
}

// 注意：所有右键菜单操作（新建、重命名、删除、复制等）已迁移到 context-menu-actions.js

// 导出模块
window.TestcaseExplorerModule = {
    loadFileTree,
    createCollapsibleCaseItem,
    createTreeItem,
    openFile,
    toggleCaseFolder,
    loadCaseScripts
};