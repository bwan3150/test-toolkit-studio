// 数据库模块统一入口
// 负责注册所有数据库相关的 IPC 处理器

const dbCore = require('./db-core');
const locatorDb = require('./locator-db');
const testcaseDb = require('./testcase-db');
const deviceDb = require('./device-db');

/**
 * 注册所有数据库相关的 IPC 处理器
 */
function registerDatabaseHandlers() {
    locatorDb.registerLocatorHandlers();
    testcaseDb.registerTestcaseHandlers();
    deviceDb.registerDeviceDbHandlers();

    console.log('✅ 所有数据库处理器已注册');
}

module.exports = {
    // 核心模块
    ...dbCore,

    // Locator 操作
    ...locatorDb,

    // Testcase 操作
    ...testcaseDb,

    // Device 操作
    ...deviceDb,

    // 统一注册
    registerDatabaseHandlers
};
