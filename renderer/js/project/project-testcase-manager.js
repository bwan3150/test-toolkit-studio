// 项目测试用例管理模块
// 负责测试用例的 CRUD 操作，基于数据库而非 CSV 文件
// 适配原有 UI 结构

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// 当前编辑的用例 ID
let editingTestcaseId = null;

// 所有用例数据缓存
let testcasesCache = [];

// 刷新用例列表（从数据库加载并显示）
async function refreshTestcaseList() {
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    const { ipcRenderer } = getGlobals();
    const testcaseList = document.getElementById('testcaseList');

    if (!testcaseList) return;

    try {
        const result = await ipcRenderer.invoke('db-testcase-getAll', projectPath);
        if (result.success) {
            testcasesCache = result.data || [];
            await renderTestcaseList(testcasesCache);
        } else {
            window.rError('获取用例列表失败:', result.error);
            testcasesCache = [];
            testcaseList.innerHTML = '<div class="text-muted">加载测试用例失败</div>';
        }
    } catch (error) {
        window.rError('获取用例列表失败:', error);
        testcasesCache = [];
        testcaseList.innerHTML = '<div class="text-muted">加载测试用例失败</div>';
    }
}

// 渲染用例列表（使用原有 UI 结构）
async function renderTestcaseList(testcases) {
    const { path, fsSync } = getGlobals();
    const testcaseList = document.getElementById('testcaseList');

    if (!testcaseList) return;

    if (testcases.length === 0) {
        testcaseList.innerHTML = `
            <div class="testcase-empty-state">
                <p>暂无测试用例</p>
                <span>点击上方「导入」或「新建」添加</span>
            </div>
        `;
        return;
    }

    // 检查哪些 case 文件夹已经存在
    const projectPath = window.AppGlobals.currentProject;

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
    thead.innerHTML = `
        <tr>
            <th>ID</th>
            <th>用例名称</th>
            <th>备注</th>
            <th>状态</th>
        </tr>
    `;
    table.appendChild(thead);

    // 创建表体
    const tbody = document.createElement('tbody');
    for (const tc of testcases) {
        const row = document.createElement('tr');
        row.className = 'testcase-row';
        row.dataset.id = tc.id;

        // 检查文件夹是否存在（只有 folderName 不为空才检查）
        let caseExists = false;
        if (tc.folderName) {
            const casePath = path.join(projectPath, tc.folderName);
            caseExists = fsSync.existsSync(casePath);
        }

        if (caseExists) {
            row.classList.add('case-created');
        }

        // 状态显示
        const statusText = tc.result === 'PASS' ? 'PASS' : tc.result === 'FAILED' ? 'FAILED' : 'N/A';
        const statusClass = tc.result === 'PASS' ? 'status-pass' : tc.result === 'FAILED' ? 'status-failed' : 'status-na';

        row.innerHTML = `
            <td>${tc.id}</td>
            <td title="${escapeHtml(tc.caseName)}">${escapeHtml(tc.caseName)}</td>
            <td title="${escapeHtml(tc.note)}">${escapeHtml(tc.note) || '-'}</td>
            <td><span class="ptc-status ${statusClass}">${statusText}</span></td>
        `;

        // 点击行打开用例
        row.style.cursor = 'pointer';
        row.addEventListener('click', () => {
            if (caseExists) {
                openTestcase(tc.id);
            } else {
                createCaseFolder(tc.id);
            }
        });

        tbody.appendChild(row);
    }
    table.appendChild(tbody);
    scrollContainer.appendChild(table);

    // 创建浮动按钮容器
    const floatingContainer = document.createElement('div');
    floatingContainer.className = 'table-floating-buttons';

    testcases.forEach((tc, index) => {
        let caseExists = false;
        if (tc.folderName) {
            const casePath = path.join(projectPath, tc.folderName);
            caseExists = fsSync.existsSync(casePath);
        }

        const btnWrapper = document.createElement('div');
        btnWrapper.className = 'floating-btn-wrapper';
        btnWrapper.dataset.rowIndex = index;

        // Edit 按钮
        const editBtn = document.createElement('button');
        editBtn.className = 'table-action-btn btn-edit';
        editBtn.innerHTML = `<img src="../../assets/icons/project/edit.svg" alt="Edit" /><span>Edit</span>`;
        editBtn.onclick = (e) => {
            e.stopPropagation();
            editTestcase(tc.id);
        };

        // Create/Open 按钮
        const actionBtn = document.createElement('button');
        actionBtn.className = 'table-action-btn';

        if (caseExists) {
            actionBtn.innerHTML = `<img src="../../assets/icons/project/open-folder.svg" alt="Open" /><span>Open</span>`;
            actionBtn.className += ' btn-exists';
            actionBtn.onclick = (e) => {
                e.stopPropagation();
                openTestcase(tc.id);
            };
        } else {
            actionBtn.innerHTML = `<img src="../../assets/icons/project/add-folder.svg" alt="Create" /><span>Create</span>`;
            actionBtn.className += ' btn-create';
            actionBtn.onclick = (e) => {
                e.stopPropagation();
                createCaseFolder(tc.id);
            };
        }

        btnWrapper.appendChild(editBtn);
        btnWrapper.appendChild(actionBtn);
        floatingContainer.appendChild(btnWrapper);
    });

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
    const tableContainer = scrollContainer.parentElement;

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

            const relativeTop = rowRect.top - containerRect.top;
            const isInScrollArea = relativeTop > -rowHeight && relativeTop < containerRect.height;

            let isBelowHeader = true;
            if (theadRect) {
                isBelowHeader = rowRect.bottom > theadRect.bottom;
            }

            if (isInScrollArea && isBelowHeader) {
                const rowTopRelativeToContainer = rowRect.top - tableContainerRect.top;
                btnWrapper.style.display = 'flex';
                btnWrapper.style.top = `${rowTopRelativeToContainer + rowHeight / 2}px`;
            } else {
                btnWrapper.style.display = 'none';
            }
        });
    }

    setTimeout(updateButtonPositions, 50);
    scrollContainer.addEventListener('scroll', updateButtonPositions);
    window.addEventListener('resize', updateButtonPositions);

    const observer = new ResizeObserver(() => {
        setTimeout(updateButtonPositions, 10);
    });
    observer.observe(table);
}

