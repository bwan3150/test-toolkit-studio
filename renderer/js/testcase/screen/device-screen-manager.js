// 设备屏幕管理器
// 负责设备屏幕截图刷新和显示

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 设备屏幕管理
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
        
        // 屏幕截图显示成功
        
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

// 更新设备信息并获取UI结构
async function updateDeviceInfoAndGetUIStructure() {
    const { ipcRenderer, path } = getGlobals();
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals.currentProject;
    
    if (!deviceSelect?.value || !projectPath) return;
    
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
                screenSize: null, // 将从获取的XML中推断
                timestamp: Date.now(),
                deviceModel: null // 可以后续获取
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
        
        // 初始化XML解析器（如果存在）
        if (window.XmlParserModule && result.xml) {
            const xmlParser = window.XmlParserModule.getParser();
            if (xmlParser) {
                // 解析XML内容
                await xmlParser.parseXmlString(result.xml);
                
                // 设置屏幕尺寸
                let screenSize = result.screenSize;
                if (!screenSize && result.xml) {
                    // 从XML推断屏幕尺寸
                    screenSize = await xmlParser.inferScreenSizeFromXML(result.xml);
                }
                
                if (screenSize) {
                    xmlParser.setScreenSize(screenSize.width, screenSize.height);
                    result.screenSize = screenSize; // 确保后续使用
                } else {
                    // 使用默认屏幕尺寸
                    screenSize = { width: 1080, height: 1920 };
                    xmlParser.setScreenSize(screenSize.width, screenSize.height);
                    result.screenSize = screenSize;
                }
                
                // 获取UI元素
                const elements = await xmlParser.getAllElements();
                
                // 显示UI元素列表
                if (window.TestcaseController && window.TestcaseController.displayUIElementList) {
                    window.TestcaseController.displayUIElementList(elements);
                }
                
                // 如果XML覆盖层已启用，刷新覆盖层
                if (window.xmlOverlayEnabled) {
                    if (window.TestcaseController && window.TestcaseController.enableXmlOverlay) {
                        window.TestcaseController.enableXmlOverlay(deviceSelect.value);
                    }
                }
                
                // 存储当前屏幕尺寸
                window.AppGlobals.currentScreenSize = screenSize;
                
                // 如果有TKE适配器，更新屏幕信息
                if (window.TkeAdapterModule && window.TkeAdapterModule.updateScreenInfo) {
                    window.TkeAdapterModule.updateScreenInfo(screenSize);
                }
            }
        }
        
    } catch (error) {
        window.rError('Error updating device info:', error);
        window.NotificationModule.showNotification('更新设备信息失败: ' + error.message, 'error');
    }
}

// XML Overlay 状态管理
let xmlOverlayEnabled = false;
let currentUIElements = [];
let currentScreenSize = null;
let xmlParser = null;
let selectedElement = null;

// XML覆盖层开关
function toggleXmlOverlay() {
    const deviceSelect = document.getElementById('deviceSelect');
    
    if (!deviceSelect?.value) {
        window.NotificationModule.showNotification('请先选择设备', 'warning');
        return;
    }
    
    xmlOverlayEnabled = !xmlOverlayEnabled;
    
    if (xmlOverlayEnabled) {
        enableXmlOverlay(deviceSelect.value);
    } else {
        disableXmlOverlay();
    }
}

