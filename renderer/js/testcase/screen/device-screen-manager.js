// 设备屏幕管理器
// 负责设备屏幕截图刷新、XML overlay 显示和 UI 元素管理

// ============================================
// 全局状态管理
// ============================================
const ScreenState = {
    // XML Overlay 状态
    xmlOverlayEnabled: false,
    currentUIElements: [],
    currentScreenSize: null,
    selectedElement: null,
    
    // 观察器
    resizeObserver: null,
    
    // 设置状态并同步到全局
    setXmlOverlayEnabled(value) {
        this.xmlOverlayEnabled = value;
        window.xmlOverlayEnabled = value; // 向后兼容
        window.rLog(`📊 XML Overlay 状态更新: ${value}`);
    },
    
    reset() {
        this.xmlOverlayEnabled = false;
        this.currentUIElements = [];
        this.currentScreenSize = null;
        this.selectedElement = null;
        window.xmlOverlayEnabled = false;
    }
};

// ============================================
// 工具函数
// ============================================
function getGlobals() {
    return window.AppGlobals;
}

// ============================================
// 屏幕截图功能
// ============================================
async function refreshDeviceScreen() {
    const { ipcRenderer, path } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals.currentProject;
    
    if (!deviceSelect?.value) {
        window.NotificationModule.showNotification('请先选择设备', 'warning');
        return;
    }
    
    if (!projectPath) {
        window.NotificationModule.showNotification('请先打开项目', 'error');
        return;
    }

    window.rLog('开始截图，设备:', deviceSelect.value, '项目路径:', projectPath);
    const result = await ipcRenderer.invoke('adb-screenshot', deviceSelect.value, projectPath);
    
    window.rLog('截图结果:', { success: result.success, hasImagePath: !!result.imagePath });
    
    if (result.success && result.imagePath) {
        const img = document.getElementById('deviceScreenshot');
        if (!img) {
            window.rError('未找到 deviceScreenshot 元素');
            return;
        }
        
        img.src = `file://${result.imagePath}?t=${Date.now()}`;
        img.style.display = 'block';
        
        const placeholder = document.querySelector('.screen-placeholder');
        if (placeholder) {
            placeholder.style.display = 'none';
        }
        
        window.rLog('截图显示成功');
        
        // 更新设备信息并获取UI结构
        await updateDeviceInfoAndGetUIStructure();
    } else {
        const error = result.error || '未知错误';
        window.rError('截图失败:', error);
        window.NotificationModule.showNotification(`截图失败: ${error}`, 'error');
        
        // 显示错误信息在屏幕占位符上
        const placeholder = document.querySelector('.screen-placeholder');
        if (placeholder) {
            placeholder.textContent = `截图失败: ${error}`;
            placeholder.style.display = 'block';
        }
    }
}

