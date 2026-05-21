// iOS WebDriverAgent (WDA) 辅助功能
// 负责显示WDA设置指导和重试连接
// 依赖：renderer-logger.js, window.AppNotifications

// 显示WDA设置指导
function showWdaSetupGuide(deviceId) {
    const guideContent = `
        <div class="wda-guide-modal">
            <h3>iOS WebDriverAgent 设置指导</h3>
            <div class="guide-content">
                <p><strong>设备ID:</strong> ${deviceId}</p>

                <h4>WDA服务未连接的可能原因：</h4>
                <ul>
                    <li>iOS设备上的WebDriverAgent应用未安装或未启动</li>
                    <li>WDA服务端口配置错误（默认8100）</li>
                    <li>网络连接问题或防火墙阻止</li>
                    <li>iOS设备和电脑不在同一网络</li>
                </ul>

                <h4>解决步骤：</h4>
                <ol>
                    <li><strong>确认WDA安装：</strong>确保iOS设备上已安装WebDriverAgent应用</li>
                    <li><strong>启动WDA服务：</strong>在iOS设备上启动WebDriverAgent应用</li>
                    <li><strong>检查端口：</strong>确认WDA服务运行在正确端口（通常是8100）</li>
                    <li><strong>网络连接：</strong>确保iOS设备和电脑在同一WiFi网络</li>
                    <li><strong>防火墙设置：</strong>检查电脑和iOS设备的防火墙设置</li>
                </ol>

                <div class="guide-note">
                    <strong>提示：</strong>可以在iOS设备的Safari浏览器中访问 http://[设备IP]:8100/status 来测试WDA服务是否正常运行。
                </div>
            </div>
            <div class="guide-actions">
                <button class="btn btn-primary" onclick="hideWdaGuide()">知道了</button>
                <button class="btn btn-secondary" onclick="retryWdaConnection('${deviceId}')">重试连接</button>
            </div>
        </div>
        <div class="modal-overlay" onclick="hideWdaGuide()"></div>
    `;

    document.body.insertAdjacentHTML('beforeend', guideContent);
}

// 隐藏WDA设置指导
function hideWdaGuide() {
    const guide = document.querySelector('.wda-guide-modal');
    const overlay = document.querySelector('.modal-overlay');
    if (guide) guide.remove();
    if (overlay) overlay.remove();
}

// 重试WDA连接
async function retryWdaConnection(deviceId) {
    hideWdaGuide();
    window.AppNotifications?.info('正在重试WDA连接...');

    // 需要调用外部的refreshConnectedDevices
    if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
        await window.DeviceManagerModule.refreshConnectedDevices();
    }
}

// 导出函数和全局注册
window.WdaHelper = {
    showWdaSetupGuide,
    hideWdaGuide,
    retryWdaConnection
};

// 注册全局函数供HTML调用
window.showWdaSetupGuide = showWdaSetupGuide;
window.hideWdaGuide = hideWdaGuide;
window.retryWdaConnection = retryWdaConnection;
