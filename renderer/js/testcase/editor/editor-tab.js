// 编辑器标签页 - 核心类
// 所有功能通过模块混入

class EditorTab {
    constructor(container, editorManager) {
        this.container = container;
        this.editorManager = editorManager; // 保存管理器引用
        this.currentMode = editorManager ? editorManager.getGlobalEditMode() : 'block'; // 从管理器读取模式
        this.buffer = null; // 基于TKE的编辑器缓冲区 - 唯一的数据源
        this.listeners = [];
        this.saveTimeout = null;
        this.isTestRunning = false;
        this.currentHighlightedLine = null; // 跟踪当前高亮的行号
        this.uniqueId = `editor-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`; // 生成唯一ID

        // 确保模块方法已混合
        this.ensureModulesMixed();

        this.init();

        // 检查混入的方法是否存在
        window.rLog('EditorTab 实例创建完成，检查混入方法:', {
            hasSetTestRunning: typeof this.setTestRunning === 'function',
            hasHighlightExecutingLine: typeof this.highlightExecutingLine === 'function',
            hasHighlightErrorLine: typeof this.highlightErrorLine === 'function',
            hasClearExecutionHighlight: typeof this.clearExecutionHighlight === 'function',
            uniqueId: this.uniqueId
        });

        // 如果仍然缺失方法，最后尝试
        if (typeof this.setTestRunning !== 'function') {
            window.rError('❌ EditorTab 实例仍然缺少 setTestRunning 方法！');
        } else {
            window.rLog('✅ EditorTab 实例成功获得 setTestRunning 方法');
        }
    }

    /**
     * 确保所有模块方法都已混合到原型中
     */
    ensureModulesMixed() {
        // 检查关键方法是否存在
        if (!this.setTestRunning && typeof window.mixinEditorModules === 'function') {
            if (window.rLog) window.rLog('🔧 EditorTab实例化时检测到setTestRunning方法缺失，尝试混合模块...');
            window.mixinEditorModules();
        }

        // 再次确认混入是否成功
        if (!this.setTestRunning) {
            window.rError('❌ 混入失败！setTestRunning 方法仍然不存在');
            window.rError('当前原型方法:', Object.getOwnPropertyNames(Object.getPrototypeOf(this)));

            // 强制重新混入
            if (window.EditorHighlighting) {
                window.rLog('🔧 强制重新混入 EditorHighlighting 方法');
                // 直接复制所有方法到实例
                for (let key in window.EditorHighlighting) {
                    if (typeof window.EditorHighlighting[key] === 'function') {
                        this[key] = window.EditorHighlighting[key].bind(this);
                        window.rLog(`  ✓ 混入方法: ${key}`);
                    }
                }
                window.rLog('✅ EditorHighlighting 方法混入成功');
            } else {
                window.rError('❌ window.EditorHighlighting 不存在！');
            }
        }
    }

    /**
     * 初始化编辑器
     */
    init() {
        window.rLog('EditorTab 初始化中...');
        this.createEditor();
        this.setupEventListeners();
        // 显示初始占位界面
        this.renderPlaceholder();
        window.rLog('EditorTab 初始化完成，当前模式:', this.currentMode);
    }

    /**
     * 创建编辑器DOM结构
     */
    createEditor() {
        const containerId = `${this.uniqueId}-container`;
        this.container.innerHTML = `
            <div class="unified-editor">
                <!-- 统一的内容容器 -->
                <div class="editor-content-container" id="${containerId}">
                    <!-- 内容将根据模式动态渲染 -->
                </div>
            </div>
        `;

        this.editorContainer = this.container.querySelector(`#${containerId}`);
    }

    /**
     * 设置事件监听器
     */
    setupEventListeners() {
        // 不再在这里监听全局快捷键，由 EditorManager 统一处理
        // 只处理编辑器内部的事件
    }

    /**
     * 公共API - 设置文件路径并加载内容
     * @param {string} filePath - 文件路径
     */
    async setFile(filePath) {
        try {
            // 创建TKE缓冲区用于文件操作
            this.buffer = new window.TKEEditorBuffer(filePath);

            // 加载文件内容
            await this.buffer.loadFromFile();

            // 监听缓冲区内容变化事件
            this.buffer.on('content-changed', (event) => {
                window.rLog('📝 TKEEditorBuffer内容变化:', event.source);

                // 重新渲染编辑器
                this.render();

                // 触发变化事件
                this.triggerChange();
            });

            // 渲染编辑器
            this.render();

            window.rLog(`📁 EditorTab文件设置完成: ${filePath}`);
        } catch (error) {
            const errorMsg = error?.message || error?.toString() || JSON.stringify(error) || '未知错误';
            window.rError(`❌ EditorTab设置文件失败: ${errorMsg}`, error);
            throw error; // 重新抛出错误供上层处理
        }
    }

    /**
     * 获取编辑器内容
     * @returns {string} 内容
     */
    getValue() {
        // 从ScriptModel获取TKS代码
        const content = this.getTKSCode();
        window.rLog(`📖 从ScriptModel获取内容长度: ${content.length}`);
        return content;
    }

    /**
     * 设置占位符
     * @param {string} text - 占位符文本
     */
    setPlaceholder(text) {
        // 实现占位符逻辑
    }