// ============================================
// UI 结构获取和解析
// ============================================
async function updateDeviceInfoAndGetUIStructure() {
    const { ipcRenderer, path } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals.currentProject;
    
    if (!deviceSelect?.value || !projectPath) return;
    
    window.rLog(`🔄 获取设备UI结构, 当前 overlay 状态: ${ScreenState.xmlOverlayEnabled}`);
    
    try {
        // 获取设备XML结构
        const result = await ipcRenderer.invoke('adb-get-ui-xml', {
            deviceId: deviceSelect.value,
            projectPath: projectPath,
            options: {
                useCompressedLayout: true,
                timeout: 30000,
                retryCount: 2
            },
            metadata: {
                screenSize: null,
                timestamp: Date.now(),
                deviceModel: null
            }
        });
        
        if (!result.success) {
            window.rError('获取设备UI结构失败:', result.error);
            if (result.error && result.error.includes('timeout')) {
                window.NotificationModule.showNotification('获取UI结构超时，请检查设备连接', 'error');
            } else {
                window.NotificationModule.showNotification('获取UI结构失败: ' + (result.error || '未知错误'), 'error');
            }
            return;
        }
        
        window.rLog('屏幕尺寸:', result.screenSize);
        
        // 显示图片尺寸信息
        const deviceImage = document.getElementById('deviceScreenshot');
        if (deviceImage && deviceImage.complete) {
            const rect = deviceImage.getBoundingClientRect();
            window.rLog('图片显示信息:', {
                left: rect.left,
                top: rect.top,
                width: rect.width,
                height: rect.height
            });
        }
        
        // 通过 TKE 提取 UI 元素
        if (result.xml) {
            window.rLog('开始通过 TKE 提取 UI 元素...');
            
            // 调用 TKE 提取元素
            const extractResult = await ipcRenderer.invoke('execute-tke-extract-elements', {
                deviceId: deviceSelect.value,
                projectPath: projectPath,
                screenWidth: result.screenSize?.width || 1080,
                screenHeight: result.screenSize?.height || 1920
            });
            
            if (extractResult.success && extractResult.elements) {
                window.rLog(`TKE 提取到 ${extractResult.elements.length} 个UI元素`);
                const elements = extractResult.elements;
                
                // 显示UI元素列表
                displayUIElementList(elements);
                
                // 如果XML overlay 已启用，更新 overlay
                if (ScreenState.xmlOverlayEnabled) {
                    window.rLog('📊 XML overlay 已启用，更新覆盖层');
                    await updateXmlOverlay(elements, result.screenSize);
                }
                
                // 存储当前屏幕尺寸
                window.AppGlobals.currentScreenSize = result.screenSize;
                
                // 如果有TKE适配器，更新屏幕信息
                if (window.TkeAdapterModule && window.TkeAdapterModule.updateScreenInfo) {
                    window.TkeAdapterModule.updateScreenInfo(result.screenSize);
                }
            } else {
                window.rError('TKE 提取UI元素失败:', extractResult.error);
            }
        }
        
    } catch (error) {
        window.rError('Error updating device info:', error);
        window.NotificationModule.showNotification('更新设备信息失败: ' + error.message, 'error');
    }
}

// ============================================
// XML Overlay 管理
// ============================================

// 切换 XML overlay 状态
async function toggleXmlOverlay() {
    window.rLog('🔘 toggleXmlOverlay 被调用');
    const deviceSelect = document.getElementById('deviceSelect');
    
    if (!deviceSelect?.value) {
        window.NotificationModule.showNotification('请先选择设备', 'warning');
        return;
    }
    
    // 切换状态
    const newState = !ScreenState.xmlOverlayEnabled;
    
    if (newState) {
        // 先尝试启用，成功后再设置状态
        await enableXmlOverlay(deviceSelect.value);
        // enableXmlOverlay 内部会设置状态
    } else {
        // 禁用时直接设置状态
        ScreenState.setXmlOverlayEnabled(false);
        disableXmlOverlay();
    }
}

