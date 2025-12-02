// Testcase 测试用例数据库操作模块
// 新结构：id(主键递增), case_name, note, folder_name(可为空), result, task_batch, ai_test, ai_prompt, ai_result, ai_comment

const { ipcMain } = require('electron');
const { getDatabase } = require('./db-core');

/**
 * 将数据库行转换为 testcase 对象
 */
function rowToTestcase(row) {
    if (!row) return null;
    return {
        id: row.id,
        caseName: row.case_name,
        note: row.note || '',
        folderName: row.folder_name || '',
        result: row.result || 'NA',
        taskBatch: row.task_batch || '',
        createdAt: row.created_at,
        updatedAt: row.updated_at,
        aiTest: !!row.ai_test,
        aiPrompt: row.ai_prompt || '',
        aiResult: row.ai_result || '',
        aiComment: row.ai_comment || ''
    };
}

/**
 * 获取所有测试用例
 */
function getAllTestcases(projectPath) {
    const db = getDatabase(projectPath);
    const rows = db.prepare('SELECT * FROM testcases ORDER BY id').all();
    return rows.map(rowToTestcase);
}

/**
 * 获取单个测试用例（通过 id）
 */
function getTestcaseById(projectPath, id) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT * FROM testcases WHERE id = ?').get(id);
    return rowToTestcase(row);
}

/**
 * 获取单个测试用例（通过 folder_name）
 */
function getTestcaseByFolder(projectPath, folderName) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT * FROM testcases WHERE folder_name = ?').get(folderName);
    return rowToTestcase(row);
}

/**
 * 获取下一个可用的 ID（用于 UI 显示预览）
 */
function getNextId(projectPath) {
    const db = getDatabase(projectPath);
    const maxId = db.prepare('SELECT MAX(id) as max FROM testcases').get();
    return (maxId?.max ?? 0) + 1;
}

/**
 * 获取下一个可用的 case 编号（用于生成 folder_name）
 */
function getNextCaseNumber(projectPath) {
    const db = getDatabase(projectPath);
    // 查找最大的 case 编号
    const rows = db.prepare("SELECT folder_name FROM testcases WHERE folder_name LIKE 'cases/case_%'").all();
    let maxNum = 0;
    for (const row of rows) {
        const match = row.folder_name.match(/case_(\d+)$/);
        if (match) {
            const num = parseInt(match[1], 10);
            if (num > maxNum) maxNum = num;
        }
    }
    return maxNum + 1;
}

/**
 * 检查 folder_name 是否已存在（仅检查非空的）
 */
function folderNameExists(projectPath, folderName, excludeId = null) {
    if (!folderName) return false;
    const db = getDatabase(projectPath);
    if (excludeId) {
        const row = db.prepare('SELECT id FROM testcases WHERE folder_name = ? AND id != ? AND folder_name != ""').get(folderName, excludeId);
        return !!row;
    }
    const row = db.prepare('SELECT id FROM testcases WHERE folder_name = ? AND folder_name != ""').get(folderName);
    return !!row;
}

/**
 * 创建测试用例（不自动生成 folder_name，由创建文件夹时填充）
 */
function createTestcase(projectPath, testcase) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const result = db.prepare(`
        INSERT INTO testcases (case_name, note, folder_name, result, task_batch, created_at, updated_at, ai_test, ai_prompt, ai_result, ai_comment)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
        testcase.caseName || '',
        testcase.note || '',
        '',  // folder_name 默认为空
        testcase.result || 'NA',
        testcase.taskBatch || '',
        now,
        now,
        testcase.aiTest ? 1 : 0,
        testcase.aiPrompt || '',
        testcase.aiResult || '',
        testcase.aiComment || ''
    );

    return {
        success: true,
        id: result.lastInsertRowid
    };
}

/**
 * 更新测试用例
 */
function updateTestcase(projectPath, id, updates) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const fields = [];
    const values = [];

    if (updates.caseName !== undefined) {
        fields.push('case_name = ?');
        values.push(updates.caseName);
    }
    if (updates.note !== undefined) {
        fields.push('note = ?');
        values.push(updates.note);
    }
    if (updates.folderName !== undefined) {
        // 检查新 folderName 是否与其他记录冲突
        if (updates.folderName && folderNameExists(projectPath, updates.folderName, id)) {
            return { success: false, error: `文件夹路径 "${updates.folderName}" 已被其他用例使用` };
        }
        fields.push('folder_name = ?');
        values.push(updates.folderName);
    }
    if (updates.result !== undefined) {
        fields.push('result = ?');
        values.push(updates.result);
    }
    if (updates.taskBatch !== undefined) {
        fields.push('task_batch = ?');
        values.push(updates.taskBatch);
    }
    if (updates.aiTest !== undefined) {
        fields.push('ai_test = ?');
        values.push(updates.aiTest ? 1 : 0);
    }
    if (updates.aiPrompt !== undefined) {
        fields.push('ai_prompt = ?');
        values.push(updates.aiPrompt);
    }
    if (updates.aiResult !== undefined) {
        fields.push('ai_result = ?');
        values.push(updates.aiResult);
    }
    if (updates.aiComment !== undefined) {
        fields.push('ai_comment = ?');
        values.push(updates.aiComment);
    }

    if (fields.length === 0) {
        return { success: true, message: '没有需要更新的字段' };
    }

    fields.push('updated_at = ?');
    values.push(now);
    values.push(id);

    db.prepare(`UPDATE testcases SET ${fields.join(', ')} WHERE id = ?`).run(...values);

    return { success: true };
}

/**
 * 删除测试用例
 */
function deleteTestcase(projectPath, id) {
    const db = getDatabase(projectPath);
    db.prepare('DELETE FROM testcases WHERE id = ?').run(id);
    return { success: true };
}

/**
 * 批量导入测试用例（从 CSV）- folder_name 默认为空
 */
function batchImportTestcases(projectPath, testcases) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const insert = db.prepare(`
        INSERT INTO testcases (case_name, note, folder_name, result, task_batch, created_at, updated_at, ai_test, ai_prompt, ai_result, ai_comment)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const results = [];
    const batchInsert = db.transaction((items) => {
        for (const tc of items) {
            const result = insert.run(
                tc.caseName || '',
                tc.note || '',
                '',  // folder_name 默认为空，等待用户点击 Create 时填充
                'NA',
                '',
                now,
                now,
                0,
                '',
                '',
                ''
            );
            results.push({
                id: result.lastInsertRowid,
                caseName: tc.caseName
            });
        }
    });

    batchInsert(testcases);
    return { success: true, count: testcases.length, imported: results };
}

