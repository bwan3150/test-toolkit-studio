// 坐标转换工具模块
// 负责在屏幕坐标、图片坐标和设备坐标之间转换

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

const CoordinateConverter = {
    // 获取图片显示信息
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
    }
};

// 导出模块
window.CoordinateConverter = CoordinateConverter;
