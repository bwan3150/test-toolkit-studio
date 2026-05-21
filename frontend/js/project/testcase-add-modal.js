// 新建/编辑测试用例模态框控制器

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

    // 设置标题和按钮
    const titleEl = document.getElementById('testcaseAddModalTitle');
    if (titleEl) titleEl.textContent = '新建测试用例';

    const confirmBtn = document.getElementById('confirmTestcaseAdd');
    if (confirmBtn) confirmBtn.textContent = '创建';

    // 重置表单
    resetTestcaseAddForm();

    // 获取下一个可用 ID
    await fetchNextId();

    // 收起 AI 设置
    collapseAiSettings();

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

    // 设置标题和按钮
    const titleEl = document.getElementById('testcaseAddModalTitle');
    if (titleEl) titleEl.textContent = '编辑测试用例';

    const confirmBtn = document.getElementById('confirmTestcaseAdd');
    if (confirmBtn) confirmBtn.textContent = '保存';

    // 填充表单
    fillForm(testcase);

    // 如果有 AI 相关数据，展开 AI 设置
    if (testcase.aiTest || testcase.aiPrompt || testcase.aiResult !== 'NOT TESTED' || testcase.aiComment) {
        expandAiSettings();
    } else {
        collapseAiSettings();
    }

    // 显示模态框
    modal.style.display = 'flex';

    setTimeout(() => {
        document.getElementById('tamCaseNameInput')?.focus();
    }, 100);
}

// 填充表单数据
function fillForm(testcase) {
    const idInput = document.getElementById('tamIdInput');
    const caseNameInput = document.getElementById('tamCaseNameInput');
    const noteInput = document.getElementById('tamNoteInput');
    const resultSelect = document.getElementById('tamResultSelect');
    const aiResultSelect = document.getElementById('tamAiResultSelect');
    const aiPromptInput = document.getElementById('tamAiPromptInput');
    const aiCommentInput = document.getElementById('tamAiCommentInput');

    if (idInput) idInput.value = testcase.id;
    if (caseNameInput) caseNameInput.value = testcase.caseName || '';
    if (noteInput) noteInput.value = testcase.note || '';
    if (resultSelect) resultSelect.value = testcase.result || 'NOT TESTED';

    // 设置 AI 生成 radio
    const aiTestValue = testcase.aiTest ? '1' : '0';
    const aiTestRadio = document.querySelector(`input[name="tamAiTest"][value="${aiTestValue}"]`);
    if (aiTestRadio) aiTestRadio.checked = true;

    if (aiResultSelect) aiResultSelect.value = testcase.aiResult || 'NOT TESTED';
    if (aiPromptInput) aiPromptInput.value = testcase.aiPrompt || '';
    if (aiCommentInput) aiCommentInput.value = testcase.aiComment || '';
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
    const resultSelect = document.getElementById('tamResultSelect');
    const aiResultSelect = document.getElementById('tamAiResultSelect');
    const aiPromptInput = document.getElementById('tamAiPromptInput');
    const aiCommentInput = document.getElementById('tamAiCommentInput');

    if (idInput) idInput.value = '';
    if (caseNameInput) caseNameInput.value = '';
    if (noteInput) noteInput.value = '';
    if (resultSelect) resultSelect.value = 'NOT TESTED';

    // 重置 AI 生成 radio 为禁用
    const aiTestRadio = document.querySelector('input[name="tamAiTest"][value="0"]');
    if (aiTestRadio) aiTestRadio.checked = true;

    if (aiResultSelect) aiResultSelect.value = 'NOT TESTED';
    if (aiPromptInput) aiPromptInput.value = '';
    if (aiCommentInput) aiCommentInput.value = '';
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

// 收集表单数据
function collectFormData() {
    // 获取 AI 生成 radio 的值
    const aiTestRadio = document.querySelector('input[name="tamAiTest"]:checked');
    const aiTestValue = aiTestRadio ? parseInt(aiTestRadio.value) : 0;

    return {
        caseName: document.getElementById('tamCaseNameInput')?.value?.trim() || '',
        note: document.getElementById('tamNoteInput')?.value?.trim() || '',
        result: document.getElementById('tamResultSelect')?.value || 'NOT TESTED',
        aiTest: aiTestValue,
        aiResult: document.getElementById('tamAiResultSelect')?.value || 'NOT TESTED',
        aiPrompt: document.getElementById('tamAiPromptInput')?.value?.trim() || '',
        aiComment: document.getElementById('tamAiCommentInput')?.value?.trim() || ''
    };
}

// 执行创建或更新
async function executeCreateOrUpdate() {
    const formData = collectFormData();

    if (!formData.caseName) {
        window.AppNotifications?.error('请输入测试用例名称');
        document.getElementById('tamCaseNameInput')?.focus();
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
            result = await ipcRenderer.invoke('db-testcase-update', projectPath, editingId, formData);
            if (result.success) {
                window.AppNotifications?.success('测试用例已更新');
            }
        } else {
            result = await ipcRenderer.invoke('db-testcase-create', projectPath, formData);
            if (result.success) {
                window.AppNotifications?.success('测试用例创建成功');
            }
        }

        if (result.success) {
            hideTestcaseAddModal();
            if (window.ProjectTestcaseManager?.refreshTestcaseList) {
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

// 切换 AI 设置折叠
function toggleAiSettings() {
    const content = document.getElementById('tamAiCollapseContent');
    const header = document.getElementById('tamAiCollapseHeader');
    if (!content || !header) return;

    const isExpanded = content.style.display !== 'none';
    if (isExpanded) {
        collapseAiSettings();
    } else {
        expandAiSettings();
    }
}

// 展开 AI 设置
function expandAiSettings() {
    const content = document.getElementById('tamAiCollapseContent');
    const header = document.getElementById('tamAiCollapseHeader');
    if (content) content.style.display = 'block';
    if (header) header.classList.add('expanded');
}

// 收起 AI 设置
function collapseAiSettings() {
    const content = document.getElementById('tamAiCollapseContent');
    const header = document.getElementById('tamAiCollapseHeader');
    if (content) content.style.display = 'none';
    if (header) header.classList.remove('expanded');
}

// 初始化模态框事件
function initializeTestcaseAddModal() {
    const modal = document.getElementById('testcaseAddModal');
    if (!modal) return;

    const closeBtn = document.getElementById('closeTestcaseAddModal');
    const cancelBtn = document.getElementById('cancelTestcaseAdd');
    const confirmBtn = document.getElementById('confirmTestcaseAdd');
    const collapseHeader = document.getElementById('tamAiCollapseHeader');

    if (closeBtn) {
        closeBtn.addEventListener('click', hideTestcaseAddModal);
    }

    if (cancelBtn) {
        cancelBtn.addEventListener('click', hideTestcaseAddModal);
    }

    if (confirmBtn) {
        confirmBtn.addEventListener('click', executeCreateOrUpdate);
    }

    if (collapseHeader) {
        collapseHeader.addEventListener('click', toggleAiSettings);
    }

    // 点击模态框外部关闭
    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            hideTestcaseAddModal();
        }
    });
}

// 导出模块
window.TestcaseAddModal = {
    show: showTestcaseAddModal,
    showEdit: showTestcaseEditModal,
    hide: hideTestcaseAddModal,
    initialize: initializeTestcaseAddModal
};
