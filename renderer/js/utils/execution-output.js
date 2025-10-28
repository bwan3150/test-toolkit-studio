// 脚本执行输出模块
// 专门用于.tks脚本执行过程中的输出信息
// 只负责生成日志数据,不涉及UI渲染
// UI渲染由 console-panel.js 负责

const ExecutionOutput = {
    /**
     * 输出脚本开始执行信息
     */
    scriptStart() {
        this.log('info', `开始执行脚本`);
    },

    /**
     * 输出脚本执行成功信息
     * @param {number} duration - 执行时长(毫秒)
     */
    scriptSuccess(duration) {
        const seconds = (duration / 1000).toFixed(2);
        this.log('success', `脚本执行成功 (耗时 ${seconds}s)`);
    },

    /**
     * 输出脚本执行失败信息
     */
    scriptFailed() {
        this.log('error', `脚本执行失败`);
    },

    /**
     * 输出步骤开始执行信息
     * @param {number} stepIndex - 步骤索引(从1开始)
     * @param {number} totalSteps - 总步骤数
     * @param {string} command - 命令内容
     */
    stepStart(stepIndex, totalSteps, command) {
        this.log('info', `[${stepIndex}/${totalSteps}] ${command}`);
    },

    /**
     * 输出步骤执行成功信息
     * @param {number} stepIndex - 步骤索引(从1开始)
     * @param {number} duration - 执行时长(毫秒)
     */
    stepSuccess(stepIndex, duration) {
        const seconds = (duration / 1000).toFixed(2);
        this.log('success', `[${stepIndex}] 执行成功 (${seconds}s)`);
    },

    /**
     * 输出步骤执行失败信息
     * @param {number} stepIndex - 步骤索引(从1开始)
     * @param {string} error - 错误信息
     */
    stepFailed(stepIndex, error) {
        this.log('error', `[${stepIndex}] 执行失败: ${error}`);
    },

    /**
     * 输出步骤被跳过信息
     * @param {number} stepIndex - 步骤索引(从1开始)
     * @param {string} reason - 跳过原因
     */
    stepSkipped(stepIndex, reason) {
        this.log('warn', `[${stepIndex}] 已跳过: ${reason}`);
    },

    /**
     * 输出设备连接信息
     * @param {string} deviceId - 设备ID
     */
    deviceConnected(deviceId) {
        this.log('info', `设备已连接: ${deviceId}`);
    },

    /**
     * 输出设备断开信息
     * @param {string} deviceId - 设备ID
     */
    deviceDisconnected(deviceId) {
        this.log('warn', `设备已断开: ${deviceId}`);
    },

    /**
     * 输出警告信息
     * @param {string} message - 警告信息
     */
    warn(message) {
        this.log('warn', message);
    },

    /**
     * 输出错误信息
     * @param {string} message - 错误信息
     */
    error(message) {
        this.log('error', message);
    },

    /**
     * 输出普通信息
     * @param {string} message - 信息内容
     */
    info(message) {
        this.log('info', message);
    },

    /**
     * 核心日志方法 - 发送日志到控制台面板
     * @param {string} type - 日志类型: info, success, warn, error
     * @param {string} message - 日志消息
     */
    log(type, message) {
        // 触发自定义事件,由 console-panel.js 监听并渲染
        const event = new CustomEvent('execution-log', {
            detail: {
                type,
                message,
                timestamp: new Date(),
                source: 'execution'
            }
        });
        document.dispatchEvent(event);

        // 同时输出到开发者控制台(用于调试)
        if (window.rLog) {
            window.rLog(`[Execution] [${type.toUpperCase()}] ${message}`);
        }
    }
};

// 导出到全局
window.ExecutionOutput = ExecutionOutput;

window.rLog('✅ 执行输出模块已加载');
