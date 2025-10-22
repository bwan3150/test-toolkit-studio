// 编辑器渲染器 - 负责渲染编辑器UI

const EditorRenderer = {
    /**
     * 渲染编辑器（根据当前模式）
     */
    render() {
        if (this.currentMode === 'text') {
            this.renderTextMode();
        } else {
            this.renderBlockMode();
        }
    },

    /**
     * 渲染文本模式
     */
    renderTextMode() {
        window.rLog('渲染文本模式...');
        const tksCode = this.buffer ? this.buffer.getRawContent() : '';

        const lineNumbersId = `${this.uniqueId}-lines`;
        const textContentId = `${this.uniqueId}-text`;

        this.editorContainer.innerHTML = `
            <div class="text-editor-view">
                <div class="text-editor-wrapper">
                    <div class="line-numbers" id="${lineNumbersId}"></div>
                    <div class="text-content" id="${textContentId}" contenteditable="true">${this.highlightTKSSyntax(tksCode)}</div>
                </div>
            </div>
        `;

        // 在编辑器范围内创建状态指示器
        const editorContainer = this.container;
        if (editorContainer) {
            // 移除旧的内联状态指示器（如果存在）
            const oldIndicators = this.editorContainer.querySelectorAll('.editor-status-indicator');
            oldIndicators.forEach(indicator => indicator.remove());

            // 更新状态指示器
            this.updateStatusIndicator();
        } else {
            window.rError('找不到编辑器容器');
        }

        // 确保DOM元素在下一个事件循环中被查询，以便HTML完全渲染
        setTimeout(() => {
            this.textContentEl = this.editorContainer.querySelector('.text-content');
            this.lineNumbersEl = this.editorContainer.querySelector('.line-numbers');

            window.rLog('文本模式DOM元素:', {
                textContentEl: this.textContentEl,
                lineNumbersEl: this.lineNumbersEl,
                statusIndicatorEl: this.statusIndicatorEl,
                textContentElExists: !!this.textContentEl,
                lineNumbersElExists: !!this.lineNumbersEl
            });

            // 在DOM元素获取后才设置监听器和更新行号
            if (this.textContentEl && this.lineNumbersEl) {
                this.setupTextModeListeners();
                this.updateLineNumbers();
            } else {
                window.rError('DOM元素获取失败:', {
                    textContentEl: !!this.textContentEl,
                    lineNumbersEl: !!this.lineNumbersEl,
                    containerHTML: this.editorContainer.innerHTML.substring(0, 200)
                });
            }
        }, 0);

        // 更新状态指示器
        this.updateStatusIndicator();
    },

    /**
     * 渲染块模式
     */
    renderBlockMode() {
        const blocksContainerId = `${this.uniqueId}-blocks`;

        this.editorContainer.innerHTML = `
            <div class="block-editor-view">
                <div class="blocks-workspace">
                    <div class="blocks-container" id="${blocksContainerId}"></div>
                </div>
            </div>
        `;

        // 在块模式下，更新状态指示器
        this.updateStatusIndicator();

        this.blocksContainer = this.editorContainer.querySelector('.blocks-container');

        window.rLog('块编辑器DOM元素:', {
            editorContainer: this.editorContainer,
            blocksContainer: this.blocksContainer
        });

        if (this.blocksContainer) {
            this.renderBlocks();
            this.setupBlockModeListeners();
        } else {
            window.rError('无法找到块编辑器DOM元素');
        }
    },

    /**
     * 渲染占位符
     */
    renderPlaceholder() {
        if (this.editorContainer) {
            this.editorContainer.innerHTML = `
                <div class="editor-loading-placeholder" style="
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 100%;
                    color: #666;
                    font-size: 14px;
                ">
                    正在加载文件...
                </div>
            `;
        }
    },

    /**
     * 更新行号显示
     */
    updateLineNumbers() {
        if (!this.lineNumbersEl || !this.textContentEl) return;

        const text = window.EditorCursor.getPlainText(this.textContentEl);
        const lines = text.split('\n');
        const lineNumbersHtml = lines.map((_, index) =>
            `<div class="line-number">${index + 1}</div>`
        ).join('');
        this.lineNumbersEl.innerHTML = lineNumbersHtml;
    },

    /**
     * 高亮TKS语法
     * @param {string} text - TKS代码
     * @returns {string} 高亮后的HTML
     */
    highlightTKSSyntax(text) {
        // 使用 TKSSyntaxHighlighter 进行语法高亮
        return window.TKSSyntaxHighlighter.highlight(text);
    },

    /**
     * 更新状态指示器
     */
    updateStatusIndicator() {
        window.rLog('更新状态指示器 - 运行状态:', this.isTestRunning, '当前模式:', this.currentMode);

        const statusBar = document.querySelector('.status-bar');
        const modeText = document.getElementById('editorModeText');

        if (!statusBar || !modeText) {
            window.rError('找不到状态栏或模式文本元素');
            return;
        }

        // 清除所有模式类
        statusBar.classList.remove('status-bar-text-mode', 'status-bar-running');

        if (this.isTestRunning) {
            // 运行中状态
            modeText.textContent = '运行中';
            statusBar.classList.add('status-bar-running');
        } else if (this.currentMode === 'text') {
            // 文本编辑模式
            modeText.textContent = '文本编辑';
            statusBar.classList.add('status-bar-text-mode');
        } else {
            // 普通模式（块模式）
            modeText.textContent = '';
            // 不添加任何类，保持默认样式
        }

        window.rLog('状态栏已更新:', modeText.textContent || '普通模式');
    },

    /**
     * 创建运行指示器
     */
    createRunningIndicator() {
        // 在块模式下运行时，更新状态栏
        this.updateStatusIndicator();
        window.rLog('块模式运行指示器已更新到状态栏');
    },

    /**
     * 移除状态指示器
     */
    removeStatusIndicator() {
        // 清理状态栏状态
        const statusBar = document.querySelector('.status-bar');
        const modeText = document.getElementById('editorModeText');

        if (statusBar) {
            statusBar.classList.remove('status-bar-text-mode', 'status-bar-running');
        }

        if (modeText) {
            modeText.textContent = '';
        }

        // 清理可能存在的所有旧的内联状态指示器（用于兼容）
        const indicators = document.querySelectorAll('.editor-status-indicator');
        indicators.forEach(indicator => indicator.remove());

        // 从编辑器视图中移除内联指示器
        if (this.editorContainer) {
            const textEditorView = this.editorContainer.querySelector('.text-editor-view');
            if (textEditorView) {
                const indicator = textEditorView.querySelector('.editor-status-indicator');
                if (indicator) {
                    indicator.remove();
                }
            }
        }
    }
};

// 导出到全局
window.EditorRenderer = EditorRenderer;

if (window.rLog) {
    window.rLog('✅ EditorRenderer 模块已加载');
}
