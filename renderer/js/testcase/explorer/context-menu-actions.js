// Case Explorer 右键菜单操作模块
// 负责处理文件和目录的右键菜单操作（新建、重命名、删除、复制等）

window.rLog('🔧 开始加载右键菜单操作模块...');

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// ============ DOM 辅助函数 ============

/**
 * 手动创建脚本项目的 DOM 元素
 * @param {string} scriptName - 脚本名称
 * @param {string} scriptPath - 脚本路径
 * @param {string} casePath - 所属 Case 路径
 * @returns {HTMLElement} 创建的脚本元素
 */
function createScriptItemDOM(scriptName, scriptPath, casePath) {
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
        e.stopPropagation();
        if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.openFile) {
            window.TestcaseExplorerModule.openFile(scriptPath);
        }
    });

    // 右键菜单
    scriptItem.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        // 调用自身模块的 renameFile, deleteFile, copyFile 等函数
        showFileContextMenu(e, scriptName, scriptPath);
    });

    return scriptItem;
}

/**
 * 显示文件右键菜单（内部使用）
 */
function showFileContextMenu(event, fileName, filePath) {
    const menuItems = [
        { text: '重命名', action: () => {
            const scriptItem = document.querySelector(`.script-item[data-script-path="${filePath}"]`);
            if (scriptItem) {
                renameFile(fileName, filePath);
            }
        }},
        { text: '删除', action: () => deleteFile(fileName, filePath) },
        { text: '复制', action: () => copyFile(fileName, filePath) },
        { text: '在文件管理器中显示', action: () => showInFileManager(filePath) }
    ];

    // 创建简单的上下文菜单
    const existingMenu = document.querySelector('.context-menu');
    if (existingMenu) existingMenu.remove();

    const contextMenu = document.createElement('div');
    contextMenu.className = 'context-menu';
    contextMenu.style.cssText = `
        position: fixed;
        left: ${event.clientX}px;
        top: ${event.clientY}px;
        background: var(--panel-bg, #252526);
        border: 1px solid var(--border-color, #3e3e42);
        border-radius: 4px;
        padding: 4px 0;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
        z-index: 10000;
        min-width: 160px;
    `;

    menuItems.forEach(item => {
        const menuItem = document.createElement('div');
        menuItem.className = 'context-menu-item';
        menuItem.textContent = item.text;
        menuItem.style.cssText = `
            padding: 6px 12px;
            cursor: pointer;
            color: var(--text-primary, #cccccc);
            font-size: 13px;
        `;
        menuItem.addEventListener('mouseenter', () => {
            menuItem.style.background = 'var(--hover-bg, #2a2d2e)';
        });
        menuItem.addEventListener('mouseleave', () => {
            menuItem.style.background = 'transparent';
        });
        menuItem.addEventListener('click', () => {
            item.action();
            contextMenu.remove();
        });
        contextMenu.appendChild(menuItem);
    });

    document.body.appendChild(contextMenu);

    const closeMenu = () => {
        if (document.body.contains(contextMenu)) {
            contextMenu.remove();
            document.removeEventListener('click', closeMenu);
        }
    };
    setTimeout(() => document.addEventListener('click', closeMenu), 10);
}

// ============ Case 右键菜单操作 ============

/**
 * 创建新脚本
 * @param {string} caseName - Case名称
 * @param {string} casePath - Case路径
 */
