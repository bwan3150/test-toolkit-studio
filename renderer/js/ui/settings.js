// 设置页面模块

// 初始化设置页面
function initializeSettingsPage() {
    const { ipcRenderer } = window.AppGlobals;
    const logoutBtn = document.getElementById('logoutBtn');
    const checkSdkBtn = document.getElementById('checkSdkBtn');
    const updateBaseUrlBtn = document.getElementById('updateBaseUrlBtn');
    const settingsBaseUrl = document.getElementById('settingsBaseUrl');
    const aboutVersion = document.getElementById('about-version');
    
    // 加载应用版本
    if (aboutVersion) {
        ipcRenderer.invoke('get-app-version').then(version => {
            aboutVersion.textContent = `Version: ${version}`;
        });
    }
    
    if (logoutBtn) {
        logoutBtn.addEventListener('click', async () => {
            await ipcRenderer.invoke('logout');
            await ipcRenderer.invoke('navigate-to-login');
        });
    }
    
    if (checkSdkBtn) {
        checkSdkBtn.addEventListener('click', checkSDKStatus);
    }
    
    if (updateBaseUrlBtn) {
        updateBaseUrlBtn.addEventListener('click', async () => {
            await ipcRenderer.invoke('store-set', 'base_url', settingsBaseUrl.value);
            window.NotificationModule.showNotification('Base URL updated', 'success');
        });
    }
    
    // 加载基础URL
    ipcRenderer.invoke('store-get', 'base_url').then(url => {
        if (url && settingsBaseUrl) {
            settingsBaseUrl.value = url;
        }
    });
}

// 加载用户信息
async function loadUserInfo() {
    const { ipcRenderer } = window.AppGlobals;
    const result = await ipcRenderer.invoke('get-user-info');
    
    if (result.success && result.data.user) {
        const user = result.data.user;
        
        // 更新侧边栏
        const userAvatar = document.querySelector('.user-avatar');
        const userName = document.querySelector('.user-name');
        
        if (userAvatar) userAvatar.textContent = user.username.charAt(0).toUpperCase();
        if (userName) userName.textContent = user.username;
        
        // 更新设置页面
        const settingsUsername = document.getElementById('settingsUsername');
        const settingsEmail = document.getElementById('settingsEmail');
        const settingsMemberGroup = document.getElementById('settingsMemberGroup');
        const settingsMemberSince = document.getElementById('settingsMemberSince');
        
        if (settingsUsername) settingsUsername.textContent = user.username;
        if (settingsEmail) settingsEmail.textContent = user.email;
        if (settingsMemberGroup) settingsMemberGroup.textContent = user.memberGroup;
        if (settingsMemberSince) settingsMemberSince.textContent = new Date(user.createdAt).toLocaleDateString();
    }
}

// 检查SDK状态
async function checkSDKStatus() {
    const { ipcRenderer } = window.AppGlobals;
    const sdkStatus = document.getElementById('sdkStatus');
    const adbStatus = document.getElementById('adbStatus');
    
    if (sdkStatus) sdkStatus.textContent = 'Checking...';
    if (adbStatus) adbStatus.textContent = 'Checking...';
    
    // 检查ADB
    const result = await ipcRenderer.invoke('adb-devices');
    
    if (result.success) {
        if (adbStatus) {
            adbStatus.textContent = `Available (v${result.adbVersion || 'Unknown'})`;
            adbStatus.className = 'status-indicator success';
        }
        if (sdkStatus) {
            const platform = process.platform === 'darwin' ? 'macOS' : process.platform === 'win32' ? 'Windows' : 'Linux';
            sdkStatus.textContent = `Built-in SDK (${platform})`;
            sdkStatus.className = 'status-indicator success';
        }
    } else {
        if (adbStatus) {
            adbStatus.textContent = 'Not Available';
            adbStatus.className = 'status-indicator error';
        }
        if (sdkStatus) {
            sdkStatus.textContent = 'Not Found';
            sdkStatus.className = 'status-indicator error';
        }
    }
}

// 导出函数
window.SettingsModule = {
    initializeSettingsPage,
    loadUserInfo,
    checkSDKStatus
};