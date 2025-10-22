// 编辑器代理模块 - 提供对活动编辑器的快捷访问
const EditorProxy = {
    /**
     * 获取指定标签的编辑器
     * @param {string} tabId - 标签ID
     * @returns {EditorTab|undefined} 编辑器实例
     */
    getEditor(tabId) {
        return this.editors.get(tabId);
    },

    /**
     * 获取当前活动编辑器
     * @returns {EditorTab|null} 编辑器实例
     */
    getActiveEditor() {
        return this.activeTabId ? this.editors.get(this.activeTabId) : null;
    },

    /**
     * 设置测试运行状态
     * @param {boolean} isRunning - 是否正在运行
     * @param {boolean} clearHighlight - 是否清除高亮
     */
    setTestRunning(isRunning, clearHighlight = false) {
        const activeEditor = this.getActiveEditor();
        window.rLog('EditorManager.setTestRunning 调用:', {
            isRunning,
            clearHighlight,
            activeTabId: this.activeTabId,
            hasActiveEditor: !!activeEditor,
            activeEditorType: activeEditor ? activeEditor.constructor.name : 'null',
            hasSetTestRunning: activeEditor ? typeof activeEditor.setTestRunning : 'no editor'
        });

        if (activeEditor) {
            if (typeof activeEditor.setTestRunning === 'function') {
                activeEditor.setTestRunning(isRunning, clearHighlight);
            } else {
                window.rError('activeEditor 没有 setTestRunning 方法!', {
                    editorMethods: Object.getOwnPropertyNames(Object.getPrototypeOf(activeEditor))
                });
            }
        } else {
            window.rWarn('没有活动的编辑器');
        }
    },

    /**
     * 高亮执行行
     * @param {number} lineNumber - 行号
     */
    highlightExecutingLine(lineNumber) {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.highlightExecutingLine(lineNumber);
        }
    },

    /**
     * 高亮错误行
     * @param {number} lineNumber - 行号
     */
    highlightErrorLine(lineNumber) {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.highlightErrorLine(lineNumber);
        }
    },

    /**
     * 清除执行高亮
     */
    clearExecutionHighlight() {
        const activeEditor = this.getActiveEditor();
        if (activeEditor) {
            activeEditor.clearExecutionHighlight();
        }
    },

    /**
     * 更新字体设置
     * @param {string} fontFamily - 字体系列
     * @param {number} fontSize - 字体大小
     */
    updateFontSettings(fontFamily, fontSize) {
        this.editors.forEach(editor => {
            if (editor.updateFontSettings) {
                editor.updateFontSettings(fontFamily, fontSize);
            }
        });
    },

    /**
     * 刷新图片定位器
     */
    refreshImageLocators() {
        const editor = this.getActiveEditor();
        if (editor && editor.refreshImageLocators) {
            editor.refreshImageLocators();
        }
    },

    /**
     * 聚焦当前编辑器
     */
    focus() {
        const editor = this.getActiveEditor();
        if (editor) editor.focus();
    },

    // ============ 兼容性属性 - 支持AppGlobals.codeEditor的使用方式 ============

    /**
     * 获取编辑器内容
     * @returns {string} 内容
     */
    get value() {
        const editor = this.getActiveEditor();
        return editor ? editor.getValue() : '';
    },

    /**
     * 设置编辑器内容
     * @param {string} val - 内容
     */
    set value(val) {
        const editor = this.getActiveEditor();
        if (editor) editor.setValue(val);
    },

    /**
     * 设置占位符
     * @param {string} text - 占位符文本
     */
    set placeholder(text) {
        const editor = this.getActiveEditor();
        if (editor) editor.setPlaceholder(text);
    },

    /**
     * 获取内容元素（用于blur等操作）
     * @returns {HTMLElement|null} 内容元素
     */
    get contentEl() {
        const editor = this.getActiveEditor();
        return editor ? editor.textContentEl : null;
    }
};

// 导出到全局
window.EditorProxy = EditorProxy;

if (window.rLog) {
    window.rLog('✅ EditorProxy 模块已加载');
}
