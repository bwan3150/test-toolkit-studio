// 项目管理模块
// 负责项目的创建、打开、历史记录管理

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 初始化项目页面
function initializeProjectPage() {
    const { ipcRenderer } = getGlobals();
    const createProjectBtn = document.getElementById('createProjectBtn');
    const openProjectBtn = document.getElementById('openProjectBtn');
    const backToProjectsBtn = document.getElementById('backToProjectsBtn');
    const importCsvBtn = document.getElementById('importCsvBtn');

    // 返回项目列表按钮
    if (backToProjectsBtn) {
        backToProjectsBtn.addEventListener('click', async () => {
            // 清除当前项目
            window.AppGlobals.setCurrentProject(null);

            // 更新UI
            document.getElementById('projectInfo').style.display = 'none';

            // 重新加载项目历史
            await loadProjectHistory();

            // 清除测试用例页面
            const fileTree = document.getElementById('fileTree');
            if (fileTree) fileTree.innerHTML = '';

            // 清除编辑器标签
            window.AppGlobals.setOpenTabs([]);
            const editorTabs = document.getElementById('editorTabs');
            if (editorTabs) {
                editorTabs.innerHTML = '';
            }

            // 清空编辑器
            if (window.AppGlobals.codeEditor) {
                window.AppGlobals.codeEditor.value = '';
                window.AppGlobals.codeEditor.placeholder = '在Project页面选择测试项并创建Case后, 在左侧文件树点击对应Case下的.tks自动化脚本开始编辑';
            }
        });
    }

    // 创建项目按钮
    if (createProjectBtn) {
        createProjectBtn.addEventListener('click', async () => {
            try {
                const projectPath = await ipcRenderer.invoke('select-directory');
                if (projectPath) {
                    const result = await ipcRenderer.invoke('create-project-structure', projectPath);
                    if (result.success) {
                        await loadProject(projectPath);
                        window.AppNotifications?.success('Project created successfully');
                    } else {
                        window.AppNotifications?.error(`Failed to create project: ${result.error}`);
                    }
                }
            } catch (error) {
                window.rError('Error in create project:', error);
                window.AppNotifications?.error(`Error: ${error.message}`);
            }
        });
    }

    // 打开项目按钮
    if (openProjectBtn) {
        openProjectBtn.addEventListener('click', async () => {
            try {
                const result = await ipcRenderer.invoke('select-directory');
                if (result && result.success && result.path) {
                    await openProject(result.path);
                } else if (typeof result === 'string') {
                    // 兼容旧格式
                    await openProject(result);
                }
            } catch (error) {
                window.rError('Error in open project:', error);
                window.AppNotifications?.error(`Error: ${error.message}`);
            }
        });
    }

    // 导入 CSV 按钮
    if (importCsvBtn) {
        importCsvBtn.addEventListener('click', () => {
            if (window.CsvImportModal && window.CsvImportModal.show) {
                window.CsvImportModal.show();
            } else {
                window.rError('CsvImportModal 模块未加载');
                window.AppNotifications?.error('CSV 导入模块未加载');
            }
        });
    }

    // 初始化测试用例管理器
    if (window.ProjectTestcaseManager && window.ProjectTestcaseManager.initialize) {
        window.ProjectTestcaseManager.initialize();
    }

    // 初始化 CSV 导入模态框
    if (window.CsvImportModal && window.CsvImportModal.initialize) {
        window.CsvImportModal.initialize();
    }

    // 初始化新建测试用例模态框
    if (window.TestcaseAddModal && window.TestcaseAddModal.initialize) {
        window.TestcaseAddModal.initialize();
    }
}