// 创建用例文件夹结构
async function createCaseFolder(id) {
    const testcase = testcasesCache.find(tc => tc.id === id);
    if (!testcase) {
        window.AppNotifications?.error('用例不存在');
        return;
    }

    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    const { path, fs, ipcRenderer } = getGlobals();

    try {
        // 首先为用例设置 folder_name（如果还没有）
        let folderName = testcase.folderName;
        if (!folderName) {
            const setResult = await ipcRenderer.invoke('db-testcase-setFolderName', projectPath, id);
            if (!setResult.success) {
                window.AppNotifications?.error(`设置文件夹名失败: ${setResult.error}`);
                return;
            }
            folderName = setResult.folderName;
        }

        const casePath = path.join(projectPath, folderName);

        // 创建 case 目录结构
        await fs.mkdir(casePath, { recursive: true });
        await fs.mkdir(path.join(casePath, 'result'), { recursive: true });
        await fs.mkdir(path.join(casePath, 'script'), { recursive: true });

        // 创建 config.json
        const config = {
            name: folderName.split('/').pop(),
            description: testcase.caseName,
            createdAt: new Date().toISOString()
        };
        await fs.writeFile(
            path.join(casePath, 'config.json'),
            JSON.stringify(config, null, 2)
        );

        // 创建样例脚本
        const sampleScript = `用例: ${folderName.split('/').pop()}
脚本名: script_001
详情:
    请在这里描述此测试脚本信息
步骤:
    启动 [com.example.app, .MainActivity]
    等待 [2000]
    点击 [{200,400}]
    断言 [{示例元素}, 存在]
`;
        await fs.writeFile(
            path.join(casePath, 'script', 'script_001.tks'),
            sampleScript
        );

        window.AppNotifications?.success(`用例文件夹已创建: ${folderName}`);

        // 刷新列表
        await refreshTestcaseList();

        // 导航到 testcase 页面并打开
        openTestcase(id);

    } catch (error) {
        window.rError('创建用例文件夹失败:', error);
        window.AppNotifications?.error(`创建失败: ${error.message}`);
    }
}

// 打开用例（跳转到 testcase 页面）
async function openTestcase(id) {
    const testcase = testcasesCache.find(tc => tc.id === id);
    if (!testcase) {
        window.AppNotifications?.error('用例不存在');
        return;
    }

    if (!testcase.folderName) {
        // 如果没有 folder_name，需要先创建
        await createCaseFolder(id);
        return;
    }

    // 导航到 testcase 页面
    window.PageNavigator.navigateTo('testcase');

    // 重新加载文件树
    await window.TestcaseController.loadFileTree();

    // 尝试展开对应的 case 文件夹
    const caseName = testcase.folderName.split('/').pop();
    setTimeout(async () => {
        const caseContainer = document.querySelector(`[data-case-path*="${caseName}"]`);
        if (caseContainer) {
            const scriptsContainer = caseContainer.querySelector('.scripts-container');
            if (scriptsContainer && scriptsContainer.classList.contains('collapsed')) {
                const casePath = caseContainer.dataset.casePath;
                await window.TestcaseController.toggleCaseFolder(caseContainer, casePath, true);
            }
        }
    }, 500);
}

// 显示新增用例弹窗
function showAddTestcaseModal() {
    if (window.TestcaseAddModal && window.TestcaseAddModal.show) {
        window.TestcaseAddModal.show();
    } else {
        window.rError('TestcaseAddModal 模块未加载');
        window.AppNotifications?.error('新建用例模块未加载');
    }
}

// 编辑用例
function editTestcase(id) {
    const testcase = testcasesCache.find(tc => tc.id === id);
    if (!testcase) {
        window.AppNotifications?.error('用例不存在');
        return;
    }

    if (window.TestcaseAddModal && window.TestcaseAddModal.showEdit) {
        window.TestcaseAddModal.showEdit(testcase);
    } else {
        window.rError('TestcaseAddModal 模块未加载');
        window.AppNotifications?.error('编辑用例模块未加载');
    }
}

// HTML 转义
function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// 导出模块
window.ProjectTestcaseManager = {
    refreshTestcaseList,
    openTestcase,
    createCaseFolder,
    showAddTestcaseModal,
    editTestcase
};
