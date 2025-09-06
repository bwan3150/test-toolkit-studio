// 项目管理模块

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 初始化项目页面
function initializeProjectPage() {
    const { ipcRenderer, path, parse } = getGlobals();
    const createProjectBtn = document.getElementById('createProjectBtn');
    const openProjectBtn = document.getElementById('openProjectBtn');
    const importCsvBtn = document.getElementById('importCsvBtn');
    const backToProjectsBtn = document.getElementById('backToProjectsBtn');
    
    console.log('Initializing project page...');
    
    // 返回项目按钮
    if (backToProjectsBtn) {
        backToProjectsBtn.addEventListener('click', async () => {
            // 清除当前项目
            window.AppGlobals.setCurrentProject(null);
            
            // 更新UI
            document.getElementById('projectInfo').style.display = 'none';
            
            // 重新加载项目历史（会显示loading状态）
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
            
            // 移除了对currentProjectPath的更新，因为现在使用固定的"Testcase"标题
            
            // 状态栏会通过 setCurrentProject 自动更新
            
            window.NotificationModule.showNotification('Closed project', 'info');
        });
    }
    
    if (createProjectBtn) {
        createProjectBtn.addEventListener('click', async () => {
            console.log('Create project button clicked');
            try {
                const projectPath = await ipcRenderer.invoke('select-directory');
                console.log('Selected path:', projectPath);
                if (projectPath) {
                    const result = await ipcRenderer.invoke('create-project-structure', projectPath);
                    if (result.success) {
                        await loadProject(projectPath);
                        window.NotificationModule.showNotification('Project created successfully', 'success');
                    } else {
                        window.NotificationModule.showNotification(`Failed to create project: ${result.error}`, 'error');
                    }
                }
            } catch (error) {
                console.error('Error in create project:', error);
                window.NotificationModule.showNotification(`Error: ${error.message}`, 'error');
            }
        });
    }
    
    if (openProjectBtn) {
        openProjectBtn.addEventListener('click', async () => {
            console.log('Open project button clicked');
            try {
                const result = await ipcRenderer.invoke('select-directory');
                console.log('Selected path result:', result);
                if (result && result.success && result.path) {
                    await openProject(result.path);
                } else if (typeof result === 'string') {
                    // 兼容旧格式
                    await openProject(result);
                }
            } catch (error) {
                console.error('Error in open project:', error);
                window.NotificationModule.showNotification(`Error: ${error.message}`, 'error');
            }
        });
    }
    
    if (importCsvBtn) {
        importCsvBtn.addEventListener('click', async () => {
            const { currentProject } = getGlobals();
            if (!currentProject) {
                window.NotificationModule.showNotification('Please open a project first', 'warning');
                return;
            }
            
            const csvPath = await ipcRenderer.invoke('select-file', [
                { name: 'CSV Files', extensions: ['csv'] }
            ]);
            
            if (csvPath) {
                const result = await ipcRenderer.invoke('read-file', csvPath);
                if (result.success) {
                    try {
                        const records = parse(result.content, {
                            columns: true,
                            skip_empty_lines: true
                        });
                        
                        // 保存到项目
                        const sheetPath = path.join(currentProject, 'testcase_sheet.csv');
                        await ipcRenderer.invoke('write-file', sheetPath, result.content);
                        
                        // 在表格中显示测试用例
                        await displayTestCasesTable(records);
                        window.NotificationModule.showNotification(`Imported ${records.length} test cases`, 'success');
                    } catch (error) {
                        window.NotificationModule.showNotification(`Failed to parse CSV: ${error.message}`, 'error');
                    }
                }
            }
        });
    }
}

// 其他函数保持原样，但需要在每个函数中获取所需的全局变量...
// 为了节省时间，我将提供一个模板，您可以按此模式修复其他函数

