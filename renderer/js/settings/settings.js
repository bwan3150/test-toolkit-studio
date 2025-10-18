// 设置页面模块

// 初始化设置页面
function initializeSettingsPage() {
    const { ipcRenderer } = window.AppGlobals;
    const logoutBtn = document.getElementById('logoutBtn');
    const checkToolsBtn = document.getElementById('checkToolsBtn');
    const updateBaseUrlBtn = document.getElementById('updateBaseUrlBtn');
    const settingsBaseUrl = document.getElementById('settingsBaseUrl');
    const aboutVersion = document.getElementById('about-version');
    const exportLogsBtn = document.getElementById('exportLogsBtn');
    const exportLogStatus = document.getElementById('exportLogStatus');
    const betaUpdatesToggle = document.getElementById('betaUpdatesToggle');

    // 编辑器字体设置
    const editorFontFamily = document.getElementById('editorFontFamily');
    const editorFontSize = document.getElementById('editorFontSize');
    const applyFontSettings = document.getElementById('applyFontSettings');

    // 加载应用版本
    if (aboutVersion) {
        ipcRenderer.invoke('get-app-version').then(version => {
            aboutVersion.textContent = version;
        });
    }

    // 加载 Beta 更新设置
    if (betaUpdatesToggle) {
        ipcRenderer.invoke('store-get', 'receive_beta_updates').then(enabled => {
            betaUpdatesToggle.checked = enabled === true;
        });

        // 监听 toggle 变化
        betaUpdatesToggle.addEventListener('change', async () => {
            const enabled = betaUpdatesToggle.checked;
            await ipcRenderer.invoke('store-set', 'receive_beta_updates', enabled);
            await ipcRenderer.invoke('set-update-channel', enabled ? 'beta' : 'latest');
            window.rLog(`Beta updates ${enabled ? 'enabled' : 'disabled'}`);
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
    
    // 自动检查工具状态
    checkAllToolsStatus();
    
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
    
    // 上传日志按钮
    if (exportLogsBtn) {
        exportLogsBtn.addEventListener('click', async () => {
            try {
                // 禁用按钮防止重复点击
                exportLogsBtn.disabled = true;
                
                // 显示正在上传状态
                if (exportLogStatus) {
                    exportLogStatus.textContent = 'Uploading...';
                    exportLogStatus.style.display = 'inline';
                }
                
                // 调用日志上传功能
                const result = await window.RendererLogger.uploadLogs();
                
                if (result.success) {
                    if (exportLogStatus) {
                        exportLogStatus.textContent = 'Succeed';
                        exportLogStatus.style.color = '#4ec9b0';
                    }
                    window.NotificationModule.showNotification('Log Upload Successfully', 'success');
                } else {
                    if (exportLogStatus) {
                        exportLogStatus.textContent = result.message || 'Failed';
                        exportLogStatus.style.color = '#f48771';
                    }
                    window.NotificationModule.showNotification(result.message || 'Failed', 'error');
                }
                
                // 3秒后隐藏状态
                setTimeout(() => {
                    if (exportLogStatus) {
                        exportLogStatus.style.display = 'none';
                    }
                }, 3000);
                
            } catch (error) {
                window.rError('上传日志失败:', error);
                window.NotificationModule.showNotification(`Failed: ${error.message}`, 'error');
                if (exportLogStatus) {
                    exportLogStatus.textContent = 'Failed';
                    exportLogStatus.style.color = '#f48771';
                    exportLogStatus.style.display = 'inline';
                }
            } finally {
                // 重新启用按钮
                exportLogsBtn.disabled = false;
            }
        });
    }
    
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
    const tkeVersionStatus = document.getElementById('tkeVersionStatus');
    const tkeAdbVersionStatus = document.getElementById('tkeAdbVersionStatus');
    const opencvVersionStatus = document.getElementById('opencvVersionStatus');
    const testerAiVersionStatus = document.getElementById('testerAiVersionStatus');

    // 设置所有状态为检查中
    if (tkeVersionStatus) {
        tkeVersionStatus.textContent = 'Checking...';
        tkeVersionStatus.className = 'status-indicator checking';
    }
    if (tkeAdbVersionStatus) {
        tkeAdbVersionStatus.textContent = 'Checking...';
        tkeAdbVersionStatus.className = 'status-indicator checking';
    }
    if (opencvVersionStatus) {
        opencvVersionStatus.textContent = 'Checking...';
        opencvVersionStatus.className = 'status-indicator checking';
    }
    if (testerAiVersionStatus) {
        testerAiVersionStatus.textContent = 'Checking...';
        testerAiVersionStatus.className = 'status-indicator checking';
    }

    // 检查 TKE 引擎版本
    try {
        const tkeResult = await ipcRenderer.invoke('get-tke-version');
        if (tkeVersionStatus) {
            if (tkeResult.success) {
                tkeVersionStatus.textContent = `${tkeResult.version}`;
                tkeVersionStatus.className = 'status-indicator success';
            } else {
                tkeVersionStatus.textContent = tkeResult.error || 'Not Available';
                tkeVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (tkeVersionStatus) {
            tkeVersionStatus.textContent = `Error: ${error.message}`;
            tkeVersionStatus.className = 'status-indicator error';
        }
    }

    // 检查 TKE 内嵌 ADB 版本
    try {
        const tkeAdbResult = await ipcRenderer.invoke('get-tke-adb-version');
        if (tkeAdbVersionStatus) {
            if (tkeAdbResult.success) {
                tkeAdbVersionStatus.textContent = `${tkeAdbResult.version}`;
                tkeAdbVersionStatus.className = 'status-indicator success';
            } else {
                tkeAdbVersionStatus.textContent = tkeAdbResult.error || 'Not Available';
                tkeAdbVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (tkeAdbVersionStatus) {
            tkeAdbVersionStatus.textContent = `Error: ${error.message}`;
            tkeAdbVersionStatus.className = 'status-indicator error';
        }
    }

    // 检查 AI 测试模块版本（较快，先检查）
    try {
        const testerAiResult = await ipcRenderer.invoke('get-tester-ai-version');
        if (testerAiVersionStatus) {
            if (testerAiResult.success) {
                testerAiVersionStatus.textContent = `${testerAiResult.version}`;
                testerAiVersionStatus.className = 'status-indicator success';
            } else {
                testerAiVersionStatus.textContent = testerAiResult.error || 'Not Available';
                testerAiVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (testerAiVersionStatus) {
            testerAiVersionStatus.textContent = `Error: ${error.message}`;
            testerAiVersionStatus.className = 'status-indicator error';
        }
    }

    // 检查本地视觉模块（TKE-OpenCV）版本（较慢，放最后）
    try {
        const opencvResult = await ipcRenderer.invoke('get-tke-opencv-version');
        if (opencvVersionStatus) {
            if (opencvResult.success) {
                opencvVersionStatus.textContent = `${opencvResult.version}`;
                opencvVersionStatus.className = 'status-indicator success';
            } else {
                opencvVersionStatus.textContent = opencvResult.error || 'Not Available';
                opencvVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (opencvVersionStatus) {
            opencvVersionStatus.textContent = `Error: ${error.message}`;
            opencvVersionStatus.className = 'status-indicator error';
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
