// 导入测试用例模态框控制器
// 支持从文件(CSV/Excel)导入和从文本导入

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// 模态框状态
let csvImportState = {
    activeTab: 'file',  // 'file' 或 'text'
    step: 1,
    fileName: '',
    records: [],
    headers: [],
    selectedColumn: null,
    textLines: []  // 文本导入的行
};

// 显示导入模态框
function showCsvImportModal() {
    const modal = document.getElementById('csvImportModal');
    if (!modal) {
        window.rError('找不到导入模态框');
        return;
    }

    // 重置状态
    resetCsvImportState();

    // 显示模态框
    modal.style.display = 'flex';
}

// 隐藏导入模态框
function hideCsvImportModal() {
    const modal = document.getElementById('csvImportModal');
    if (modal) {
        modal.style.display = 'none';
    }
    resetCsvImportState();
}

// 重置状态
function resetCsvImportState() {
    csvImportState = {
        activeTab: 'file',
        step: 1,
        fileName: '',
        records: [],
        headers: [],
        selectedColumn: null,
        textLines: []
    };

    // 重置 UI
    const step1 = document.getElementById('csvImportStep1');
    const step2 = document.getElementById('csvImportStep2');
    const backBtn = document.getElementById('backCsvImport');
    const confirmBtn = document.getElementById('confirmCsvImport');
    const textInput = document.getElementById('textImportInput');
    const lineCount = document.getElementById('textLineCount');

    if (step1) step1.style.display = 'block';
    if (step2) step2.style.display = 'none';
    if (backBtn) backBtn.style.display = 'none';
    if (confirmBtn) confirmBtn.disabled = true;
    if (textInput) textInput.value = '';
    if (lineCount) lineCount.textContent = '0 条用例';

    // 重置 Tab
    switchTab('file');
}

// 切换 Tab
function switchTab(tabName) {
    csvImportState.activeTab = tabName;

    // 更新 Tab 样式
    document.querySelectorAll('.cim-tab').forEach(tab => {
        tab.classList.toggle('active', tab.dataset.tab === tabName);
    });

    // 更新内容显示
    document.getElementById('tabFile')?.classList.toggle('active', tabName === 'file');
    document.getElementById('tabText')?.classList.toggle('active', tabName === 'text');

    // 更新按钮状态
    updateConfirmButtonState();

    // 隐藏上一步按钮（文本模式不需要）
    const backBtn = document.getElementById('backCsvImport');
    if (backBtn) {
        backBtn.style.display = (tabName === 'file' && csvImportState.step === 2) ? 'inline-block' : 'none';
    }
}

// 更新确认按钮状态
function updateConfirmButtonState() {
    const confirmBtn = document.getElementById('confirmCsvImport');
    if (!confirmBtn) return;

    if (csvImportState.activeTab === 'file') {
        // 文件模式：需要选择列
        confirmBtn.disabled = !csvImportState.selectedColumn;
    } else {
        // 文本模式：需要有内容
        confirmBtn.disabled = csvImportState.textLines.length === 0;
    }
}

// 处理文本输入变化
function handleTextInput(text) {
    // 按行分割，过滤空行
    const lines = text.split('\n')
        .map(line => line.trim())
        .filter(line => line.length > 0);

    csvImportState.textLines = lines;

    // 更新行数显示
    const lineCount = document.getElementById('textLineCount');
    if (lineCount) {
        lineCount.textContent = `${lines.length} 条用例`;
    }

    // 更新按钮状态
    updateConfirmButtonState();
}

// 处理文件选择
async function handleFileSelect(file) {
    if (!file) return;

    const fileName = file.name.toLowerCase();
    const isCSV = fileName.endsWith('.csv');
    const isExcel = fileName.endsWith('.xlsx') || fileName.endsWith('.xls');

    if (!isCSV && !isExcel) {
        window.AppNotifications?.error('请选择 CSV 或 Excel 文件');
        return;
    }

    try {
        const { ipcRenderer, parse } = getGlobals();

        // 读取文件内容
        const arrayBuffer = await file.arrayBuffer();

        // 尝试检测编码并解码
        let content;
        try {
            content = new TextDecoder('utf-8').decode(arrayBuffer);
            if (content.charCodeAt(0) === 0xFEFF) {
                content = content.slice(1);
            }
        } catch (e) {
            try {
                content = new TextDecoder('gbk').decode(arrayBuffer);
            } catch (e2) {
                content = new TextDecoder('utf-8', { fatal: false }).decode(arrayBuffer);
            }
        }

        let records = [];
        let headers = [];

        if (isCSV) {
            records = parse(content, {
                columns: true,
                skip_empty_lines: true,
                relax_quotes: true,
                relax_column_count: true,
                trim: true
            });

            if (records.length > 0) {
                headers = Object.keys(records[0]);
            }

            window.rLog(`CSV 解析完成: ${records.length} 行数据`);
        } else {
            window.AppNotifications?.warn('Excel 文件支持开发中，请先使用 CSV 格式');
            return;
        }

        if (records.length === 0) {
            window.AppNotifications?.error('文件为空或格式错误');
            return;
        }

        // 保存状态
        csvImportState.fileName = file.name;
        csvImportState.records = records;
        csvImportState.headers = headers;

        // 切换到步骤2
        showStep2();

    } catch (error) {
        window.rError('解析文件失败:', error);
        window.AppNotifications?.error(`解析文件失败: ${error.message}`);
    }
}

