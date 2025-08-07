// 可调整大小面板功能模块
function initializeResizablePanels() {
    const verticalResizer = document.getElementById('verticalResizer');
    const rightPanel = document.getElementById('rightPanel');
    const consoleContainer = document.getElementById('consoleContainer');
    const toggleConsoleBtn = document.getElementById('toggleConsoleBtn');
    const testcaseContainer = document.querySelector('.testcase-container');

    let isResizing = false;
    let startX, startWidth;

    // 垂直调整器（调整右面板宽度）
    if (verticalResizer && rightPanel && testcaseContainer) {
        verticalResizer.addEventListener('mousedown', (e) => {
            isResizing = true;
            startX = e.clientX;
            startWidth = rightPanel.offsetWidth;
            verticalResizer.classList.add('dragging');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';
        });
    }

    // 全局鼠标移动处理器
    document.addEventListener('mousemove', (e) => {
        if (!isResizing) return;

        if (verticalResizer && verticalResizer.classList.contains('dragging')) {
            // 垂直调整
            const deltaX = startX - e.clientX;
            const newWidth = startWidth + deltaX;
            const minWidth = 200;
            const maxWidth = 600;
            
            if (newWidth >= minWidth && newWidth <= maxWidth) {
                rightPanel.style.width = newWidth + 'px';
            }
        }
    });

    // 全局鼠标释放处理器
    document.addEventListener('mouseup', () => {
        if (isResizing) {
            isResizing = false;
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
            
            if (verticalResizer) verticalResizer.classList.remove('dragging');
        }
    });

    // 控制台切换功能
    if (toggleConsoleBtn && consoleContainer) {
        toggleConsoleBtn.addEventListener('click', () => {
            consoleContainer.classList.toggle('collapsed');
        });
    }
}

// 导出函数
window.ResizablePanelsModule = {
    initializeResizablePanels
};