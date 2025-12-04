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
    // 列宽规范：ID=50, 文本=350, 单选=120, 多选=100, 勾选=60, 时间=145, 按钮=80
    const thead = document.createElement('thead');
    thead.innerHTML = `
        <tr>
            <th class="col-id">ID</th>
            <th class="col-text">用例名称</th>
            <th class="col-text">备注</th>
            <th class="col-select">测试结果</th>
            <th class="col-checkbox">AI生成</th>
            <th class="col-multiselect">批量任务</th>
            <th class="col-text">AI Prompt</th>
            <th class="col-select">AI Result</th>
            <th class="col-text">AI Comment</th>
            <th class="col-datetime">创建时间</th>
            <th class="col-datetime">更新时间</th>
            <th class="col-action">编辑用例</th>
            <th class="col-action">测试脚本</th>
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

        // 测试结果选项
        const resultOptions = ['NOT TESTED', 'PASS', 'FAILED', 'N/A', 'BLOCKED'];
        const currentResult = tc.result || 'NOT TESTED';
        const resultClass = getResultClass(currentResult);

        // 批量任务选项
        const batchOptions = ['默认'];
        const currentBatch = tc.taskBatch || '';

        // 格式化时间
        const createdAt = tc.createdAt ? formatDateTime(tc.createdAt) : '-';
        const updatedAt = tc.updatedAt ? formatDateTime(tc.updatedAt) : '-';

        // AI Result 状态
        const aiResultValue = tc.aiResult || 'NOT TESTED';
        const aiResultClass = getResultClass(aiResultValue);

        // 编辑和脚本按钮
        const scriptBtnClass = caseExists ? 'btn-exists' : 'btn-create';
        const scriptBtnText = caseExists ? 'Open' : 'Create';
        const scriptBtnIcon = caseExists ? 'open-folder' : 'add-folder';

        row.innerHTML = `
            <td class="cell-id">${tc.id}</td>
            <td class="cell-editable cell-text" data-field="caseName" data-id="${tc.id}" title="${escapeHtml(tc.caseName)}">${escapeHtml(tc.caseName)}</td>
            <td class="cell-editable cell-text" data-field="note" data-id="${tc.id}" title="${escapeHtml(tc.note)}">${escapeHtml(tc.note) || '-'}</td>
            <td class="cell-editable cell-select" data-field="result" data-id="${tc.id}">
                <span class="ptc-status ${resultClass}">${currentResult}</span>
            </td>
            <td class="cell-editable cell-checkbox" data-field="aiTest" data-id="${tc.id}">
                <input type="checkbox" class="tc-checkbox" ${tc.aiTest ? 'checked' : ''} onchange="window.ProjectTestcaseManager.updateField(${tc.id}, 'aiTest', this.checked ? 1 : 0)" />
            </td>
            <td class="cell-editable cell-multiselect" data-field="taskBatch" data-id="${tc.id}">
                <span class="batch-tags">${currentBatch ? `<span class="batch-tag">${escapeHtml(currentBatch)}</span>` : '<span class="batch-empty">-</span>'}</span>
            </td>
            <td class="cell-editable cell-text" data-field="aiPrompt" data-id="${tc.id}" title="${escapeHtml(tc.aiPrompt)}">${escapeHtml(tc.aiPrompt) || '-'}</td>
            <td class="cell-editable cell-select" data-field="aiResult" data-id="${tc.id}">
                <span class="ptc-status ${aiResultClass}">${aiResultValue}</span>
            </td>
            <td class="cell-editable cell-text" data-field="aiComment" data-id="${tc.id}" title="${escapeHtml(tc.aiComment)}">${escapeHtml(tc.aiComment) || '-'}</td>
            <td class="cell-datetime" title="${tc.createdAt || ''}">${createdAt}</td>
            <td class="cell-datetime" title="${tc.updatedAt || ''}">${updatedAt}</td>
            <td class="cell-action">
                <button class="table-action-btn btn-edit" data-id="${tc.id}">
                    <img src="../../assets/icons/project/edit.svg" alt="Edit" /><span>Edit</span>
                </button>
            </td>
            <td class="cell-action">
                <button class="table-action-btn ${scriptBtnClass}" data-id="${tc.id}" data-exists="${caseExists}">
                    <img src="../../assets/icons/project/${scriptBtnIcon}.svg" alt="${scriptBtnText}" /><span>${scriptBtnText}</span>
                </button>
            </td>
        `;

        tbody.appendChild(row);
    }
    table.appendChild(tbody);

    // 绑定内联编辑事件
    bindInlineEditEvents(tbody);

    // 绑定按钮事件
    bindActionButtons(tbody);

    scrollContainer.appendChild(table);
    tableContainer.appendChild(scrollContainer);

    // 清除现有内容并添加表格容器
    testcaseList.innerHTML = '';
    testcaseList.appendChild(tableContainer);
}

// 绑定操作按钮事件
function bindActionButtons(tbody) {
    // 编辑按钮
    tbody.querySelectorAll('.btn-edit').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const id = parseInt(btn.dataset.id);
            editTestcase(id);
        });
    });

    // 脚本按钮 (Open/Create)
    tbody.querySelectorAll('.btn-exists, .btn-create').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            const id = parseInt(btn.dataset.id);
            const exists = btn.dataset.exists === 'true';
            if (exists) {
                openTestcase(id);
            } else {
                createCaseFolder(id);
            }
        });
    });
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

// 格式化日期时间
function formatDateTime(isoString) {
    if (!isoString) return '-';
    try {
        const date = new Date(isoString);
        const year = date.getFullYear();
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        return `${year}-${month}-${day} ${hours}:${minutes}`;
    } catch {
        return '-';
    }
}

// 获取测试结果的 CSS 类
function getResultClass(result) {
    switch (result) {
        case 'PASS': return 'status-pass';
        case 'FAILED': return 'status-failed';
        case 'BLOCKED': return 'status-blocked';
        case 'N/A': return 'status-na';
        default: return 'status-not-tested';
    }
}

// 绑定内联编辑事件
function bindInlineEditEvents(tbody) {
    // 文本单元格点击编辑
    tbody.querySelectorAll('.cell-text').forEach(cell => {
        cell.addEventListener('click', (e) => {
            if (cell.querySelector('input')) return; // 已经在编辑中
            startTextEdit(cell);
        });
    });

    // 结果单选下拉
    tbody.querySelectorAll('.cell-select').forEach(cell => {
        cell.addEventListener('click', (e) => {
            showResultDropdown(cell);
        });
    });

    // 批量任务多选下拉
    tbody.querySelectorAll('.cell-multiselect').forEach(cell => {
        cell.addEventListener('click', (e) => {
            showBatchDropdown(cell);
        });
    });
}

// 开始文本编辑
function startTextEdit(cell) {
    const field = cell.dataset.field;
    const id = parseInt(cell.dataset.id);
    const currentValue = cell.textContent === '-' ? '' : cell.textContent;

    // 使用 textarea 支持多行编辑
    const textarea = document.createElement('textarea');
    textarea.className = 'inline-edit-textarea';
    textarea.value = currentValue;

    // 保存原始高度，确保至少和当前单元格一样高
    const cellHeight = cell.offsetHeight;

    cell.innerHTML = '';
    cell.appendChild(textarea);

    // 设置初始高度并自动调整
    textarea.style.minHeight = Math.max(cellHeight - 16, 24) + 'px';
    autoResizeTextarea(textarea);

    textarea.focus();
    textarea.select();

    const saveEdit = async () => {
        const newValue = textarea.value.trim();
        await updateField(id, field, newValue);
        cell.textContent = newValue || '-';
        cell.title = newValue;
    };

    textarea.addEventListener('blur', saveEdit);
    textarea.addEventListener('input', () => autoResizeTextarea(textarea));
    textarea.addEventListener('keydown', (e) => {
        // Shift+Enter 换行，Enter 保存
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            textarea.blur();
        } else if (e.key === 'Escape') {
            cell.textContent = currentValue || '-';
            cell.title = currentValue;
        }
    });
}

// 自动调整 textarea 高度
function autoResizeTextarea(textarea) {
    textarea.style.height = 'auto';
    textarea.style.height = textarea.scrollHeight + 'px';
}

// 显示结果下拉菜单（通用，支持 result 和 aiResult）
function showResultDropdown(cell) {
    // 如果已有下拉菜单，关闭并返回
    if (document.querySelector('.inline-dropdown')) {
        closeAllDropdowns();
        return;
    }

    const id = parseInt(cell.dataset.id);
    const field = cell.dataset.field; // 'result' 或 'aiResult'
    const options = ['NOT TESTED', 'PASS', 'FAILED', 'N/A', 'BLOCKED'];
    const currentValue = cell.querySelector('.ptc-status')?.textContent || 'NOT TESTED';

    const dropdown = document.createElement('div');
    dropdown.className = 'inline-dropdown';

    options.forEach(opt => {
        const item = document.createElement('div');
        item.className = 'dropdown-item' + (opt === currentValue ? ' active' : '');
        item.innerHTML = `<span class="ptc-status ${getResultClass(opt)}">${opt}</span>`;
        item.addEventListener('click', async (e) => {
            e.stopPropagation();
            await updateField(id, field, opt);
            cell.innerHTML = `<span class="ptc-status ${getResultClass(opt)}">${opt}</span>`;
            closeAllDropdowns();
        });
        dropdown.appendChild(item);
    });

    // 使用 fixed 定位，添加到 body
    document.body.appendChild(dropdown);
    const rect = cell.getBoundingClientRect();
    dropdown.style.position = 'fixed';
    dropdown.style.top = rect.bottom + 'px';
    dropdown.style.left = rect.left + 'px';
    dropdown.style.minWidth = rect.width + 'px';

    // 点击外部关闭
    const closeHandler = (e) => {
        if (!dropdown.contains(e.target) && e.target !== cell && !cell.contains(e.target)) {
            closeAllDropdowns();
            document.removeEventListener('click', closeHandler);
        }
    };
    setTimeout(() => {
        document.addEventListener('click', closeHandler);
    }, 10);
}

// 显示批量任务下拉菜单
function showBatchDropdown(cell) {
    // 如果已有下拉菜单，关闭并返回
    if (document.querySelector('.inline-dropdown')) {
        closeAllDropdowns();
        return;
    }

    const id = parseInt(cell.dataset.id);
    const options = ['默认'];
    const tc = testcasesCache.find(t => t.id === id);
    const currentValues = tc?.taskBatch ? tc.taskBatch.split(',') : [];

    const dropdown = document.createElement('div');
    dropdown.className = 'inline-dropdown';

    options.forEach(opt => {
        const isSelected = currentValues.includes(opt);
        const item = document.createElement('div');
        item.className = 'dropdown-item checkbox-item' + (isSelected ? ' selected' : '');
        item.innerHTML = `
            <input type="checkbox" ${isSelected ? 'checked' : ''} />
            <span>${opt}</span>
        `;
        item.addEventListener('click', async (e) => {
            e.stopPropagation();
            const checkbox = item.querySelector('input');
            checkbox.checked = !checkbox.checked;
            item.classList.toggle('selected', checkbox.checked);

            // 更新选中值
            const newValues = [];
            dropdown.querySelectorAll('.checkbox-item').forEach(it => {
                if (it.querySelector('input').checked) {
                    newValues.push(it.querySelector('span').textContent);
                }
            });
            const newValue = newValues.join(',');
            await updateField(id, 'taskBatch', newValue);

            // 更新显示
            const tagsSpan = cell.querySelector('.batch-tags');
            if (newValues.length > 0) {
                tagsSpan.innerHTML = newValues.map(v => `<span class="batch-tag">${escapeHtml(v)}</span>`).join('');
            } else {
                tagsSpan.innerHTML = '<span class="batch-empty">-</span>';
            }
        });
        dropdown.appendChild(item);
    });

    // 使用 fixed 定位，添加到 body
    document.body.appendChild(dropdown);
    const rect = cell.getBoundingClientRect();
    dropdown.style.position = 'fixed';
    dropdown.style.top = rect.bottom + 'px';
    dropdown.style.left = rect.left + 'px';
    dropdown.style.minWidth = rect.width + 'px';

    // 点击外部关闭
    const closeHandler = (e) => {
        if (!dropdown.contains(e.target) && e.target !== cell && !cell.contains(e.target)) {
            closeAllDropdowns();
            document.removeEventListener('click', closeHandler);
        }
    };
    setTimeout(() => {
        document.addEventListener('click', closeHandler);
    }, 10);
}

// 定位下拉菜单
function positionDropdown(cell, dropdown) {
    const cellRect = cell.getBoundingClientRect();
    dropdown.style.minWidth = cellRect.width + 'px';
}

// 关闭所有下拉菜单
function closeAllDropdowns() {
    document.querySelectorAll('.inline-dropdown').forEach(d => d.remove());
}

// 更新单个字段
async function updateField(id, field, value) {
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    const { ipcRenderer } = getGlobals();

    try {
        const updateData = {};
        updateData[field] = value;

        const result = await ipcRenderer.invoke('db-testcase-update', projectPath, id, updateData);
        if (!result.success) {
            window.AppNotifications?.error(`更新失败: ${result.error}`);
        } else {
            // 更新缓存
            const tc = testcasesCache.find(t => t.id === id);
            if (tc) {
                tc[field] = value;
            }
        }
    } catch (error) {
        window.rError('更新字段失败:', error);
        window.AppNotifications?.error(`更新失败: ${error.message}`);
    }
}

// 导出模块
window.ProjectTestcaseManager = {
    refreshTestcaseList,
    openTestcase,
    createCaseFolder,
    showAddTestcaseModal,
    editTestcase,
    updateField
};
