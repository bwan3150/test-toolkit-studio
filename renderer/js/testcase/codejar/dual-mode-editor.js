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

        window.rLog(`DualModeEditor 创建: ${filePath}`);
    }

    /**
     * 初始化编辑器
     */
    async init() {
        window.rLog('初始化双模式编辑器...');

        // 创建容器
        this.createContainers();

        // 初始化块编辑器（默认模式）
        this.blockEditor = new window.BlockModeEditor(this.blockContainer, this.filePath);
        await this.blockEditor.init();

        // 转发事件
        this.blockEditor.on('dirty-changed', (data) => {
            this.emit('dirty-changed', data);
        });

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
        this.container.appendChild(this.textContainer);

        // 块编辑器容器
        this.blockContainer = document.createElement('div');
        this.blockContainer.className = 'editor-mode-container block-mode-container';
        this.container.appendChild(this.blockContainer);
    }

    /**
     * 切换到文本模式
     */
    async showTextMode() {
        this.currentMode = 'text';

        // 如果文本编辑器未初始化，先初始化
        if (!this.textEditor) {
            window.rLog('初始化文本编辑器...');

            // 保存块编辑器内容（确保最新）
            if (this.blockEditor && this.blockEditor.isDirtyState()) {
                await this.blockEditor.save();
            }

            this.textEditor = new window.CodeJarAdapter(this.textContainer, this.filePath);
            await this.textEditor.init();

            // 转发事件
            this.textEditor.on('dirty-changed', (data) => {
                this.emit('dirty-changed', data);
            });
        } else {
            // 如果块模式有修改，需要重新加载文本编辑器
            if (this.blockEditor && this.blockEditor.isDirtyState()) {
                await this.blockEditor.save();

                // 销毁并重新创建文本编辑器
                this.textEditor.destroy();
                this.textEditor = new window.CodeJarAdapter(this.textContainer, this.filePath);
                await this.textEditor.init();

                this.textEditor.on('dirty-changed', (data) => {
                    this.emit('dirty-changed', data);
                });
            }
        }

        this.blockContainer.style.display = 'none';
        this.textContainer.style.display = 'block';
        window.StatusBarModule?.updateEditorMode('text', 'idle');
        window.rLog('切换到文本模式');
    }

    /**
     * 切换到块模式
     */
    async showBlockMode() {
        this.currentMode = 'block';

        // 如果块编辑器未初始化，先初始化
        if (!this.blockEditor) {
            window.rLog('初始化块编辑器...');

            // 保存文本编辑器内容（确保最新）
            if (this.textEditor && this.textEditor.isDirtyState()) {
                await this.textEditor.save();
            }

            this.blockEditor = new window.BlockModeEditor(this.blockContainer, this.filePath);
            await this.blockEditor.init();

            // 转发事件
            this.blockEditor.on('dirty-changed', (data) => {
                this.emit('dirty-changed', data);
            });
        } else {
            // 如果文本模式有修改，需要重新加载块编辑器
            if (this.textEditor && this.textEditor.isDirtyState()) {
                await this.textEditor.save();

                // 销毁并重新创建块编辑器
                this.blockEditor.destroy();
                this.blockEditor = new window.BlockModeEditor(this.blockContainer, this.filePath);
                await this.blockEditor.init();

                this.blockEditor.on('dirty-changed', (data) => {
                    this.emit('dirty-changed', data);
                });
            }
        }

        this.textContainer.style.display = 'none';
        this.blockContainer.style.display = 'block';
        window.StatusBarModule?.updateEditorMode('block', 'idle');
        window.rLog('切换到块模式');
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
        const currentEditor = this.getCurrentEditor();
        if (currentEditor) {
            await currentEditor.save();

            // 如果在块模式保存，需要使文本模式的编辑器重新加载
            if (this.currentMode === 'block' && this.textEditor) {
                window.rLog('块模式保存后，文本编辑器需要重新加载');
                this.textEditor._needsReload = true;
            }
        }
    }

    /**
     * 获取内容
     */
    getContent() {
        const currentEditor = this.getCurrentEditor();
        return currentEditor?.getContent() || '';
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
        const currentEditor = this.getCurrentEditor();
        return currentEditor?.isDirtyState() || false;
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
        const currentEditor = this.getCurrentEditor();
        if (currentEditor && currentEditor.highlightExecutingLine) {
            currentEditor.highlightExecutingLine(lineNumber);
        }
    }

    /**
     * 高亮错误行
     */
    highlightErrorLine(lineNumber) {
        const currentEditor = this.getCurrentEditor();
        if (currentEditor && currentEditor.highlightErrorLine) {
            currentEditor.highlightErrorLine(lineNumber);
        }
    }

    /**
     * 设置测试运行状态
     */
    setTestRunning(isRunning, clearHighlight) {
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
