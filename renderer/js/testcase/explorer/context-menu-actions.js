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
    const { path, ipcRenderer } = getGlobals();

    // 弹窗输入脚本名称
    const scriptName = prompt('请输入脚本名称（不含扩展名）:');
    if (!scriptName || scriptName.trim() === '') {
        return;
    }

    const scriptDir = path.join(casePath, 'script');
    const scriptPath = path.join(scriptDir, scriptName.trim() + '.tks');

    try {
        // 使用 IPC handler 创建文件
        const result = await ipcRenderer.invoke('fs-create-file', scriptPath, '// 新测试脚本\n');

        if (result.success) {
            window.AppNotifications?.success('脚本创建成功');

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }

            // 打开新创建的文件
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.openFile) {
                await window.TestcaseExplorerModule.openFile(scriptPath);
            }
        } else {
            window.AppNotifications?.error(`创建失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('创建脚本失败:', error);
        window.AppNotifications?.error(`创建失败: ${error.message}`);
    }
}

/**
 * 重命名 Case
 * @param {string} oldName - 旧名称
 * @param {string} oldPath - 旧路径
 */
async function renameCase(oldName, oldPath) {
    const { path, ipcRenderer } = getGlobals();

    const newName = prompt('请输入新名称:', oldName);
    if (!newName || newName.trim() === '' || newName === oldName) {
        return;
    }

    const parentDir = path.dirname(oldPath);
    const newPath = path.join(parentDir, newName.trim());

    try {
        const result = await ipcRenderer.invoke('fs-rename', oldPath, newPath);

        if (result.success) {
            window.AppNotifications?.success('重命名成功');

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }
        } else {
            window.AppNotifications?.error(`重命名失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('重命名失败:', error);
        window.AppNotifications?.error(`重命名失败: ${error.message}`);
    }
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
    const { path, ipcRenderer } = getGlobals();

    const nameWithoutExt = path.parse(oldName).name;
    const ext = path.parse(oldName).ext;

    const newName = prompt('请输入新名称:', nameWithoutExt);
    if (!newName || newName.trim() === '' || newName === nameWithoutExt) {
        return;
    }

    const parentDir = path.dirname(oldPath);
    const newPath = path.join(parentDir, newName.trim() + ext);

    try {
        const result = await ipcRenderer.invoke('fs-rename', oldPath, newPath);

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
            window.AppNotifications?.error(`重命名失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('重命名失败:', error);
        window.AppNotifications?.error(`重命名失败: ${error.message}`);
    }
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
    const { path, ipcRenderer } = getGlobals();

    const nameWithoutExt = path.parse(fileName).name;
    const ext = path.parse(fileName).ext;

    const newName = prompt('请输入新文件名称:', nameWithoutExt + '_copy');
    if (!newName || newName.trim() === '') {
        return;
    }

    const parentDir = path.dirname(filePath);
    const newPath = path.join(parentDir, newName.trim() + ext);

    try {
        const result = await ipcRenderer.invoke('fs-copy-file', filePath, newPath);

        if (result.success) {
            window.AppNotifications?.success('复制成功');

            // 刷新文件树
            if (window.TestcaseExplorerModule && window.TestcaseExplorerModule.loadFileTree) {
                await window.TestcaseExplorerModule.loadFileTree();
            }
        } else {
            window.AppNotifications?.error(`复制失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('复制失败:', error);
        window.AppNotifications?.error(`复制失败: ${error.message}`);
    }
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
