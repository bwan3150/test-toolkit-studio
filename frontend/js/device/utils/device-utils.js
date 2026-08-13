// 设备管理工具函数
// 提供设备管理中常用的辅助功能

/**
 * 复制到剪贴板的辅助函数
 * @param {string} elementId - 包含要复制内容的元素ID
 */
function copyToClipboard(elementId) {
    const element = document.getElementById(elementId);
    if (element && element.textContent && element.textContent !== '-') {
        navigator.clipboard.writeText(element.textContent).then(() => {
            window.AppNotifications?.success('已复制到剪贴板');
        }).catch(err => {
            window.AppNotifications?.error('复制失败');
        });
    }
}

// 导出函数
window.copyToClipboard = copyToClipboard;