// 启用XML覆盖层
async function enableXmlOverlay(deviceId) {
    try {
        window.NotificationModule.showNotification('正在加载UI树结构...', 'info');
        
        let result;
        const { ipcRenderer } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;
        
        // 1. 优先尝试从工作区读取UI树
        if (projectPath) {
            try {
                const { fs, path } = getGlobals();
                const xmlPath = path.join(projectPath, 'workarea', 'current_ui_tree.xml');
                const xmlContent = await fs.readFile(xmlPath, 'utf8');
                
                result = {
                    success: true,
                    xml: xmlContent,
                    screenSize: null,
                    source: 'workarea'
                };
                
                window.rLog('从工作区读取UI树成功');
            } catch (workareaError) {
                window.rLog('工作区UI树不存在或读取失败，将重新获取:', workareaError.message);
                result = null;
            }
        }
        
        // 2. 如果工作区没有，则重新获取
        if (!result) {
            result = await ipcRenderer.invoke('adb-ui-dump-enhanced', deviceId);
            if (!result.success) {
                throw new Error(result.error);
            }
            result.source = 'adb';
        }
        
        window.rLog('=== UI数据获取成功 ===');
        window.rLog('数据来源:', result.source);
        window.rLog('XML长度:', result.xml ? result.xml.length : 0);
        window.rLog('屏幕尺寸:', result.screenSize);
        
        // 2. 初始化XML解析器
        if (!xmlParser) {
            xmlParser = window.XMLParserTKEModule.createParser();
        }
        
        // 设置屏幕尺寸
        let screenSize = result.screenSize;
        if (!screenSize && result.xml) {
            screenSize = await xmlParser.inferScreenSizeFromXML(result.xml);
        }
        
        if (screenSize) {
            xmlParser.setScreenSize(screenSize.width, screenSize.height);
            result.screenSize = screenSize;
        } else {
            screenSize = { width: 1080, height: 1920 };
            xmlParser.setScreenSize(screenSize.width, screenSize.height);
            result.screenSize = screenSize;
        }
        
        // 3. 解析XML并提取UI元素
        let optimizedTree = await xmlParser.optimizeUITree(result.xml);
        if (!optimizedTree) {
            window.rLog('UI树优化失败，尝试使用原始XML');
            try {
                const parser = new DOMParser();
                const doc = parser.parseFromString(result.xml, 'text/xml');
                optimizedTree = doc.documentElement;
                if (!optimizedTree || optimizedTree.nodeName === 'parsererror') {
                    throw new Error('XML格式不正确');
                }
            } catch (parseError) {
                throw new Error(`XML解析完全失败: ${parseError.message}`);
            }
        }
        
        currentUIElements = await xmlParser.extractUIElements(optimizedTree, result.xml);
        window.rLog('提取的UI元素:', currentUIElements);
        
        // 暴露到全局，供TKS集成模块使用
        window.currentUIElements = currentUIElements;
        
        if (currentUIElements.length === 0) {
            window.rLog('未提取到任何UI元素，可能需要检查XML格式或优化算法');
        }
        
        // 存储当前屏幕尺寸
        currentScreenSize = result.screenSize;
        
        // 4. 在屏幕截图上创建可交互叠层
        await createUIOverlay(currentUIElements, result.screenSize);
        
        // 5. 显示元素列表
        displayUIElementList(currentUIElements);
        
        // 更新按钮状态
        const toggleBtn = document.getElementById('toggleXmlBtn');
        if (toggleBtn) {
            toggleBtn.style.background = '#4CAF50';
            toggleBtn.setAttribute('title', '关闭XML Overlay');
        }
        
        window.NotificationModule.showNotification(
            `XML Overlay已启用，识别到${currentUIElements.length}个元素`, 
            'success'
        );
        
        // 在控制台输出日志
        if (window.TestcaseController && window.TestcaseController.ConsoleManager) {
            window.TestcaseController.ConsoleManager.addLog(
                `XML Overlay已启用，成功识别到${currentUIElements.length}个UI元素`, 
                'success'
            );
        }
        
    } catch (error) {
        window.rError('启用XML Overlay失败:', error);
        window.NotificationModule.showNotification(`启用XML Overlay失败: ${error.message}`, 'error');
        xmlOverlayEnabled = false;
    }
}

// 禁用XML覆盖层
function disableXmlOverlay() {
    // 移除UI叠层
    const screenContent = document.getElementById('screenContent');
    if (screenContent) {
        const overlay = screenContent.querySelector('.ui-overlay');
        if (overlay) overlay.remove();
    }
    
    // 清空UI元素列表
    displayUIElementList([]);
    
    // 重置状态
    currentUIElements = [];
    selectedElement = null;
    
    // 更新按钮状态
    const toggleBtn = document.getElementById('toggleXmlBtn');
    if (toggleBtn) {
        toggleBtn.style.background = '';
        toggleBtn.setAttribute('title', '显示XML Overlay');
    }
    
    // 在控制台输出日志
    if (window.TestcaseController && window.TestcaseController.ConsoleManager) {
        window.TestcaseController.ConsoleManager.addLog('XML Overlay已关闭', 'info');
    }
    
    window.NotificationModule.showNotification('XML Overlay已关闭', 'info');
}

