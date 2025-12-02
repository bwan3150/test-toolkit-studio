// Device 设备配置数据库操作模块
// 项目级设备配置，存储在项目数据库中
// device_name 作为主键

const { ipcMain } = require('electron');
const { getDatabase } = require('./db-core');

/**
 * 将数据库行转换为 device 对象
 */
function rowToDevice(row) {
    if (!row) return null;
    return {
        deviceName: row.device_name,
        model: row.model,
        note: row.note,
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
function getDevice(projectPath, deviceName) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT * FROM devices WHERE device_name = ?').get(deviceName);
    return rowToDevice(row);
}

/**
 * 检查设备名是否存在
 */
function deviceExists(projectPath, deviceName) {
    const db = getDatabase(projectPath);
    const row = db.prepare('SELECT device_name FROM devices WHERE device_name = ?').get(deviceName);
    return !!row;
}

/**
 * 创建或更新设备配置
 * @param {string} projectPath - 项目路径
 * @param {object} device - 设备配置
 * @param {string} oldDeviceName - 原设备名（编辑时用于重命名）
 */
function saveDevice(projectPath, device, oldDeviceName = null) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    // 根据 platform 自动设置 platformName
    const platformName = device.platform === 'ios' ? 'iOS' : 'Android';

    const isEditing = !!oldDeviceName;
    const isRenaming = isEditing && oldDeviceName !== device.deviceName;

    // 检查新名称是否已存在（新增或重命名时）
    if (!isEditing || isRenaming) {
        if (deviceExists(projectPath, device.deviceName)) {
            return { success: false, error: `设备名称 "${device.deviceName}" 已存在` };
        }
    }

    if (isEditing) {
        if (isRenaming) {
            // 重命名：先删除旧记录，再插入新记录
            db.prepare('DELETE FROM devices WHERE device_name = ?').run(oldDeviceName);
        } else {
            // 更新（不改名）
            db.prepare(`
                UPDATE devices SET
                    model = ?, note = ?, platform = ?, platform_version = ?,
                    platform_name = ?, connection_type = ?, device_id = ?,
                    port = ?, bundle_id = ?, wda_port = ?, automation_name = ?,
                    new_command_timeout = ?, no_reset = ?, updated_at = ?
                WHERE device_name = ?
            `).run(
                device.model || '',
                device.note || '',
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
                device.deviceName
            );
            return { success: true, deviceName: device.deviceName };
        }
    }

    // 插入新记录（新增或重命名后）
    db.prepare(`
        INSERT INTO devices (
            device_name, model, note, platform, platform_version, platform_name,
            connection_type, device_id, port, bundle_id, wda_port,
            automation_name, new_command_timeout, no_reset,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
        device.deviceName,
        device.model || '',
        device.note || '',
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

    return { success: true, deviceName: device.deviceName };
}

/**
 * 删除设备配置
 */
function deleteDevice(projectPath, deviceName) {
    const db = getDatabase(projectPath);
    db.prepare('DELETE FROM devices WHERE device_name = ?').run(deviceName);
    return { success: true };
}

/**
 * 批量保存设备（用于导入/迁移）
 */
function batchSaveDevices(projectPath, devices) {
    const db = getDatabase(projectPath);
    const now = new Date().toISOString();

    const insert = db.prepare(`
        INSERT OR REPLACE INTO devices (
            device_name, model, note, platform, platform_version, platform_name,
            connection_type, device_id, port, bundle_id, wda_port,
            automation_name, new_command_timeout, no_reset,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const batchInsert = db.transaction((items) => {
        for (const device of items) {
            const platformName = device.platform === 'ios' ? 'iOS' : 'Android';
            insert.run(
                device.deviceName,
                device.model || '',
                device.note || '',
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

    ipcMain.handle('db-device-get', async (event, projectPath, deviceName) => {
        try {
            const device = getDevice(projectPath, deviceName);
            return { success: true, data: device };
        } catch (error) {
            console.error('获取 device 失败:', error);
            return { success: false, error: error.message };
        }
    });

    // 保存设备（新增或编辑）
    // device 对象包含设备配置
    // oldDeviceName 用于编辑时标识原设备（可选）
    ipcMain.handle('db-device-save', async (event, projectPath, device, oldDeviceName) => {
        try {
            return saveDevice(projectPath, device, oldDeviceName);
        } catch (error) {
            console.error('保存 device 失败:', error);
            return { success: false, error: error.message };
        }
    });

    ipcMain.handle('db-device-delete', async (event, projectPath, deviceName) => {
        try {
            return deleteDevice(projectPath, deviceName);
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
    deviceExists,
    saveDevice,
    deleteDevice,
    batchSaveDevices,
    registerDeviceDbHandlers
};
