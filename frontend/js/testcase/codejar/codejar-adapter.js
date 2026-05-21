/**
 * CodeJar 编辑器适配器
 * 封装 CodeJar 库，提供 TKS 语法高亮支持
 */
(window.rLog || console.log)('🔵 codejar-adapter.js 开始加载');

class CodeJarAdapter {
    constructor(container, filePath) {
        this.container = container;
        this.filePath = filePath;
        this.jar = null;
        this.isDirty = false;
        this.originalContent = '';
        this.eventHandlers = new Map();
        this.highlighter = null; // 执行高亮器
        this.editorDiv = null; // CodeJar 编辑器 div
        this.lineNumberController = null; // 行号控制器

        window.rLog(`📝 CodeJarAdapter 创建: ${filePath}`);
    }

    /**
     * 初始化编辑器
     */
    async init() {
        window.rLog('🔧 初始化 CodeJar 编辑器...');

        // 1. 从文件加载内容
        await this.loadFromFile();

        // 2. 创建编辑器容器
        this.editorDiv = document.createElement('div');
        this.editorDiv.className = 'codejar-editor';
        this.container.appendChild(this.editorDiv);

        // 3. 初始化 CodeJar，使用 TKS 语法高亮
        this.jar = window.CodeJar(this.editorDiv, this.highlight.bind(this), {
            tab: '    ', // 4个空格
            indentOn: /[:{\[]$/,
            spellcheck: false,
            catchTab: true,
            preserveIdent: true,
            addClosing: true,
            history: true
        });

        // 4. 创建执行高亮器
        window.rLog('检查 ExecutionHighlighter:', {
            exists: !!window.ExecutionHighlighter,
            type: typeof window.ExecutionHighlighter,
            isConstructor: window.ExecutionHighlighter && typeof window.ExecutionHighlighter === 'function'
        });

        if (window.ExecutionHighlighter && typeof window.ExecutionHighlighter === 'function') {
            try {
                this.highlighter = new window.ExecutionHighlighter(this.editorDiv);
                window.rLog('✅ ExecutionHighlighter 创建成功');
            } catch (error) {
                window.rError('❌ ExecutionHighlighter 创建失败:', error);
            }
        } else {
            window.rError('❌ ExecutionHighlighter 未正确加载:', typeof window.ExecutionHighlighter);
        }

        // 5. 创建行号控制器
        if (window.LineNumberController && typeof window.LineNumberController === 'function') {
            try {
                this.lineNumberController = new window.LineNumberController(
                    this.editorDiv,
                    this.handleLineExecute.bind(this)
                );
                window.rLog('✅ LineNumberController 创建成功');
            } catch (error) {
                window.rError('❌ LineNumberController 创建失败:', error);
            }
        } else {
            window.rError('❌ LineNumberController 未正确加载');
        }

        // 6. 设置初始内容
        this.jar.updateCode(this.originalContent);

        // 7. 监听内容变化
        this.jar.onUpdate(code => {
            this.onContentChange(code);
        });

        window.rLog('✅ CodeJar 编辑器初始化完成');
    }

    /**
     * 从文件加载内容
     */
    async loadFromFile() {
        try {
            const { fs } = window.AppGlobals;
            const content = await fs.readFile(this.filePath, 'utf-8');
            this.originalContent = content;
            window.rLog(`✅ 文件加载成功: ${this.filePath}`);
        } catch (error) {
            window.rError('文件加载失败:', error);
            this.originalContent = '';
            throw error;
        }
    }

    /**
     * 内容变化处理
     */
    onContentChange(code) {
        const wasDirty = this.isDirty;
        this.isDirty = (code !== this.originalContent);

        // 如果 dirty 状态变化，触发事件
        if (wasDirty !== this.isDirty) {
            this.emit('dirty-changed', { isDirty: this.isDirty });
        }
    }

    /**
     * 语法高亮函数
     */
    highlight(editor) {
        const code = editor.textContent;

        // 使用 TKS 语法高亮器
        if (window.TKSSyntaxHighlighter) {
            const html = window.TKSSyntaxHighlighter.highlight(code);
            editor.innerHTML = html;
        } else {
            // 降级：只做 HTML 转义
            editor.innerHTML = this.escapeHTML(code);
        }
    }

    /**
     * HTML 转义
     */
    escapeHTML(text) {
        const map = {
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#039;'
        };
        return text.replace(/[&<>"']/g, m => map[m]);
    }

    /**
     * 保存文件
     */
    async save() {
        try {
            window.rLog('💾 保存文件...');
            const { fs } = window.AppGlobals;
            const content = this.jar.toString();
            await fs.writeFile(this.filePath, content, 'utf-8');

            this.originalContent = content;
            this.isDirty = false;
            this.emit('dirty-changed', { isDirty: false });

            window.rLog(`✅ 文件保存成功: ${this.filePath}`);
        } catch (error) {
            window.rError('保存文件失败:', error);
            throw error;
        }
    }

    /**
     * 获取内容
     */
    getContent() {
        return this.jar ? this.jar.toString() : '';
    }

    /**
     * 更新内容（用于块编辑器同步修改）
     */
    updateContent(newContent) {
        if (this.jar) {
            this.jar.updateCode(newContent);
            window.rLog('📝 CodeJar内容已更新');
        }
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
        return this.isDirty;
    }

    /**
     * 聚焦编辑器
     */
    focus() {
        if (this.editorDiv) {
            this.editorDiv.focus();
        }
    }

    /**
     * 锁定编辑器（禁止编辑）
     */
    lock() {
        if (this.editorDiv) {
            this.editorDiv.contentEditable = 'false';
            this.editorDiv.style.opacity = '0.6';
            this.editorDiv.style.cursor = 'not-allowed';
            window.rLog('🔒 编辑器已锁定');
        }
    }

    /**
     * 解锁编辑器（允许编辑）
     */
    unlock() {
        if (this.editorDiv) {
            this.editorDiv.contentEditable = 'true';
            this.editorDiv.style.opacity = '1';
            this.editorDiv.style.cursor = 'text';
            window.rLog('🔓 编辑器已解锁');
        }
    }

    /**
     * 高亮正在执行的行
     * @param {number} lineNumber - 行号（1-based）
     */
    highlightExecutingLine(lineNumber) {
        if (this.highlighter) {
            this.highlighter.highlightExecutingLine(lineNumber);
        }
    }

    /**
     * 高亮错误行
     * @param {number} lineNumber - 行号（1-based）
     */
    highlightErrorLine(lineNumber) {
        if (this.highlighter) {
            this.highlighter.highlightErrorLine(lineNumber);
        }
    }

    /**
     * 设置测试运行状态
     * @param {boolean} isRunning - 是否正在运行
     * @param {boolean} clearHighlight - 是否清除高亮
     */
    setTestRunning(isRunning, clearHighlight) {
        if (this.highlighter) {
            this.highlighter.setTestRunning(isRunning, clearHighlight);
        }
    }

    /**
     * 处理单行执行
     * @param {number} lineNumber - 行号（1-based）
     * @param {string} lineContent - 行内容
     */
    handleLineExecute(lineNumber, lineContent) {
        window.rLog(`📍 CodeJarAdapter 收到单行执行请求: 行${lineNumber}`);

        // 调用单行执行器
        if (window.SingleLineRunner) {
            window.SingleLineRunner.executeLine(lineNumber, lineContent);
        } else {
            window.rError('SingleLineRunner 未加载');
            window.AppNotifications?.error('单行执行功能不可用');
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
        window.rLog('🗑️  销毁 CodeJar 编辑器');

        if (this.lineNumberController) {
            this.lineNumberController.destroy();
            this.lineNumberController = null;
        }

        if (this.highlighter) {
            this.highlighter.destroy();
            this.highlighter = null;
        }

        if (this.jar) {
            this.jar.destroy();
            this.jar = null;
        }

        this.container.innerHTML = '';
        this.eventHandlers.clear();
        this.editorDiv = null;
    }
}

// 导出到全局
window.CodeJarAdapter = CodeJarAdapter;
(window.rLog || console.log)('✅ CodeJarAdapter 模块已加载');
