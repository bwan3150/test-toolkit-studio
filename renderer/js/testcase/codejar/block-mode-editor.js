/**
 * 块模式编辑器
 * 基于CodeJar接口，但使用旧的块编辑器UI和交互逻辑
 * 只显示"步骤:"下面的命令块，不显示元数据
 * @version 1.0.0
 */
(window.rLog || console.log)('block-mode-editor.js 开始加载');

class BlockModeEditor {
    constructor(container, filePath) {
        this.container = container;
        this.filePath = filePath;
        this.isDirty = false;
        this.originalContent = '';
        this.eventHandlers = new Map();

        // 编辑器状态
        this.commands = []; // 命令数组
        this.headerLines = []; // 文件头部内容（用例、脚本名、详情等）

        // DOM元素
        this.blocksContainer = null;

        window.rLog(`BlockModeEditor 创建: ${filePath}`);
    }

    /**
     * 初始化编辑器
     */
    async init() {
        window.rLog('初始化块模式编辑器...');

        // 1. 从文件加载内容
        await this.loadFromFile();

        // 2. 解析脚本
        this.parseScript();

        // 3. 创建UI
        this.createUI();

        // 4. 渲染块
        this.renderBlocks();

        // 5. 设置事件监听
        this.setupBlockModeListeners();

        window.rLog('块模式编辑器初始化完成');
    }

    /**
     * 从文件加载内容
     */
    async loadFromFile() {
        try {
            const { fs } = window.AppGlobals;
            const content = await fs.readFile(this.filePath, 'utf-8');
            this.originalContent = content;
            window.rLog(`文件加载成功: ${this.filePath}`);
        } catch (error) {
            window.rError('文件加载失败:', error);
            this.originalContent = '';
            throw error;
        }
    }

    /**
     * 解析脚本
     */
    parseScript() {
        const lines = this.originalContent.split('\n');
        this.commands = [];
        this.headerLines = [];

        let inStepsSection = false;

        lines.forEach(line => {
            const trimmed = line.trim();

            if (trimmed === '步骤:') {
                inStepsSection = true;
                // 保存头部（包括"步骤:"这行）
                this.headerLines.push(line);
                return;
            }

            if (!inStepsSection) {
                // 保存头部内容
                this.headerLines.push(line);
                return;
            }

            // 在步骤部分
            if (!trimmed || trimmed.startsWith('#')) {
                // 跳过空行和注释
                return;
            }

            // 解析命令
            const command = this.parseCommandLine(trimmed);
            if (command) {
                this.commands.push(command);
            }
        });

        window.rLog(`解析了 ${this.commands.length} 个命令`);
    }

    /**
     * 解析单行命令
     */
    parseCommandLine(line) {
        // 匹配命令模式: 命令 [参数1, 参数2, ...]
        const match = line.match(/^(\S+)\s+\[(.+)\]$/);
        if (!match) {
            window.rError('无法解析命令行:', line);
            return null;
        }

        const commandType = match[1];
        const paramsStr = match[2];

        // 查找命令定义
        const definition = window.CommandUtils?.findCommandDefinition(commandType);
        if (!definition) {
            window.rError('未知命令类型:', commandType);
            return null;
        }

        // 解析参数
        const paramValues = this.parseParams(paramsStr);

        // 构建命令对象
        const params = {};
        definition.params.forEach((param, index) => {
            params[param.name] = paramValues[index] || param.default || '';
        });

        return {
            type: definition.type,
            params: params
        };
    }

    /**
     * 解析参数字符串
     */
    parseParams(paramsStr) {
        const params = [];
        let current = '';
        let inBracket = 0;
        let inQuote = false;

        for (let i = 0; i < paramsStr.length; i++) {
            const char = paramsStr[i];

            if (char === '{') {
                inBracket++;
                current += char;
            } else if (char === '}') {
                inBracket--;
                current += char;
            } else if (char === '"') {
                inQuote = !inQuote;
                // 不包含引号本身
            } else if (char === ',' && inBracket === 0 && !inQuote) {
                params.push(current.trim());
                current = '';
            } else {
                current += char;
            }
        }

        if (current) {
            params.push(current.trim());
        }

        return params;
    }

