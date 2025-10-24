// 截图选择器模块
// 负责截图取坐标模式的框选、确认、保存逻辑

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

const ScreenshotSelector = {
    isSelecting: false,
    startX: 0,
    startY: 0,
    endX: 0,
    endY: 0,

    // 设置截图模式
    setup(coordinateConverter) {
        const screenContent = document.getElementById('screenContent');
        const screenshotSelector = document.getElementById('screenshotSelector');

        if (!screenshotSelector) {
            window.rError('截图选择器未找到');
            return;
        }

        const selectorBox = screenshotSelector.querySelector('.selector-box');
        const confirmBtn = document.getElementById('confirmScreenshotBtn');
        const cancelBtn = document.getElementById('cancelScreenshotBtn');

        window.rLog('截图模式设置:', {
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
            if (!this.isInScreenshotMode()) return;

            // 检查是否点击在控制按钮上,如果是则不处理选择
            if (e.target.closest('.selector-controls')) {
                return;
            }

            // 获取相对于 screenContent 的坐标
            const rect = screenContent.getBoundingClientRect();
            const screenX = e.clientX - rect.left;
            const screenY = e.clientY - rect.top;

            // 检查是否在图片区域内
            if (!coordinateConverter.isPointInImage(screenX, screenY)) return;

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
            if (!isSelecting || !this.isInScreenshotMode()) return;

            const rect = screenContent.getBoundingClientRect();
            const currentX = e.clientX - rect.left;
            const currentY = e.clientY - rect.top;

            // 限制在图片区域内
            const imageInfo = coordinateConverter.getImageDisplayInfo();
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
            if (!isSelecting || !this.isInScreenshotMode()) return;

            isSelecting = false;
            const rect = screenContent.getBoundingClientRect();
            const endX = e.clientX - rect.left;
            const endY = e.clientY - rect.top;

            // 限制在图片区域内
            const imageInfo = coordinateConverter.getImageDisplayInfo();
            if (imageInfo) {
                const clampedEndX = Math.max(imageInfo.left, Math.min(endX, imageInfo.left + imageInfo.width));
                const clampedEndY = Math.max(imageInfo.top, Math.min(endY, imageInfo.top + imageInfo.height));

                // 转换为图片内坐标
                const startCoords = coordinateConverter.screenToImageCoords(startX, startY);
                const endCoords = coordinateConverter.screenToImageCoords(clampedEndX, clampedEndY);

                this.startX = Math.min(startCoords.x, endCoords.x);
                this.startY = Math.min(startCoords.y, endCoords.y);
                this.endX = Math.max(startCoords.x, endCoords.x);
                this.endY = Math.max(startCoords.y, endCoords.y);

                // 如果选择区域太小,忽略
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
            window.rLog('截图确认按钮找到,绑定事件');

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
                    window.rLog('正在处理中,跳过重复点击');
                    return;
                }

                isProcessing = true;
                this.captureSelectedArea(coordinateConverter).finally(() => {
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

    // 检查当前是否处于截图模式
    isInScreenshotMode() {
        const screenContent = document.getElementById('screenContent');
        return screenContent && screenContent.classList.contains('screenshot-mode');
    },

    // 截取选中区域
    async captureSelectedArea(coordinateConverter) {
        const { fs, path } = getGlobals();
        const projectPath = window.AppGlobals.currentProject;

        if (!projectPath) {
            window.AppNotifications?.error('请先打开项目');
            return;
        }

        // 使用 workarea 中的截图文件
        const screenshotPath = path.join(projectPath, 'workarea', 'current_screenshot.png');

        try {
            // 检查截图文件是否存在(fs 已经是 fs.promises)
            const screenshotExists = await fs.access(screenshotPath).then(() => true).catch(() => false);
            if (!screenshotExists) {
                window.AppNotifications?.error('请先刷新设备屏幕截图');
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
            const deviceStartCoords = coordinateConverter.imageToDeviceCoords(this.startX, this.startY);
            const deviceEndCoords = coordinateConverter.imageToDeviceCoords(this.endX, this.endY);

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
            } while (!await this.saveImageLocator(alias, base64Image)); // 如果保存失败(重名)则重新输入

            // 隐藏选择器
            document.getElementById('screenshotSelector').style.display = 'none';

        } catch (error) {
            window.rError('截取图片失败:', error);
            window.AppNotifications?.error('截取失败: ' + error.message);
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
                    window.AppNotifications?.warn('请输入别名');
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

        if (!projectPath) {
            window.AppNotifications?.error('请先打开项目');
            return false;
        }

        try {
            // 确保locator/img目录存在
            const locatorDir = path.join(projectPath, 'locator');
            const imgDir = path.join(locatorDir, 'img');
            await fs.mkdir(imgDir, { recursive: true });

            // 检查别名是否已经存在于LocatorLibraryPanel中
            if (window.LocatorLibraryPanel && window.LocatorLibraryPanel.locators[alias]) {
                window.AppNotifications?.error(`定位器名称 "${alias}" 已存在,请使用其他名称`);
                return false;
            }

            // 生成图片文件名
            const imageFileName = `${alias}.png`;
            const imagePath = path.join(imgDir, imageFileName);

            // 保存图片文件
            const imageBuffer = Buffer.from(base64Image.split(',')[1], 'base64');
            await fs.writeFile(imagePath, imageBuffer);

            // 创建图像定位器对象
            const locator = {
                type: 'image',
                locator_type: 'Image',  // 兼容toolkit-engine
                name: alias,
                path: `locator/img/${imageFileName}`,  // 相对于项目根目录的路径
                createdAt: new Date().toISOString()
            };

            // 添加到LocatorLibraryPanel并保存到element.json
            if (window.LocatorLibraryPanel) {
                window.LocatorLibraryPanel.locators[alias] = locator;
                await window.LocatorLibraryPanel.saveLocators();

                // 重新渲染定位器列表
                window.LocatorLibraryPanel.renderLocators();

                // 切换到Locator库标签
                const locatorTab = document.getElementById('locatorLibTab');
                if (locatorTab) {
                    locatorTab.click();
                }
            }

            window.AppNotifications?.success(`图像定位器 "${alias}" 已保存`);
            window.rLog(`图像定位器已保存: ${imagePath}`);
            return true;

        } catch (error) {
            window.rError('保存图片定位器失败:', error);
            window.AppNotifications?.error('保存失败: ' + error.message);
            return false;
        }
    }
};

// 导出模块
window.ScreenshotSelector = ScreenshotSelector;
