// 新建/编辑测试用例模态框控制器
// 用于逐条新建或编辑测试用例

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// 当前是否为编辑模式
let isEditMode = false;
// 编辑模式下的用例 ID
let editingId = null;

// 显示新建用例模态框
async function showTestcaseAddModal() {
    isEditMode = false;
    editingId = null;

    const modal = document.getElementById('testcaseAddModal');
    if (!modal) {
        window.rError('找不到新建用例模态框');
        return;
    }

    // 设置标题
    const titleEl = document.getElementById('testcaseAddModalTitle');
    if (titleEl) titleEl.textContent = '新建测试用例';

    // 设置按钮文字
    const confirmBtn = document.getElementById('confirmTestcaseAdd');
    if (confirmBtn) confirmBtn.textContent = '创建';

    // 重置表单
    resetTestcaseAddForm();

    // 获取下一个可用 ID
    await fetchNextId();

    // 显示模态框
    modal.style.display = 'flex';

    // 聚焦到用例名称输入框
    setTimeout(() => {
        document.getElementById('tamCaseNameInput')?.focus();
    }, 100);
}

// 显示编辑用例模态框
async function showTestcaseEditModal(testcase) {
    isEditMode = true;
    editingId = testcase.id;

    const modal = document.getElementById('testcaseAddModal');
    if (!modal) {
        window.rError('找不到编辑用例模态框');
        return;
    }

    // 设置标题
    const titleEl = document.getElementById('testcaseAddModalTitle');
    if (titleEl) titleEl.textContent = '编辑测试用例';

    // 设置按钮文字
    const confirmBtn = document.getElementById('confirmTestcaseAdd');
    if (confirmBtn) confirmBtn.textContent = '保存';

    // 填充表单
    const idInput = document.getElementById('tamIdInput');
    const caseNameInput = document.getElementById('tamCaseNameInput');
    const noteInput = document.getElementById('tamNoteInput');
    const aiTestYes = document.querySelector('input[name="tamAiTest"][value="1"]');
    const aiTestNo = document.querySelector('input[name="tamAiTest"][value="0"]');

    if (idInput) idInput.value = testcase.id;
    if (caseNameInput) caseNameInput.value = testcase.caseName || '';
    if (noteInput) noteInput.value = testcase.note || '';
    if (testcase.aiTest) {
        if (aiTestYes) aiTestYes.checked = true;
    } else {
        if (aiTestNo) aiTestNo.checked = true;
    }

    // 显示模态框
    modal.style.display = 'flex';

    // 聚焦到用例名称输入框
    setTimeout(() => {
        document.getElementById('tamCaseNameInput')?.focus();
    }, 100);
}

// 隐藏模态框
function hideTestcaseAddModal() {
    const modal = document.getElementById('testcaseAddModal');
    if (modal) {
        modal.style.display = 'none';
    }
    isEditMode = false;
    editingId = null;
    resetTestcaseAddForm();
}

// 重置表单
function resetTestcaseAddForm() {
    const idInput = document.getElementById('tamIdInput');
    const caseNameInput = document.getElementById('tamCaseNameInput');
    const noteInput = document.getElementById('tamNoteInput');
    const aiTestNo = document.querySelector('input[name="tamAiTest"][value="0"]');

    if (idInput) idInput.value = '';
    if (caseNameInput) caseNameInput.value = '';
    if (noteInput) noteInput.value = '';
    if (aiTestNo) aiTestNo.checked = true;
}

// 获取下一个可用 ID
async function fetchNextId() {
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) return;

    const { ipcRenderer } = getGlobals();

    try {
        const result = await ipcRenderer.invoke('db-testcase-getNextId', projectPath);
        if (result.success) {
            const idInput = document.getElementById('tamIdInput');
            if (idInput) {
                idInput.value = result.nextId;
            }
        }
    } catch (error) {
        window.rError('获取下一个ID失败:', error);
    }
}

// 执行创建或更新
async function executeCreateOrUpdate() {
    const caseNameInput = document.getElementById('tamCaseNameInput');
    const noteInput = document.getElementById('tamNoteInput');
    const aiTestChecked = document.querySelector('input[name="tamAiTest"]:checked');

    const caseName = caseNameInput?.value?.trim();
    if (!caseName) {
        window.AppNotifications?.error('请输入测试用例名称');
        caseNameInput?.focus();
        return;
    }

    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) {
        window.AppNotifications?.error('请先打开项目');
        return;
    }

    const { ipcRenderer } = getGlobals();

    try {
        let result;
        if (isEditMode && editingId) {
            // 更新模式
            result = await ipcRenderer.invoke('db-testcase-update', projectPath, editingId, {
                caseName: caseName,
                note: noteInput?.value?.trim() || '',
                aiTest: aiTestChecked?.value === '1' ? 1 : 0
            });

            if (result.success) {
                window.AppNotifications?.success('测试用例已更新');
            }
        } else {
            // 创建模式
            result = await ipcRenderer.invoke('db-testcase-create', projectPath, {
                caseName: caseName,
                note: noteInput?.value?.trim() || '',
                aiTest: aiTestChecked?.value === '1' ? 1 : 0
            });

            if (result.success) {
                window.AppNotifications?.success('测试用例创建成功');
            }
        }

        if (result.success) {
            hideTestcaseAddModal();

            // 刷新用例列表
            if (window.ProjectTestcaseManager && window.ProjectTestcaseManager.refreshTestcaseList) {
                await window.ProjectTestcaseManager.refreshTestcaseList();
            }
        } else {
            window.AppNotifications?.error(`操作失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('操作失败:', error);
        window.AppNotifications?.error(`操作失败: ${error.message}`);
    }
}

// 初始化模态框事件
function initializeTestcaseAddModal() {
    const modal = document.getElementById('testcaseAddModal');
    if (!modal) return;

    const closeBtn = document.getElementById('closeTestcaseAddModal');
    const cancelBtn = document.getElementById('cancelTestcaseAdd');
    const confirmBtn = document.getElementById('confirmTestcaseAdd');

    // 关闭按钮
    if (closeBtn) {
        closeBtn.addEventListener('click', hideTestcaseAddModal);
    }

    // 取消按钮
    if (cancelBtn) {
        cancelBtn.addEventListener('click', hideTestcaseAddModal);
    }

    // 创建/保存按钮
    if (confirmBtn) {
        confirmBtn.addEventListener('click', executeCreateOrUpdate);
    }

    // 点击模态框外部关闭
    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            hideTestcaseAddModal();
        }
    });

    // Ctrl+Enter 提交
    const caseNameInput = document.getElementById('tamCaseNameInput');
    if (caseNameInput) {
        caseNameInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                e.preventDefault();
                executeCreateOrUpdate();
            }
        });
    }
}

// 导出模块
window.TestcaseAddModal = {
    show: showTestcaseAddModal,
    showEdit: showTestcaseEditModal,
    hide: hideTestcaseAddModal,
    initialize: initializeTestcaseAddModal
};
