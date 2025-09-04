// 设置页面模块

// 初始化设置页面
function initializeSettingsPage() {
    const { ipcRenderer } = window.AppGlobals;
    const logoutBtn = document.getElementById('logoutBtn');
    const checkToolsBtn = document.getElementById('checkToolsBtn');
    const updateBaseUrlBtn = document.getElementById('updateBaseUrlBtn');
    const settingsBaseUrl = document.getElementById('settingsBaseUrl');
    const aboutVersion = document.getElementById('about-version');
    
    // 编辑器字体设置
    const editorFontFamily = document.getElementById('editorFontFamily');
    const editorFontSize = document.getElementById('editorFontSize');
    const applyFontSettings = document.getElementById('applyFontSettings');
    
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
    
    if (checkToolsBtn) {
        checkToolsBtn.addEventListener('click', checkAllToolsStatus);
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
    
    // 应用字体设置
    if (applyFontSettings) {
        applyFontSettings.addEventListener('click', async () => {
            const fontFamily = editorFontFamily.value;
            const fontSize = parseInt(editorFontSize.value);
            
            if (fontSize < 10 || fontSize > 24) {
                window.NotificationModule.showNotification('字体大小必须在10-24px之间', 'error');
                return;
            }
            
            // 保存设置到本地存储
            await ipcRenderer.invoke('store-set', 'editor_font_family', fontFamily);
            await ipcRenderer.invoke('store-set', 'editor_font_size', fontSize);
            
            // 应用到当前编辑器
            applyEditorFontSettings(fontFamily, fontSize);
            
            window.NotificationModule.showNotification('字体设置已应用', 'success');
        });
    }
    
    // 加载已保存的字体设置
    loadEditorFontSettings();
}

// 加载编辑器字体设置
async function loadEditorFontSettings() {
    const { ipcRenderer } = window.AppGlobals;
    
    try {
        const fontFamily = await ipcRenderer.invoke('store-get', 'editor_font_family');
        const fontSize = await ipcRenderer.invoke('store-get', 'editor_font_size');
        
        const editorFontFamily = document.getElementById('editorFontFamily');
        const editorFontSize = document.getElementById('editorFontSize');
        
        if (fontFamily && editorFontFamily) {
            editorFontFamily.value = fontFamily;
        }
        
        if (fontSize && editorFontSize) {
            editorFontSize.value = fontSize;
        }
        
        // 应用到编辑器
        if (fontFamily || fontSize) {
            applyEditorFontSettings(
                fontFamily || "var(--font-mono)", 
                fontSize || 14
            );
        }
    } catch (error) {
        console.warn('Failed to load editor font settings:', error);
    }
}

// 应用字体设置到编辑器
function applyEditorFontSettings(fontFamily, fontSize) {
    // 更新编辑器字体设置
    if (window.AppGlobals.codeEditor && window.AppGlobals.codeEditor.updateFontSettings) {
        window.AppGlobals.codeEditor.updateFontSettings(fontFamily, fontSize);
    }
    
    // 更新CSS变量（如果使用）
    document.documentElement.style.setProperty('--editor-font-family', fontFamily);
    document.documentElement.style.setProperty('--editor-font-size', fontSize + 'px');
    document.documentElement.style.setProperty('--editor-line-height', (fontSize + 7) + 'px'); // fontSize + 7 for line-height
}

// 加载用户信息
async function loadUserInfo() {
    const { ipcRenderer } = window.AppGlobals;
    
    try {
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
        } else {
            // 获取用户信息失败，可能认证过期
            window.rWarn('获取用户信息失败，可能认证已过期');
            if (window.ApiClient) {
                await window.ApiClient.logout();
            }
        }
    } catch (error) {
        window.rError('获取用户信息时发生错误:', error);
        if (window.ApiClient) {
            await window.ApiClient.logout();
        }
    }
}

// 检查所有内置工具状态
async function checkAllToolsStatus() {
    const { ipcRenderer } = window.AppGlobals;
    const sdkStatus = document.getElementById('sdkStatus');
    const adbStatus = document.getElementById('adbStatus');
    const aaptStatus = document.getElementById('aaptStatus');
    const tkeStatus = document.getElementById('tkeStatus');
    
    // 重置所有状态为检查中
    if (sdkStatus) sdkStatus.textContent = 'Checking...';
    if (adbStatus) adbStatus.textContent = 'Checking...';
    if (aaptStatus) aaptStatus.textContent = 'Checking...';
    if (tkeStatus) tkeStatus.textContent = 'Checking...';
    
    const platform = process.platform === 'darwin' ? 'macOS' : process.platform === 'win32' ? 'Windows' : 'Linux';
    
    // 检查ADB和Android SDK
    try {
        const adbResult = await ipcRenderer.invoke('adb-devices');
        
        if (adbResult.success) {
            if (adbStatus) {
                adbStatus.textContent = `Available`;
                adbStatus.className = 'status-indicator success';
            }
            if (sdkStatus) {
                sdkStatus.textContent = `Built-in (${platform})`;
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
    } catch (error) {
        if (adbStatus) {
            adbStatus.textContent = 'Error';
            adbStatus.className = 'status-indicator error';
        }
        if (sdkStatus) {
            sdkStatus.textContent = 'Error';
            sdkStatus.className = 'status-indicator error';
        }
    }
    
    // 检查AAPT（如果有实现）
    try {
        const aaptResult = await ipcRenderer.invoke('check-aapt-status');
        if (aaptResult && aaptResult.success) {
            if (aaptStatus) {
                aaptStatus.textContent = `Available`;
                aaptStatus.className = 'status-indicator success';
            }
        } else {
            if (aaptStatus) {
                aaptStatus.textContent = 'Not Available';
                aaptStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (aaptStatus) {
            aaptStatus.textContent = 'Not Implemented';
            aaptStatus.className = 'status-indicator warning';
        }
    }
    
    // 检查TKE
    try {
        const tkeResult = await ipcRenderer.invoke('check-tke-status');
        if (tkeResult && tkeResult.success) {
            if (tkeStatus) {
                tkeStatus.textContent = `Available (v${tkeResult.version || 'Unknown'})`;
                tkeStatus.className = 'status-indicator success';
            }
        } else {
            if (tkeStatus) {
                tkeStatus.textContent = tkeResult?.error || 'Not Available';
                tkeStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (tkeStatus) {
            tkeStatus.textContent = 'Not Available';
            tkeStatus.className = 'status-indicator error';
        }
    }
}

// 导出函数
window.SettingsModule = {
    initializeSettingsPage,
    loadUserInfo,
    checkAllToolsStatus,
    loadEditorFontSettings,
    applyEditorFontSettings
};