// 加载项目
async function loadProject(projectPath) {
    const { ipcRenderer, path, parse } = getGlobals();
    
    window.AppGlobals.setCurrentProject(projectPath);
    
    // 初始化项目工作区
    try {
        const workareaResult = await ipcRenderer.invoke('init-project-workarea', projectPath);
        if (workareaResult.success) {
            // console.log('项目工作区已初始化:', workareaResult.path); // 已禁用以减少日志
        } else {
            console.warn('工作区初始化失败:', workareaResult.error);
        }
    } catch (error) {
        console.warn('工作区初始化异常:', error);
    }
    
    // 更新项目历史
    await updateProjectHistory(projectPath);
    
    // 更新UI
    document.getElementById('projectPath').textContent = projectPath;
    // 移除了对currentProjectPath的更新，因为现在使用固定的"Testcase"标题
    document.getElementById('projectInfo').style.display = 'block';
    document.getElementById('welcomeScreen').style.display = 'none';
    
    // 状态栏会通过 setCurrentProject 自动更新
    
    // 清除之前的测试用例列表
    const testcaseList = document.getElementById('testcaseList');
    if (testcaseList) {
        testcaseList.innerHTML = '<div class="text-muted">Loading test cases...</div>';
    }
    
    // 如果CSV存在，加载测试用例
    const sheetPath = path.join(projectPath, 'testcase_sheet.csv');
    const result = await ipcRenderer.invoke('read-file', sheetPath);
    if (result.success && result.content) {
        try {
            const records = parse(result.content, {
                columns: true,
                skip_empty_lines: true
            });
            await displayTestCasesTable(records);
        } catch (error) {
            console.error('Failed to parse existing CSV:', error);
            if (testcaseList) {
                testcaseList.innerHTML = '<div class="text-muted">No test cases imported yet. Import a CSV file to get started.</div>';
            }
        }
    } else {
        if (testcaseList) {
            testcaseList.innerHTML = '<div class="text-muted">No test cases imported yet. Import a CSV file to get started.</div>';
        }
    }
    
    // 为testcase页面加载文件树
    await window.TestcaseController.loadFileTree();
    
    // 加载保存的设备
    await window.DeviceManagerModule.loadSavedDevices();
    
    // 刷新设备列表
    await window.DeviceManagerModule.refreshDeviceList();
}