// 启用 XML overlay
async function enableXmlOverlay(deviceId) {
    window.rLog(`🎯 启用 XML Overlay, deviceId = ${deviceId}`);
    
    try {
        window.NotificationModule.showNotification('正在准备截图和UI树...', 'info');
        
        const { ipcRenderer } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;
        
        // 1. 先确保有截图
        const deviceImage = document.getElementById('deviceScreenshot');
        if (!deviceImage || !deviceImage.complete || deviceImage.naturalWidth === 0) {
            window.rLog('设备截图未加载，先刷新屏幕');
            await refreshDeviceScreen();
            
            // 等待图片加载完成
            await new Promise((resolve, reject) => {
                const img = document.getElementById('deviceScreenshot');
                if (img.complete) {
                    resolve();
                } else {
                    img.onload = resolve;
                    img.onerror = () => reject(new Error('截图加载失败'));
                }
            });
        }
        
        // 2. 通过 TKE 提取 UI 元素（从工作区的 current_ui_tree.xml）
        // 先用默认尺寸调用TKE
        const extractResult = await ipcRenderer.invoke('execute-tke-extract-elements', {
            deviceId: deviceId,
            projectPath: projectPath,
            screenWidth: 1080,  // 临时值
            screenHeight: 2400  // 临时值
        });
        
        if (!extractResult.success || !extractResult.elements) {
            throw new Error('TKE提取UI元素失败: ' + (extractResult.error || '未知错误'));
        }
        
        // 3. 从元素的bounds中推断实际屏幕尺寸
        let screenSize = { width: 1080, height: 1920 };  // 默认值
        
        // 查找根节点（通常第一个元素）来确定屏幕尺寸
        if (extractResult.elements.length > 0) {
            const rootElement = extractResult.elements[0];
            if (rootElement.bounds && rootElement.bounds.length === 4) {
                // bounds格式: [x1, y1, x2, y2]
                const inferredWidth = rootElement.bounds[2] - rootElement.bounds[0];
                const inferredHeight = rootElement.bounds[3] - rootElement.bounds[1];
                
                // 如果推断的尺寸合理（大于800x600），使用它
                if (inferredWidth >= 800 && inferredHeight >= 600) {
                    screenSize = { width: inferredWidth, height: inferredHeight };
                    window.rLog(`从XML根节点推断屏幕尺寸: ${screenSize.width}x${screenSize.height}`);
                }
            }
        }
        
        // 如果有保存的屏幕尺寸且合理，也可以使用
        if (window.AppGlobals.currentScreenSize && 
            window.AppGlobals.currentScreenSize.width > 0 && 
            window.AppGlobals.currentScreenSize.height > 0) {
            // 比较两个尺寸，如果差异太大，使用XML推断的
            const savedSize = window.AppGlobals.currentScreenSize;
            const widthDiff = Math.abs(savedSize.width - screenSize.width);
            const heightDiff = Math.abs(savedSize.height - screenSize.height);
            
            if (widthDiff < 100 && heightDiff < 100) {
                // 差异不大，使用保存的尺寸（可能更准确）
                screenSize = savedSize;
                window.rLog(`使用保存的屏幕尺寸: ${screenSize.width}x${screenSize.height}`);
            }
        }
        
        // 存储元素和屏幕尺寸
        ScreenState.currentUIElements = extractResult.elements;
        ScreenState.currentScreenSize = screenSize;
        
        // 4. 创建 overlay
        await createUIOverlay(ScreenState.currentUIElements, ScreenState.currentScreenSize);
        
        // 5. 显示元素列表
        displayUIElementList(ScreenState.currentUIElements);
        
        // 6. 更新按钮状态
        const toggleBtn = document.getElementById('toggleXmlBtn');
        if (toggleBtn) {
            toggleBtn.style.background = '#4CAF50';
            toggleBtn.setAttribute('title', '关闭XML Overlay');
        }
        
        // 成功后设置状态为 true
        ScreenState.setXmlOverlayEnabled(true);
        
        window.NotificationModule.showNotification(
            `XML Overlay已启用，识别到${ScreenState.currentUIElements.length}个元素`, 
            'success'
        );
        
        window.rLog(`✅ XML Overlay 启用成功! 元素数量 = ${ScreenState.currentUIElements.length}`);
        
    } catch (error) {
        const errorMsg = error?.message || error?.toString() || JSON.stringify(error) || '未知错误';
        window.rError('❌ 启用XML Overlay失败:', errorMsg, error);
        window.NotificationModule.showNotification(`启用XML Overlay失败: ${errorMsg}`, 'error');
        ScreenState.setXmlOverlayEnabled(false);
    }
}