/**
 * 为用例设置 folder_name（创建文件夹后调用）
 */
function setFolderName(projectPath, id) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    // 获取下一个可用的 case 编号
    const nextNum = getNextCaseNumber(projectPath);
    const folderName = `cases/case_${String(nextNum).padStart(3, '0')}`;

    db.prepare('UPDATE testcases SET folder_name = ?, updated_at = ? WHERE id = ?').run(folderName, now, id);

    return { success: true, folderName };
}

/**
 * 注册 Testcase IPC 处理器
 */
function registerTestcaseHandlers() {
    ipcMain.handle('db-testcase-getAll', async (event, projectPath) => {
        try {
            const testcases = getAllTestcases(projectPath);
            return { success: true, data: testcases };
        } catch (error) {
            console.error('获取 testcases 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-getById', async (event, projectPath, id) => {
        try {
            const testcase = getTestcaseById(projectPath, id);
            return { success: true, data: testcase };
        } catch (error) {
            console.error('获取 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-getByFolder', async (event, projectPath, folderName) => {
        try {
            const testcase = getTestcaseByFolder(projectPath, folderName);
            return { success: true, data: testcase };
        } catch (error) {
            console.error('获取 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-getNextId', async (event, projectPath) => {
        try {
            const nextId = getNextId(projectPath);
            return { success: true, nextId };
        } catch (error) {
            console.error('获取下一个ID失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-create', async (event, projectPath, testcase) => {
        try {
            return createTestcase(projectPath, testcase);
        } catch (error) {
            console.error('创建 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-update', async (event, projectPath, id, updates) => {
        try {
            return updateTestcase(projectPath, id, updates);
        } catch (error) {
            console.error('更新 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-delete', async (event, projectPath, id) => {
        try {
            return deleteTestcase(projectPath, id);
        } catch (error) {
            console.error('删除 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-setFolderName', async (event, projectPath, id) => {
        try {
            return setFolderName(projectPath, id);
        } catch (error) {
            console.error('设置 folder_name 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-batchImport', async (event, projectPath, testcases) => {
        try {
            return batchImportTestcases(projectPath, testcases);
        } catch (error) {
            console.error('批量导入 testcases 失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 兼容旧的 IPC 调用（暂时保留）
    ipcMain.handle('db-testcase-get', async (event, projectPath, folderName) => {
        try {
            const testcase = getTestcaseByFolder(projectPath, folderName);
            return { success: true, data: testcase };
        } catch (error) {
            console.error('获取 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-getMap', async (event, projectPath) => {
        try {
            // 兼容旧格式 { id: folderName }
            const testcases = getAllTestcases(projectPath);
            const map = {};
            testcases.forEach(tc => {
                if (tc.folderName) {
                    map[tc.id] = tc.folderName;
                }
            });
            return { success: true, data: map };
        } catch (error) {
            console.error('获取 testcase map 失败:', error);
            return { success: false, error: error.message };
        }
    });

    console.log('Testcase 数据库处理器已注册');
}

module.exports = {
    getAllTestcases,
    getTestcaseById,
    getTestcaseByFolder,
    getNextId,
    createTestcase,
    updateTestcase,
    deleteTestcase,
    setFolderName,
    batchImportTestcases,
    registerTestcaseHandlers
};