// 创建UI覆盖层
async function createUIOverlay(elements, screenSize) {
    const screenContent = document.getElementById('screenContent');
    const deviceImage = document.getElementById('deviceScreenshot');
    
    if (!deviceImage || !screenContent) {
        throw new Error('找不到设备截图容器');
    }
    
    // 确保图片已加载
    if (!deviceImage.complete || deviceImage.naturalHeight === 0) {
        await refreshDeviceScreen();
        await new Promise(resolve => {
            if (deviceImage.complete) {
                resolve();
            } else {
                deviceImage.onload = resolve;
                setTimeout(resolve, 3000); // 3秒超时
            }
        });
    }
    
    // 移除旧的叠层
    const oldOverlay = screenContent.querySelector('.ui-overlay');
    if (oldOverlay) oldOverlay.remove();
    
    // 计算图片在容器中的实际显示区域
    const imageDisplayInfo = calculateImageDisplayArea(deviceImage, screenContent);
    if (!imageDisplayInfo) {
        window.rError('无法计算图片显示区域');
        return;
    }
    
    window.rLog('图片显示信息:', imageDisplayInfo);
    window.rLog('设备屏幕尺寸:', screenSize);
    
    // 创建新的叠层容器
    const overlay = document.createElement('div');
    overlay.className = 'ui-overlay';
    overlay.style.cssText = `
        position: absolute;
        left: ${imageDisplayInfo.left}px;
        top: ${imageDisplayInfo.top}px;
        width: ${imageDisplayInfo.width}px;
        height: ${imageDisplayInfo.height}px;
        pointer-events: none;
        z-index: 10;
    `;
    
    // 为每个UI元素创建可视化标记
    elements.forEach((element, index) => {
        const marker = createElementMarker(element, screenSize, imageDisplayInfo);
        overlay.appendChild(marker);
    });
    
    // 设置容器为相对定位
    screenContent.style.position = 'relative';
    screenContent.appendChild(overlay);
}

// 计算图片在容器中的实际显示区域
function calculateImageDisplayArea(image, container) {
    if (!image.complete || !image.naturalWidth || !image.naturalHeight) {
        return null;
    }
    
    const containerStyle = getComputedStyle(container);
    const containerPadding = {
        left: parseFloat(containerStyle.paddingLeft) || 0,
        top: parseFloat(containerStyle.paddingTop) || 0,
        right: parseFloat(containerStyle.paddingRight) || 0,
        bottom: parseFloat(containerStyle.paddingBottom) || 0
    };
    
    // 容器可用空间
    const availableWidth = container.clientWidth - containerPadding.left - containerPadding.right;
    const availableHeight = container.clientHeight - containerPadding.top - containerPadding.bottom;
    
    // 图片原始尺寸
    const imageNaturalRatio = image.naturalWidth / image.naturalHeight;
    const availableRatio = availableWidth / availableHeight;
    
    let displayWidth, displayHeight;
    
    // 模拟 object-fit: contain 的行为
    if (imageNaturalRatio > availableRatio) {
        displayWidth = availableWidth;
        displayHeight = availableWidth / imageNaturalRatio;
    } else {
        displayHeight = availableHeight;
        displayWidth = availableHeight * imageNaturalRatio;
    }
    
    // 计算图片在容器中的位置（居中显示）
    const left = containerPadding.left + (availableWidth - displayWidth) / 2;
    const top = containerPadding.top + (availableHeight - displayHeight) / 2;
    
    return {
        left,
        top,
        width: displayWidth,
        height: displayHeight,
        scaleX: displayWidth / image.naturalWidth,
        scaleY: displayHeight / image.naturalHeight
    };
}

// 创建元素标记
function createElementMarker(element, screenSize, imageDisplayInfo) {
    const marker = document.createElement('div');
    marker.className = 'ui-element-marker';
    marker.dataset.elementIndex = element.index;
    
    // 使用设备屏幕尺寸和图片显示比例计算位置
    const scaleX = imageDisplayInfo.width / screenSize.width;
    const scaleY = imageDisplayInfo.height / screenSize.height;
    
    // 计算在overlay中的相对位置
    const left = element.bounds[0] * scaleX;
    const top = element.bounds[1] * scaleY;
    const width = element.width * scaleX;
    const height = element.height * scaleY;
    
    marker.style.cssText = `
        position: absolute;
        left: ${left}px;
        top: ${top}px;
        width: ${width}px;
        height: ${height}px;
    `;
    
    // 添加元素索引标签
    const label = document.createElement('div');
    label.className = 'element-label';
    label.textContent = element.index;
    marker.appendChild(label);
    
    // 添加交互事件
    marker.addEventListener('mouseenter', (e) => showElementTooltip(element, marker, e));
    marker.addEventListener('mouseleave', hideElementTooltip);
    marker.addEventListener('click', (e) => {
        e.stopPropagation();
        selectUIElement(element);
    });
    
    return marker;
}