// 禁用 XML overlay
function disableXmlOverlay() {
    window.rLog('📊 禁用 XML Overlay');
    
    // 移除UI叠层
    const screenContent = document.getElementById('screenContent');
    if (screenContent) {
        const overlay = screenContent.querySelector('.ui-overlay');
        if (overlay) {
            overlay.remove();
        }
    }
    
    // 清空UI元素列表
    displayUIElementList([]);
    
    // 重置状态
    ScreenState.currentUIElements = [];
    ScreenState.selectedElement = null;
    ScreenState.setXmlOverlayEnabled(false);
    
    // 更新按钮状态
    const toggleBtn = document.getElementById('toggleXmlBtn');
    if (toggleBtn) {
        toggleBtn.style.background = '';
        toggleBtn.setAttribute('title', '启用XML Overlay');
    }
    
    // 断开 ResizeObserver
    if (ScreenState.resizeObserver) {
        ScreenState.resizeObserver.disconnect();
        ScreenState.resizeObserver = null;
    }
    
    window.NotificationModule.showNotification('XML Overlay已关闭', 'info');
}

// 更新 XML overlay（当屏幕刷新时）
async function updateXmlOverlay(elements, screenSize) {
    window.rLog('🔄 更新 XML Overlay');
    
    // 更新状态
    ScreenState.currentUIElements = elements;
    ScreenState.currentScreenSize = screenSize;
    
    // 重新创建 overlay
    await createUIOverlay(elements, screenSize);
}

// ============================================
// UI Overlay 渲染
// ============================================

// 创建UI覆盖层
async function createUIOverlay(elements, screenSize) {
    window.rLog(`创建UI覆盖层，元素数量: ${elements.length}`);
    
    const screenContent = document.getElementById('screenContent');
    const deviceImage = document.getElementById('deviceScreenshot');
    
    if (!screenContent || !deviceImage) {
        window.rError('未找到必要的DOM元素');
        return;
    }
    
    // 移除旧的覆盖层
    const existingOverlay = screenContent.querySelector('.ui-overlay');
    if (existingOverlay) {
        existingOverlay.remove();
    }
    
    // 创建新的覆盖层
    const overlay = document.createElement('div');
    overlay.className = 'ui-overlay';
    overlay.style.cssText = `
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        pointer-events: none;
        z-index: 10;
    `;
    
    // 确保容器是相对定位
    screenContent.style.position = 'relative';
    screenContent.appendChild(overlay);
    
    // 设置 ResizeObserver 来监听容器大小变化
    setupResizeObserver(screenContent, deviceImage);
    
    // 渲染元素框
    renderUIElements(overlay, elements, screenSize);
}

// 设置 ResizeObserver
function setupResizeObserver(screenContent, deviceImage) {
    // 如果已有观察器，先断开
    if (ScreenState.resizeObserver) {
        ScreenState.resizeObserver.disconnect();
    }
    
    // 创建新的观察器
    ScreenState.resizeObserver = new ResizeObserver((entries) => {
        window.rLog('🔄 ResizeObserver 触发！');
        
        // 输出每个元素的大小变化
        entries.forEach(entry => {
            const name = entry.target.id || entry.target.className;
            window.rLog(`📏 元素 ${name} 大小变化:`, {
                width: entry.contentRect.width,
                height: entry.contentRect.height
            });
        });
        
        // 检查条件并更新
        if (ScreenState.xmlOverlayEnabled && ScreenState.currentUIElements.length > 0) {
            window.rLog('✅ 条件满足，重新渲染 XML overlay');
            // 直接重新渲染所有元素以适应新尺寸
            const overlay = document.querySelector('.ui-overlay');
            if (overlay) {
                renderUIElements(overlay, ScreenState.currentUIElements, ScreenState.currentScreenSize);
            }
        } else {
            window.rLog(`❌ 条件不满足:`, {
                xmlOverlayEnabled: ScreenState.xmlOverlayEnabled,
                elementsCount: ScreenState.currentUIElements.length
            });
        }
    });
    
    // 开始观察
    ScreenState.resizeObserver.observe(screenContent);
    if (deviceImage) {
        ScreenState.resizeObserver.observe(deviceImage);
    }
    
    window.rLog('✅ ResizeObserver 已设置');
}