    /**
     * 创建UI结构
     */
    createUI() {
        this.container.innerHTML = `
            <div class="unified-editor">
                <div class="editor-content-container">
                    <div class="blocks-container" id="blocksContainer">
                        <!-- 块将在这里渲染 -->
                    </div>
                </div>
            </div>
        `;

        this.blocksContainer = this.container.querySelector('#blocksContainer');
    }

    /**
     * 渲染所有命令块
     */
    renderBlocks() {
        if (!this.blocksContainer) return;

        // 使用 BlockUIBuilder 渲染
        if (window.BlockUIBuilder && window.BlockUIBuilder.renderBlocks) {
            // 临时设置上下文
            const originalContainer = window.BlockUIBuilder.blocksContainer;
            const originalGetCommands = window.BlockUIBuilder.getCommands;
            const originalRenderVisualElements = window.BlockUIBuilder.renderVisualElements;

            window.BlockUIBuilder.blocksContainer = this.blocksContainer;
            window.BlockUIBuilder.getCommands = () => this.commands;
            // 绑定 renderVisualElements 到 BlockUIBuilder
            window.BlockUIBuilder.renderVisualElements = originalRenderVisualElements.bind(window.BlockUIBuilder);

            window.BlockUIBuilder.renderBlocks.call(window.BlockUIBuilder);

            // 恢复
            window.BlockUIBuilder.blocksContainer = originalContainer;
            window.BlockUIBuilder.getCommands = originalGetCommands;
            window.BlockUIBuilder.renderVisualElements = originalRenderVisualElements;
        }
    }

    /**
     * 设置块模式事件监听
     */
    setupBlockModeListeners() {
        if (!this.blocksContainer) return;

        // 监听参数输入变化
        this.blocksContainer.querySelectorAll('.param-hole').forEach(input => {
            input.addEventListener('input', (e) => {
                const commandIndex = parseInt(e.target.dataset.commandIndex);
                const paramName = e.target.dataset.param;

                if (this.commands[commandIndex]) {
                    this.commands[commandIndex].params[paramName] = e.target.value;
                    this.triggerChange();
                }
            });
        });

        // 监听删除按钮
        this.blocksContainer.querySelectorAll('.block-delete').forEach(btn => {
            btn.addEventListener('click', (e) => {
                const index = parseInt(e.target.dataset.index);
                this.deleteCommand(index);
            });
        });

        // 设置拖拽功能
        if (window.BlockUIDrag && window.BlockUIDrag.setupDragAndDrop) {
            const originalContainer = window.BlockUIDrag.blocksContainer;
            const originalCommands = window.BlockUIDrag.getCommands;
            const originalRender = window.BlockUIDrag.renderBlocks;
            const originalTrigger = window.BlockUIDrag.triggerChange;

            window.BlockUIDrag.blocksContainer = this.blocksContainer;
            window.BlockUIDrag.getCommands = () => this.commands;
            window.BlockUIDrag.renderBlocks = () => {
                this.renderBlocks();
                this.setupBlockModeListeners();
            };
            window.BlockUIDrag.triggerChange = () => this.triggerChange();

            window.BlockUIDrag.setupDragAndDrop.call(this);

            // 恢复
            window.BlockUIDrag.blocksContainer = originalContainer;
            window.BlockUIDrag.getCommands = originalCommands;
            window.BlockUIDrag.renderBlocks = originalRender;
            window.BlockUIDrag.triggerChange = originalTrigger;
        }

        // 设置菜单功能（添加块）
        if (window.BlockUIMenus && window.BlockUIMenus.setupMenus) {
            const originalContainer = window.BlockUIMenus.blocksContainer;
            const originalCommands = window.BlockUIMenus.getCommands;
            const originalRender = window.BlockUIMenus.renderBlocks;
            const originalTrigger = window.BlockUIMenus.triggerChange;

            window.BlockUIMenus.blocksContainer = this.blocksContainer;
            window.BlockUIMenus.getCommands = () => this.commands;
            window.BlockUIMenus.renderBlocks = () => {
                this.renderBlocks();
                this.setupBlockModeListeners();
            };
            window.BlockUIMenus.triggerChange = () => this.triggerChange();

            window.BlockUIMenus.setupMenus.call(this);

            // 恢复
            window.BlockUIMenus.blocksContainer = originalContainer;
            window.BlockUIMenus.getCommands = originalCommands;
            window.BlockUIMenus.renderBlocks = originalRender;
            window.BlockUIMenus.triggerChange = originalTrigger;
        }
    }