async function createNewScript(caseName, casePath) {
    window.rLog(`📝 创建新脚本被调用 - caseName: ${caseName}, casePath: ${casePath}`);

    const { path, ipcRenderer } = getGlobals();

    // 查找可用的文件名
    const scriptDir = path.join(casePath, 'script');
    let scriptName = 'new_script';
    let scriptPath = path.join(scriptDir, scriptName + '.tks');

    // 如果文件已存在，添加序号
    let counter = 1;
    while (true) {
        const checkResult = await ipcRenderer.invoke('file-exists', scriptPath);
        if (!checkResult.exists) {
            break;
        }
        scriptName = `new_script(${counter})`;
        scriptPath = path.join(scriptDir, scriptName + '.tks');
        counter++;
    }

    window.rLog(`📝 准备创建文件: ${scriptPath}`);

    try {
        // 使用 IPC handler 创建文件
        const result = await ipcRenderer.invoke('fs-create-file', scriptPath, '// 新测试脚本\n');
        window.rLog(`📝 IPC调用结果:`, result);

        if (result.success) {
            window.AppNotifications?.success('脚本创建成功');

            // 找到对应的 case-container 和 scripts-container
            const caseContainers = document.querySelectorAll('.case-container');
            let scriptsContainer = null;

            for (const container of caseContainers) {
                if (container.dataset.casePath === casePath) {
                    scriptsContainer = container.querySelector('.scripts-container');

                    // 如果 case 是折叠状态，先展开
                    const caseItem = container.querySelector('.case-item');
                    if (caseItem && !caseItem.classList.contains('expanded')) {
                        // 调用 TestcaseExplorerModule 的 toggleCaseFolder 来展开
                        if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.toggleCaseFolder) {
                            window.TestcaseExplorerModule.toggleCaseFolder(caseName, casePath);
                        }
                        // 重新获取 scripts-container（因为可能刚被创建）
                        await new Promise(resolve => setTimeout(resolve, 50));
                        scriptsContainer = container.querySelector('.scripts-container');
                    }
                    break;
                }
            }

            if (!scriptsContainer) {
                window.rError('📝 无法找到 scripts-container，回退到刷新文件树');
                if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                    await window.TestcaseExplorerModule.loadFileTree();
                }
                return;
            }

            // 手动创建新的脚本 DOM 元素
            const newScriptItem = createScriptItemDOM(scriptName + '.tks', scriptPath, casePath);
            scriptsContainer.appendChild(newScriptItem);

            window.rLog(`📝 已手动添加新脚本到DOM: ${scriptPath}`);

            // 等待短暂时间确保 DOM 更新
            await new Promise(resolve => setTimeout(resolve, 50));

            // 进入重命名模式
            startInlineRename(newScriptItem, scriptName + '.tks', scriptPath, true);
        } else {
            window.rError(`📝 创建失败: ${result.error}`);
            window.AppNotifications?.error(`创建失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('📝 创建脚本异常:', error);
        window.AppNotifications?.error(`创建失败: ${error.message}`);
    }
}

/**
 * 重命名 Case
 * @param {string} oldName - 旧名称
 * @param {string} oldPath - 旧路径
 */
async function renameCase(oldName, oldPath) {
    window.rLog(`📝 重命名Case被调用 - oldName: ${oldName}, oldPath: ${oldPath}`);

    // 找到对应的 case-container 元素
    const caseContainers = document.querySelectorAll('.case-container');
    for (const container of caseContainers) {
        if (container.dataset.casePath === oldPath) {
            window.rLog(`📝 找到Case元素，进入内联编辑模式`);
            startCaseInlineRename(container, oldName, oldPath);
            return;
        }
    }

    window.rError('📝 未找到对应的Case元素');
}

/**
 * 删除 Case
 * @param {string} caseName - Case名称
 * @param {string} casePath - Case路径
 */
async function deleteCase(caseName, casePath) {
    const { ipcRenderer } = getGlobals();

    if (!confirm(`确定要删除 case "${caseName}" 吗？此操作不可恢复。`)) {
        return;
    }

    try {
        const result = await ipcRenderer.invoke('fs-delete-directory', casePath);

        if (result.success) {
            window.AppNotifications?.success('删除成功');

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }
        } else {
            window.AppNotifications?.error(`删除失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('删除失败:', error);
        window.AppNotifications?.error(`删除失败: ${error.message}`);
    }
}

// ============ 脚本文件右键菜单操作 ============

/**
 * 重命名文件
 * @param {string} oldName - 旧文件名
 * @param {string} oldPath - 旧文件路径
 */