    /**
     * 在光标位置插入文本
     * @param {string} text - 要插入的文本
     */
    insertText(text) {
        if (this.currentMode === 'text' && this.textContentEl) {
            // 在光标位置插入文本
            const selection = window.getSelection();
            if (selection.rangeCount > 0) {
                const range = selection.getRangeAt(0);
                const textNode = document.createTextNode(text);
                range.insertNode(textNode);

                // 移动光标到插入文本的末尾
                range.setStartAfter(textNode);
                range.setEndAfter(textNode);
                selection.removeAllRanges();
                selection.addRange(range);

                // 更新脚本模型
                const tksCode = this.textContentEl.textContent || '';
                // ScriptModel 已移除，直接使用 TKEEditorBuffer
                this.updateLineNumbers();
                this.triggerChange();
            } else {
                // 如果没有选区，追加到末尾
                this.textContentEl.innerText += text;
                // ScriptModel 已移除，直接使用 TKEEditorBuffer
                this.updateLineNumbers();
                this.triggerChange();
            }
        }
    }

    /**
     * 聚焦编辑器
     */
    focus() {
        if (this.currentMode === 'text' && this.textContentEl) {
            this.textContentEl.focus();
        }
    }

    /**
     * 添加事件监听器
     * @param {string} event - 事件类型
     * @param {Function} callback - 回调函数
     */
    on(event, callback) {
        this.listeners.push({ type: event, callback });
    }

    /**
     * 触发内容变化事件
     */
    triggerChange() {
        window.rLog(`📤 triggerChange 被调用，模式: ${this.currentMode}`);

        // 获取当前内容
        const content = this.getTKSCode();

        // 通知监听器
        this.listeners.forEach(listener => {
            if (listener.type === 'change') {
                listener.callback(content);
            }
        });

        // 异步保存到文件
        if (this.buffer) {
            this.buffer.updateFromText(content).catch(error => {
                window.rError(`❌ 保存文件失败: ${error.message}`);
            });
        }

        // 自动保存
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            if (window.EditorManager && window.EditorManager.saveCurrentFile) {
                window.EditorManager.saveCurrentFile();
            }
        }, 1000);
    }

    /**
     * 销毁编辑器
     */
    destroy() {
        clearTimeout(this.saveTimeout);
        // 不再需要移除全局快捷键监听器，因为它在 EditorManager 中
        this.removeStatusIndicator();
        this.hideCommandMenu();
        this.hideContextMenu();

        // 清理拖拽监听器
        if (this.dragOverHandler) {
            this.blocksContainer.removeEventListener('dragover', this.dragOverHandler);
        }
        if (this.dragLeaveHandler) {
            this.blocksContainer.removeEventListener('dragleave', this.dragLeaveHandler);
        }
        if (this.dropHandler) {
            this.blocksContainer.removeEventListener('drop', this.dropHandler);
        }

        this.listeners = [];
    }
}

// 导出到全局
window.EditorTab = EditorTab;

// 将拆分的模块方法混入到EditorTab原型中
// 必须在类导出后立即混入，以确保方法可用
function mixinEditorModules() {
    // 使用安全的日志记录方式
    if (window.rLog) {
        window.rLog('🔧 开始混入EditorTab模块，可用模块:', {
            BlockDefinitions: !!window.BlockDefinitions,
            CommandUtils: !!window.CommandUtils,
            CommandParser: !!window.CommandParser,
            CommandOperations: !!window.CommandOperations,
            BlockUIBuilder: !!window.BlockUIBuilder,
            BlockUIMenus: !!window.BlockUIMenus,
            BlockUIDrag: !!window.BlockUIDrag,
            BlockInputHandler: !!window.BlockInputHandler,
            TextInputHandler: !!window.TextInputHandler,
            EditorModeSwitcher: !!window.EditorModeSwitcher,
            EditorRenderer: !!window.EditorRenderer,
            EditorHighlighting: !!window.EditorHighlighting,
            EditorLineMapping: !!window.EditorLineMapping,
            EditorFontSettings: !!window.EditorFontSettings,
            EditorDragDrop: !!window.EditorDragDrop
        });
    }

    // 混入所有模块
    const modules = [
        'CommandUtils',
        'CommandParser',
        'CommandOperations',
        'BlockUIBuilder',
        'BlockUIMenus',
        'BlockUIDrag',
        'BlockInputHandler',
        'TextInputHandler',
        'EditorModeSwitcher',
        'EditorRenderer',
        'EditorHighlighting',
        'EditorLineMapping',
        'EditorFontSettings',
        'EditorDragDrop'
    ];

    modules.forEach(moduleName => {
        if (window[moduleName]) {
            Object.assign(EditorTab.prototype, window[moduleName]);
            if (window.rLog) {
                window.rLog(`✅ ${moduleName} 模块已混入`);
            }
        } else {
            if (window.rWarn) {
                window.rWarn(`⚠️  ${moduleName} 模块未找到`);
            }
        }
    });

    if (window.rLog) {
        window.rLog('✅ 所有EditorTab模块混入完成');
    }
}

// 将混合函数暴露到全局，以便实例化时调用
window.mixinEditorModules = mixinEditorModules;

// 立即尝试混合模块
mixinEditorModules();

// 如果第一次混合失败，使用 setTimeout 延迟混合
if (!EditorTab.prototype.setupLocatorInputDragDrop) {
    setTimeout(() => {
        if (window.rLog) window.rLog('🔧 延迟混合执行...');
        mixinEditorModules();
    }, 0);
}

if (window.rLog) {
    window.rLog('✅ EditorTab 核心类已导出到全局');
}