// 加载项目
async function loadProject(projectPath) {
    const { ipcRenderer } = getGlobals();

    window.AppGlobals.setCurrentProject(projectPath);

    // 初始化项目工作区
    try {
        const workareaResult = await ipcRenderer.invoke('init-project-workarea', projectPath);
        if (!workareaResult.success) {
            window.rLog('工作区初始化失败:', workareaResult.error);
        }
    } catch (error) {
        window.rLog('工作区初始化异常:', error);
    }

    // 更新项目历史
    await updateProjectHistory(projectPath);

    // 更新UI
    document.getElementById('projectPath').textContent = projectPath;
    document.getElementById('projectInfo').style.display = 'block';
    document.getElementById('welcomeScreen').style.display = 'none';
    document.getElementById('projectLoading').style.display = 'none';

    // 加载测试用例列表
    if (window.ProjectTestcaseManager && window.ProjectTestcaseManager.refreshTestcaseList) {
        await window.ProjectTestcaseManager.refreshTestcaseList();
    }

    // 为testcase页面加载文件树
    if (window.TestcaseController && window.TestcaseController.loadFileTree) {
        await window.TestcaseController.loadFileTree();
    }

    // 加载保存的设备
    if (window.DeviceManagerModule && window.DeviceManagerModule.loadSavedDevices) {
        await window.DeviceManagerModule.loadSavedDevices();
    }

    // 刷新设备列表
    if (window.DeviceManagerModule && window.DeviceManagerModule.refreshDeviceList) {
        await window.DeviceManagerModule.refreshDeviceList();
    }
}

// 更新项目历史
async function updateProjectHistory(projectPath) {
    const { ipcRenderer } = getGlobals();
    let projectHistory = await ipcRenderer.invoke('store-get', 'project_history') || [];

    // 如果已经存在则移除
    projectHistory = projectHistory.filter(p => p.path !== projectPath);

    // 添加到开头
    projectHistory.unshift({
        path: projectPath,
        lastAccessed: new Date().toISOString()
    });

    // 只保留最近10个项目
    projectHistory = projectHistory.slice(0, 10);

    await ipcRenderer.invoke('store-set', 'project_history', projectHistory);
}

// 加载项目历史
async function loadProjectHistory() {
    // 显示loading状态
    const projectLoading = document.getElementById('projectLoading');
    const welcomeScreen = document.getElementById('welcomeScreen');

    if (projectLoading) projectLoading.style.display = 'flex';
    if (welcomeScreen) welcomeScreen.style.display = 'none';

    try {
        // 模拟加载延迟以显示loading效果
        await new Promise(resolve => setTimeout(resolve, 800));

        const { ipcRenderer, path, fsSync } = getGlobals();
        let projectHistory = await ipcRenderer.invoke('store-get', 'project_history') || [];

        // 验证项目路径是否仍然有效，移除无效的项目
        const validProjects = [];
        for (const project of projectHistory) {
            if (fsSync.existsSync(project.path)) {
                validProjects.push(project);
            }
        }

        // 如果有无效项目被移除，更新存储
        if (validProjects.length !== projectHistory.length) {
            await ipcRenderer.invoke('store-set', 'project_history', validProjects);
            projectHistory = validProjects;
        }

        const welcomeContent = document.getElementById('welcomeContent');
        const recentProjects = document.getElementById('recentProjects');
        const projectList = document.getElementById('projectList');
        const welcomeScreenEl = document.querySelector('.project-welcome');

        if (projectHistory.length > 0) {
            // 有项目时隐藏欢迎内容
            if (welcomeContent) welcomeContent.style.display = 'none';

            // 显示项目时移除居中类
            if (welcomeScreenEl) welcomeScreenEl.classList.remove('show-welcome');

            if (recentProjects && projectList) {
                projectList.innerHTML = '';

                // 按最后访问日期排序（最近的在前）
                projectHistory.sort((a, b) => new Date(b.lastAccessed) - new Date(a.lastAccessed));

                // 显示所有最近项目（限制为10个）
                projectHistory.slice(0, 10).forEach((project) => {
                    const projectItem = document.createElement('div');
                    projectItem.className = 'project-item';

                    const projectName = path.basename(project.path);
                    const lastAccessed = new Date(project.lastAccessed);
                    const dateStr = formatDate(lastAccessed);

                    projectItem.innerHTML = `
                        <svg class="project-item-icon" viewBox="0 0 24 24">
                            <path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>
                        </svg>
                        <div class="project-item-info">
                            <div class="project-item-name">${projectName}</div>
                            <div class="project-item-path" title="${project.path}">${project.path}</div>
                        </div>
                        <div class="project-item-date">${dateStr}</div>
                        <button class="project-item-remove" onclick="removeFromHistory('${project.path.replace(/'/g, "\\'").replace(/\\/g, "\\\\")}')" title="Remove from history">
                            <svg viewBox="0 0 24 24" width="16" height="16">
                                <path fill="currentColor" d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                            </svg>
                        </button>
                    `;

                    projectItem.addEventListener('click', (e) => {
                        if (!e.target.closest('.project-item-remove')) {
                            openProject(project.path);
                        }
                    });

                    projectList.appendChild(projectItem);
                });

                recentProjects.style.display = 'block';
            }
        } else {
            // 没有项目时显示欢迎内容
            if (welcomeContent) welcomeContent.style.display = 'block';
            if (recentProjects) recentProjects.style.display = 'none';

            // 为居中欢迎内容添加类
            if (welcomeScreenEl) welcomeScreenEl.classList.add('show-welcome');
        }

        // 隐藏loading，显示welcome screen
        if (projectLoading) projectLoading.style.display = 'none';
        if (welcomeScreen) welcomeScreen.style.display = 'flex';

    } catch (error) {
        window.rError('Error loading project history:', error);

        // 出错时也要隐藏loading，显示欢迎界面
        if (projectLoading) projectLoading.style.display = 'none';
        if (welcomeScreen) welcomeScreen.style.display = 'flex';

        // 显示欢迎内容
        const welcomeContent = document.getElementById('welcomeContent');
        const recentProjects = document.getElementById('recentProjects');
        if (welcomeContent) welcomeContent.style.display = 'block';
        if (recentProjects) recentProjects.style.display = 'none';
    }
}

