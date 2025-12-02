// Testcase 测试用例数据库操作模块

const { ipcMain } = require('electron');
const { getDatabase } = require('./db-core');

/**
 * 将数据库行转换为 testcase 对象
 */
function rowToTestcase(row) {
    if (!row) return null;
    return {
        id: row.id,
        sortOrder: row.sort_order,
        folderName: row.folder_name,
        displayName: row.display_name,
        created_at: row.created_at,
        updated_at: row.updated_at
    };
}

/**
 * 获取所有测试用例
 */
function getAllTestcases(projectPath) {
    const db = getDatabase(projectPath);
    const rows = db.prepare('SELECT * FROM testcases ORDER BY sort_order').all();
    return rows.map(rowToTestcase);
}

/**
 * 获取测试用例映射（兼容旧格式 { sortOrder: folderName }）
 */
function getTestcaseMap(projectPath) {
    const db = getDatabase(projectPath);
    const rows = db.prepare('SELECT sort_order, folder_name FROM testcases ORDER BY sort_order').all();

    const result = {};
    rows.forEach(row => {
        result[row.sort_order] = row.folder_name;
    });
    return result;
}

/**
 * 获取单个测试用例
 */
function getTestcase(projectPath, folderName) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT * FROM testcases WHERE folder_name = ?').get(folderName);
    return rowToTestcase(row);
}

/**
 * 创建测试用例
 */
function createTestcase(projectPath, testcase) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    // 获取下一个排序号
    const maxOrder = db.prepare('SELECT MAX(sort_order) as max FROM testcases').get();
    const nextOrder = (maxOrder?.max ?? -1) + 1;

    db.prepare(`
        INSERT INTO testcases (sort_order, folder_name, display_name, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
    `).run(
        testcase.sortOrder ?? nextOrder,
        testcase.folderName,
        testcase.displayName || testcase.folderName,
        now,
        now
    );

    return { success: true, sortOrder: testcase.sortOrder ?? nextOrder };
}

/**
 * 更新测试用例
 */
function updateTestcase(projectPath, folderName, updates) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const fields = [];
    const values = [];

    if (updates.displayName !== undefined) {
        fields.push('display_name = ?');
        values.push(updates.displayName);
    }
    if (updates.sortOrder !== undefined) {
        fields.push('sort_order = ?');
        values.push(updates.sortOrder);
    }
    if (updates.folderName !== undefined) {
        fields.push('folder_name = ?');
        values.push(updates.folderName);
    }

    fields.push('updated_at = ?');
    values.push(now);
    values.push(folderName);

    db.prepare(`UPDATE testcases SET ${fields.join(', ')} WHERE folder_name = ?`).run(...values);

    return { success: true };
}

/**
 * 删除测试用例
 */
function deleteTestcase(projectPath, folderName) {
    const db = getDatabase(projectPath);
    db.prepare('DELETE FROM testcases WHERE folder_name = ?').run(folderName);
    return { success: true };
}

/**
 * 重新排序测试用例
 */
function reorderTestcases(projectPath, orderMap) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const update = db.prepare(`
        UPDATE testcases SET sort_order = ?, updated_at = ? WHERE folder_name = ?
    `);

    const batchUpdate = db.transaction((items) => {
        for (const [folderName, sortOrder] of Object.entries(items)) {
            update.run(sortOrder, now, folderName);
        }
    });

    batchUpdate(orderMap);
    return { success: true };
}

/**
 * 批量保存测试用例（用于导入/迁移）
 */
function batchSaveTestcases(projectPath, testcaseMap) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const insert = db.prepare(`
        INSERT OR REPLACE INTO testcases (sort_order, folder_name, display_name, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
    `);

    const batchInsert = db.transaction((items) => {
        for (const [sortOrder, folderName] of Object.entries(items)) {
            insert.run(
                parseInt(sortOrder),
                folderName,
                folderName,
                now,
                now
            );
        }
    });

    batchInsert(testcaseMap);
    return { success: true, count: Object.keys(testcaseMap).length };
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

    ipcMain.handle('db-testcase-getMap', async (event, projectPath) => {
        try {
            const map = getTestcaseMap(projectPath);
            return { success: true, data: map };
        } catch (error) {
            console.error('获取 testcase map 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-get', async (event, projectPath, folderName) => {
        try {
            const testcase = getTestcase(projectPath, folderName);
            return { success: true, data: testcase };
        } catch (error) {
            console.error('获取 testcase 失败:', error);
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

    ipcMain.handle('db-testcase-update', async (event, projectPath, folderName, updates) => {
        try {
            return updateTestcase(projectPath, folderName, updates);
        } catch (error) {
            console.error('更新 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-delete', async (event, projectPath, folderName) => {
        try {
            return deleteTestcase(projectPath, folderName);
        } catch (error) {
            console.error('删除 testcase 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-reorder', async (event, projectPath, orderMap) => {
        try {
            return reorderTestcases(projectPath, orderMap);
        } catch (error) {
            console.error('重排序 testcases 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-testcase-batchSave', async (event, projectPath, testcaseMap) => {
        try {
            return batchSaveTestcases(projectPath, testcaseMap);
        } catch (error) {
            console.error('批量保存 testcases 失败:', error);
            return { success: false, error: error.message };
        }
    });

    console.log('Testcase 数据库处理器已注册');
}

module.exports = {
    getAllTestcases,
    getTestcaseMap,
    getTestcase,
    createTestcase,
    updateTestcase,
    deleteTestcase,
    reorderTestcases,
    batchSaveTestcases,
    registerTestcaseHandlers
};
