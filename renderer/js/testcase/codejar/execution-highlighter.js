/**
 * CodeJar 执行高亮控制器
 * 负责在代码执行时高亮当前执行行和错误行
 * @version 1.0.1
 * @updated 2025-10-23 15:15:00
 */
(window.rLog || console.log)('🔵 execution-highlighter.js 开始加载 v1.0.1');

class ExecutionHighlighter {
    constructor(editorElement) {
        this.editorElement = editorElement; // CodeJar 的 contenteditable div
        this.currentHighlightedLine = null;
        this.isTestRunning = false;

        window.rLog('📝 ExecutionHighlighter 创建');
    }

    /**
     * 高亮正在执行的行
     * @param {number} lineNumber - 行号（1-based）
     */
    highlightExecutingLine(lineNumber) {
        window.rLog(`🔆 高亮执行行: ${lineNumber}`);

        // 清除之前的高亮
        this.clearHighlight();

        // 设置当前高亮行
        this.currentHighlightedLine = lineNumber;

        // 添加高亮
        this.addLineHighlight(lineNumber, 'executing');
    }

    /**
     * 高亮错误行
     * @param {number} lineNumber - 行号（1-based）
     */
    highlightErrorLine(lineNumber) {
        window.rLog(`❌ 高亮错误行: ${lineNumber}`);

        // 清除之前的高亮
        this.clearHighlight();

        // 设置当前高亮行
        this.currentHighlightedLine = lineNumber;

        // 添加错误高亮
        this.addLineHighlight(lineNumber, 'error');
    }

    /**
     * 添加行高亮
     * @param {number} lineNumber - 行号（1-based）
     * @param {string} type - 高亮类型 ('executing' 或 'error')
     */
    addLineHighlight(lineNumber, type) {
        if (!this.editorElement) {
            window.rError('编辑器元素不存在');
            return;
        }

        // 获取编辑器内容
        const content = this.editorElement.textContent || '';
        const lines = content.split('\n');

        // 验证行号
        if (lineNumber < 1 || lineNumber > lines.length) {
            window.rError(`行号 ${lineNumber} 超出范围 (1-${lines.length})`);
            return;
        }

        // 计算行的起始和结束位置
        let startPos = 0;
        for (let i = 0; i < lineNumber - 1; i++) {
            startPos += lines[i].length + 1; // +1 for newline
        }
        const endPos = startPos + lines[lineNumber - 1].length;

        // 创建高亮标记
        this.createHighlightMarker(lineNumber, type);

        // 滚动到高亮行
        this.scrollToLine(lineNumber);
    }

    /**
     * 创建高亮标记（在编辑器外部添加背景层）
     * @param {number} lineNumber - 行号（1-based）
     * @param {string} type - 高亮类型
     */
    createHighlightMarker(lineNumber, type) {
        // 移除旧的高亮标记
        const oldMarkers = this.editorElement.parentElement.querySelectorAll('.execution-highlight-marker');
        oldMarkers.forEach(marker => marker.remove());

        // 获取编辑器的行高
        const lineHeight = this.getLineHeight();

        // 创建高亮标记元素
        const marker = document.createElement('div');
        marker.className = `execution-highlight-marker ${type}`;
        marker.style.position = 'absolute';
        marker.style.left = '0';
        marker.style.right = '0';
        marker.style.top = `${(lineNumber - 1) * lineHeight}px`;
        marker.style.height = `${lineHeight}px`;
        marker.style.pointerEvents = 'none';
        marker.style.zIndex = '1';

        // 设置背景色
        if (type === 'executing') {
            marker.style.background = 'rgba(255, 255, 0, 0.15)'; // 黄色半透明
            marker.style.borderLeft = '3px solid #ffcc00';
        } else if (type === 'error') {
            marker.style.background = 'rgba(255, 0, 0, 0.15)'; // 红色半透明
            marker.style.borderLeft = '3px solid #ff0000';
        }

        // 确保父容器是 relative 定位
        if (!this.editorElement.parentElement.style.position) {
            this.editorElement.parentElement.style.position = 'relative';
        }

        // 插入到编辑器容器中
        this.editorElement.parentElement.insertBefore(marker, this.editorElement);
    }

    /**
     * 获取行高
     */
    getLineHeight() {
        const style = window.getComputedStyle(this.editorElement);
        const lineHeight = parseFloat(style.lineHeight);

        if (isNaN(lineHeight)) {
            // 如果 line-height 是 normal 或其他非数字值，计算实际行高
            const fontSize = parseFloat(style.fontSize);
            return fontSize * 1.6; // 默认 1.6 倍
        }

        return lineHeight;
    }

    /**
     * 滚动到指定行
     * @param {number} lineNumber - 行号（1-based）
     */
    scrollToLine(lineNumber) {
        const lineHeight = this.getLineHeight();
        const targetY = (lineNumber - 1) * lineHeight;

        // 滚动到目标位置，保持一定的上下文
        const offset = lineHeight * 3; // 显示前后3行
        this.editorElement.scrollTop = Math.max(0, targetY - offset);
    }

    /**
     * 清除所有高亮
     */
    clearHighlight() {
        // 移除高亮标记
        if (this.editorElement && this.editorElement.parentElement) {
            const markers = this.editorElement.parentElement.querySelectorAll('.execution-highlight-marker');
            markers.forEach(marker => marker.remove());
        }

        this.currentHighlightedLine = null;
    }

    /**
     * 设置测试运行状态
     * @param {boolean} isRunning - 是否正在运行
     * @param {boolean} clearHighlight - 是否清除高亮
     */
    setTestRunning(isRunning, clearHighlight) {
        window.rLog(`🎯 设置测试运行状态: ${isRunning}, 清除高亮: ${clearHighlight}`);

        this.isTestRunning = isRunning;

        if (clearHighlight) {
            this.clearHighlight();
        }
    }

    /**
     * 销毁高亮器
     */
    destroy() {
        this.clearHighlight();
        this.editorElement = null;
    }
}

// 导出到全局
window.ExecutionHighlighter = ExecutionHighlighter;
(window.rLog || console.log)('✅ ExecutionHighlighter 模块已加载');