async function renameFile(oldName, oldPath) {
    window.rLog(`📝 重命名文件被调用 - oldName: ${oldName}, oldPath: ${oldPath}`);

    // 找到对应的 script-item 元素
    const scriptItems = document.querySelectorAll('.script-item');
    for (const item of scriptItems) {
        if (item.dataset.scriptPath === oldPath) {
            window.rLog(`📝 找到文件元素，进入内联编辑模式`);
            startInlineRename(item, oldName, oldPath, false);
            return;
        }
    }

    window.rError('📝 未找到对应的文件元素');
}

/**
 * 删除文件
 * @param {string} fileName - 文件名
 * @param {string} filePath - 文件路径
 */
async function deleteFile(fileName, filePath) {
    const { ipcRenderer } = getGlobals();

    if (!confirm(`确定要删除文件 "${fileName}" 吗？此操作不可恢复。`)) {
        return;
    }

    try {
        const result = await ipcRenderer.invoke('fs-delete-file', filePath);

        if (result.success) {
            window.AppNotifications?.success('删除成功');

            // 如果删除的是当前打开的文件，关闭对应的标签页
            if (window.AppGlobals.currentScript === filePath) {
                window.AppGlobals.currentScript = null;
                if (window.EditorManager && window.EditorManager.closeTabByPath) {
                    window.EditorManager.closeTabByPath(filePath);
                }
            }

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }
        } else {
            window.AppNotifications?.error(`删除失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('删除失败:', error);
        window.AppNotifications?.error(`删除失败: ${error.message}`);
    }
}

/**
 * 复制文件
 * @param {string} fileName - 文件名
 * @param {string} filePath - 文件路径
 */
async function copyFile(fileName, filePath) {
    window.rLog(`📝 复制文件被调用 - fileName: ${fileName}, filePath: ${filePath}`);

    const { path, ipcRenderer } = getGlobals();

    const nameWithoutExt = path.parse(fileName).name;
    const ext = path.parse(fileName).ext;

    // 生成默认的副本名称
    const newName = nameWithoutExt + '_copy';
    const parentDir = path.dirname(filePath);
    const newPath = path.join(parentDir, newName + ext);
    window.rLog(`📝 准备复制: ${filePath} -> ${newPath}`);

    try {
        const result = await ipcRenderer.invoke('fs-copy-file', filePath, newPath);
        window.rLog(`📝 复制IPC结果:`, result);

        if (result.success) {
            window.AppNotifications?.success('复制成功');

            // 找到原文件的 DOM 元素以确定所属的 case
            const originalScriptItems = document.querySelectorAll('.script-item');
            let casePath = null;
            let scriptsContainer = null;

            for (const item of originalScriptItems) {
                if (item.dataset.scriptPath === filePath) {
                    casePath = item.dataset.casePath;
                    // 找到所属 case 的 scripts-container
                    const caseContainer = item.closest('.case-item');
                    if (caseContainer) {
                        scriptsContainer = caseContainer.querySelector('.scripts-container');
                    }
                    break;
                }
            }

            if (!scriptsContainer || !casePath) {
                window.rError('📝 无法找到 scripts-container 或 casePath，回退到刷新文件树');
                if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                    await window.TestcaseExplorerModule.loadFileTree();
                }
                return;
            }

            // 手动创建新的脚本 DOM 元素
            const newScriptItem = createScriptItemDOM(newName + ext, newPath, casePath);
            scriptsContainer.appendChild(newScriptItem);

            window.rLog(`📝 已手动添加新脚本到DOM: ${newPath}`);

            // 等待短暂时间确保 DOM 更新
            await new Promise(resolve => setTimeout(resolve, 50));

            // 进入重命名模式
            startInlineRename(newScriptItem, newName + ext, newPath, true);
        } else {
            window.rError(`📝 复制失败: ${result.error}`);
            window.AppNotifications?.error(`复制失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('📝 复制异常:', error);
        window.AppNotifications?.error(`复制失败: ${error.message}`);
    }
}

// ============ 内联编辑功能 ============

/**
 * 启动 Case 内联重命名
 * @param {HTMLElement} caseContainer - Case 容器元素
 * @param {string} oldName - 旧名称
 * @param {string} oldPath - 旧路径
 */
function startCaseInlineRename(caseContainer, oldName, oldPath) {
    const { path, ipcRenderer } = getGlobals();

    const caseLabel = caseContainer.querySelector('.case-label');
    if (!caseLabel) {
        window.rError('📝 未找到 case-label 元素');
        return;
    }

    // 创建输入框
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'case-name-input';
    input.value = oldName;
    input.style.cssText = `
        background: var(--input-bg, #3c3c3c);
        border: 1px solid var(--accent-primary, #0e639c);
        border-radius: 2px;
        color: var(--text-primary, #cccccc);
        font-size: 13px;
        padding: 2px 4px;
        outline: none;
        width: 100%;
        font-family: inherit;
    `;

    // 替换 label 为 input
    caseLabel.style.display = 'none';
    caseLabel.parentNode.insertBefore(input, caseLabel.nextSibling);

    // 聚焦并选择文本
    input.focus();
    input.setSelectionRange(input.value.length, input.value.length);

    // 完成重命名的函数
    const finishRename = async () => {
        const newName = input.value.trim();

        // 移除输入框，恢复 label
        input.remove();
        caseLabel.style.display = '';

        if (!newName || newName === oldName) {
            window.rLog('📝 用户取消重命名或未修改');
            return;
        }

        const parentDir = path.dirname(oldPath);
        const newPath = path.join(parentDir, newName);
        window.rLog(`📝 准备重命名Case: ${oldPath} -> ${newPath}`);

        try {
            const result = await ipcRenderer.invoke('fs-rename', oldPath, newPath);
            window.rLog(`📝 重命名IPC结果:`, result);

            if (result.success) {
                window.AppNotifications?.success('重命名成功');

                // 直接更新 DOM，不重新加载文件树
                caseLabel.textContent = newName;
                caseContainer.dataset.casePath = newPath;
                caseContainer.dataset.caseName = newName;

                // 更新所有脚本的 casePath
                const scriptItems = caseContainer.querySelectorAll('.script-item');
                scriptItems.forEach(item => {
                    const oldScriptPath = item.dataset.scriptPath;
                    const scriptFileName = path.basename(oldScriptPath);
                    const newScriptPath = path.join(newPath, 'script', scriptFileName);
                    item.dataset.scriptPath = newScriptPath;
                    item.dataset.casePath = newPath;
                });

                // 更新 expandedCases 集合
                if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.updateExpandedCasePath) {
                    window.TestcaseExplorerModule.updateExpandedCasePath(oldPath, newPath);
                }

                window.rLog('📝 Case重命名完成，已更新DOM和展开状态');
            } else {
                window.rError(`📝 重命名失败: ${result.error}`);
                window.AppNotifications?.error(`重命名失败: ${result.error}`);
            }
        } catch (error) {
            window.rError('📝 重命名异常:', error);
            window.AppNotifications?.error(`重命名失败: ${error.message}`);
        }
    };

    // 取消重命名的函数
    const cancelRename = () => {
        window.rLog('📝 取消重命名');
        input.remove();
        caseLabel.style.display = '';
    };

    // 回车确认
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            e.stopPropagation();
            finishRename();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            cancelRename();
        }
    });

    // 失去焦点时确认
    input.addEventListener('blur', () => {
        finishRename();
    });

    // 阻止点击事件冒泡
    input.addEventListener('click', (e) => {
        e.stopPropagation();
    });
}

