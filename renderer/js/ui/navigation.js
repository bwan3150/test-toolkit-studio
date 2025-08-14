// 导航管理模块
function initializeNavigation() {
    const navItems = document.querySelectorAll('.nav-item');
    const pages = document.querySelectorAll('.page');
    
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const targetPage = item.dataset.page;
            
            // 更新激活的导航项
            navItems.forEach(nav => nav.classList.remove('active'));
            item.classList.add('active');
            
            // 更新激活的页面
            pages.forEach(page => page.classList.remove('active'));
            document.getElementById(`${targetPage}Page`).classList.add('active');
            
            // 页面特定的动作
            if (targetPage === 'device') {
                window.DeviceManagerModule.refreshConnectedDevices();
            } else if (targetPage === 'log') {
                if (window.LogManagerModule) {
                    window.LogManagerModule.refreshDeviceList();
                }
            } else if (targetPage === 'settings') {
                window.SettingsModule.checkSDKStatus();
            } else if (targetPage === 'report') {
                if (window.TestReportModule) {
                    window.TestReportModule.onPageActivated();
                }
            }
        });
    });
}

// 导出函数
window.NavigationModule = {
    initializeNavigation
};