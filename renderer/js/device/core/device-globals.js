// 设备管理全局变量管理
// 提供访问应用全局变量的统一接口

/**
 * 获取全局变量的辅助函数
 * @returns {Object} 包含应用全局变量的对象
 */
function getGlobals() {
    return window.AppGlobals;
}

// 导出函数（用于模块化引用）
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { getGlobals };
}