// 格式化日期
function formatDate(date) {
    const now = new Date();
    const diff = now - date;
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
        return 'Today';
    } else if (days === 1) {
        return 'Yesterday';
    } else if (days < 7) {
        return `${days} days ago`;
    } else {
        return date.toLocaleDateString();
    }
}

// 打开项目
async function openProject(projectPath) {
    const { fsSync, path } = getGlobals();

    // 确保projectPath是字符串
    if (typeof projectPath === 'object' && projectPath.path) {
        projectPath = projectPath.path;
    } else if (typeof projectPath !== 'string') {
        window.rError('Invalid project path:', projectPath);
        window.AppNotifications?.error('无效的项目路径');
        return;
    }

    // 检查目录是否存在
    if (!fsSync.existsSync(projectPath)) {
        // 询问用户是否要重新选择项目路径或从历史记录中移除
        const choice = confirm(
            `项目文件夹未找到:\n${projectPath}\n\n点击"确定"重新选择项目文件夹，点击"取消"从历史记录中移除该项目。`
        );

        if (choice) {
            // 用户选择重新选择文件夹
            const { ipcRenderer } = getGlobals();
            const result = await ipcRenderer.invoke('select-directory');
            if (result.success && result.path) {
                // 更新历史记录中的路径
                await updateProjectPath(projectPath, result.path);
                await loadProject(result.path);
            }
        } else {
            // 用户选择从历史记录中移除
            await removeFromHistory(projectPath);
        }
        return;
    }

    await loadProject(projectPath);
}

// 从历史记录中移除项目
async function removeFromHistory(projectPath) {
    const { ipcRenderer } = getGlobals();
    let projectHistory = await ipcRenderer.invoke('store-get', 'project_history') || [];
    projectHistory = projectHistory.filter(p => p.path !== projectPath);
    await ipcRenderer.invoke('store-set', 'project_history', projectHistory);
    await loadProjectHistory();
    window.AppNotifications?.success('Project removed from history');
}

// 更新项目路径
async function updateProjectPath(oldPath, newPath) {
    const { ipcRenderer } = getGlobals();
    let projectHistory = await ipcRenderer.invoke('store-get', 'project_history') || [];

    // 找到并更新路径
    const projectIndex = projectHistory.findIndex(p => p.path === oldPath);
    if (projectIndex !== -1) {
        projectHistory[projectIndex].path = newPath;
        projectHistory[projectIndex].lastAccessed = new Date().toISOString();
        await ipcRenderer.invoke('store-set', 'project_history', projectHistory);
        await loadProjectHistory();
        window.AppNotifications?.success('项目路径已更新');
    }
}

// 全局函数
window.removeFromHistory = removeFromHistory;

// 导出函数
window.ProjectManagerModule = {
    initializeProjectPage,
    loadProject,
    loadProjectHistory,
    formatDate,
    openProject,
    updateProjectHistory,
    removeFromHistory
};
