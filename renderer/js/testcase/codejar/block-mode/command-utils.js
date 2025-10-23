// 命令工具方法 - 用于查找和获取命令定义信息

const CommandUtils = {
    /**
     * 根据命令类型查找命令定义
     * @param {string} type - 命令类型
     * @returns {Object|null} 命令定义对象
     */
    findCommandDefinition(type) {
        for (const category of Object.values(window.BlockDefinitions)) {
            const cmd = category.commands.find(c => c.type === type);
            if (cmd) return cmd;
        }
        return null;
    },

    /**
     * 根据命令类型查找所属类别
     * @param {string} type - 命令类型
     * @returns {Object|null} 类别对象
     */
    findCommandCategory(type) {
        for (const [key, category] of Object.entries(window.BlockDefinitions)) {
            if (category.commands.find(c => c.type === type)) {
                return category;
            }
        }
        return null;
    },

    /**
     * 获取类别的中文名称
     * @param {string} key - 类别键名
     * @returns {string} 类别中文名称
     */
    getCategoryName(key) {
        const names = {
            application: '应用控制',
            action: '动作操作',
            input: '输入操作',
            control: '时间控制',
            navigation: '导航操作',
            assertion: '断言验证'
        };
        return names[key] || key;
    }
};

// 导出到全局
window.CommandUtils = CommandUtils;

if (window.rLog) {
    window.rLog('✅ CommandUtils 模块已加载');
}