// 更新 overlay 位置（当容器大小变化时）
function updateOverlayPosition() {
    window.rLog('🎯 updateOverlayPosition 被调用');
    
    const screenContent = document.getElementById('screenContent');
    const overlay = screenContent?.querySelector('.ui-overlay');
    
    if (!overlay || !ScreenState.currentUIElements.length) {
        return;
    }
    
    // 重新渲染元素框
    renderUIElements(overlay, ScreenState.currentUIElements, ScreenState.currentScreenSize);
}

// 渲染UI元素框
function renderUIElements(overlay, elements, screenSize) {
    const deviceImage = document.getElementById('deviceScreenshot');
    if (!deviceImage) return;
    
    // 清空现有内容
    overlay.innerHTML = '';
    
    // 获取图片实际显示尺寸
    const imgRect = deviceImage.getBoundingClientRect();
    const scaleX = imgRect.width / screenSize.width;
    const scaleY = imgRect.height / screenSize.height;
    
    window.rLog(`渲染比例: scaleX=${scaleX}, scaleY=${scaleY}`);
    
    // 为每个元素创建框
    elements.forEach((element, index) => {
        if (!element.bounds || element.bounds.length !== 4) return;
        
        const [x1, y1, x2, y2] = element.bounds;
        
        // 创建元素框
        const elementBox = document.createElement('div');
        elementBox.className = 'ui-element-marker';  // 使用正确的CSS类名
        elementBox.dataset.index = index;
        elementBox.dataset.elementIndex = element.index;  // 兼容原有代码
        
        // 计算缩放后的位置和大小
        const left = x1 * scaleX;
        const top = y1 * scaleY;
        const width = (x2 - x1) * scaleX;
        const height = (y2 - y1) * scaleY;
        
        // 只设置位置和尺寸，样式交给CSS处理
        elementBox.style.cssText = `
            position: absolute;
            left: ${left}px;
            top: ${top}px;
            width: ${width}px;
            height: ${height}px;
        `;
        
        // CSS已经处理了悬停效果，不需要JS
        
        // 添加点击事件
        elementBox.addEventListener('click', () => {
            selectElement(index);
        });
        
        overlay.appendChild(elementBox);
    });
    
    window.rLog(`✅ 渲染了 ${elements.length} 个UI元素框`);
}

// ============================================
// UI 元素列表显示
// ============================================

