// 导航管理模块
function initializeNavigation() {
    const navItems = document.querySelectorAll('.nav-item');
    const pages = document.querySelectorAll('.page');
    
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const targetPage = item.dataset.page;
            console.log(`切换到页面: ${targetPage}`);
            
            // 更新激活的导航项
            navItems.forEach(nav => nav.classList.remove('active'));
            item.classList.add('active');
            
            // 更新激活的页面
            pages.forEach(page => page.classList.remove('active'));
            const targetPageElement = document.getElementById(`${targetPage}Page`);
            if (targetPageElement) {
                targetPageElement.classList.add('active');
                console.log(`页面 ${targetPage}Page 已激活`);
            } else {
                console.error(`页面元素 ${targetPage}Page 未找到`);
            }
            
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
            } else if (targetPage === 'testcase') {
                // 确保底部面板正确初始化和显示
                if (window.TestcaseManagerModule && window.TestcaseManagerModule.initializeBottomPanelDisplay) {
                    setTimeout(() => {
                        window.TestcaseManagerModule.initializeBottomPanelDisplay();
                    }, 100);
                }
            }
        });
    });
}

// 导出函数
window.NavigationModule = {
    initializeNavigation
};