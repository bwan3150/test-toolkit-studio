// Locator 元素定位数据库操作模块

const { ipcMain } = require('electron');
const { getDatabase } = require('./db-core');

/**
 * 将数据库行转换为 locator 对象
 */
function rowToLocator(row) {
    if (!row) return null;
    return {
        name: row.name,
        note: row.note,
        xpath: row.xpath,
        resource_id: row.resource_id,
        text: row.text,
        content_desc: row.content_desc,
        class_name: row.class_name,
        bounds: (row.bounds_x1 !== null) ? {
            x1: row.bounds_x1,
            y1: row.bounds_y1,
            x2: row.bounds_x2,
            y2: row.bounds_y2
        } : null,
        clickable: !!row.clickable,
        enabled: !!row.enabled,
        img_path: row.img_path,
        created_at: row.created_at,
        updated_at: row.updated_at
    };
}

/**
 * 获取所有 locators
 */
function getAllLocators(projectPath) {
    const db = getDatabase(projectPath);
    const rows = db.prepare('SELECT * FROM locators ORDER BY name').all();

    // 转换为 { name: locator } 格式，兼容旧代码
    const result = {};
    rows.forEach(row => {
        result[row.name] = rowToLocator(row);
    });
    return result;
}

/**
 * 获取单个 locator
 */
function getLocator(projectPath, name) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT * FROM locators WHERE name = ?').get(name);
    return rowToLocator(row);
}

/**
 * 创建或更新 locator
 */
function saveLocator(projectPath, locator) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const existing = db.prepare('SELECT name FROM locators WHERE name = ?').get(locator.name);

    if (existing) {
        // 更新
        db.prepare(`
            UPDATE locators SET
                note = ?, xpath = ?, resource_id = ?, text = ?, content_desc = ?,
                class_name = ?, bounds_x1 = ?, bounds_y1 = ?, bounds_x2 = ?, bounds_y2 = ?,
                clickable = ?, enabled = ?, img_path = ?, updated_at = ?
            WHERE name = ?
        `).run(
            locator.note || null,
            locator.xpath || null,
            locator.resource_id || null,
            locator.text || null,
            locator.content_desc || null,
            locator.class_name || null,
            locator.bounds?.x1 ?? null,
            locator.bounds?.y1 ?? null,
            locator.bounds?.x2 ?? null,
            locator.bounds?.y2 ?? null,
            locator.clickable ? 1 : 0,
            locator.enabled ? 1 : 0,
            locator.img_path || null,
            now,
            locator.name
        );
    } else {
        // 插入
        db.prepare(`
            INSERT INTO locators (
                name, note, xpath, resource_id, text, content_desc, class_name,
                bounds_x1, bounds_y1, bounds_x2, bounds_y2,
                clickable, enabled, img_path, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `).run(
            locator.name,
            locator.note || null,
            locator.xpath || null,
            locator.resource_id || null,
            locator.text || null,
            locator.content_desc || null,
            locator.class_name || null,
            locator.bounds?.x1 ?? null,
            locator.bounds?.y1 ?? null,
            locator.bounds?.x2 ?? null,
            locator.bounds?.y2 ?? null,
            locator.clickable ? 1 : 0,
            locator.enabled ? 1 : 0,
            locator.img_path || null,
            now,
            now
        );
    }

    return { success: true };
}

/**
 * 重命名 locator
 */
function renameLocator(projectPath, oldName, newName) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    // 检查新名称是否已存在
    const existing = db.prepare('SELECT name FROM locators WHERE name = ?').get(newName);
    if (existing) {
        return { success: false, error: '名称已存在' };
    }

    db.prepare(`
        UPDATE locators SET name = ?, updated_at = ? WHERE name = ?
    `).run(newName, now, oldName);

    return { success: true };
}

/**
 * 删除 locator
 */
function deleteLocator(projectPath, name) {
    const db = getDatabase(projectPath);
    db.prepare('DELETE FROM locators WHERE name = ?').run(name);
    return { success: true };
}

/**
 * 批量保存 locators（用于导入/迁移）
 */
function batchSaveLocators(projectPath, locators) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const insert = db.prepare(`
        INSERT OR REPLACE INTO locators (
            name, note, xpath, resource_id, text, content_desc, class_name,
            bounds_x1, bounds_y1, bounds_x2, bounds_y2,
            clickable, enabled, img_path, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const batchInsert = db.transaction((items) => {
        for (const [name, loc] of Object.entries(items)) {
            insert.run(
                name,
                loc.note || null,
                loc.xpath || null,
                loc.resource_id || null,
                loc.text || null,
                loc.content_desc || null,
                loc.class_name || null,
                loc.bounds?.x1 ?? null,
                loc.bounds?.y1 ?? null,
                loc.bounds?.x2 ?? null,
                loc.bounds?.y2 ?? null,
                loc.clickable ? 1 : 0,
                loc.enabled ? 1 : 0,
                loc.img_path || null,
                loc.created_at || now,
                loc.updated_at || now
            );
        }
    });

    batchInsert(locators);
    return { success: true, count: Object.keys(locators).length };
}

/**
 * 注册 Locator IPC 处理器
 */
function registerLocatorHandlers() {
    ipcMain.handle('db-locator-getAll', async (event, projectPath) => {
        try {
            const locators = getAllLocators(projectPath);
            return { success: true, data: locators };
        } catch (error) {
            console.error('获取 locators 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-locator-get', async (event, projectPath, name) => {
        try {
            const locator = getLocator(projectPath, name);
            return { success: true, data: locator };
        } catch (error) {
            console.error('获取 locator 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-locator-save', async (event, projectPath, locator) => {
        try {
            return saveLocator(projectPath, locator);
        } catch (error) {
            console.error('保存 locator 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-locator-rename', async (event, projectPath, oldName, newName) => {
        try {
            return renameLocator(projectPath, oldName, newName);
        } catch (error) {
            console.error('重命名 locator 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-locator-delete', async (event, projectPath, name) => {
        try {
            return deleteLocator(projectPath, name);
        } catch (error) {
            console.error('删除 locator 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-locator-batchSave', async (event, projectPath, locators) => {
        try {
            return batchSaveLocators(projectPath, locators);
        } catch (error) {
            console.error('批量保存 locators 失败:', error);
            return { success: false, error: error.message };
        }
    });

    console.log('Locator 数据库处理器已注册');
}

module.exports = {
    getAllLocators,
    getLocator,
    saveLocator,
    renameLocator,
    deleteLocator,
    batchSaveLocators,
    registerLocatorHandlers
};
