// CSV 导入模态框控制器
// 负责 CSV/Excel 文件的选择、解析和批量导入测试用例

// 获取全局变量
function getGlobals() {
    return window.AppGlobals;
}

// 模态框状态
let csvImportState = {
    step: 1,
    fileName: '',
    records: [],
    headers: [],
    selectedColumn: null
};

// 显示 CSV 导入模态框
function showCsvImportModal() {
    const modal = document.getElementById('csvImportModal');
    if (!modal) {
        window.rError('找不到 CSV 导入模态框');
        return;
    }

    // 重置状态
    resetCsvImportState();

    // 显示模态框
    modal.style.display = 'flex';
}

// 隐藏 CSV 导入模态框
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
        step: 1,
        fileName: '',
        records: [],
        headers: [],
        selectedColumn: null
    };

    // 重置 UI
    const step1 = document.getElementById('csvImportStep1');
    const step2 = document.getElementById('csvImportStep2');
    const backBtn = document.getElementById('backCsvImport');
    const confirmBtn = document.getElementById('confirmCsvImport');

    if (step1) step1.style.display = 'block';
    if (step2) step2.style.display = 'none';
    if (backBtn) backBtn.style.display = 'none';
    if (confirmBtn) confirmBtn.disabled = true;
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
            // 首先尝试 UTF-8
            content = new TextDecoder('utf-8').decode(arrayBuffer);
            // 检查是否有乱码（BOM 或常见乱码字符）
            if (content.charCodeAt(0) === 0xFEFF) {
                content = content.slice(1); // 移除 BOM
            }
        } catch (e) {
            // 如果 UTF-8 失败，尝试 GBK/GB2312（中文 Windows 常用）
            try {
                content = new TextDecoder('gbk').decode(arrayBuffer);
            } catch (e2) {
                content = new TextDecoder('utf-8', { fatal: false }).decode(arrayBuffer);
            }
        }

        let records = [];
        let headers = [];

        if (isCSV) {
            // 解析 CSV - 不设置行数限制
            records = parse(content, {
                columns: true,
                skip_empty_lines: true,
                relax_quotes: true,        // 允许不规范的引号
                relax_column_count: true,  // 允许列数不一致
                trim: true                 // 去除空白
            });

            if (records.length > 0) {
                headers = Object.keys(records[0]);
            }

            window.rLog(`CSV 解析完成: ${records.length} 行数据`);
        } else {
            // Excel 文件需要通过 IPC 处理
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

// 从文件路径处理 CSV（使用 Node.js fs 直接读取）
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
        // 使用 Node.js fs 直接读取文件（二进制方式）
        const buffer = fsSync.readFileSync(filePath);

        // 尝试不同的编码
        let content;
        try {
            // 首先尝试 UTF-8
            content = buffer.toString('utf8');
            // 检查并移除 BOM
            if (content.charCodeAt(0) === 0xFEFF) {
                content = content.slice(1);
            }
        } catch (e) {
            // 如果有问题，使用默认编码
            content = buffer.toString();
        }

        // 解析 CSV - 不设置行数限制
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

        // 保存状态
        csvImportState.fileName = fileName;
        csvImportState.records = records;
        csvImportState.headers = headers;

        // 切换到步骤2
        showStep2();

    } catch (error) {
        window.rError('读取文件失败:', error);
        window.AppNotifications?.error(`读取文件失败: ${error.message}`);
    }
}

// 显示步骤2
function showStep2() {
    csvImportState.step = 2;

    const step1 = document.getElementById('csvImportStep1');
    const step2 = document.getElementById('csvImportStep2');
    const backBtn = document.getElementById('backCsvImport');

    if (step1) step1.style.display = 'none';
    if (step2) step2.style.display = 'block';
    if (backBtn) backBtn.style.display = 'inline-block';

    // 更新文件信息
    document.getElementById('csvFileName').textContent = csvImportState.fileName;
    document.getElementById('csvRowCount').textContent = `${csvImportState.records.length} 行数据`;

    // 生成列选项
    renderColumnOptions();

    // 生成预览表格
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

    // 添加点击事件
    container.querySelectorAll('.cim-column-option').forEach(option => {
        option.addEventListener('click', () => {
            // 移除其他选中状态
            container.querySelectorAll('.cim-column-option').forEach(o => o.classList.remove('selected'));
            // 添加选中状态
            option.classList.add('selected');
            // 保存选中的列
            csvImportState.selectedColumn = option.dataset.header;
            // 启用确认按钮
            document.getElementById('confirmCsvImport').disabled = false;
            // 更新预览表格高亮
            updatePreviewHighlight();
        });
    });
}

// 渲染预览表格
function renderPreviewTable() {
    const container = document.getElementById('csvPreviewTable');
    if (!container) return;

    // 只显示前 5 行
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
            // 截断过长的文本
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

    // 移除所有高亮
    container.querySelectorAll('.selected-column').forEach(el => {
        el.classList.remove('selected-column');
    });

    // 添加选中列高亮
    if (csvImportState.selectedColumn) {
        container.querySelectorAll(`[data-header="${csvImportState.selectedColumn}"]`).forEach(el => {
            el.classList.add('selected-column');
        });
    }
}

// 执行导入
async function executeImport() {
    if (!csvImportState.selectedColumn || csvImportState.records.length === 0) {
        window.AppNotifications?.error('请先选择用例名称列');
        return;
    }

    const projectPath = window.AppGlobals.currentProject;
    if (!projectPath) {
        window.AppNotifications?.error('请先打开项目');
        return;
    }

    const { ipcRenderer } = getGlobals();

    // 准备导入数据
    const testcases = csvImportState.records.map(record => ({
        caseName: record[csvImportState.selectedColumn] || '',
        note: ''
    }));

    try {
        // 批量导入
        const result = await ipcRenderer.invoke('db-testcase-batchImport', projectPath, testcases);

        if (result.success) {
            window.AppNotifications?.success(`成功导入 ${result.count} 条测试用例`);
            hideCsvImportModal();

            // 刷新用例列表
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

// 初始化 CSV 导入模态框事件
function initializeCsvImportModal() {
    const modal = document.getElementById('csvImportModal');
    if (!modal) return;

    const closeBtn = document.getElementById('closeCsvImportModal');
    const cancelBtn = document.getElementById('cancelCsvImport');
    const backBtn = document.getElementById('backCsvImport');
    const confirmBtn = document.getElementById('confirmCsvImport');
    const selectFileBtn = document.getElementById('selectCsvFile');
    const dropzone = document.getElementById('csvDropzone');

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

    // 选择文件按钮
    if (selectFileBtn) {
        selectFileBtn.addEventListener('click', async (e) => {
            e.stopPropagation();
            const { ipcRenderer } = getGlobals();
            const result = await ipcRenderer.invoke('select-file', [
                { name: 'CSV/Excel Files', extensions: ['csv', 'xlsx', 'xls'] }
            ]);
            if (result) {
                // 直接处理文件路径
                await handleFileFromPath(result);
            }
        });
    }

    // 拖拽区域
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

        // 点击也可以选择文件
        dropzone.addEventListener('click', (e) => {
            if (e.target === dropzone || e.target.closest('.cim-dropzone-icon') || e.target.tagName === 'P') {
                document.getElementById('selectCsvFile')?.click();
            }
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
