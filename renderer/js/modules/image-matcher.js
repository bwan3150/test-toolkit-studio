// 图像匹配模块
// 使用 jimp 进行简单的模板匹配

(function() {
    let Jimp;
    
    // 初始化 Jimp
    const initJimp = async () => {
        if (!Jimp) {
            if (window.nodeRequire) {
                const jimpModule = window.nodeRequire('jimp');
                Jimp = jimpModule.Jimp || jimpModule;
            } else {
                const jimpModule = require('jimp');
                Jimp = jimpModule.Jimp || jimpModule;
            }
        }
        return Jimp;
    };

    /**
     * 图像匹配器类
     */
    class ImageMatcher {
        constructor() {
            this.threshold = 0.8; // 默认匹配阈值
        }

        /**
         * 简单模板匹配 - 找到最佳匹配位置并返回坐标
         * @param {string} screenshotPath - 当前屏幕截图路径
         * @param {string} templatePath - 模板图片路径
         * @returns {Promise<Object>} 匹配结果
         */
        async templateMatch(screenshotPath, templatePath) {
            try {
                const jimp = await initJimp();
                
                // 加载图像
                const screenshot = await jimp.read(screenshotPath);
                const template = await jimp.read(templatePath);
                
                // 获取图像尺寸
                const screenWidth = screenshot.bitmap.width;
                const screenHeight = screenshot.bitmap.height;
                const templateWidth = template.bitmap.width;
                const templateHeight = template.bitmap.height;
                
                console.log(`截图尺寸: ${screenWidth}x${screenHeight}, 模板尺寸: ${templateWidth}x${templateHeight}`);
                
                if (templateWidth > screenWidth || templateHeight > screenHeight) {
                    return {
                        success: false,
                        error: '模板图片大于截图，无法匹配'
                    };
                }
                
                // 简单滑动窗口匹配
                let bestMatch = null;
                let bestSimilarity = 0;
                
                // 每10像素搜索一次，快速找到大致位置
                for (let y = 0; y <= screenHeight - templateHeight; y += 10) {
                    for (let x = 0; x <= screenWidth - templateWidth; x += 10) {
                        const similarity = this.calculateSimpleSimilarity(screenshot, template, x, y);
                        
                        if (similarity > bestSimilarity) {
                            bestSimilarity = similarity;
                            bestMatch = { x, y, similarity };
                        }
                    }
                }
                
                if (!bestMatch || bestSimilarity < 0.6) {
                    return {
                        success: false,
                        error: `未找到匹配点，最佳相似度: ${bestSimilarity.toFixed(3)}`
                    };
                }
                
                // 在最佳匹配点附近精确搜索
                const searchRange = 20;
                let finalMatch = bestMatch;
                let finalSimilarity = bestSimilarity;
                
                const startX = Math.max(0, bestMatch.x - searchRange);
                const endX = Math.min(screenWidth - templateWidth, bestMatch.x + searchRange);
                const startY = Math.max(0, bestMatch.y - searchRange);
                const endY = Math.min(screenHeight - templateHeight, bestMatch.y + searchRange);
                
                for (let y = startY; y <= endY; y++) {
                    for (let x = startX; x <= endX; x++) {
                        const similarity = this.calculateSimpleSimilarity(screenshot, template, x, y);
                        
                        if (similarity > finalSimilarity) {
                            finalSimilarity = similarity;
                            finalMatch = { x, y, similarity };
                        }
                    }
                }
                
                // 计算中心坐标
                const centerX = finalMatch.x + Math.floor(templateWidth / 2);
                const centerY = finalMatch.y + Math.floor(templateHeight / 2);
                
                console.log(`找到匹配点: (${finalMatch.x}, ${finalMatch.y}), 中心: (${centerX}, ${centerY}), 相似度: ${finalSimilarity.toFixed(3)}`);
                
                return {
                    success: true,
                    center_x: centerX,
                    center_y: centerY,
                    confidence: finalSimilarity,
                    bounding_box: {
                        x: finalMatch.x,
                        y: finalMatch.y,
                        width: templateWidth,
                        height: templateHeight
                    }
                };
                
            } catch (error) {
                return {
                    success: false,
                    error: `图像匹配发生错误: ${error.message}`
                };
            }
        }

        /**
         * 简单相似度计算 - 对比RGB像素差异
         * @param {Jimp} screenshot - 截图对象
         * @param {Jimp} template - 模板对象
         * @param {number} x - 截图中的 x 坐标
         * @param {number} y - 截图中的 y 坐标
         * @returns {number} 相似度 (0-1)
         */
        calculateSimpleSimilarity(screenshot, template, x, y) {
            const templateWidth = template.bitmap.width;
            const templateHeight = template.bitmap.height;
            
            let totalDiff = 0;
            let pixelCount = 0;
            
            // 每3个像素采样一次以提高速度
            const step = 3;
            
            for (let ty = 0; ty < templateHeight; ty += step) {
                for (let tx = 0; tx < templateWidth; tx += step) {
                    const screenPixel = screenshot.getPixelColor(x + tx, y + ty);
                    const templatePixel = template.getPixelColor(tx, ty);
                    
                    // 转换为RGB
                    const screenR = (screenPixel >> 24) & 0xFF;
                    const screenG = (screenPixel >> 16) & 0xFF;
                    const screenB = (screenPixel >> 8) & 0xFF;
                    
                    const templateR = (templatePixel >> 24) & 0xFF;
                    const templateG = (templatePixel >> 16) & 0xFF;
                    const templateB = (templatePixel >> 8) & 0xFF;
                    
                    // 计算RGB差异
                    const diff = Math.abs(screenR - templateR) + 
                                Math.abs(screenG - templateG) + 
                                Math.abs(screenB - templateB);
                    
                    totalDiff += diff;
                    pixelCount++;
                }
            }
            
            // 归一化相似度
            const maxDiff = 255 * 3; // RGB最大差异
            const avgDiff = totalDiff / pixelCount;
            const similarity = 1 - (avgDiff / maxDiff);
            
            return Math.max(0, Math.min(1, similarity));
        }
    }

    // 导出模块
    if (typeof window !== 'undefined') {
        window.ImageMatcher = ImageMatcher;
    } else if (typeof module !== 'undefined' && module.exports) {
        module.exports = ImageMatcher;
    }
})();