// 坐标点取值模式模块
// 负责点击设备屏幕获取坐标并复制到剪贴板

const CoordinateMode = {
    // 设置坐标模式
    setup(coordinateConverter) {
        const screenContent = document.getElementById('screenContent');
        const coordinateMarker = document.getElementById('coordinateMarker');
        const coordinateDot = coordinateMarker?.querySelector('.coordinate-dot');
        const coordinateLabel = coordinateMarker?.querySelector('.coordinate-label');

        screenContent.addEventListener('click', async (e) => {
            if (!this.isInCoordinateMode()) return;

            // 获取相对于 screenContent 的坐标
            const rect = screenContent.getBoundingClientRect();
            const screenX = e.clientX - rect.left;
            const screenY = e.clientY - rect.top;

            // 检查是否在图片区域内
            if (!coordinateConverter.isPointInImage(screenX, screenY)) return;

            // 转换为图片内坐标,然后转换为设备坐标
            const imageCoords = coordinateConverter.screenToImageCoords(screenX, screenY);
            const deviceCoords = coordinateConverter.imageToDeviceCoords(imageCoords.x, imageCoords.y);

            // 显示坐标标记
            if (coordinateMarker) {
                coordinateMarker.style.display = 'block';
                coordinateMarker.style.left = screenX + 'px';
                coordinateMarker.style.top = screenY + 'px';
            }

            // 更新坐标标签(新TKS语法格式)
            const coordText = `{${deviceCoords.x},${deviceCoords.y}}`;
            if (coordinateLabel) {
                coordinateLabel.textContent = coordText;
            }

            // 复制到剪贴板
            try {
                await navigator.clipboard.writeText(coordText);
                window.AppNotifications?.success(`坐标 ${coordText} 已复制到剪贴板`);
            } catch (err) {
                window.rError('Failed to copy coordinates:', err);
                window.AppNotifications?.error('复制坐标失败');
            }

            // 3秒后自动隐藏标记
            setTimeout(() => {
                if (coordinateMarker) {
                    coordinateMarker.style.display = 'none';
                }
            }, 3000);
        });
    },

    // 检查当前是否处于坐标模式
    isInCoordinateMode() {
        const screenContent = document.getElementById('screenContent');
        return screenContent && screenContent.classList.contains('coordinate-mode');
    }
};

// 导出模块
window.CoordinateMode = CoordinateMode;