// 从文件路径处理 CSV
async function handleFileFromPath(filePath) {
    const { fsSync, parse } = getGlobals();
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop();
    const fileNameLower = fileName.toLowerCase();
    const isCSV = fileNameLower.endsWith('.csv');
    const isExcel = fileNameLower.endsWith('.xlsx') || fileNameLower.endsWith('.xls');

    if (!isCSV && !isExcel) {
        window.AppNotifications?.error('请选择 CSV 或 Excel 文件');
        return;
    }

    if (isExcel) {
        window.AppNotifications?.warn('Excel 文件支持开发中，请先使用 CSV 格式');
        return;
    }

    try {
        const buffer = fsSync.readFileSync(filePath);

        let content;
        try {
            content = buffer.toString('utf8');
            if (content.charCodeAt(0) === 0xFEFF) {
                content = content.slice(1);
            }
        } catch (e) {
            content = buffer.toString();
        }

        const records = parse(content, {
            columns: true,
            skip_empty_lines: true,
            relax_quotes: true,
            relax_column_count: true,
            trim: true
        });

        if (records.length === 0) {
            window.AppNotifications?.error('文件为空或格式错误');
            return;
        }

        const headers = Object.keys(records[0]);

        window.rLog(`CSV 解析完成: ${records.length} 行数据`);

        csvImportState.fileName = fileName;
        csvImportState.records = records;
        csvImportState.headers = headers;

        showStep2();

    } catch (error) {
        window.rError('读取文件失败:', error);
        window.AppNotifications?.error(`读取文件失败: ${error.message}`);
    }
}

// 显示步骤2（文件导入）
function showStep2() {
    csvImportState.step = 2;

    const step1 = document.getElementById('csvImportStep1');
    const step2 = document.getElementById('csvImportStep2');
    const backBtn = document.getElementById('backCsvImport');

    if (step1) step1.style.display = 'none';
    if (step2) step2.style.display = 'block';
    if (backBtn) backBtn.style.display = 'inline-block';

    document.getElementById('csvFileName').textContent = csvImportState.fileName;
    document.getElementById('csvRowCount').textContent = `${csvImportState.records.length} 行数据`;

    renderColumnOptions();
    renderPreviewTable();
}

// 显示步骤1
function showStep1() {
    csvImportState.step = 1;
    csvImportState.selectedColumn = null;

    const step1 = document.getElementById('csvImportStep1');
    const step2 = document.getElementById('csvImportStep2');
    const backBtn = document.getElementById('backCsvImport');
    const confirmBtn = document.getElementById('confirmCsvImport');

    if (step1) step1.style.display = 'block';
    if (step2) step2.style.display = 'none';
    if (backBtn) backBtn.style.display = 'none';
    if (confirmBtn) confirmBtn.disabled = true;
}

// 渲染列选项
function renderColumnOptions() {
    const container = document.getElementById('csvColumnSelect');
    if (!container) return;

    container.innerHTML = csvImportState.headers.map((header, index) => `
        <div class="cim-column-option" data-column="${index}" data-header="${header}">
            ${header}
        </div>
    `).join('');

    container.querySelectorAll('.cim-column-option').forEach(option => {
        option.addEventListener('click', () => {
            container.querySelectorAll('.cim-column-option').forEach(o => o.classList.remove('selected'));
            option.classList.add('selected');
            csvImportState.selectedColumn = option.dataset.header;
            updateConfirmButtonState();
            updatePreviewHighlight();
        });
    });
}

// 渲染预览表格
function renderPreviewTable() {
    const container = document.getElementById('csvPreviewTable');
    if (!container) return;

    const previewRecords = csvImportState.records.slice(0, 5);

    let html = '<table><thead><tr>';
    csvImportState.headers.forEach(header => {
        html += `<th data-header="${header}">${header}</th>`;
    });
    html += '</tr></thead><tbody>';

    previewRecords.forEach(record => {
        html += '<tr>';
        csvImportState.headers.forEach(header => {
            const value = record[header] || '';
            const displayValue = value.length > 50 ? value.substring(0, 50) + '...' : value;
            html += `<td data-header="${header}" title="${value}">${displayValue}</td>`;
        });
        html += '</tr>';
    });

    html += '</tbody></table>';
    container.innerHTML = html;
}

