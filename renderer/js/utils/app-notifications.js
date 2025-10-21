// 应用层通知模块
// 用于应用层面的操作反馈(保存成功、加载失败等)
// 显示为右下角Toast,保持精简,无emoji

const AppNotifications = {
    /**
     * 显示Toast通知
     * @param {string} message - 消息内容
     * @param {string} type - 通知类型: success, error, warning, info
     */
    show(message, type = 'info') {
        // 创建Toast元素
        const toast = document.createElement('div');
        toast.className = `app-toast app-toast-${type}`;
        toast.textContent = message;

        // 设置样式
        const colors = {
            success: '#4ec9b0',
            error: '#f48771',
            warning: '#ce9178',
            info: '#569cd6'
        };

        toast.style.cssText = `
            position: fixed;
            bottom: 20px;
            right: 20px;
            padding: 12px 20px;
            background: ${colors[type] || colors.info};
            color: white;
            border-radius: 6px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
            z-index: 10000;
            animation: slideInUp 0.3s ease;
            font-size: 13px;
            max-width: 400px;
            word-wrap: break-word;
        `;

        document.body.appendChild(toast);

        // 3秒后自动移除
        setTimeout(() => {
            toast.style.animation = 'slideOutDown 0.3s ease';
            setTimeout(() => {
                if (toast.parentNode) {
                    document.body.removeChild(toast);
                }
            }, 300);
        }, 3000);
    },

    /**
     * 显示成功通知
     * @param {string} message - 通知消息
     */
    success(message) {
        this.show(message, 'success');
    },

    /**
     * 显示错误通知
     * @param {string} message - 错误消息
     */
    error(message) {
        this.show(message, 'error');
    },

    /**
     * 显示警告通知
     * @param {string} message - 警告消息
     */
    warn(message) {
        this.show(message, 'warning');
    },

    /**
     * 显示信息通知
     * @param {string} message - 信息消息
     */
    info(message) {
        this.show(message, 'info');
    },

    // === 常用场景的便捷方法 ===

    /**
     * 文件保存成功
     * @param {string} fileName - 文件名(可选)
     */
    fileSaved(fileName) {
        const msg = fileName ? `已保存: ${fileName}` : '保存成功';
        this.success(msg);
    },

    /**
     * 文件保存失败
     * @param {string} error - 错误信息
     */
    fileSaveFailed(error) {
        this.error(`保存失败: ${error}`);
    },

    /**
     * 项目打开成功
     * @param {string} projectName - 项目名称
     */
    projectOpened(projectName) {
        this.success(`已打开: ${projectName}`);
    },

    /**
     * 需要选择设备
     */
    deviceRequired() {
        this.warn('请先选择设备');
    },

    /**
     * 需要打开项目
     */
    projectRequired() {
        this.warn('请先打开项目');
    },

    /**
     * 复制成功
     */
    copied() {
        this.success('已复制');
    }
};

// 导出到全局
window.AppNotifications = AppNotifications;

// 兼容旧的NotificationModule接口
window.NotificationModule = {
    showNotification: (message, type) => AppNotifications.show(message, type)
};

window.rLog('✅ 应用通知模块已加载');
