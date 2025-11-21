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
            // 清除 Sentry 用户上下文
            if (window.clearSentryUser) {
                window.clearSentryUser();
            }

            await ipcRenderer.invoke('logout');
            await ipcRenderer.invoke('navigate-to-login');
        });
    }
    
    if (checkToolsBtn) {
        checkToolsBtn.addEventListener('click', checkAllToolsStatus);
    }

    // 不在初始化时检查工具状态，等待页面激活时由PageStateManager触发
    
    if (updateBaseUrlBtn) {
        updateBaseUrlBtn.addEventListener('click', async () => {
            await ipcRenderer.invoke('store-set', 'base_url', settingsBaseUrl.value);
            window.AppNotifications?.success('Base URL updated');
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
                exportLogsBtn.classList.add('btn-loading');
                exportLogsBtn.innerHTML = `
                    <svg class="btn-icon btn-spinner" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                        <path fill="currentColor" d="M12,4V2A10,10 0 0,0 2,12H4A8,8 0 0,1 12,4Z"/>
                    </svg>
                    上传中
                `;

                // 调用日志上传功能
                const result = await window.RendererLogger.uploadLogs();

                if (result.success) {
                    window.rLog('✅ 日志上传成功');

                    // 显示成功状态
                    exportLogsBtn.classList.remove('btn-loading');
                    exportLogsBtn.classList.add('btn-success');
                    exportLogsBtn.innerHTML = `
                        <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                            <path fill="currentColor" d="M9,20.42L2.79,14.21L5.62,11.38L9,14.77L18.88,4.88L21.71,7.71L9,20.42Z"/>
                        </svg>
                        已上传
                    `;

                    // 2秒后恢复按钮
                    setTimeout(() => {
                        exportLogsBtn.classList.remove('btn-success');
                        exportLogsBtn.disabled = false;
                        exportLogsBtn.innerHTML = `
                            <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                                <path fill="currentColor" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20M12,11L8,15H10.5V18H13.5V15H16L12,11Z"/>
                            </svg>
                            上传
                        `;
                    }, 2000);
                } else {
                    window.rError('日志上传失败:', result.message);

                    // 显示失败状态
                    exportLogsBtn.classList.remove('btn-loading');
                    exportLogsBtn.classList.add('btn-error');
                    exportLogsBtn.innerHTML = `
                        <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                            <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                        </svg>
                        失败
                    `;

                    // 2秒后恢复按钮
                    setTimeout(() => {
                        exportLogsBtn.classList.remove('btn-error');
                        exportLogsBtn.disabled = false;
                        exportLogsBtn.innerHTML = `
                            <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                                <path fill="currentColor" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20M12,11L8,15H10.5V18H13.5V15H16L12,11Z"/>
                            </svg>
                            上传
                        `;
                    }, 2000);
                }

            } catch (error) {
                window.rError('上传日志失败:', error);

                // 显示错误状态
                exportLogsBtn.classList.remove('btn-loading');
                exportLogsBtn.classList.add('btn-error');
                exportLogsBtn.innerHTML = `
                    <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                        <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                    </svg>
                    失败
                `;

                // 2秒后恢复按钮
                setTimeout(() => {
                    exportLogsBtn.classList.remove('btn-error');
                    exportLogsBtn.disabled = false;
                    exportLogsBtn.innerHTML = `
                        <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
                            <path fill="currentColor" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20M12,11L8,15H10.5V18H13.5V15H16L12,11Z"/>
                        </svg>
                        上传
                    `;
                }, 2000);
            }
        });
    }
    
    // 应用字体设置
    if (applyFontSettings) {
        applyFontSettings.addEventListener('click', async () => {
            const fontFamily = editorFontFamily.value;
            const fontSize = parseInt(editorFontSize.value);
            
            if (fontSize < 10 || fontSize > 24) {
                window.AppNotifications?.error('字体大小必须在10-24px之间');
                return;
            }
            
            // 保存设置到本地存储
            await ipcRenderer.invoke('store-set', 'editor_font_family', fontFamily);
            await ipcRenderer.invoke('store-set', 'editor_font_size', fontSize);
            
            // 应用到当前编辑器
            applyEditorFontSettings(fontFamily, fontSize);
            
            window.AppNotifications?.success('字体设置已应用');
        });
    }
    
    // 加载已保存的字体设置
    loadEditorFontSettings();

    // 初始化缓存管理模块
    if (window.CacheManager) {
        window.CacheManager.init();
    }

    // PageStateManager调试工具
    const viewPageCacheBtn = document.getElementById('viewPageCacheBtn');
    const clearPageCacheBtn = document.getElementById('clearPageCacheBtn');
    const pageCacheModal = document.getElementById('pageCacheModal');
    const closePageCacheModal = document.getElementById('closePageCacheModal');
    const pageCacheContent = document.getElementById('pageCacheContent');
    const pagesCachedCount = document.getElementById('pagesCachedCount');

    // 更新缓存计数
    function updatePageCacheCount() {
        if (window.PageStateManager && pagesCachedCount) {
            const cacheInfo = window.PageStateManager.getCacheInfo();
            const cachedPages = Object.values(cacheInfo).filter(page => page.state !== 'inactive').length;
            pagesCachedCount.textContent = cachedPages;
        }
    }

    // 查看缓存详情
    if (viewPageCacheBtn) {
        viewPageCacheBtn.addEventListener('click', () => {
            if (!window.PageStateManager) {
                window.AppNotifications?.error('PageStateManager未加载');
                return;
            }

            const cacheInfo = window.PageStateManager.getCacheInfo();
            const formattedInfo = JSON.stringify(cacheInfo, null, 2);

            if (pageCacheContent && pageCacheModal) {
                pageCacheContent.textContent = formattedInfo;
                pageCacheModal.style.display = 'flex';
            }
        });
    }

    // 清空页面缓存
    if (clearPageCacheBtn) {
        clearPageCacheBtn.addEventListener('click', () => {
            if (!window.PageStateManager) {
                window.AppNotifications?.error('PageStateManager未加载');
                return;
            }

            window.PageStateManager.clearPageCache();
            window.AppNotifications?.success('页面缓存已清空');
            updatePageCacheCount();
        });
    }

    // 关闭模态框
    if (closePageCacheModal) {
        closePageCacheModal.addEventListener('click', () => {
            if (pageCacheModal) {
                pageCacheModal.style.display = 'none';
            }
        });
    }

    // 点击模态框外部关闭
    if (pageCacheModal) {
        pageCacheModal.addEventListener('click', (e) => {
            if (e.target === pageCacheModal) {
                pageCacheModal.style.display = 'none';
            }
        });
    }

    // 初始化计数
    updatePageCacheCount();

    // 导出updatePageCacheCount函数，供PageStateManager钩子使用
    window.updatePageCacheCount = updatePageCacheCount;
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
    const tkeAaptVersionStatus = document.getElementById('tkeAaptVersionStatus');
    const opencvVersionStatus = document.getElementById('opencvVersionStatus');
    const testerAiVersionStatus = document.getElementById('testerAiVersionStatus');
    const ffmpegVersionStatus = document.getElementById('ffmpegVersionStatus');
    const scrcpyVersionStatus = document.getElementById('scrcpyVersionStatus');

    // 设置所有状态为检查中
    if (tkeVersionStatus) {
        tkeVersionStatus.textContent = 'Checking...';
        tkeVersionStatus.className = 'status-indicator checking';
    }
    if (tkeAdbVersionStatus) {
        tkeAdbVersionStatus.textContent = 'Checking...';
        tkeAdbVersionStatus.className = 'status-indicator checking';
    }
    if (tkeAaptVersionStatus) {
        tkeAaptVersionStatus.textContent = 'Checking...';
        tkeAaptVersionStatus.className = 'status-indicator checking';
    }
    if (opencvVersionStatus) {
        opencvVersionStatus.textContent = 'Checking...';
        opencvVersionStatus.className = 'status-indicator checking';
    }
    if (testerAiVersionStatus) {
        testerAiVersionStatus.textContent = 'Checking...';
        testerAiVersionStatus.className = 'status-indicator checking';
    }
    if (ffmpegVersionStatus) {
        ffmpegVersionStatus.textContent = 'Checking...';
        ffmpegVersionStatus.className = 'status-indicator checking';
    }
    if (scrcpyVersionStatus) {
        scrcpyVersionStatus.textContent = 'Checking...';
        scrcpyVersionStatus.className = 'status-indicator checking';
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

    // 检查 TKE 内嵌 AAPT 版本
    try {
        const tkeAaptResult = await ipcRenderer.invoke('get-tke-aapt-version');
        if (tkeAaptVersionStatus) {
            if (tkeAaptResult.success) {
                tkeAaptVersionStatus.textContent = `${tkeAaptResult.version}`;
                tkeAaptVersionStatus.className = 'status-indicator success';
            } else {
                tkeAaptVersionStatus.textContent = tkeAaptResult.error || 'Not Available';
                tkeAaptVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (tkeAaptVersionStatus) {
            tkeAaptVersionStatus.textContent = `Error: ${error.message}`;
            tkeAaptVersionStatus.className = 'status-indicator error';
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

    // 检查 FFmpeg 版本
    try {
        const ffmpegResult = await ipcRenderer.invoke('get-ffmpeg-version');
        if (ffmpegVersionStatus) {
            if (ffmpegResult.success) {
                ffmpegVersionStatus.textContent = `${ffmpegResult.version}`;
                ffmpegVersionStatus.className = 'status-indicator success';
            } else {
                ffmpegVersionStatus.textContent = ffmpegResult.error || 'Not Available';
                ffmpegVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (ffmpegVersionStatus) {
            ffmpegVersionStatus.textContent = `Error: ${error.message}`;
            ffmpegVersionStatus.className = 'status-indicator error';
        }
    }

    // 检查 TKE-Scrcpy 版本
    try {
        const scrcpyResult = await ipcRenderer.invoke('get-tke-scrcpy-version');
        if (scrcpyVersionStatus) {
            if (scrcpyResult.success) {
                scrcpyVersionStatus.textContent = `${scrcpyResult.version}`;
                scrcpyVersionStatus.className = 'status-indicator success';
            } else {
                scrcpyVersionStatus.textContent = scrcpyResult.error || 'Not Available';
                scrcpyVersionStatus.className = 'status-indicator error';
            }
        }
    } catch (error) {
        if (scrcpyVersionStatus) {
            scrcpyVersionStatus.textContent = `Error: ${error.message}`;
            scrcpyVersionStatus.className = 'status-indicator error';
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

// 注册Settings页面到PageStateManager
// 页面激活时检查工具状态和缓存统计
if (window.PageStateManager) {
    window.PageStateManager.registerPage('settings', {
        onActivate: async () => {
            window.rLog('🔄 Settings页面激活，检查工具状态...');

            // 检查所有工具状态
            await checkAllToolsStatus();

            // 更新媒体缓存统计
            if (window.CacheManager && window.CacheManager.updateCacheStats) {
                await window.CacheManager.updateCacheStats();
            }

            // 更新页面缓存计数
            if (window.updatePageCacheCount) {
                window.updatePageCacheCount();
            }

            window.rLog('✅ Settings页面数据加载完成');
        },
        onRefresh: async () => {
            window.rLog('🔄 Settings页面刷新...');

            // 重新检查工具状态
            await checkAllToolsStatus();

            // 更新媒体缓存统计
            if (window.CacheManager && window.CacheManager.updateCacheStats) {
                await window.CacheManager.updateCacheStats();
            }

            // 更新页面缓存计数
            if (window.updatePageCacheCount) {
                window.updatePageCacheCount();
            }

            window.rLog('✅ Settings页面刷新完成');
        },
        ttl: 300000 // 300秒（5分钟）缓存（工具版本不会频繁变化）
    });
}
