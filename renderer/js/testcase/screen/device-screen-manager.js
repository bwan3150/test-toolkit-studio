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

    // 如果XML overlay已启用，先移除overlay UI（但保持状态）
    const wasXmlOverlayEnabled = ScreenState.xmlOverlayEnabled;
    if (wasXmlOverlayEnabled) {
        window.rLog('🔄 刷新前先移除XML overlay UI');
        const screenContent = document.getElementById('screenContent');
        const existingOverlay = screenContent?.querySelector('.ui-overlay');
        if (existingOverlay) {
            existingOverlay.remove();
        }
    }

    window.rLog('开始截图，设备:', deviceSelect.value, '项目路径:', projectPath);

    // 使用新的 TKE controller capture 命令
    const captureResult = await ipcRenderer.invoke('tke-controller-capture', deviceSelect.value, projectPath);

    if (!captureResult.success) {
        const error = captureResult.error || '未知错误';
        window.rError('截图失败:', error);
        window.NotificationModule.showNotification(`截图失败: ${error}`, 'error');

        // 隐藏截图，显示默认占位符
        const img = document.getElementById('deviceScreenshot');
        if (img) {
            img.style.display = 'none';
        }
        const placeholder = document.querySelector('.screen-placeholder');
        if (placeholder) {
            placeholder.textContent = 'No device connected';
            placeholder.style.display = 'block';
        }
        return;
    }

    // 解析 TKE 返回的 JSON
    let result;
    try {
        result = JSON.parse(captureResult.output);
    } catch (e) {
        window.rError('解析TKE输出失败:', e);
        window.NotificationModule.showNotification('解析截图结果失败', 'error');
        return;
    }

    window.rLog('截图结果:', { success: result.success, hasScreenshot: !!result.screenshot });

    if (result.success && result.screenshot) {
        const imagePath = result.screenshot;
        const img = document.getElementById('deviceScreenshot');
        if (!img) {
            window.rError('未找到 deviceScreenshot 元素');
            return;
        }

        // 等待图片加载完成
        await new Promise((resolve) => {
            img.onload = () => {
                img.style.display = 'block';
                const placeholder = document.querySelector('.screen-placeholder');
                if (placeholder) {
                    placeholder.style.display = 'none';
                }
                window.rLog('截图显示成功');

                // 给浏览器一点时间完成布局
                setTimeout(resolve, 50);
            };
            img.src = `file://${imagePath}?t=${Date.now()}`;
        });

        // 更新设备信息并获取UI结构（传入xml路径）
        await updateDeviceInfoAndGetUIStructure(result.xml);
    }
}

