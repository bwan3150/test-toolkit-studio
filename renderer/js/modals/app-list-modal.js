// App 列表模态窗模块
// 负责显示设备上的所有第三方应用,并提供卸载/启动操作
// 依赖：window.AppGlobals

(function() {
    let currentDeviceId = null;

    // 获取全局变量
    function getGlobals() {
        return window.AppGlobals;
    }

    // 打开模态窗
    async function open(deviceId) {
        if (!deviceId) {
            window.rError('设备ID不能为空');
            return;
        }

        currentDeviceId = deviceId;
        const modal = document.getElementById('appListModal');
        if (!modal) {
            window.rError('找不到App列表模态窗');
            return;
        }

        // 显示模态窗
        modal.style.display = 'flex';

        // 加载App列表
        await loadAppList();
    }

    // 关闭模态窗
    function close() {
        const modal = document.getElementById('appListModal');
        if (modal) {
            modal.style.display = 'none';
        }
        currentDeviceId = null;
    }

    // 加载App列表
    async function loadAppList() {
        if (!currentDeviceId) return;

        const loadingDiv = document.getElementById('appListLoading');
        const contentDiv = document.getElementById('appListContent');
        const emptyDiv = document.getElementById('appListEmpty');

        // 显示加载中
        if (loadingDiv) loadingDiv.style.display = 'flex';
        if (contentDiv) contentDiv.innerHTML = '';
        if (emptyDiv) emptyDiv.style.display = 'none';

        try {
            const { ipcRenderer } = getGlobals();
            const result = await ipcRenderer.invoke('tke-app-list', { deviceId: currentDeviceId });

            if (loadingDiv) loadingDiv.style.display = 'none';

            if (result.success) {
                const data = JSON.parse(result.output);
                const apps = data.apps || [];

                if (apps.length === 0) {
                    if (emptyDiv) emptyDiv.style.display = 'block';
                } else {
                    renderAppList(apps);
                }
            } else {
                window.rError('加载App列表失败:', result.error);
                if (emptyDiv) {
                    emptyDiv.innerHTML = '<p>加载失败: ' + (result.error || '未知错误') + '</p>';
                    emptyDiv.style.display = 'block';
                }
            }
        } catch (error) {
            if (loadingDiv) loadingDiv.style.display = 'none';
            window.rError('加载App列表异常:', error);
            if (emptyDiv) {
                emptyDiv.innerHTML = '<p>加载失败: ' + error.message + '</p>';
                emptyDiv.style.display = 'block';
            }
        }
    }

    // 渲染App列表
    function renderAppList(apps) {
        const contentDiv = document.getElementById('appListContent');
        if (!contentDiv) return;

        let html = '<div class="app-list-table">';

        // 表头
        html += `
            <div class="app-list-row app-list-header-row">
                <div class="app-list-cell app-name-cell">应用包名</div>
                <div class="app-list-cell app-version-cell">版本</div>
                <div class="app-list-cell app-actions-cell">操作</div>
            </div>
        `;

        // 应用行
        apps.forEach(app => {
            const packageName = app.package_name || '';
            const versionName = app.version_name || '-';
            const versionCode = app.version_code || '-';
            const versionText = versionName !== '-' ? `${versionName} (${versionCode})` : '-';

            html += `
                <div class="app-list-row" data-package="${packageName}">
                    <div class="app-list-cell app-name-cell">
                        <div class="app-name" title="${packageName}">${packageName}</div>
                    </div>
                    <div class="app-list-cell app-version-cell">
                        <span class="app-version">${versionText}</span>
                    </div>
                    <div class="app-list-cell app-actions-cell">
                        <button class="btn btn-outline btn-small btn-app-launch" data-package="${packageName}" title="启动应用" disabled>
                            <svg viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
                                <path d="M8 5v14l11-7z"/>
                            </svg>
                            启动
                        </button>
                        <button class="btn btn-danger btn-small btn-app-uninstall" data-package="${packageName}" title="卸载应用">
                            <svg viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
                                <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                            </svg>
                            卸载
                        </button>
                    </div>
                </div>
            `;
        });

        html += '</div>';
        contentDiv.innerHTML = html;

        // 绑定卸载按钮事件
        const uninstallBtns = contentDiv.querySelectorAll('.btn-app-uninstall');
        uninstallBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                const packageName = btn.getAttribute('data-package');
                uninstallApp(packageName);
            });
        });

        // 启动按钮暂时禁用(未实现)
        // const launchBtns = contentDiv.querySelectorAll('.btn-app-launch');
        // launchBtns.forEach(btn => {
        //     btn.addEventListener('click', () => {
        //         const packageName = btn.getAttribute('data-package');
        //         launchApp(packageName);
        //     });
        // });
    }

    // 卸载应用
    async function uninstallApp(packageName) {
        if (!confirm(`确定要卸载应用 ${packageName} 吗?`)) {
            return;
        }

        try {
            const { ipcRenderer } = getGlobals();
            const result = await ipcRenderer.invoke('tke-app-uninstall', {
                package: packageName,
                deviceId: currentDeviceId
            });

            if (result.success) {
                const data = JSON.parse(result.output);
                if (data.success) {
                    window.rLog('卸载成功:', data.message);
                    if (window.AppNotifications) {
                        window.AppNotifications.success(data.message);
                    }
                    // 重新加载列表
                    await loadAppList();
                } else {
                    window.rError('卸载失败:', data.message);
                    if (window.AppNotifications) {
                        window.AppNotifications.error(data.message);
                    }
                }
            } else {
                window.rError('卸载失败:', result.error);
                if (window.AppNotifications) {
                    window.AppNotifications.error(result.error || '卸载失败');
                }
            }
        } catch (error) {
            window.rError('卸载应用异常:', error);
            if (window.AppNotifications) {
                window.AppNotifications.error('卸载失败: ' + error.message);
            }
        }
    }

    // 启动应用 (TODO: 未实现)
    // async function launchApp(packageName) {
    //     window.rLog('启动应用:', packageName);
    //     // TODO: 实现启动应用逻辑
    // }

    // 初始化事件监听
    document.addEventListener('DOMContentLoaded', function() {
        // 关闭按钮
        const closeBtn = document.getElementById('closeAppListModal');
        if (closeBtn) {
            closeBtn.addEventListener('click', close);
        }

        // 点击模态窗外部关闭
        const modal = document.getElementById('appListModal');
        if (modal) {
            modal.addEventListener('click', function(e) {
                if (e.target === modal) {
                    close();
                }
            });
        }

        // 刷新按钮
        const refreshBtn = document.getElementById('refreshAppListBtn');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', loadAppList);
        }
    });

    // 导出模块
    window.AppListModal = {
        open,
        close
    };
})();
