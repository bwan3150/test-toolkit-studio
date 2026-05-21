// Overlay渲染模块
// 负责渲染UI元素框、处理ResizeObserver、元素选择等

// 创建UI覆盖层
async function createUIOverlay(elements, screenSize) {
    window.rLog(`创建UI覆盖层,元素数量: ${elements.length}`);

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

    // 创建新的覆盖层,直接覆盖在图片上
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

        // 按z-index从大到小遍历,找到第一个包含鼠标位置的元素
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

    window.rLog(`✅ Overlay 已添加到 DOM,className: ${overlay.className}`);
    window.rLog(`Overlay 样式: ${overlay.style.cssText}`);

    // 设置 ResizeObserver 来监听容器大小变化
    setupResizeObserver(screenContent, deviceImage);

    // 渲染元素框
    renderUIElements(overlay, elements, screenSize);
}

// 设置 ResizeObserver
function setupResizeObserver(screenContent, deviceImage) {
    // 如果已有观察器,先断开
    if (window.ScreenState.resizeObserver) {
        window.ScreenState.resizeObserver.disconnect();
    }

    // 创建新的观察器
    window.ScreenState.resizeObserver = new ResizeObserver((entries) => {
        window.rLog('🔄 ResizeObserver 触发!');

        // 输出每个元素的大小变化
        entries.forEach(entry => {
            const name = entry.target.id || entry.target.className;
            window.rLog(`📏 元素 ${name} 大小变化:`, {
                width: entry.contentRect.width,
                height: entry.contentRect.height
            });
        });

        // 检查条件并更新
        if (window.ScreenState.xmlOverlayEnabled && window.ScreenState.currentUIElements.length > 0) {
            window.rLog('✅ 条件满足,更新 overlay 位置和元素');
            // 调用updateOverlayPosition来更新overlay位置和重新渲染元素
            updateOverlayPosition();
        } else {
            window.rLog(`❌ 条件不满足:`, {
                xmlOverlayEnabled: window.ScreenState.xmlOverlayEnabled,
                elementsCount: window.ScreenState.currentUIElements.length
            });
        }
    });

    // 开始观察
    window.ScreenState.resizeObserver.observe(screenContent);
    if (deviceImage) {
        window.ScreenState.resizeObserver.observe(deviceImage);
    }

    window.rLog('✅ ResizeObserver 已设置');
}

// 更新 overlay 位置(当容器大小变化时)
function updateOverlayPosition() {
    window.rLog('🎯 updateOverlayPosition 被调用');

    const screenContent = document.getElementById('screenContent');
    const deviceImage = document.getElementById('deviceScreenshot');
    const overlay = screenContent?.querySelector('.ui-overlay');

    if (!overlay || !deviceImage || !window.ScreenState.currentUIElements.length) {
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
    renderUIElements(overlay, window.ScreenState.currentUIElements, window.ScreenState.currentScreenSize);
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

// 选择元素
function selectElement(index) {
    window.rLog(`选择元素: ${index}`);

    // 更新选中状态
    window.ScreenState.selectedElement = index;

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
    const element = window.ScreenState.currentUIElements[index];
    if (element) {
        // 触发自定义事件,通知其他模块
        const event = new CustomEvent('elementSelected', {
            detail: { element, index }
        });
        document.dispatchEvent(event);

        window.rLog('选中元素详情:', element);
    }
}

// 从列表选择元素(保留给全局使用)
window.selectElementByIndex = function(index) {
    selectElement(index);
};

// 导出模块
window.OverlayRenderer = {
    createUIOverlay,
    setupResizeObserver,
    updateOverlayPosition,
    renderUIElements,
    selectElement
};