// ============================================
// UI 结构获取和解析
// ============================================
async function updateDeviceInfoAndGetUIStructure(xmlPath) {
    const { ipcRenderer, path } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals.currentProject;

    if (!deviceSelect?.value || !projectPath) return;

    window.rLog(`🔄 获取设备UI结构, 当前 overlay 状态: ${ScreenState.xmlOverlayEnabled}`);

    try {
        // 1. 首先使用 TKE fetcher infer-screen-size 推断屏幕尺寸
        const sizeResult = await ipcRenderer.invoke('tke-fetcher-infer-screen-size', projectPath);

        let screenSize = { width: 1080, height: 1920 }; // 默认值
        if (sizeResult.success) {
            try {
                const sizeData = JSON.parse(sizeResult.output);
                screenSize = {
                    width: sizeData.width,
                    height: sizeData.height
                };
                window.rLog('屏幕尺寸:', screenSize);
            } catch (e) {
                window.rWarn('解析屏幕尺寸失败，使用默认值:', e);
            }
        }

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

        // 2. 使用 TKE fetcher extract-ui-elements 提取 UI 元素
        window.rLog('开始通过 TKE 提取 UI 元素...');

        const extractResult = await ipcRenderer.invoke('tke-fetcher-extract-ui-elements', {
            projectPath: projectPath,
            screenWidth: screenSize.width,
            screenHeight: screenSize.height
        });

        if (!extractResult.success) {
            window.rError('TKE 提取UI元素失败:', extractResult.error);
            window.NotificationModule.showNotification('提取UI元素失败: ' + (extractResult.error || '未知错误'), 'error');
            return;
        }

        // 解析 TKE 返回的 JSON
        let elements;
        try {
            elements = JSON.parse(extractResult.output);
        } catch (e) {
            window.rError('解析TKE输出失败:', e);
            window.NotificationModule.showNotification('解析UI元素失败', 'error');
            return;
        }

        if (Array.isArray(elements)) {
            window.rLog(`TKE 提取到 ${elements.length} 个UI元素`);

            // 显示UI元素列表
            displayUIElementList(elements);

            // 如果XML overlay 已启用，重新创建overlay UI
            if (ScreenState.xmlOverlayEnabled) {
                window.rLog('📊 重新创建XML overlay UI');

                // 更新状态
                ScreenState.currentUIElements = elements;
                ScreenState.currentScreenSize = screenSize;

                // 创建新的overlay
                await createUIOverlay(elements, screenSize);
            }

            // 存储当前屏幕尺寸
            window.AppGlobals.currentScreenSize = screenSize;

            // 如果有TKE适配器，更新屏幕信息
            if (window.TkeAdapterModule && window.TkeAdapterModule.updateScreenInfo) {
                window.TkeAdapterModule.updateScreenInfo(screenSize);
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
        
        // 2. 使用 TKE fetcher infer-screen-size 推断屏幕尺寸
        const sizeResult = await ipcRenderer.invoke('tke-fetcher-infer-screen-size', projectPath);

        let screenSize = { width: 1080, height: 1920 }; // 默认值
        if (sizeResult.success) {
            try {
                const sizeData = JSON.parse(sizeResult.output);
                screenSize = {
                    width: sizeData.width,
                    height: sizeData.height
                };
                window.rLog('推断屏幕尺寸:', screenSize);
            } catch (e) {
                window.rWarn('解析屏幕尺寸失败，使用默认值:', e);
            }
        }

        // 如果有保存的屏幕尺寸且合理，也可以使用
        if (window.AppGlobals.currentScreenSize &&
            window.AppGlobals.currentScreenSize.width > 0 &&
            window.AppGlobals.currentScreenSize.height > 0) {
            // 比较两个尺寸，如果差异太大，使用推断的
            const savedSize = window.AppGlobals.currentScreenSize;
            const widthDiff = Math.abs(savedSize.width - screenSize.width);
            const heightDiff = Math.abs(savedSize.height - screenSize.height);

            if (widthDiff < 100 && heightDiff < 100) {
                // 差异不大，使用保存的尺寸（可能更准确）
                screenSize = savedSize;
                window.rLog(`使用保存的屏幕尺寸: ${screenSize.width}x${screenSize.height}`);
            }
        }

        // 3. 使用 TKE fetcher extract-ui-elements 提取 UI 元素
        const extractResult = await ipcRenderer.invoke('tke-fetcher-extract-ui-elements', {
            projectPath: projectPath,
            screenWidth: screenSize.width,
            screenHeight: screenSize.height
        });

        if (!extractResult.success) {
            throw new Error('TKE提取UI元素失败: ' + (extractResult.error || '未知错误'));
        }

        // 解析 TKE 返回的 JSON
        let elements;
        try {
            elements = JSON.parse(extractResult.output);
        } catch (e) {
            throw new Error('解析TKE输出失败: ' + e.message);
        }

        if (!Array.isArray(elements)) {
            throw new Error('TKE返回的数据格式不正确');
        }

        // 存储元素和屏幕尺寸
        ScreenState.currentUIElements = elements;
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
    
    // 先移除旧的overlay
    const screenContent = document.getElementById('screenContent');
    const existingOverlay = screenContent?.querySelector('.ui-overlay');
    if (existingOverlay) {
        existingOverlay.remove();
    }
    
    // 等待一帧，确保DOM更新完成
    await new Promise(resolve => requestAnimationFrame(resolve));
    
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
        window.rLog('移除旧的覆盖层');
        existingOverlay.remove();
    }

    // 获取图片的实际显示位置和大小
    const imgRect = deviceImage.getBoundingClientRect();
    const containerRect = screenContent.getBoundingClientRect();

    // 计算图片相对于容器的偏移
    const offsetLeft = imgRect.left - containerRect.left;
    const offsetTop = imgRect.top - containerRect.top;

    window.rLog(`容器位置: left=${containerRect.left}, top=${containerRect.top}, width=${containerRect.width}, height=${containerRect.height}`);
    window.rLog(`图片位置: left=${imgRect.left}, top=${imgRect.top}, width=${imgRect.width}, height=${imgRect.height}`);
    window.rLog(`偏移量: offsetLeft=${offsetLeft}, offsetTop=${offsetTop}`);

    // 创建新的覆盖层，直接覆盖在图片上
    const overlay = document.createElement('div');
    overlay.className = 'ui-overlay';
    overlay.style.cssText = `
        position: absolute;
        top: ${offsetTop}px;
        left: ${offsetLeft}px;
        width: ${imgRect.width}px;
        height: ${imgRect.height}px;
        pointer-events: auto;
        z-index: 1000;
    `;

    // 添加CSS样式到head (如果还没有)
    if (!document.getElementById('ui-overlay-styles')) {
        const style = document.createElement('style');
        style.id = 'ui-overlay-styles';
        style.textContent = `
            .ui-element-box {
                pointer-events: none;
            }
            .ui-element-box.active {
                pointer-events: auto;
                outline: 2px solid #2196F3 !important;
                outline-offset: -2px !important;
                background: rgba(33, 150, 243, 0.15) !important;
                z-index: 100000 !important;
            }
        `;
        document.head.appendChild(style);
    }

    // 添加鼠标移动监听器来动态启用最顶层元素的pointer-events
    overlay.addEventListener('mousemove', (e) => {
        // 临时禁用所有元素的pointer-events来找到真正最顶层的元素
        const allBoxes = overlay.querySelectorAll('.ui-element-box');

        // 先移除所有active状态
        allBoxes.forEach(box => box.classList.remove('active'));

        // 按z-index从大到小遍历，找到第一个包含鼠标位置的元素
        const sortedBoxes = Array.from(allBoxes).sort((a, b) => {
            const zA = parseInt(a.dataset.baseZIndex) || 0;
            const zB = parseInt(b.dataset.baseZIndex) || 0;
            return zB - zA; // 从大到小
        });

        const rect = overlay.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        // 找到鼠标下方z-index最大的元素
        for (const box of sortedBoxes) {
            const boxRect = box.getBoundingClientRect();
            const relLeft = boxRect.left - rect.left;
            const relTop = boxRect.top - rect.top;
            const relRight = relLeft + boxRect.width;
            const relBottom = relTop + boxRect.height;

            if (mouseX >= relLeft && mouseX <= relRight &&
                mouseY >= relTop && mouseY <= relBottom) {
                box.classList.add('active');
                break; // 只激活最顶层的一个元素
            }
        }
    });

    // 鼠标离开overlay时移除所有active状态
    overlay.addEventListener('mouseleave', () => {
        const allBoxes = overlay.querySelectorAll('.ui-element-box');
        allBoxes.forEach(box => box.classList.remove('active'));
    });

    // 确保容器是相对定位
    screenContent.style.position = 'relative';
    screenContent.appendChild(overlay);

    window.rLog(`✅ Overlay 已添加到 DOM，className: ${overlay.className}`);
    window.rLog(`Overlay 样式: ${overlay.style.cssText}`);

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
            window.rLog('✅ 条件满足，更新 overlay 位置和元素');
            // 调用updateOverlayPosition来更新overlay位置和重新渲染元素
            updateOverlayPosition();
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
    const deviceImage = document.getElementById('deviceScreenshot');
    const overlay = screenContent?.querySelector('.ui-overlay');
    
    if (!overlay || !deviceImage || !ScreenState.currentUIElements.length) {
        return;
    }
    
    // 重新计算图片位置并更新overlay位置
    const imgRect = deviceImage.getBoundingClientRect();
    const containerRect = screenContent.getBoundingClientRect();
    
    // 计算图片相对于容器的偏移
    const offsetLeft = imgRect.left - containerRect.left;
    const offsetTop = imgRect.top - containerRect.top;
    
    // 更新overlay的位置和大小
    overlay.style.left = `${offsetLeft}px`;
    overlay.style.top = `${offsetTop}px`;
    overlay.style.width = `${imgRect.width}px`;
    overlay.style.height = `${imgRect.height}px`;
    
    // 重新渲染元素框
    renderUIElements(overlay, ScreenState.currentUIElements, ScreenState.currentScreenSize);
}

// 渲染UI元素框
function renderUIElements(overlay, elements, screenSize) {
    const deviceImage = document.getElementById('deviceScreenshot');
    if (!deviceImage) {
        window.rError('renderUIElements: deviceImage 未找到');
        return;
    }

    // 清空现有内容
    overlay.innerHTML = '';

    // 获取图片实际显示尺寸
    const imgRect = deviceImage.getBoundingClientRect();
    const scaleX = imgRect.width / screenSize.width;
    const scaleY = imgRect.height / screenSize.height;

    window.rLog(`渲染比例: scaleX=${scaleX}, scaleY=${scaleY}`);
    window.rLog(`图片尺寸: ${imgRect.width}x${imgRect.height}, 屏幕尺寸: ${screenSize.width}x${screenSize.height}`);

    // 按z_index排序元素,从小到大渲染(大元素先渲染,小元素后渲染)
    // 这样小元素在DOM中靠后,会自然覆盖在大元素上方
    const sortedElements = elements
        .map((el, idx) => ({ element: el, originalIndex: idx }))
        .sort((a, b) => {
            const zIndexA = a.element.z_index || (100 + a.originalIndex);
            const zIndexB = b.element.z_index || (100 + b.originalIndex);
            return zIndexA - zIndexB; // 从小到大排序
        });

    // 为每个元素创建框 (按z_index从小到大的顺序渲染)
    sortedElements.forEach(({ element, originalIndex }) => {
        const index = originalIndex;
        // TKE 返回的 bounds 是对象格式: {x1, y1, x2, y2}
        if (!element.bounds || typeof element.bounds !== 'object') return;

        const { x1, y1, x2, y2 } = element.bounds;

        // 创建元素框
        const elementBox = document.createElement('div');
        elementBox.className = 'ui-element-marker';  // 使用正确的CSS类名
        elementBox.dataset.index = index; // 保存索引
        elementBox.dataset.elementIndex = element.index;  // 兼容原有代码

        // 计算缩放后的位置和大小
        const left = x1 * scaleX;
        const top = y1 * scaleY;
        const width = (x2 - x1) * scaleX;
        const height = (y2 - y1) * scaleY;

        // 使用TKE计算好的z_index,如果没有则使用默认值
        const baseZIndex = element.z_index || (100 + index);

        // 计算元素面积用于调试
        const area = (x2 - x1) * (y2 - y1);

        // 设置元素框样式 - 使用outline而不是border,避免占据空间干扰鼠标事件
        // pointer-events默认为none,只有在mousemove时动态激活最顶层元素
        elementBox.style.cssText = `
            position: absolute;
            left: ${left}px;
            top: ${top}px;
            width: ${width}px;
            height: ${height}px;
            outline: 1px solid rgba(33, 150, 243, 0.6);
            outline-offset: -1px;
            background: transparent;
            box-sizing: border-box;
            z-index: ${baseZIndex};
            transition: all 0.2s ease;
        `;

        // 存储面积信息
        elementBox.dataset.area = area;

        // 存储原始 z-index 以便 hover 时使用
        elementBox.dataset.baseZIndex = baseZIndex;

        // 记录前5个元素的z-index信息用于调试
        if (index < 5) {
            window.rLog(`元素${index}: bounds=[${x1},${y1}][${x2},${y2}], area=${area}, z-index=${baseZIndex}`);
        }

        // 添加元素索引标签 (一直显示)
        const label = document.createElement('div');
        label.className = 'element-label';
        label.textContent = element.index !== undefined ? element.index : index;  // 显示元素的index
        label.style.cssText = `
            position: absolute;
            top: -20px;
            left: 0;
            background: #2196F3;
            color: white;
            padding: 2px 6px;
            font-size: 12px;
            font-weight: bold;
            border-radius: 3px;
            z-index: 101;
            pointer-events: none;
        `;
        elementBox.appendChild(label);

        // 添加hover类用于CSS控制
        elementBox.classList.add('ui-element-box');

        // 添加点击事件
        elementBox.addEventListener('click', (e) => {
            e.stopPropagation(); // 阻止事件冒泡
            window.rLog(`点击了元素 ${index}`);
            selectElement(index);
        });

        overlay.appendChild(elementBox);
    });

    window.rLog(`✅ 渲染了 ${elements.length} 个UI元素框`);
    window.rLog(`Overlay子元素数量: ${overlay.children.length}`);
}

// ============================================
// UI 元素列表显示
// ============================================

function displayUIElementList(elements) {
    // 触发自定义事件，通知UI元素列表面板更新
    const event = new CustomEvent('uiElementsUpdated', {
        detail: { elements }
    });
    document.dispatchEvent(event);
    
    window.rLog(`触发UI元素更新事件，元素数量: ${elements.length}`);
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
    
    // 获取选中的元素
    const element = ScreenState.currentUIElements[index];
    if (element) {
        // 触发自定义事件，通知其他模块
        const event = new CustomEvent('elementSelected', {
            detail: { element, index }
        });
        document.dispatchEvent(event);
        
        window.rLog('选中元素详情:', element);
    }
}

// 从列表选择元素（保留给全局使用）
window.selectElementByIndex = function(index) {
    selectElement(index);
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