    /**
     * 删除命令
     */
    deleteCommand(index) {
        if (index >= 0 && index < this.commands.length) {
            this.commands.splice(index, 1);
            this.renderBlocks();
            this.setupBlockModeListeners();
            this.triggerChange();
        }
    }

    /**
     * 获取命令数组（供模块使用）
     */
    getCommands() {
        return this.commands;
    }

    /**
     * 触发变化事件
     */
    triggerChange() {
        const wasDirty = this.isDirty;
        const currentContent = this.toString();
        this.isDirty = (currentContent !== this.originalContent);

        if (wasDirty !== this.isDirty) {
            this.emit('dirty-changed', { isDirty: this.isDirty });
        }
    }

    /**
     * 将命令数组转换为脚本文本
     */
    toString() {
        const lines = [...this.headerLines]; // 复制头部内容

        // 添加步骤
        this.commands.forEach(command => {
            const definition = window.CommandUtils?.findCommandDefinition(command.type);
            if (definition) {
                const tksCommand = definition.tksCommand || command.type;
                const paramValues = definition.params.map(p => command.params[p.name] || '').filter(v => v);
                const paramStr = paramValues.join(', ');
                lines.push(`    ${tksCommand} [${paramStr}]`);
            }
        });

        return lines.join('\n');
    }

    /**
     * 保存文件
     */
    async save() {
        try {
            window.rLog('保存文件...');
            const { fs } = window.AppGlobals;
            const content = this.toString();
            await fs.writeFile(this.filePath, content, 'utf-8');

            this.originalContent = content;
            this.isDirty = false;
            this.emit('dirty-changed', { isDirty: false });

            window.rLog(`文件保存成功: ${this.filePath}`);
        } catch (error) {
            window.rError('保存文件失败:', error);
            throw error;
        }
    }

    /**
     * 获取内容
     */
    getContent() {
        return this.toString();
    }

    /**
     * 获取原始内容（用于脚本执行）
     */
    getRawContent() {
        return this.toString();
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
        // 块编辑器不需要特别的焦点处理
    }

    /**
     * 锁定编辑器（禁止编辑）
     */
    lock() {
        if (this.blocksContainer) {
            this.blocksContainer.style.opacity = '0.6';
            this.blocksContainer.style.pointerEvents = 'none';
        }
        window.rLog('块编辑器已锁定');
    }

    /**
     * 解锁编辑器（允许编辑）
     */
    unlock() {
        if (this.blocksContainer) {
            this.blocksContainer.style.opacity = '1';
            this.blocksContainer.style.pointerEvents = 'auto';
        }
        window.rLog('块编辑器已解锁');
    }

    /**
     * 高亮正在执行的行
     */
    highlightExecutingLine(lineNumber) {
        // 计算块索引（需要跳过头部行数）
        const headerLineCount = this.headerLines.length;
        const blockIndex = lineNumber - headerLineCount - 1;

        if (blockIndex >= 0 && blockIndex < this.commands.length) {
            const blockEl = this.blocksContainer.querySelector(`[data-index="${blockIndex}"]`);
            if (blockEl) {
                // 移除之前的高亮
                this.blocksContainer.querySelectorAll('.executing-block').forEach(el => {
                    el.classList.remove('executing-block');
                });
                blockEl.classList.add('executing-block');
                blockEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
        }
    }

    /**
     * 高亮错误行
     */
    highlightErrorLine(lineNumber) {
        const headerLineCount = this.headerLines.length;
        const blockIndex = lineNumber - headerLineCount - 1;

        if (blockIndex >= 0 && blockIndex < this.commands.length) {
            const blockEl = this.blocksContainer.querySelector(`[data-index="${blockIndex}"]`);
            if (blockEl) {
                blockEl.classList.add('error-block');
            }
        }
    }

    /**
     * 设置测试运行状态
     */
    setTestRunning(isRunning, clearHighlight) {
        if (clearHighlight && this.blocksContainer) {
            this.blocksContainer.querySelectorAll('.executing-block, .error-block').forEach(el => {
                el.classList.remove('executing-block', 'error-block');
            });
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
        window.rLog('销毁块模式编辑器');
        this.container.innerHTML = '';
        this.eventHandlers.clear();
    }
}

// 导出到全局
window.BlockModeEditor = BlockModeEditor;
(window.rLog || console.log)('BlockModeEditor 模块已加载');
