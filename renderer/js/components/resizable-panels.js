// 可调整大小面板功能模块
function initializeResizablePanels() {
    const verticalResizer = document.getElementById('verticalResizer');
    const explorerResizer = document.getElementById('explorerResizer');
    const rightPanel = document.getElementById('rightPanel');
    const fileExplorer = document.getElementById('fileExplorer');
    const testcaseContainer = document.querySelector('.testcase-container');

    let isResizing = false;
    let startX, startWidth;
    let currentResizer = null;

    // 垂直调整器（调整右面板宽度）
    if (verticalResizer && rightPanel && testcaseContainer) {
        verticalResizer.addEventListener('mousedown', (e) => {
            isResizing = true;
            currentResizer = 'vertical';
            startX = e.clientX;
            startWidth = rightPanel.offsetWidth;
            verticalResizer.classList.add('dragging');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';
        });
    }
    
    // Explorer调整器（调整左侧面板宽度）
    if (explorerResizer && fileExplorer) {
        explorerResizer.addEventListener('mousedown', (e) => {
            isResizing = true;
            currentResizer = 'explorer';
            startX = e.clientX;
            startWidth = fileExplorer.offsetWidth;
            explorerResizer.classList.add('dragging');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';
            e.preventDefault();
        });
    }

    // 全局鼠标移动处理器
    let rafId = null;
    document.addEventListener('mousemove', (e) => {
        if (!isResizing) return;

        // 使用requestAnimationFrame优化性能
        if (rafId) {
            cancelAnimationFrame(rafId);
        }
        
        rafId = requestAnimationFrame(() => {
            switch(currentResizer) {
                case 'vertical':
                    // 右面板垂直调整
                    const deltaX = startX - e.clientX;
                    const newRightWidth = Math.max(200, Math.min(600, startWidth + deltaX));
                    rightPanel.style.width = newRightWidth + 'px';
                    break;
                    
                case 'explorer':
                    // Explorer水平调整
                    const explorerDeltaX = e.clientX - startX;
                    const newExplorerWidth = Math.max(150, Math.min(500, startWidth + explorerDeltaX));
                    fileExplorer.style.width = newExplorerWidth + 'px';
                    break;
            }
        });
    });

    // 全局鼠标释放处理器
    document.addEventListener('mouseup', () => {
        if (isResizing) {
            isResizing = false;
            currentResizer = null;
            
            // 清理requestAnimationFrame
            if (rafId) {
                cancelAnimationFrame(rafId);
                rafId = null;
            }
            
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
            
            // 移除所有拖拽条的dragging类
            if (verticalResizer) verticalResizer.classList.remove('dragging');
            if (explorerResizer) explorerResizer.classList.remove('dragging');
            
            // 面板调整完成后，重新计算XML标记位置
            if (window.TestcaseController && window.TestcaseController.recalculateXmlMarkersPosition) {
                // 延迟一下等DOM更新完成，使用更短的延迟提高响应性
                setTimeout(() => {
                    window.TestcaseController.recalculateXmlMarkersPosition();
                }, 16); // 约1帧的时间
            }
        }
    });
}

// 导出函数
window.ResizablePanelsModule = {
    initializeResizablePanels
};