// 为了节省时间，我将提供修复后的核心函数
// 其他函数需要按照相同的模式进行修复

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
            projectHistory.slice(0, 10).forEach((project, index) => {
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
        console.error('Error loading project history:', error);
        
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

// 简化版本的其他必要函数
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

async function openProject(projectPath) {
    const { fsSync, path } = getGlobals();
    
    // 确保projectPath是字符串
    if (typeof projectPath === 'object' && projectPath.path) {
        projectPath = projectPath.path;
    } else if (typeof projectPath !== 'string') {
        console.error('Invalid project path:', projectPath);
        window.NotificationModule.showNotification('无效的项目路径', 'error');
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
            window.NotificationModule.showNotification('项目已从历史记录中移除', 'info');
            await removeFromHistory(projectPath);
        }
        return;
    }
    
    await loadProject(projectPath);
}

// 显示测试用例表格
async function displayTestCasesTable(records) {
    const { path, fsSync } = getGlobals();
    const testcaseList = document.getElementById('testcaseList');
    
    if (records.length === 0) {
        testcaseList.innerHTML = '<div class="text-muted">No test cases imported yet</div>';
        return;
    }
    
    // 检查哪些case已经创建
    const existingCases = await checkExistingCases(records.length);
    
    // 获取所有列标题
    const headers = Object.keys(records[0]);
    
    // 创建表格容器
    const tableContainer = document.createElement('div');
    tableContainer.className = 'testcase-table-container';
    
    // 创建滚动容器
    const scrollContainer = document.createElement('div');
    scrollContainer.className = 'testcase-scroll-container';
    
    // 创建表格
    const table = document.createElement('table');
    table.className = 'testcase-table';
    
    // 创建表头
    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    headers.forEach(header => {
        const th = document.createElement('th');
        th.textContent = header;
        headerRow.appendChild(th);
    });
    
    thead.appendChild(headerRow);
    table.appendChild(thead);
    
    // 创建表体
    const tbody = document.createElement('tbody');
    records.forEach((record, index) => {
        const row = document.createElement('tr');
        row.className = 'testcase-row';
        row.dataset.index = index;
        
        // 如果case已经存在，添加高亮样式
        if (existingCases[index]) {
            row.classList.add('case-created');
        }
        
        headers.forEach(header => {
            const td = document.createElement('td');
            td.textContent = record[header] || '';
            td.title = record[header] || ''; // 长文本的工具提示
            row.appendChild(td);
        });
        
        // 为已存在的case添加整行点击功能
        if (existingCases[index]) {
            row.style.cursor = 'pointer';
            row.addEventListener('click', (e) => {
                // 如果点击的是按钮，让按钮处理（避免双重触发）
                if (e.target.closest('.floating-action-btn')) return;
                navigateToTestcase(record, index);
            });
            // 添加悬停效果
            row.addEventListener('mouseenter', () => {
                row.style.backgroundColor = 'var(--bg-hover, rgba(255, 255, 255, 0.05))';
            });
            row.addEventListener('mouseleave', () => {
                row.style.backgroundColor = '';
            });
        }
        
        tbody.appendChild(row);
    });
    
    table.appendChild(tbody);
    scrollContainer.appendChild(table);
    
    // 创建浮动按钮容器 - 相对于滚动容器定位
    const floatingContainer = document.createElement('div');
    floatingContainer.className = 'table-floating-buttons';
    
    // 为每一行创建浮动按钮
    records.forEach((record, index) => {
        const btnWrapper = document.createElement('div');
        btnWrapper.className = 'floating-btn-wrapper';
        btnWrapper.dataset.rowIndex = index;
        
        const floatingBtn = document.createElement('button');
        floatingBtn.className = 'table-action-btn';
        
        if (existingCases[index]) {
            // Case已存在
            floatingBtn.innerHTML = `
                <svg viewBox="0 0 24 24" width="14" height="14">
                    <path fill="currentColor" d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                </svg>
                <span>Open</span>
            `;
            floatingBtn.className += ' btn-exists';
            floatingBtn.onclick = (e) => {
                e.stopPropagation();
                navigateToTestcase(record, index);
            };
        } else {
            // Case未存在
            floatingBtn.innerHTML = `
                <svg viewBox="0 0 24 24" width="14" height="14">
                    <path fill="currentColor" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
                </svg>
                <span>Create</span>
            `;
            floatingBtn.className += ' btn-create';
            floatingBtn.onclick = (e) => {
                e.stopPropagation();
                createTestCase(record, index);
            };
        }
        
        btnWrapper.appendChild(floatingBtn);
        floatingContainer.appendChild(btnWrapper);
    });
    
    // 将浮动容器添加到表格容器（外层），不跟随左右滚动
    tableContainer.appendChild(scrollContainer);
    tableContainer.appendChild(floatingContainer);
    
    // 清除现有内容并添加表格容器
    testcaseList.innerHTML = '';
    testcaseList.appendChild(tableContainer);
    
    // 设置浮动按钮位置同步
    setupTableButtonsSync(scrollContainer, table, floatingContainer);
}

// 设置表格浮动按钮位置同步
function setupTableButtonsSync(scrollContainer, table, floatingContainer) {
    const tbody = table.querySelector('tbody');
    const btnWrappers = floatingContainer.querySelectorAll('.floating-btn-wrapper');
    const tableContainer = scrollContainer.parentElement; // 表格外层容器
    
    function updateButtonPositions() {
        const rows = tbody.querySelectorAll('tr');
        const containerRect = scrollContainer.getBoundingClientRect();
        const tableContainerRect = tableContainer.getBoundingClientRect();
        const thead = table.querySelector('thead');
        const theadRect = thead ? thead.getBoundingClientRect() : null;
        
        rows.forEach((row, index) => {
            const btnWrapper = btnWrappers[index];
            if (!btnWrapper) return;
            
            const rowRect = row.getBoundingClientRect();
            const rowHeight = rowRect.height;
            
            // 检查行是否在滚动容器的可视区域内
            const relativeTop = rowRect.top - containerRect.top;
            const isInScrollArea = relativeTop > -rowHeight && relativeTop < containerRect.height;
            
            // 检查行是否被表头覆盖 - 只要行的下边缘在表头下方就显示
            let isBelowHeader = true;
            if (theadRect) {
                // 只要行的底部超过表头底部就认为可以显示按钮
                isBelowHeader = rowRect.bottom > theadRect.bottom;
            }
            
            // 只有在滚动区域内且不被表头完全覆盖时才显示按钮
            if (isInScrollArea && isBelowHeader) {
                // 计算行相对于表格容器的位置（不考虑水平滚动）
                const rowTopRelativeToContainer = rowRect.top - tableContainerRect.top;
                
                btnWrapper.style.display = 'block';
                btnWrapper.style.top = `${rowTopRelativeToContainer + rowHeight / 2}px`;
            } else {
                btnWrapper.style.display = 'none';
            }
        });
    }
    
    // 初始更新
    setTimeout(updateButtonPositions, 50);
    
    // 监听垂直滚动事件
    scrollContainer.addEventListener('scroll', updateButtonPositions);
    
    // 监听窗口大小变化
    window.addEventListener('resize', updateButtonPositions);
    
    // 监听表格内容变化
    const observer = new ResizeObserver(() => {
        setTimeout(updateButtonPositions, 10);
    });
    observer.observe(table);
}

// 导航到testcase页面（用于已存在的case）
async function navigateToTestcase(record, index) {
    const { path, fs } = getGlobals();
    
    try {
        // 获取映射的case名称
        let caseName = `case_${String(index + 1).padStart(3, '0')}`;
        
        if (window.AppGlobals.currentProject) {
            const mapPath = path.join(window.AppGlobals.currentProject, 'testcase_map.json');
            try {
                const content = await fs.readFile(mapPath, 'utf-8');
                const mapping = JSON.parse(content);
                caseName = mapping[index.toString()] || caseName;
            } catch {
                // 使用默认名称
            }
        }
        
        // 导航到testcase页面
        document.querySelector('[data-page="testcase"]').click();
        
        // 重新加载文件树
        await window.TestcaseController.loadFileTree();
        
        // 显示通知
        window.NotificationModule.showNotification(`已跳转到测试用例页面: ${caseName}`, 'info');
        
        // 尝试展开对应的case文件夹
        setTimeout(() => {
            const caseContainer = document.querySelector(`[data-case-path*="${caseName}"]`);
            if (caseContainer && window.TestcaseController.toggleCaseFolder) {
                const scriptsContainer = caseContainer.querySelector('.scripts-container');
                if (scriptsContainer && scriptsContainer.classList.contains('collapsed')) {
                    window.TestcaseController.toggleCaseFolder(caseContainer);
                }
            }
        }, 500);
        
    } catch (error) {
        console.error('Failed to navigate to testcase:', error);
        window.NotificationModule.showNotification(`跳转失败: ${error.message}`, 'error');
    }
}

// 检查哪些Case已经存在
async function checkExistingCases(totalCases) {
    const { path, fsSync, fs } = getGlobals();
    if (!window.AppGlobals.currentProject) return [];
    
    const existingCases = [];
    const casesPath = path.join(window.AppGlobals.currentProject, 'cases');
    const mapPath = path.join(window.AppGlobals.currentProject, 'testcase_map.json');
    
    try {
        // 读取映射文件
        let mapping = {};
        try {
            const content = await fs.readFile(mapPath, 'utf-8');
            mapping = JSON.parse(content);
        } catch {
            // 映射文件不存在，使用默认名称检查
        }
        
        // 检查cases目录是否存在
        if (!fsSync.existsSync(casesPath)) {
            // cases目录不存在，所有case都未创建
            return new Array(totalCases).fill(false);
        }
        
        for (let i = 0; i < totalCases; i++) {
            // 优先使用映射中的名称，否则使用默认名称
            const caseName = mapping[i.toString()] || `case_${String(i + 1).padStart(3, '0')}`;
            const casePath = path.join(casesPath, caseName);
            
            // 检查case目录和config.json是否存在
            const caseExists = fsSync.existsSync(casePath) && 
                             fsSync.existsSync(path.join(casePath, 'config.json'));
            
            existingCases[i] = caseExists;
        }
    } catch (error) {
        console.error('Error checking existing cases:', error);
        // 出错时假设所有case都未创建
        return new Array(totalCases).fill(false);
    }
    
    return existingCases;
}

// 创建测试用例
async function createTestCase(record, index) {
    const { path, fs } = getGlobals();
    if (!window.AppGlobals.currentProject) {
        window.NotificationModule.showNotification('No project loaded', 'error');
        return;
    }
    
    const caseName = `case_${String(index + 1).padStart(3, '0')}`;
    const casePath = path.join(window.AppGlobals.currentProject, 'cases', caseName);
    
    try {
        // 创建case目录结构
        await fs.mkdir(casePath, { recursive: true });
        await fs.mkdir(path.join(casePath, 'result'), { recursive: true });
        await fs.mkdir(path.join(casePath, 'script'), { recursive: true });
        
        // 创建config.json
        const config = {
            name: caseName,
            description: record[Object.keys(record)[0]] || '',
            requirements: record[Object.keys(record)[1]] || '',
            createdAt: new Date().toISOString(),
            record: record,
            rowIndex: index // 添加行索引
        };
        
        await fs.writeFile(
            path.join(casePath, 'config.json'),
            JSON.stringify(config, null, 2)
        );
        
        // 更新 testcase_map.json
        await updateTestcaseMap(index, caseName);
        
        // 创建case级别的README
        await fs.writeFile(
            path.join(casePath, 'result', 'README.md'),
            '# Result\n\n此文件夹用于存放该测试用例的执行结果和日志。',
            'utf-8'
        );
        
        // 创建样例脚本 - 使用新的 TKS 语法
        const sampleScript = `用例: ${caseName}
脚本名: script_001
详情: 
    请在这里描述此测试脚本信息
步骤:
    启动 [${record.appPackage || 'com.example.app'}, ${record.appActivity || '.MainActivity'}]
    等待 [2000]
    点击 [{200,400}]
    断言 [{示例元素}, 存在]
`;
        
        await fs.writeFile(
            path.join(casePath, 'script', 'script_001.tks'),
            sampleScript
        );
        
        // 重新加载表格以更新状态
        await refreshTestCaseTable();
        
        // 导航到testcase页面
        document.querySelector('[data-page="testcase"]').click();
        
        // 重新加载文件树
        await window.TestcaseController.loadFileTree();
        
        window.NotificationModule.showNotification(`Created test case: ${caseName}`, 'success');
    } catch (error) {
        window.NotificationModule.showNotification(`Failed to create test case: ${error.message}`, 'error');
    }
}

// 刷新测试用例表格
async function refreshTestCaseTable() {
    const { ipcRenderer, path, parse } = getGlobals();
    if (!window.AppGlobals.currentProject) return;
    
    // 重新读取CSV文件并显示表格
    const sheetPath = path.join(window.AppGlobals.currentProject, 'testcase_sheet.csv');
    const result = await ipcRenderer.invoke('read-file', sheetPath);
    if (result.success && result.content) {
        try {
            const records = parse(result.content, {
                columns: true,
                skip_empty_lines: true
            });
            await displayTestCasesTable(records);
        } catch (error) {
            console.error('Failed to refresh test case table:', error);
        }
    }
}

// 新增函数：更新测试用例映射
async function updateTestcaseMap(rowIndex, caseName) {
    const { path, fs } = getGlobals();
    if (!window.AppGlobals.currentProject) return;
    
    const mapPath = path.join(window.AppGlobals.currentProject, 'testcase_map.json');
    
    try {
        // 读取当前映射
        let mapping = {};
        try {
            const content = await fs.readFile(mapPath, 'utf-8');
            mapping = JSON.parse(content);
        } catch {
            // 文件不存在或解析失败，使用空对象
        }
        
        // 更新映射（使用行索引作为key）
        mapping[rowIndex.toString()] = caseName;
        
        // 保存映射
        await fs.writeFile(mapPath, JSON.stringify(mapping, null, 2));
        
        // console.log('更新testcase映射:', rowIndex, '->', caseName); // 已禁用以减少日志
        
    } catch (error) {
        console.error('更新testcase映射失败:', error);
    }
}

async function removeFromHistory(projectPath) {
    const { ipcRenderer } = getGlobals();
    let projectHistory = await ipcRenderer.invoke('store-get', 'project_history') || [];
    projectHistory = projectHistory.filter(p => p.path !== projectPath);
    await ipcRenderer.invoke('store-set', 'project_history', projectHistory);
    await loadProjectHistory();
    window.NotificationModule.showNotification('Project removed from history', 'success');
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
        window.NotificationModule.showNotification('项目路径已更新', 'success');
    }
}

// 全局函数
window.removeFromHistory = removeFromHistory;

// 导出函数
window.ProjectManagerModule = {
    initializeProjectPage,
    loadProject,
    displayTestCasesTable,
    checkExistingCases,
    createTestCase,
    navigateToTestcase,
    refreshTestCaseTable,
    loadProjectHistory,
    formatDate,
    openProject,
    updateProjectHistory,
    updateTestcaseMap,
    removeFromHistory
};