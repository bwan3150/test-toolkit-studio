// 设备屏幕模式管理器
// 支持四种模式：普通屏幕、XML overlay、截图取坐标、坐标点取值

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 计算图片显示区域信息
function calculateImageDisplayArea(img, container) {
    const imgRect = img.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    
    return {
        left: imgRect.left - containerRect.left,
        top: imgRect.top - containerRect.top,
        width: imgRect.width,
        height: imgRect.height
    };
}

const ScreenModeManager = {
    initialized: false,
    currentMode: 'normal', // 'normal', 'xml', 'screenshot', 'coordinate'
    screenshotSelector: null,
    isSelecting: false,
    startX: 0,
    startY: 0,
    endX: 0,
    endY: 0,
    
    // 统一的坐标转换系统
    getImageDisplayInfo() {
        const deviceImage = document.getElementById('deviceScreenshot');
        const screenContent = document.getElementById('screenContent');
        if (!deviceImage || !screenContent) return null;
        
        return calculateImageDisplayArea(deviceImage, screenContent);
    },
    
    // 将屏幕坐标转换为图片内坐标
    screenToImageCoords(screenX, screenY) {
        const imageInfo = this.getImageDisplayInfo();
        if (!imageInfo) return { x: screenX, y: screenY };
        
        // 转换为图片内相对坐标
        const imageX = screenX - imageInfo.left;
        const imageY = screenY - imageInfo.top;
        
        // 确保坐标在图片范围内
        const clampedX = Math.max(0, Math.min(imageX, imageInfo.width));
        const clampedY = Math.max(0, Math.min(imageY, imageInfo.height));
        
        return { x: clampedX, y: clampedY };
    },
    
    // 将图片内坐标转换为实际设备坐标
    imageToDeviceCoords(imageX, imageY) {
        const deviceImage = document.getElementById('deviceScreenshot');
        const imageInfo = this.getImageDisplayInfo();
        if (!deviceImage || !imageInfo) return { x: imageX, y: imageY };
        
        // 计算缩放比例
        const scaleX = deviceImage.naturalWidth / imageInfo.width;
        const scaleY = deviceImage.naturalHeight / imageInfo.height;
        
        return {
            x: Math.round(imageX * scaleX),
            y: Math.round(imageY * scaleY)
        };
    },
    
    // 检查点是否在图片区域内
    isPointInImage(screenX, screenY) {
        const imageCoords = this.screenToImageCoords(screenX, screenY);
        const imageInfo = this.getImageDisplayInfo();
        if (!imageInfo) return false;
        
        return imageCoords.x >= 0 && imageCoords.x <= imageInfo.width &&
               imageCoords.y >= 0 && imageCoords.y <= imageInfo.height;
    },
    
    // 初始化模式管理器
    init() {
        // 防止重复初始化
        if (this.initialized) {
            window.rLog('ScreenModeManager 已经初始化过了，跳过重复初始化');
            return;
        }
        
        this.setupModeButtons();
        this.setupScreenshotMode();
        this.setupCoordinateMode();
        
        this.initialized = true;
        window.rLog('ScreenModeManager 初始化完成');
    },
    
    // 设置模式切换滑块
    setupModeButtons() {
        const modeOptions = document.querySelectorAll('.mode-option');
        const modeSlider = document.getElementById('modeSlider');
        
        modeOptions.forEach(option => {
            option.addEventListener('click', () => {
                // 检查是否被禁用（测试运行期间）
                if (option.classList.contains('disabled')) {
                    return;
                }
                
                const mode = option.dataset.mode;
                // 处理不同的模式名称映射
                let actualMode = mode;
                if (mode === 'crop') {
                    actualMode = 'screenshot';
                }
                
                this.switchMode(actualMode);
            });
        });
        
        // 初始化滑块位置
        this.updateSliderPosition('normal');
    },
    
    // 切换模式
    switchMode(mode) {
        this.currentMode = mode;
        const screenContent = document.getElementById('screenContent');
        const screenshotSelector = document.getElementById('screenshotSelector');
        const coordinateMarker = document.getElementById('coordinateMarker');
        
        // 重置所有模式选项状态
        document.querySelectorAll('.mode-option').forEach(option => option.classList.remove('active'));
        
        // 清理之前的模式状态
        screenContent.classList.remove('screenshot-mode', 'coordinate-mode');
        if (screenshotSelector) screenshotSelector.style.display = 'none';
        if (coordinateMarker) coordinateMarker.style.display = 'none';
        
        // 先禁用任何活动的 XML overlay
        const existingOverlay = document.querySelector('.ui-overlay');
        if (existingOverlay) {
            existingOverlay.remove();
        }
        
        // 更新滑块位置和激活状态
        let uiMode = mode;
        if (mode === 'screenshot') {
            uiMode = 'crop'; // UI中显示为crop模式
        }
        
        this.updateSliderPosition(uiMode);
        
        switch(mode) {
            case 'normal':
                // 纯屏幕模式，不显示任何覆盖层
                window.xmlOverlayEnabled = false;
                break;
                
            case 'xml':
                // 启用XML overlay
                window.xmlOverlayEnabled = true;
                const deviceSelect = document.getElementById('deviceSelect');
                if (deviceSelect?.value && window.TestcaseController?.enableXmlOverlay) {
                    window.TestcaseController.enableXmlOverlay(deviceSelect.value);
                }
                break;
                
            case 'screenshot':
                screenContent.classList.add('screenshot-mode');
                window.xmlOverlayEnabled = false;
                break;
                
            case 'coordinate':
                screenContent.classList.add('coordinate-mode');
                window.xmlOverlayEnabled = false;
                break;
        }
        
        // 显示或隐藏缩放控制
        this.updateZoomControlsVisibility();
    },
    
    // 更新滑块位置
    updateSliderPosition(mode) {
        const modeSlider = document.getElementById('modeSlider');
        const modeOptions = document.querySelectorAll('.mode-option');
        
        if (!modeSlider) return;
        
        // 设置滑块的data-active属性来控制指示器位置
        modeSlider.setAttribute('data-active', mode);
        
        // 更新选项的激活状态
        modeOptions.forEach(option => {
            if (option.dataset.mode === mode) {
                option.classList.add('active');
            } else {
                option.classList.remove('active');
            }
        });
        
        // 为不同模式设置不同颜色
        const sliderIndicator = document.getElementById('sliderIndicator');
        if (sliderIndicator) {
            sliderIndicator.className = 'slider-indicator';
            sliderIndicator.classList.add(`mode-${mode}`);
        }
    },
    
    // 设置测试运行状态 - 禁用/启用模式切换
    setTestRunning(isRunning) {
        const modeOptions = document.querySelectorAll('.mode-option');
        
        modeOptions.forEach(option => {
            if (isRunning) {
                option.classList.add('disabled');
            } else {
                option.classList.remove('disabled');
            }
        });
        
        // 如果测试开始，强制切换到纯屏幕模式
        if (isRunning && this.currentMode !== 'normal') {
            this.switchMode('normal');
        }
    },
    
    // 更新缩放控制的可见性
    updateZoomControlsVisibility() {
        const screenZoomControls = document.getElementById('screenZoomControls');
        const deviceImage = document.getElementById('deviceScreenshot');
        
        if (screenZoomControls && deviceImage && deviceImage.style.display !== 'none') {
            screenZoomControls.style.display = 'flex';
        } else if (screenZoomControls) {
            screenZoomControls.style.display = 'none';
        }
    },
    
    // 设置截图模式
    setupScreenshotMode() {
        const screenContent = document.getElementById('screenContent');
        const screenshotSelector = document.getElementById('screenshotSelector');
        
        if (!screenshotSelector) {
            window.rError('截图选择器未找到');
            return;
        }
        
        const selectorBox = screenshotSelector.querySelector('.selector-box');
        const confirmBtn = document.getElementById('confirmScreenshotBtn');
        const cancelBtn = document.getElementById('cancelScreenshotBtn');
        
        // 备用查找方式
        if (!confirmBtn) {
            const altConfirmBtn = screenshotSelector.querySelector('#confirmScreenshotBtn');
            window.rLog('使用备用方式查找确认按钮:', !!altConfirmBtn);
        }
        
        if (!cancelBtn) {
            const altCancelBtn = screenshotSelector.querySelector('#cancelScreenshotBtn');
            window.rLog('使用备用方式查找取消按钮:', !!altCancelBtn);
        }
        
        window.rLog('截图模式设置：', {
            screenContent: !!screenContent,
            screenshotSelector: !!screenshotSelector,
            selectorBox: !!selectorBox,
            confirmBtn: !!confirmBtn,
            cancelBtn: !!cancelBtn
        });
        
        let isSelecting = false;
        let startX = 0, startY = 0;
        
        // 鼠标按下开始选择
        screenContent.addEventListener('mousedown', (e) => {
            if (this.currentMode !== 'screenshot') return;
            
            // 检查是否点击在控制按钮上，如果是则不处理选择
            if (e.target.closest('.selector-controls')) {
                return;
            }
            
            // 获取相对于 screenContent 的坐标
            const rect = screenContent.getBoundingClientRect();
            const screenX = e.clientX - rect.left;
            const screenY = e.clientY - rect.top;
            
            // 检查是否在图片区域内
            if (!this.isPointInImage(screenX, screenY)) return;
            
            isSelecting = true;
            startX = screenX;
            startY = screenY;
            
            screenshotSelector.style.display = 'block';
            selectorBox.style.left = startX + 'px';
            selectorBox.style.top = startY + 'px';
            selectorBox.style.width = '0px';
            selectorBox.style.height = '0px';
            
            // 隐藏控制按钮
            screenshotSelector.querySelector('.selector-controls').style.display = 'none';
            
            e.preventDefault();
        });
        
        // 鼠标移动更新选择框
        screenContent.addEventListener('mousemove', (e) => {
            if (!isSelecting || this.currentMode !== 'screenshot') return;
            
            const rect = screenContent.getBoundingClientRect();
            const currentX = e.clientX - rect.left;
            const currentY = e.clientY - rect.top;
            
            // 限制在图片区域内
            const imageInfo = this.getImageDisplayInfo();
            if (imageInfo) {
                const clampedX = Math.max(imageInfo.left, Math.min(currentX, imageInfo.left + imageInfo.width));
                const clampedY = Math.max(imageInfo.top, Math.min(currentY, imageInfo.top + imageInfo.height));
                
                const left = Math.min(startX, clampedX);
                const top = Math.min(startY, clampedY);
                const width = Math.abs(clampedX - startX);
                const height = Math.abs(clampedY - startY);
                
                selectorBox.style.left = left + 'px';
                selectorBox.style.top = top + 'px';
                selectorBox.style.width = width + 'px';
                selectorBox.style.height = height + 'px';
            }
        });
        
        // 鼠标释放结束选择
        screenContent.addEventListener('mouseup', (e) => {
            if (!isSelecting || this.currentMode !== 'screenshot') return;
            
            isSelecting = false;
            const rect = screenContent.getBoundingClientRect();
            const endX = e.clientX - rect.left;
            const endY = e.clientY - rect.top;
            
            // 限制在图片区域内
            const imageInfo = this.getImageDisplayInfo();
            if (imageInfo) {
                const clampedEndX = Math.max(imageInfo.left, Math.min(endX, imageInfo.left + imageInfo.width));
                const clampedEndY = Math.max(imageInfo.top, Math.min(endY, imageInfo.top + imageInfo.height));
                
                // 转换为图片内坐标
                const startCoords = this.screenToImageCoords(startX, startY);
                const endCoords = this.screenToImageCoords(clampedEndX, clampedEndY);
                
                this.startX = Math.min(startCoords.x, endCoords.x);
                this.startY = Math.min(startCoords.y, endCoords.y);
                this.endX = Math.max(startCoords.x, endCoords.x);
                this.endY = Math.max(startCoords.y, endCoords.y);
                
                // 如果选择区域太小，忽略
                if ((this.endX - this.startX) < 10 || (this.endY - this.startY) < 10) {
                    screenshotSelector.style.display = 'none';
                    return;
                }
                
                // 显示控制按钮
                const controls = screenshotSelector.querySelector('.selector-controls');
                controls.style.display = 'flex';
                controls.style.left = Math.max(startX, clampedEndX) + 'px';
                controls.style.top = Math.min(startY, clampedEndY) + 'px';
            }
        });
        
        // 确认按钮
        if (confirmBtn) {
            window.rLog('截图确认按钮找到，绑定事件');
            
            // 移除已存在的事件监听器避免重复绑定
            if (confirmBtn._screenshotClickHandler) {
                confirmBtn.removeEventListener('click', confirmBtn._screenshotClickHandler);
            }
            
            // 防止重复处理
            let isProcessing = false;
            
            const clickHandler = (e) => {
                e.preventDefault();
                e.stopPropagation();
                
                if (isProcessing) {
                    window.rLog('正在处理中，跳过重复点击');
                    return;
                }
                
                isProcessing = true;
                this.captureSelectedArea().finally(() => {
                    isProcessing = false;
                });
            };
            
            // 保存处理器引用并添加监听器
            confirmBtn._screenshotClickHandler = clickHandler;
            confirmBtn.addEventListener('click', clickHandler);
            
        } else {
            window.rError('截图确认按钮未找到');
        }
        
        // 取消按钮
        if (cancelBtn) {
            // 移除已存在的事件监听器避免重复绑定
            if (cancelBtn._screenshotCancelHandler) {
                cancelBtn.removeEventListener('click', cancelBtn._screenshotCancelHandler);
            }
            
            const cancelHandler = () => {
                window.rLog('截图取消按钮被点击');
                screenshotSelector.style.display = 'none';
            };
            
            // 保存处理器引用并添加监听器
            cancelBtn._screenshotCancelHandler = cancelHandler;
            cancelBtn.addEventListener('click', cancelHandler);
        } else {
            window.rError('截图取消按钮未找到');
        }
    },
    
    // 截取选中区域
    async captureSelectedArea() {
        const { fs, path } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;
        
        if (!projectPath) {
            window.NotificationModule.showNotification('请先打开项目', 'error');
            return;
        }
        
        // 使用 workarea 中的截图文件
        const screenshotPath = path.join(projectPath, 'workarea', 'current_screenshot.png');
        
        try {
            // 检查截图文件是否存在（fs 已经是 fs.promises）
            const screenshotExists = await fs.access(screenshotPath).then(() => true).catch(() => false);
            if (!screenshotExists) {
                window.NotificationModule.showNotification('请先刷新设备屏幕截图', 'error');
                document.getElementById('screenshotSelector').style.display = 'none';
                return;
            }
            
            // 读取截图文件
            const imageBuffer = await fs.readFile(screenshotPath);
            const base64Full = `data:image/png;base64,${imageBuffer.toString('base64')}`;
            
            // 创建临时图片对象来获取原始尺寸
            const tempImg = new Image();
            tempImg.src = base64Full;
            
            await new Promise((resolve) => {
                tempImg.onload = resolve;
            });
            
            // 使用统一的坐标转换系统
            const deviceStartCoords = this.imageToDeviceCoords(this.startX, this.startY);
            const deviceEndCoords = this.imageToDeviceCoords(this.endX, this.endY);
            
            const realStartX = deviceStartCoords.x;
            const realStartY = deviceStartCoords.y;
            const realEndX = deviceEndCoords.x;
            const realEndY = deviceEndCoords.y;
            
            // 创建Canvas来裁切图片
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            const width = realEndX - realStartX;
            const height = realEndY - realStartY;
            
            canvas.width = width;
            canvas.height = height;
            
            // 从完整图片绘制裁切区域
            ctx.drawImage(tempImg, realStartX, realStartY, width, height, 0, 0, width, height);
            
            // 转换为Base64
            const base64Image = canvas.toDataURL('image/png');
            
            // 弹出对话框让用户输入别名
            let alias;
            do {
                alias = await this.promptForAlias();
                if (!alias) {
                    document.getElementById('screenshotSelector').style.display = 'none';
                    return;
                }
            } while (!await this.saveImageLocator(alias, base64Image)); // 如果保存失败（重名）则重新输入
            
            // 隐藏选择器
            document.getElementById('screenshotSelector').style.display = 'none';
            
        } catch (error) {
            window.rError('截取图片失败:', error);
            window.NotificationModule.showNotification('截取失败: ' + error.message, 'error');
            document.getElementById('screenshotSelector').style.display = 'none';
        }
    },
    
    // 提示用户输入别名
    async promptForAlias() {
        return new Promise((resolve) => {
            const modal = document.createElement('div');
            modal.className = 'modal-overlay';
            modal.innerHTML = `
                <div class="modal-dialog">
                    <h3>保存截图定位器</h3>
                    <input type="text" id="imageAliasInput" placeholder="请输入截图别名" autofocus>
                    <div style="display: flex; gap: 10px; justify-content: flex-end;">
                        <button class="btn btn-secondary" id="cancelAliasBtn">取消</button>
                        <button class="btn btn-primary" id="confirmAliasBtn">确定</button>
                    </div>
                </div>
            `;
            document.body.appendChild(modal);
            
            const input = modal.querySelector('#imageAliasInput');
            const confirmBtn = modal.querySelector('#confirmAliasBtn');
            const cancelBtn = modal.querySelector('#cancelAliasBtn');
            
            const confirm = () => {
                const value = input.value.trim();
                if (value) {
                    document.body.removeChild(modal);
                    resolve(value);
                } else {
                    window.NotificationModule.showNotification('请输入别名', 'warning');
                }
            };
            
            const cancel = () => {
                document.body.removeChild(modal);
                resolve(null);
            };
            
            confirmBtn.addEventListener('click', confirm);
            cancelBtn.addEventListener('click', cancel);
            input.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') confirm();
                if (e.key === 'Escape') cancel();
            });
            
            input.focus();
        });
    },
    
    // 保存图片定位器
    async saveImageLocator(alias, base64Image) {
        const { fs, path } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;
        
        if (!projectPath || !window.AppGlobals.currentScript) {
            window.NotificationModule.showNotification('请先选择脚本文件', 'error');
            return false;
        }
        
        try {
            // 确保images目录存在
            const imagesDir = path.join(projectPath, 'images');
            await fs.mkdir(imagesDir, { recursive: true });
            
            // 生成唯一的图片文件名
            const timestamp = Date.now();
            const imageFileName = `${alias}_${timestamp}.png`;
            const imagePath = path.join(imagesDir, imageFileName);
            
            // 检查别名是否已经存在
            const existingFiles = await fs.readdir(imagesDir);
            const existingAliases = existingFiles
                .filter(file => file.startsWith(alias + '_') && file.endsWith('.png'))
                .map(file => file.replace(/_\d+\.png$/, ''));
            
            if (existingAliases.includes(alias)) {
                window.NotificationModule.showNotification(`别名 "${alias}" 已存在，请使用其他名称`, 'error');
                return false;
            }
            
            // 保存图片文件
            const imageBuffer = Buffer.from(base64Image.split(',')[1], 'base64');
            await fs.writeFile(imagePath, imageBuffer);
            
            // 刷新定位器列表
            if (window.LocatorManagerModule && window.LocatorManagerModule.refreshImageLocatorList) {
                window.LocatorManagerModule.refreshImageLocatorList();
            }
            
            window.NotificationModule.showNotification(`截图定位器 "${alias}" 已保存`, 'success');
            return true;
            
        } catch (error) {
            window.rError('保存图片定位器失败:', error);
            window.NotificationModule.showNotification('保存失败: ' + error.message, 'error');
            return false;
        }
    },
    
    // 设置坐标模式
    setupCoordinateMode() {
        const screenContent = document.getElementById('screenContent');
        const coordinateMarker = document.getElementById('coordinateMarker');
        const coordinateDot = coordinateMarker?.querySelector('.coordinate-dot');
        const coordinateLabel = coordinateMarker?.querySelector('.coordinate-label');
        
        screenContent.addEventListener('click', async (e) => {
            if (this.currentMode !== 'coordinate') return;
            
            // 获取相对于 screenContent 的坐标
            const rect = screenContent.getBoundingClientRect();
            const screenX = e.clientX - rect.left;
            const screenY = e.clientY - rect.top;
            
            // 检查是否在图片区域内
            if (!this.isPointInImage(screenX, screenY)) return;
            
            // 转换为图片内坐标，然后转换为设备坐标
            const imageCoords = this.screenToImageCoords(screenX, screenY);
            const deviceCoords = this.imageToDeviceCoords(imageCoords.x, imageCoords.y);
            
            // 显示坐标标记
            if (coordinateMarker) {
                coordinateMarker.style.display = 'block';
                coordinateMarker.style.left = screenX + 'px';
                coordinateMarker.style.top = screenY + 'px';
            }
            
            // 更新坐标标签（新TKS语法格式）
            const coordText = `{${deviceCoords.x},${deviceCoords.y}}`;
            if (coordinateLabel) {
                coordinateLabel.textContent = coordText;
            }
            
            // 复制到剪贴板
            try {
                await navigator.clipboard.writeText(coordText);
                window.NotificationModule.showNotification(`坐标 ${coordText} 已复制到剪贴板`, 'success');
            } catch (err) {
                window.rError('Failed to copy coordinates:', err);
                window.NotificationModule.showNotification('复制坐标失败', 'error');
            }
            
            // 3秒后自动隐藏标记
            setTimeout(() => {
                if (coordinateMarker) {
                    coordinateMarker.style.display = 'none';
                }
            }, 3000);
        });
    }
};

// 延迟初始化模式管理器，确保在 testcase 页面显示后再初始化
function initializeScreenModeManager() {
    window.rLog('开始初始化 ScreenModeManager');
    
    // 检查必要的 DOM 元素是否存在
    const screenContent = document.getElementById('screenContent');
    const screenshotSelector = document.getElementById('screenshotSelector');
    
    if (screenContent && screenshotSelector) {
        window.rLog('DOM 元素已准备好，初始化 ScreenModeManager');
        ScreenModeManager.init();
    } else {
        window.rLog('DOM 元素未准备好，延迟初始化', {
            screenContent: !!screenContent,
            screenshotSelector: !!screenshotSelector
        });
        // 延迟重试
        setTimeout(initializeScreenModeManager, 500);
    }
}

// 导出模块
window.ScreenModeManagerModule = {
    ScreenModeManager,
    initializeScreenModeManager
};