// 更新预览高亮
function updatePreviewHighlight() {
    const container = document.getElementById('csvPreviewTable');
    if (!container) return;

    container.querySelectorAll('.selected-column').forEach(el => {
        el.classList.remove('selected-column');
    });

    if (csvImportState.selectedColumn) {
        container.querySelectorAll(`[data-header="${csvImportState.selectedColumn}"]`).forEach(el => {
            el.classList.add('selected-column');
        });
    }
}

// 执行导入
async function executeImport() {
    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) {
        window.AppNotifications?.error('请先打开项目');
        return;
    }

    const { ipcRenderer } = getGlobals();
    let testcases = [];

    if (csvImportState.activeTab === 'file') {
        // 文件导入
        if (!csvImportState.selectedColumn || csvImportState.records.length === 0) {
            window.AppNotifications?.error('请先选择用例名称列');
            return;
        }
        testcases = csvImportState.records.map(record => ({
            caseName: record[csvImportState.selectedColumn] || '',
            note: ''
        }));
    } else {
        // 文本导入
        if (csvImportState.textLines.length === 0) {
            window.AppNotifications?.error('请输入测试用例');
            return;
        }
        testcases = csvImportState.textLines.map(line => ({
            caseName: line,
            note: ''
        }));
    }

    try {
        const result = await ipcRenderer.invoke('db-testcase-batchImport', projectPath, testcases);

        if (result.success) {
            window.AppNotifications?.success(`成功导入 ${result.count} 条测试用例`);
            hideCsvImportModal();

            if (window.ProjectTestcaseManager && window.ProjectTestcaseManager.refreshTestcaseList) {
                await window.ProjectTestcaseManager.refreshTestcaseList();
            }
        } else {
            window.AppNotifications?.error(`导入失败: ${result.error}`);
        }
    } catch (error) {
        window.rError('导入失败:', error);
        window.AppNotifications?.error(`导入失败: ${error.message}`);
    }
}

// 打开文件选择对话框
async function openFileDialog() {
    const { ipcRenderer } = getGlobals();
    const result = await ipcRenderer.invoke('select-file', [
        { name: 'CSV/Excel Files', extensions: ['csv', 'xlsx', 'xls'] }
    ]);
    if (result) {
        await handleFileFromPath(result);
    }
}

// 初始化导入模态框事件
function initializeCsvImportModal() {
    const modal = document.getElementById('csvImportModal');
    if (!modal) return;

    const closeBtn = document.getElementById('closeCsvImportModal');
    const cancelBtn = document.getElementById('cancelCsvImport');
    const backBtn = document.getElementById('backCsvImport');
    const confirmBtn = document.getElementById('confirmCsvImport');
    const dropzone = document.getElementById('csvDropzone');
    const textInput = document.getElementById('textImportInput');

    // 关闭按钮
    if (closeBtn) {
        closeBtn.addEventListener('click', hideCsvImportModal);
    }

    // 取消按钮
    if (cancelBtn) {
        cancelBtn.addEventListener('click', hideCsvImportModal);
    }

    // 上一步按钮
    if (backBtn) {
        backBtn.addEventListener('click', showStep1);
    }

    // 确认导入按钮
    if (confirmBtn) {
        confirmBtn.addEventListener('click', executeImport);
    }

    // Tab 切换
    document.querySelectorAll('.cim-tab').forEach(tab => {
        tab.addEventListener('click', () => {
            switchTab(tab.dataset.tab);
        });
    });

    // 拖拽区域 - 点击直接打开文件选择
    if (dropzone) {
        dropzone.addEventListener('dragover', (e) => {
            e.preventDefault();
            dropzone.classList.add('dragover');
        });

        dropzone.addEventListener('dragleave', (e) => {
            e.preventDefault();
            dropzone.classList.remove('dragover');
        });

        dropzone.addEventListener('drop', async (e) => {
            e.preventDefault();
            dropzone.classList.remove('dragover');
            const files = e.dataTransfer.files;
            if (files.length > 0) {
                await handleFileSelect(files[0]);
            }
        });

        // 点击打开文件选择
        dropzone.addEventListener('click', openFileDialog);
    }

    // 文本输入
    if (textInput) {
        textInput.addEventListener('input', (e) => {
            handleTextInput(e.target.value);
        });
    }

    // 点击模态框外部关闭
    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            hideCsvImportModal();
        }
    });
}

// 导出模块
window.CsvImportModal = {
    show: showCsvImportModal,
    hide: hideCsvImportModal,
    initialize: initializeCsvImportModal
};
