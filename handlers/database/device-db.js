// Device 设备配置数据库操作模块
// 项目级设备配置，存储在项目数据库中

const { ipcMain } = require('electron');
const { getDatabase } = require('./db-core');

/**
 * 将数据库行转换为 device 对象（与旧 YAML 结构一致）
 */
function rowToDevice(row) {
    if (!row) return null;
    return {
        id: row.id,
        deviceName: row.device_name,
        platform: row.platform,
        platformVersion: row.platform_version,
        platformName: row.platform_name,
        connectionType: row.connection_type,
        deviceId: row.device_id,
        port: row.port,
        bundleId: row.bundle_id,
        wdaPort: row.wda_port,
        automationName: row.automation_name,
        newCommandTimeout: row.new_command_timeout,
        noReset: !!row.no_reset,
        created_at: row.created_at,
        updated_at: row.updated_at
    };
}

/**
 * 获取所有设备配置
 */
function getAllDevices(projectPath) {
    const db = getDatabase(projectPath);
    const rows = db.prepare('SELECT * FROM devices ORDER BY device_name').all();
    return rows.map(rowToDevice);
}

/**
 * 获取单个设备配置
 */
function getDevice(projectPath, id) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT * FROM devices WHERE id = ?').get(id);
    return rowToDevice(row);
}

/**
 * 创建或更新设备配置（与旧 YAML 结构一致）
 */
function saveDevice(projectPath, device) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    // 生成 ID（如果没有）
    const id = device.id || `device_${Date.now()}`;

    // 根据 platform 自动设置 platformName
    const platformName = device.platform === 'ios' ? 'iOS' : 'Android';

    const existing = db.prepare('SELECT id FROM devices WHERE id = ?').get(id);

    if (existing) {
        // 更新
        db.prepare(`
            UPDATE devices SET
                device_name = ?, platform = ?, platform_version = ?,
                platform_name = ?, connection_type = ?, device_id = ?,
                port = ?, bundle_id = ?, wda_port = ?, automation_name = ?,
                new_command_timeout = ?, no_reset = ?, updated_at = ?
            WHERE id = ?
        `).run(
            device.deviceName,
            device.platform,
            device.platformVersion || '',
            platformName,
            device.connectionType || '',
            device.deviceId || '',
            device.port || '',
            device.bundleId || '',
            device.wdaPort || '',
            device.automationName || '',
            device.newCommandTimeout || '6000',
            device.noReset ? 1 : 0,
            now,
            id
        );
    } else {
        // 插入
        db.prepare(`
            INSERT INTO devices (
                id, device_name, platform, platform_version, platform_name,
                connection_type, device_id, port, bundle_id, wda_port,
                automation_name, new_command_timeout, no_reset,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        `).run(
            id,
            device.deviceName,
            device.platform,
            device.platformVersion || '',
            platformName,
            device.connectionType || '',
            device.deviceId || '',
            device.port || '',
            device.bundleId || '',
            device.wdaPort || '',
            device.automationName || '',
            device.newCommandTimeout || '6000',
            device.noReset ? 1 : 0,
            now,
            now
        );
    }

    return { success: true, id };
}

/**
 * 删除设备配置
 */
function deleteDevice(projectPath, id) {
    const db = getDatabase(projectPath);
    db.prepare('DELETE FROM devices WHERE id = ?').run(id);
    return { success: true };
}

/**
 * 批量保存设备（用于导入/迁移，与旧 YAML 结构一致）
 */
function batchSaveDevices(projectPath, devices) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const insert = db.prepare(`
        INSERT OR REPLACE INTO devices (
            id, device_name, platform, platform_version, platform_name,
            connection_type, device_id, port, bundle_id, wda_port,
            automation_name, new_command_timeout, no_reset,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const batchInsert = db.transaction((items) => {
        for (const device of items) {
            const id = device.id || `device_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
            const platformName = device.platform === 'ios' ? 'iOS' : 'Android';
            insert.run(
                id,
                device.deviceName,
                device.platform,
                device.platformVersion || '',
                platformName,
                device.connectionType || '',
                device.deviceId || '',
                device.port || '',
                device.bundleId || '',
                device.wdaPort || '',
                device.automationName || '',
                device.newCommandTimeout || '6000',
                device.noReset ? 1 : 0,
                device.created_at || now,
                device.updated_at || now
            );
        }
    });

    batchInsert(devices);
    return { success: true, count: devices.length };
}

/**
 * 注册 Device IPC 处理器
 */
function registerDeviceDbHandlers() {
    ipcMain.handle('db-device-getAll', async (event, projectPath) => {
        try {
            const devices = getAllDevices(projectPath);
            return { success: true, data: devices };
        } catch (error) {
            console.error('获取 devices 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-device-get', async (event, projectPath, id) => {
        try {
            const device = getDevice(projectPath, id);
            return { success: true, data: device };
        } catch (error) {
            console.error('获取 device 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-device-save', async (event, projectPath, device) => {
        try {
            return saveDevice(projectPath, device);
        } catch (error) {
            console.error('保存 device 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-device-delete', async (event, projectPath, id) => {
        try {
            return deleteDevice(projectPath, id);
        } catch (error) {
            console.error('删除 device 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-device-batchSave', async (event, projectPath, devices) => {
        try {
            return batchSaveDevices(projectPath, devices);
        } catch (error) {
            console.error('批量保存 devices 失败:', error);
            return { success: false, error: error.message };
        }
    });

    console.log('Device 数据库处理器已注册');
}

module.exports = {
    getAllDevices,
    getDevice,
    saveDevice,
    deleteDevice,
    batchSaveDevices,
    registerDeviceDbHandlers
};
