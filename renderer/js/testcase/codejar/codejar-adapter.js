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
        const editorDiv = document.createElement('div');
        editorDiv.className = 'codejar-editor';
        this.container.appendChild(editorDiv);

        // 3. 初始化 CodeJar，使用 TKS 语法高亮
        this.jar = window.CodeJar(editorDiv, this.highlight.bind(this), {
            tab: '    ', // 4个空格
            indentOn: /[:{\[]$/,
            spellcheck: false,
            catchTab: true,
            preserveIdent: true,
            addClosing: true,
            history: true
        });

        // 4. 设置初始内容
        this.jar.updateCode(this.originalContent);

        // 5. 监听内容变化
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
     * 检查是否有未保存的修改
     */
    isDirtyState() {
        return this.isDirty;
    }

    /**
     * 聚焦编辑器
     */
    focus() {
        const editorDiv = this.container.querySelector('.codejar-editor');
        if (editorDiv) {
            editorDiv.focus();
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

        if (this.jar) {
            this.jar.destroy();
            this.jar = null;
        }

        this.container.innerHTML = '';
        this.eventHandlers.clear();
    }
}

// 导出到全局
window.CodeJarAdapter = CodeJarAdapter;
(window.rLog || console.log)('✅ CodeJarAdapter 模块已加载');
