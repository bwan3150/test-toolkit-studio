// Case Explorer 右键菜单操作模块
// 负责处理文件和目录的右键菜单操作（新建、重命名、删除、复制等）

window.rLog('🔧 开始加载右键菜单操作模块...');

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
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

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }

            // 等待 DOM 更新
            await new Promise(resolve => setTimeout(resolve, 100));

            // 找到新创建的文件并进入重命名模式
            const scriptItems = document.querySelectorAll('.script-item');
            for (const item of scriptItems) {
                if (item.dataset.scriptPath === scriptPath) {
                    window.rLog(`📝 找到新创建的文件，进入重命名模式`);
                    startInlineRename(item, scriptName + '.tks', scriptPath, true);
                    break;
                }
            }
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

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }

            // 等待 DOM 更新
            await new Promise(resolve => setTimeout(resolve, 100));

            // 找到复制的文件并进入重命名模式
            const scriptItems = document.querySelectorAll('.script-item');
            for (const item of scriptItems) {
                if (item.dataset.scriptPath === newPath) {
                    window.rLog(`📝 找到复制的文件，进入重命名模式`);
                    startInlineRename(item, newName + ext, newPath, true);
                    break;
                }
            }
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

                // 刷新文件树
                if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                    await window.TestcaseExplorerModule.loadFileTree();
                }
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

                // 刷新文件树
                if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                    await window.TestcaseExplorerModule.loadFileTree();
                }
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