/**
 * 启动脚本文件内联重命名
 * @param {HTMLElement} scriptItem - 脚本元素
 * @param {string} oldName - 旧文件名
 * @param {string} oldPath - 旧文件路径
 * @param {boolean} selectAll - 是否全选文本
 */
function startInlineRename(scriptItem, oldName, oldPath, selectAll = false) {
    const { path, ipcRenderer } = getGlobals();

    const scriptLabel = scriptItem.querySelector('.script-label');
    if (!scriptLabel) {
        window.rError('📝 未找到 script-label 元素');
        return;
    }

    const nameWithoutExt = path.parse(oldName).name;
    const ext = path.parse(oldName).ext;

    // 保存原始文本
    const originalText = scriptLabel.textContent;

    // 创建输入框
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'script-name-input';
    input.value = nameWithoutExt;
    input.style.cssText = `
        background: var(--input-bg, #3c3c3c);
        border: 1px solid var(--accent-primary, #0e639c);
        border-radius: 2px;
        color: var(--text-primary, #cccccc);
        font-size: 13px;
        padding: 2px 4px;
        outline: none;
        width: 100%;
        font-family: inherit;
    `;

    // 替换 label 为 input
    scriptLabel.style.display = 'none';
    scriptItem.insertBefore(input, scriptLabel.nextSibling);

    // 聚焦并选择文本
    input.focus();
    if (selectAll) {
        input.select();
    } else {
        // 不全选时，光标放在末尾
        input.setSelectionRange(input.value.length, input.value.length);
    }

    // 完成重命名的函数
    const finishRename = async () => {
        const newName = input.value.trim();

        // 移除输入框，恢复 label
        input.remove();
        scriptLabel.style.display = '';

        if (!newName || newName === nameWithoutExt) {
            // 用户取消或未修改
            window.rLog('📝 用户取消重命名或未修改');
            return;
        }

        const parentDir = path.dirname(oldPath);
        const newPath = path.join(parentDir, newName + ext);
        window.rLog(`📝 准备重命名: ${oldPath} -> ${newPath}`);

        try {
            const result = await ipcRenderer.invoke('fs-rename', oldPath, newPath);
            window.rLog(`📝 重命名IPC结果:`, result);

            if (result.success) {
                window.AppNotifications?.success('重命名成功');

                // 如果重命名的是当前打开的文件，更新 EditorManager
                if (window.AppGlobals.currentScript === oldPath) {
                    window.AppGlobals.currentScript = newPath;
                    if (window.EditorManager && window.EditorManager.updateCurrentFilePath) {
                        window.EditorManager.updateCurrentFilePath(newPath);
                    }
                }

                // 直接更新 DOM，不重建整个树
                scriptLabel.textContent = newName + ext;
                scriptItem.dataset.scriptPath = newPath;
                scriptItem.dataset.scriptName = newName + ext;

                window.rLog(`📝 已更新DOM: 脚本路径 ${newPath}`);
            } else {
                window.rError(`📝 重命名失败: ${result.error}`);
                window.AppNotifications?.error(`重命名失败: ${result.error}`);
            }
        } catch (error) {
            window.rError('📝 重命名异常:', error);
            window.AppNotifications?.error(`重命名失败: ${error.message}`);
        }
    };

    // 取消重命名的函数
    const cancelRename = () => {
        window.rLog('📝 取消重命名');
        input.remove();
        scriptLabel.style.display = '';
    };

    // 回车确认
    input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            e.stopPropagation();
            finishRename();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            cancelRename();
        }
    });

    // 失去焦点时确认
    input.addEventListener('blur', () => {
        finishRename();
    });

    // 阻止点击事件冒泡
    input.addEventListener('click', (e) => {
        e.stopPropagation();
    });
}

// ============ 通用操作 ============

/**
 * 在文件管理器中显示
 * @param {string} targetPath - 目标路径
 */
async function showInFileManager(targetPath) {
    const { ipcRenderer } = getGlobals();

    try {
        const result = await ipcRenderer.invoke('fs-show-in-folder', targetPath);

        if (!result.success) {
            window.AppNotifications?.error(`打开失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('打开文件管理器失败:', error);
        window.AppNotifications?.error(`打开失败: ${error.message}`);
    }
}

// 导出模块
try {
    window.ContextMenuActions = {
        // Case 操作
        createNewScript,
        renameCase,
        deleteCase,

        // 文件操作
        renameFile,
        deleteFile,
        copyFile,

        // 通用操作
        showInFileManager
    };

    window.rLog('✅ 右键菜单操作模块已加载');
    window.rLog('ContextMenuActions 方法:', Object.keys(window.ContextMenuActions));
} catch (error) {
    window.rError('❌ 右键菜单操作模块导出失败:', error);
}
