/**
 * 双模式编辑器
 * 封装 CodeJarAdapter (文本模式) 和 BlockModeEditor (块模式)
 * 支持模式切换和双向同步
 * @version 1.0.0
 */
(window.rLog || console.log)('dual-mode-editor.js 开始加载');

class DualModeEditor {
    constructor(container, filePath) {
        this.container = container;
        this.filePath = filePath;
        this.currentMode = 'block'; // 'text' | 'block' - 默认块模式

        this.textEditor = null; // CodeJarAdapter 实例
        this.blockEditor = null; // BlockModeEditor 实例

        this.isDirty = false;
        this.originalContent = '';
        this.eventHandlers = new Map();

        // 创建双容器
        this.textContainer = null;
        this.blockContainer = null;

        // 高亮状态追踪(用于模式切换时保持高亮)
        this.currentHighlightedLine = null; // 当前高亮的行号(1-based)
        this.currentHighlightType = null; // 'executing' 或 'error'

        window.rLog(`DualModeEditor 创建: ${filePath}`);
    }

    /**
     * 初始化编辑器
     */
    async init() {
        window.rLog('初始化双模式编辑器...');

        // 创建容器
        this.createContainers();

        // 始终先初始化文本编辑器(CodeJar)作为数据源
        this.textEditor = new window.CodeJarAdapter(this.textContainer, this.filePath);
        await this.textEditor.init();

        // 转发文本编辑器的dirty事件
        this.textEditor.on('dirty-changed', (data) => {
            this.emit('dirty-changed', data);
        });

        // 初始化块编辑器作为可视化UI层(基于textEditor)
        this.blockEditor = new window.BlockModeEditor(this.blockContainer, this.textEditor);
        await this.blockEditor.init();

        // 默认显示块模式
        this.showBlockMode();

        window.rLog('双模式编辑器初始化完成');
    }

    /**
     * 创建双容器结构
     */
    createContainers() {
        this.container.innerHTML = '';

        // 文本编辑器容器
        this.textContainer = document.createElement('div');
        this.textContainer.className = 'editor-mode-container text-mode-container';
        this.textContainer.style.display = 'none';
        this.textContainer.style.height = '100%';
        this.container.appendChild(this.textContainer);

        // 块编辑器容器
        this.blockContainer = document.createElement('div');
        this.blockContainer.className = 'editor-mode-container block-mode-container';
        this.blockContainer.style.height = '100%';
        this.container.appendChild(this.blockContainer);
    }

    /**
     * 切换到文本模式
     */
    async showTextMode() {
        this.currentMode = 'text';

        // 文本编辑器始终存在,只需要切换显示即可
        this.blockContainer.style.display = 'none';
        this.textContainer.style.display = 'block';
        window.StatusBarModule?.updateEditorMode('text', 'idle');

        // 恢复高亮状态
        if (this.currentHighlightedLine !== null && this.textEditor) {
            if (this.currentHighlightType === 'executing') {
                this.textEditor.highlightExecutingLine(this.currentHighlightedLine);
            } else if (this.currentHighlightType === 'error') {
                this.textEditor.highlightErrorLine(this.currentHighlightedLine);
            }
            window.rLog(`文本模式恢复高亮: 行${this.currentHighlightedLine}, 类型${this.currentHighlightType}`);
        }

        // 聚焦文本编辑器
        if (this.textEditor) {
            this.textEditor.focus();
        }

        window.rLog('切换到文本模式');
    }

    /**
     * 切换到块模式
     */
    async showBlockMode() {
        this.currentMode = 'block';

        // 块编辑器切换显示时,需要从textEditor重新解析内容
        if (this.blockEditor) {
            await this.blockEditor.refresh();
        }

        this.textContainer.style.display = 'none';
        this.blockContainer.style.display = 'block';
        window.StatusBarModule?.updateEditorMode('block', 'idle');

        // 恢复高亮状态
        if (this.currentHighlightedLine !== null && this.blockEditor) {
            if (this.currentHighlightType === 'executing') {
                this.blockEditor.highlightExecutingLine(this.currentHighlightedLine);
            } else if (this.currentHighlightType === 'error') {
                this.blockEditor.highlightErrorLine(this.currentHighlightedLine);
            }
            window.rLog(`块模式恢复高亮: 行${this.currentHighlightedLine}, 类型${this.currentHighlightType}`);
        }

        window.rLog('切换到块模式');
        window.rLog('blockContainer display:', this.blockContainer.style.display);
        window.rLog('blockContainer 可见:', this.blockContainer.offsetHeight > 0);

        // 检查子元素
        const blocksContainer = this.blockContainer.querySelector('.blocks-container');
        window.rLog('blocksContainer 找到:', !!blocksContainer);
        window.rLog('blocksContainer display:', blocksContainer?.style.display);
        window.rLog('blocksContainer innerHTML长度:', blocksContainer?.innerHTML.length);
    }