// 显示元素提示框
function showElementTooltip(element, marker, event) {
    const oldTooltip = document.querySelector('.element-tooltip');
    if (oldTooltip) oldTooltip.remove();
    
    const tooltip = document.createElement('div');
    tooltip.className = 'element-tooltip';
    tooltip.innerHTML = `
        <div class="tooltip-header">[${element.index}] ${element.className.split('.').pop()}</div>
        <div class="tooltip-content">
            ${element.text ? `<div><strong>文本:</strong> ${element.text}</div>` : ''}
            ${element.contentDesc ? `<div><strong>描述:</strong> ${element.contentDesc}</div>` : ''}
            ${element.hint ? `<div><strong>提示:</strong> ${element.hint}</div>` : ''}
            ${element.resourceId ? `<div><strong>ID:</strong> ${element.resourceId.split('/').pop()}</div>` : ''}
            <div><strong>位置:</strong> (${element.centerX}, ${element.centerY})</div>
            <div><strong>尺寸:</strong> ${element.width}x${element.height}</div>
        </div>
    `;
    
    tooltip.style.cssText = `
        position: fixed;
        background: rgba(0, 0, 0, 0.9);
        color: white;
        padding: 10px;
        border-radius: 5px;
        font-size: 12px;
        max-width: 300px;
        z-index: 1000;
        pointer-events: none;
        box-shadow: 0 2px 10px rgba(0,0,0,0.5);
    `;
    
    document.body.appendChild(tooltip);
    
    // 定位提示框
    const rect = marker.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    
    let left = rect.right + 10;
    let top = rect.top;
    
    // 防止超出屏幕
    if (left + tooltipRect.width > window.innerWidth) {
        left = rect.left - tooltipRect.width - 10;
    }
    if (top + tooltipRect.height > window.innerHeight) {
        top = window.innerHeight - tooltipRect.height - 10;
    }
    
    tooltip.style.left = Math.max(10, left) + 'px';
    tooltip.style.top = Math.max(10, top) + 'px';
}

// 隐藏元素提示框
function hideElementTooltip() {
    const tooltip = document.querySelector('.element-tooltip');
    if (tooltip) tooltip.remove();
}

