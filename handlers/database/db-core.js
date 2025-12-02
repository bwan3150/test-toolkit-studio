// SQLite 数据库核心模块
// 负责数据库连接管理和初始化

const Database = require('better-sqlite3');
const path = require('path');
const fs = require('fs');

// 数据库连接缓存 (projectPath -> db instance)
const dbConnections = new Map();

// 数据库文件名
const DB_FILENAME = 'project.db';

/**
 * 获取项目数据库路径
 * @param {string} projectPath - 项目根目录
 * @returns {string} 数据库文件完整路径
 */
function getDbPath(projectPath) {
    return path.join(projectPath, DB_FILENAME);
}

/**
 * 获取或创建数据库连接
 * @param {string} projectPath - 项目根目录
 * @returns {Database} 数据库实例
 */
function getDatabase(projectPath) {
    if (!projectPath) {
        throw new Error('项目路径不能为空');
    }

    // 检查缓存
    if (dbConnections.has(projectPath)) {
        return dbConnections.get(projectPath);
    }

    const dbPath = getDbPath(projectPath);

    // 创建数据库连接
    const db = new Database(dbPath);

    // 启用外键约束
    db.pragma('foreign_keys = ON');

    // 缓存连接
    dbConnections.set(projectPath, db);

    // 确保表已创建
    ensureTables(db);

    console.log(`数据库已连接: ${dbPath}`);
    return db;
}

/**
 * 确保数据库表已创建
 * @param {Database} db - 数据库实例
 */
function ensureTables(db) {
    // 创建 locators 表
    db.exec(`
        CREATE TABLE IF NOT EXISTS locators (
            name TEXT PRIMARY KEY,
            note TEXT,
            xpath TEXT,
            resource_id TEXT,
            text TEXT,
            content_desc TEXT,
            class_name TEXT,
            bounds_x1 INTEGER,
            bounds_y1 INTEGER,
            bounds_x2 INTEGER,
            bounds_y2 INTEGER,
            clickable INTEGER DEFAULT 0,
            enabled INTEGER DEFAULT 1,
            img_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    `);

    // 创建 testcases 表
    // id: 唯一测试用例编号(主键递增)
    // case_name: 测试用例名称
    // note: 测试用例备注
    // folder_name: 用例文件夹路径(如 cases/case_001)，可为空，创建文件夹后才填充
    // result: 测试结果状态 (PASS/FAILED/NA)
    // task_batch: 批量测试任务分组，多选用逗号分隔
    // ai_test: 是否启用AI测试 (0/1)
    // ai_prompt: 给AI的提示词
    // ai_result: AI测试结果 (PASS/FAILED/NA)
    // ai_comment: AI测试后的总结
    db.exec(`
        CREATE TABLE IF NOT EXISTS testcases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            case_name TEXT NOT NULL,
            note TEXT DEFAULT '',
            folder_name TEXT DEFAULT '',
            result TEXT DEFAULT 'NA',
            task_batch TEXT DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            ai_test INTEGER DEFAULT 0,
            ai_prompt TEXT DEFAULT '',
            ai_result TEXT DEFAULT '',
            ai_comment TEXT DEFAULT ''
        )
    `);

    // 创建 devices 表（device_name 作为主键）
    db.exec(`
        CREATE TABLE IF NOT EXISTS devices (
            device_name TEXT PRIMARY KEY,
            model TEXT,
            note TEXT,
            platform TEXT NOT NULL,
            platform_version TEXT,
            platform_name TEXT,
            connection_type TEXT,
            device_id TEXT,
            port TEXT,
            bundle_id TEXT,
            wda_port TEXT,
            automation_name TEXT,
            new_command_timeout TEXT DEFAULT '6000',
            no_reset INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
    `);

}

/**
 * 关闭项目数据库连接
 * @param {string} projectPath - 项目根目录
 */
function closeDatabase(projectPath) {
    if (dbConnections.has(projectPath)) {
        const db = dbConnections.get(projectPath);
        db.close();
        dbConnections.delete(projectPath);
        console.log(`数据库已关闭: ${projectPath}`);
    }
}

/**
 * 关闭所有数据库连接
 */
function closeAllDatabases() {
    for (const [projectPath, db] of dbConnections) {
        db.close();
        console.log(`数据库已关闭: ${projectPath}`);
    }
    dbConnections.clear();
}

/**
 * 初始化数据库表结构（现在由 getDatabase 自动调用 ensureTables）
 * @param {string} projectPath - 项目根目录
 */
function initializeTables(projectPath) {
    // getDatabase 会自动调用 ensureTables 创建表
    getDatabase(projectPath);
    console.log(`数据库表已初始化: ${projectPath}`);
}

/**
 * 创建新项目数据库
 * @param {string} projectPath - 项目根目录
 */
function createProjectDatabase(projectPath) {
    const dbPath = getDbPath(projectPath);

    // 如果数据库已存在，先关闭连接
    closeDatabase(projectPath);

    // 删除旧数据库文件（如果存在）
    if (fs.existsSync(dbPath)) {
        fs.unlinkSync(dbPath);
    }

    // 初始化表结构
    initializeTables(projectPath);

    console.log(`新项目数据库已创建: ${dbPath}`);
}

/**
 * 检查数据库是否存在
 * @param {string} projectPath - 项目根目录
 * @returns {boolean}
 */
function databaseExists(projectPath) {
    return fs.existsSync(getDbPath(projectPath));
}

module.exports = {
    getDatabase,
    closeDatabase,
    closeAllDatabases,
    initializeTables,
    createProjectDatabase,
    databaseExists,
    getDbPath,
    DB_FILENAME
};