    /**
     * 切换模式
     */
    async toggleMode() {
        if (this.currentMode === 'text') {
            await this.showBlockMode();
        } else {
            await this.showTextMode();
        }
    }

    /**
     * 获取当前活动编辑器
     */
    getCurrentEditor() {
        return this.currentMode === 'text' ? this.textEditor : this.blockEditor;
    }

    /**
     * 保存文件
     */
    async save() {
        // 始终通过textEditor保存
        if (this.textEditor) {
            await this.textEditor.save();
        }
    }

    /**
     * 获取内容
     */
    getContent() {
        // 始终从textEditor获取内容
        return this.textEditor?.getContent() || '';
    }

    /**
     * 获取原始内容（用于脚本执行）
     */
    getRawContent() {
        return this.getContent();
    }

    /**
     * 检查是否有未保存的修改
     */
    isDirtyState() {
        // 始终检查textEditor的dirty状态
        return this.textEditor?.isDirtyState() || false;
    }

    /**
     * 聚焦编辑器
     */
    focus() {
        const currentEditor = this.getCurrentEditor();
        if (currentEditor) {
            currentEditor.focus();
        }
    }

    /**
     * 锁定编辑器（禁止编辑）
     */
    lock() {
        if (this.textEditor) this.textEditor.lock();
        if (this.blockEditor) this.blockEditor.lock();
    }

    /**
     * 解锁编辑器（允许编辑）
     */
    unlock() {
        if (this.textEditor) this.textEditor.unlock();
        if (this.blockEditor) this.blockEditor.unlock();
    }

    /**
     * 高亮正在执行的行
     */
    highlightExecutingLine(lineNumber) {
        // 保存高亮状态(用于模式切换时恢复)
        this.currentHighlightedLine = lineNumber;
        this.currentHighlightType = 'executing';

        const currentEditor = this.getCurrentEditor();
        if (currentEditor && currentEditor.highlightExecutingLine) {
            currentEditor.highlightExecutingLine(lineNumber);
        }

        window.rLog(`DualModeEditor: 保存执行高亮状态 - 行${lineNumber}`);
    }

    /**
     * 高亮错误行
     */
    highlightErrorLine(lineNumber) {
        // 保存高亮状态(用于模式切换时恢复)
        this.currentHighlightedLine = lineNumber;
        this.currentHighlightType = 'error';

        const currentEditor = this.getCurrentEditor();
        if (currentEditor && currentEditor.highlightErrorLine) {
            currentEditor.highlightErrorLine(lineNumber);
        }

        window.rLog(`DualModeEditor: 保存错误高亮状态 - 行${lineNumber}`);
    }

    /**
     * 设置测试运行状态
     */
    setTestRunning(isRunning, clearHighlight) {
        // 如果需要清除高亮,也要清除保存的状态
        if (clearHighlight) {
            this.currentHighlightedLine = null;
            this.currentHighlightType = null;
            window.rLog('DualModeEditor: 清除高亮状态');
        }

        if (this.textEditor && this.textEditor.setTestRunning) {
            this.textEditor.setTestRunning(isRunning, clearHighlight);
        }
        if (this.blockEditor && this.blockEditor.setTestRunning) {
            this.blockEditor.setTestRunning(isRunning, clearHighlight);
        }
    }

    /**
     * 事件监听
     */
    on(event, handler) {
        if (!this.eventHandlers.has(event)) {
            this.eventHandlers.set(event, []);
        }
        this.eventHandlers.get(event).push(handler);
    }

    /**
     * 触发事件
     */
    emit(event, data) {
        const handlers = this.eventHandlers.get(event);
        if (handlers) {
            handlers.forEach(handler => handler(data));
        }
    }

    /**
     * 销毁编辑器
     */
    destroy() {
        window.rLog('销毁双模式编辑器');

        if (this.textEditor) {
            this.textEditor.destroy();
            this.textEditor = null;
        }

        if (this.blockEditor) {
            this.blockEditor.destroy();
            this.blockEditor = null;
        }

        this.container.innerHTML = '';
        this.eventHandlers.clear();
    }
}

// 导出到全局
window.DualModeEditor = DualModeEditor;
(window.rLog || console.log)('DualModeEditor 模块已加载');