function displayUIElementList(elements) {
    // 使用嵌入式UI面板
    const bottomPanel = document.getElementById('uiElementsBottomPanel');
    const elementsContainer = document.getElementById('elementsListContainer');
    
    if (!bottomPanel || !elementsContainer) {
        window.rError('嵌入式UI面板元素未找到');
        return;
    }
    
    window.rLog(`准备显示 ${elements.length} 个UI元素到UI库中`);
    
    // 确保底部面板可见
    bottomPanel.style.display = 'flex';
    bottomPanel.classList.remove('collapsed');
    bottomPanel.style.maxHeight = '300px';
    
    // 确保tab内容可见
    const tabContent = document.getElementById('uiElementsPanelContent');
    const elementsListPane = document.getElementById('elementsListPane');
    if (tabContent) tabContent.style.display = 'block';
    if (elementsListPane) {
        elementsListPane.style.display = 'block';
        elementsListPane.classList.add('active');
    }
    
    // 计算每个元素的尺寸和位置信息
    const elementsWithSize = elements.map(el => {
        const width = el.bounds ? (el.bounds[2] - el.bounds[0]) : (el.width || 0);
        const height = el.bounds ? (el.bounds[3] - el.bounds[1]) : (el.height || 0);
        const centerX = el.bounds ? Math.round((el.bounds[0] + el.bounds[2]) / 2) : (el.centerX || 0);
        const centerY = el.bounds ? Math.round((el.bounds[1] + el.bounds[3]) / 2) : (el.centerY || 0);
        
        return { ...el, width, height, centerX, centerY };
    });
    
    // 生成元素列表HTML
    if (elements.length > 0) {
        const elementsHTML = elementsWithSize.map(el => `
            <div class="element-item" data-index="${el.index}">
                <div class="element-main" onclick="selectElementByIndex(${el.index})">
                    <div class="element-header">
                        <span class="element-index">[${el.index}]</span>
                        <span class="element-type">${el.className ? el.className.split('.').pop() : 'Unknown'}</span>
                    </div>
                    ${el.text ? `<div class="element-text">文本: ${el.text}</div>` : ''}
                    ${el.contentDesc ? `<div class="element-desc">描述: ${el.contentDesc}</div>` : ''}
                    ${el.hint ? `<div class="element-hint">提示: ${el.hint}</div>` : ''}
                    <div class="element-size">${el.width}×${el.height} @ (${el.centerX},${el.centerY})</div>
                </div>
                <div class="element-actions">
                    <button class="btn-icon-small save-to-locator-btn" 
                            onclick="event.stopPropagation(); saveElementToLocatorFromList(${el.index})" 
                            title="入库"
                            style="background: transparent; border: none; padding: 4px;">
                        <svg viewBox="0 0 24 24" width="20" height="20">
                            <path fill="#FF9800" d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
                        </svg>
                    </button>
                </div>
            </div>
        `).join('');
        
        elementsContainer.innerHTML = elementsHTML;
        window.rLog(`✅ 已将 ${elements.length} 个UI元素显示在UI库中`);
        
        // 滚动到底部面板
        setTimeout(() => {
            bottomPanel.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        }, 100);
        
    } else {
        // 显示空状态
        const emptyStateHTML = !ScreenState.xmlOverlayEnabled 
            ? '<div class="empty-state"><div class="empty-state-text">XML Overlay未启用</div></div>'
            : '<div class="empty-state"><div class="empty-state-text">暂无UI元素</div></div>';
        
        elementsContainer.innerHTML = emptyStateHTML;
    }
}

// ============================================
// 元素选择功能
// ============================================

function selectElement(index) {
    window.rLog(`选择元素: ${index}`);
    
    // 更新选中状态
    ScreenState.selectedElement = index;
    
    // 高亮选中的元素框
    const overlay = document.querySelector('.ui-overlay');
    if (overlay) {
        const boxes = overlay.querySelectorAll('.ui-element-marker');
        boxes.forEach((box, i) => {
            if (i === index) {
                box.classList.add('selected');
            } else {
                box.classList.remove('selected');
            }
        });
    }
    
    // 高亮列表中的元素
    const listItems = document.querySelectorAll('.element-item');
    listItems.forEach((item, i) => {
        if (i === index) {
            item.classList.add('selected');
            item.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        } else {
            item.classList.remove('selected');
        }
    });
    
    // 显示元素详情
    if (ScreenState.currentUIElements[index]) {
        const element = ScreenState.currentUIElements[index];
        window.rLog('选中元素详情:', element);
    }
}

// 从列表选择元素
window.selectElementByIndex = function(index) {
    selectElement(index);
};

// 保存元素到定位器库
window.saveElementToLocatorFromList = function(index) {
    const element = ScreenState.currentUIElements[index];
    if (element) {
        window.rLog('保存元素到定位器库:', element);
        // TODO: 实现保存逻辑
        window.NotificationModule.showNotification('元素已保存到定位器库', 'success');
    }
};

// ============================================
// 导出模块
// ============================================

window.DeviceScreenManagerModule = {
    refreshDeviceScreen,
    updateDeviceInfoAndGetUIStructure,
    toggleXmlOverlay,
    enableXmlOverlay,
    disableXmlOverlay,
    displayUIElementList,
    selectElement,
    
    // 导出状态（只读）
    getState: () => ({
        xmlOverlayEnabled: ScreenState.xmlOverlayEnabled,
        currentUIElements: ScreenState.currentUIElements,
        currentScreenSize: ScreenState.currentScreenSize,
        selectedElement: ScreenState.selectedElement
    })
};