// 显示UI元素列表
function displayUIElementList(elements) {
    // 使用嵌入式UI面板
    const bottomPanel = document.getElementById('uiElementsBottomPanel');
    const elementsContainer = document.getElementById('elementsListContainer');
    
    if (!bottomPanel || !elementsContainer) {
        window.rError('嵌入式UI面板元素未找到', bottomPanel, elementsContainer);
        return;
    }
    
    window.rLog(`准备显示 ${elements.length} 个UI元素到UI库中`);
    
    // 确保底部面板可见且未折叠
    bottomPanel.style.display = 'flex';
    bottomPanel.classList.remove('collapsed');
    bottomPanel.style.maxHeight = '300px';
    window.rLog('显示底部UI元素面板，移除折叠状态');
    
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
        // 计算宽度和高度
        const width = el.bounds ? (el.bounds[2] - el.bounds[0]) : (el.width || 0);
        const height = el.bounds ? (el.bounds[3] - el.bounds[1]) : (el.height || 0);
        // 计算中心点
        const centerX = el.bounds ? Math.round((el.bounds[0] + el.bounds[2]) / 2) : (el.centerX || 0);
        const centerY = el.bounds ? Math.round((el.bounds[1] + el.bounds[3]) / 2) : (el.centerY || 0);
        
        return { ...el, width, height, centerX, centerY };
    });
    
    window.rLog('处理后的元素信息（前3个）:', elementsWithSize.slice(0, 3));
    
    // 生成原始样式的元素列表HTML
    const elementsHTML = elementsWithSize.map(el => `
        <div class="element-item" data-index="${el.index}">
            <div class="element-main" onclick="selectElementByIndex(${el.index})">
                <div class="element-header">
                    <span class="element-index">[${el.index}]</span>
                    <span class="element-type">${el.className.split('.').pop()}</span>
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
    
    // 更新容器内容
    if (elements.length > 0) {
        window.rLog(`准备插入HTML到容器中，HTML长度: ${elementsHTML.length}`);
        elementsContainer.innerHTML = elementsHTML;
        window.rLog(`✅ 已将 ${elements.length} 个UI元素显示在UI库中`);
        window.rLog(`容器innerHTML更新后长度: ${elementsContainer.innerHTML.length}`);
        window.rLog(`容器子元素数量: ${elementsContainer.children.length}`);
        
        // 检查容器是否可见
        const containerRect = elementsContainer.getBoundingClientRect();
        window.rLog(`容器位置和尺寸:`, {
            width: containerRect.width,
            height: containerRect.height,
            top: containerRect.top,
            left: containerRect.left,
            visible: containerRect.width > 0 && containerRect.height > 0
        });
        
        // 滚动到底部面板
        setTimeout(() => {
            bottomPanel.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            window.rLog('已滚动到底部面板');
        }, 100);
        
    } else {
        // 显示空状态信息
        let emptyStateHTML;
        if (!xmlOverlayEnabled) {
            emptyStateHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">XML Overlay未启用</div>
                </div>
            `;
        } else {
            emptyStateHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">暂无UI元素</div>
                </div>
            `;
        }
        
        elementsContainer.innerHTML = emptyStateHTML;
        window.rLog('显示UI元素空状态');
    }
}

// 选择UI元素
function selectUIElement(element) {
    window.rLog('选择UI元素:', element);
    
    // 取消之前的选择
    const previousSelected = document.querySelector('.ui-element-marker.selected');
    if (previousSelected) {
        previousSelected.classList.remove('selected');
        previousSelected.style.borderColor = '#00ff00';
        previousSelected.style.background = 'rgba(0, 255, 0, 0.1)';
    }
    
    // 高亮当前选择的元素
    const currentMarker = document.querySelector(`[data-element-index="${element.index}"]`);
    if (currentMarker) {
        currentMarker.classList.add('selected');
        currentMarker.style.borderColor = '#ff0000';
        currentMarker.style.background = 'rgba(255, 0, 0, 0.2)';
        currentMarker.style.borderWidth = '3px';
    }
    
    selectedElement = element;
}

// 通过索引选择元素
function selectElementByIndex(index) {
    const element = currentUIElements.find(el => el.index === index);
    if (element) {
        selectUIElement(element);
    }
}

// 全局函数，供HTML调用
window.selectElementByIndex = selectElementByIndex;

// 从列表直接入库元素
window.saveElementToLocatorFromList = async function(index) {
    const element = currentUIElements.find(el => el.index === index);
    if (element) {
        if (window.LocatorManagerTKEModule) {
            try {
                // 先确保Locator管理器已初始化
                let locatorManager = window.LocatorManagerTKEModule.getInstance();
                if (!locatorManager) {
                    // 如果还未初始化，先初始化
                    locatorManager = await window.LocatorManagerTKEModule.initializeLocatorManagerTKE();
                }
                await locatorManager.saveElement(element);
            } catch (error) {
                window.rError('保存元素到Locator库失败:', error);
                window.NotificationModule.showNotification('保存失败: ' + error.message, 'error');
            }
        } else {
            window.rError('LocatorManagerTKEModule未加载');
            window.NotificationModule.showNotification('Locator库未加载', 'error');
        }
    } else {
        window.rError('未找到指定索引的元素:', index);
        window.NotificationModule.showNotification('元素未找到', 'error');
    }
};

// 插入元素引用到编辑器
window.insertElementReference = function(index, action) {
    const element = currentUIElements.find(el => el.index === index);
    if (!element) return;
    
    let scriptText = '';
    switch (action) {
        case 'click':
            scriptText = `点击 [{${element.elementName || element.text || '元素'}}]`;
            break;
        case 'input':
            scriptText = `输入 [{${element.elementName || element.text || '元素'}}, 文本内容]`;
            break;
        case 'assert':
            scriptText = `断言 [{${element.elementName || element.text || '元素'}}, 存在]`;
            break;
    }
    
    // 获取当前活动的编辑器并插入文本
    const activeTab = document.querySelector('.tab.active');
    if (activeTab && window.EditorManager) {
        const tabId = activeTab.id;
        const editor = window.EditorManager.getEditor(tabId);
        if (editor) {
            editor.insertText(scriptText + '\n');
            window.NotificationModule.showNotification(`已插入: ${scriptText}`, 'success');
        } else {
            // 如果没有编辑器，复制到剪贴板
            navigator.clipboard.writeText(scriptText).then(() => {
                window.NotificationModule.showNotification(`已复制到剪贴板: ${scriptText}`, 'success');
            });
        }
    } else {
        // 复制到剪贴板作为备用
        navigator.clipboard.writeText(scriptText).then(() => {
            window.NotificationModule.showNotification(`已复制到剪贴板: ${scriptText}`, 'success');
        });
    }
};

// 导出模块
window.DeviceScreenManagerModule = {
    refreshDeviceScreen,
    updateDeviceInfoAndGetUIStructure,
    toggleXmlOverlay,
    enableXmlOverlay,
    disableXmlOverlay,
    displayUIElementList
};