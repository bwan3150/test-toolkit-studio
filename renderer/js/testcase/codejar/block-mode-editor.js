/**
 * 块模式编辑器
 * 基于CodeJar接口，但使用旧的块编辑器UI和交互逻辑
 * 只显示"步骤:"下面的命令块，不显示元数据
 * @version 1.0.0
 */
(window.rLog || console.log)('block-mode-editor.js 开始加载');

class BlockModeEditor {
    constructor(container, textEditor) {
        this.container = container;
        this.textEditor = textEditor; // CodeJarAdapter实例
        this.eventHandlers = new Map();

        // 编辑器状态
        this.commands = []; // 命令数组
        this.headerLines = []; // 文件头部内容（用例、脚本名、详情等）

        // DOM元素
        this.blocksContainer = null;

        window.rLog(`BlockModeEditor 创建，基于textEditor`);
    }

    /**
     * 初始化编辑器
     */
    async init() {
        window.rLog('初始化块模式编辑器...');

        // 1. 从textEditor读取内容
        this.loadFromTextEditor();

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
     * 从textEditor读取内容
     */
    loadFromTextEditor() {
        const content = this.textEditor.getContent();
        window.rLog(`从textEditor读取内容，长度: ${content.length}`);
        return content;
    }

    /**
     * 刷新块编辑器(从textEditor重新读取)
     */
    async refresh() {
        window.rLog('刷新块编辑器...');
        this.loadFromTextEditor();
        this.parseScript();
        this.renderBlocks();
        this.setupBlockModeListeners();
    }

    /**
     * 解析脚本
     */
    parseScript() {
        const content = this.loadFromTextEditor();
        const lines = content.split('\n');
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
        // 匹配命令模式: 命令 [参数1, 参数2, ...] 或 命令 []
        const match = line.match(/^(\S+)\s+\[(.*)\]$/);
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

        // 解析参数（允许空参数）
        const paramValues = paramsStr ? this.parseParams(paramsStr) : [];

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
            <div class="unified-editor" style="height: 100%;">
                <div class="editor-content-container" style="height: 100%;">
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

        // 如果没有命令，显示空状态页面
        if (this.commands.length === 0) {
            this.renderEmptyState();
            return;
        }

        // 有命令，渲染正式编辑页面
        let blocksHtml = '';

        this.commands.forEach((command, index) => {
            const definition = window.CommandUtils?.findCommandDefinition(command.type);
            const category = window.CommandUtils?.findCommandCategory(command.type);

            if (!definition || !category) return;

            // 创建命令内容
            let commandContent = `<span class="block-icon">${category.icon}</span><span class="command-label">${definition.label}</span>`;

            // 添加参数输入框
            definition.params.forEach(param => {
                const value = command.params[param.name] || param.default || '';
                const paramId = `param-${index}-${param.name}`;

                if (param.type === 'select') {
                    const optionsHtml = param.options.map(opt =>
                        `<option value="${opt}" ${value === opt ? 'selected' : ''}>${opt}</option>`
                    ).join('');
                    commandContent += `
                        <select class="param-hole" id="${paramId}" data-param="${param.name}" data-command-index="${index}">
                            ${optionsHtml}
                        </select>
                    `;
                } else if (param.type === 'element') {
                    // element 类型参数，检查是否已填入元素
                    const imageMatch = value.match(/^@\{(.+)\}$/);
                    const xmlMatch = value.match(/^\{(.+)\}$/);

                    if (imageMatch || xmlMatch) {
                        // 已填入元素，显示可视化卡片
                        const elementName = imageMatch ? imageMatch[1] : xmlMatch[1];
                        const isImage = !!imageMatch;

                        if (isImage) {
                            // 图片元素 - 显示图片预览
                            commandContent += `
                                <div class="param-visual-card" data-param="${param.name}" data-command-index="${index}">
                                    <svg width="16" height="16" viewBox="0 0 24 24" style="flex-shrink: 0;">
                                        <rect x="3" y="3" width="18" height="18" fill="#4a90e2" opacity="0.2" rx="2"/>
                                        <circle cx="8.5" cy="8.5" r="1.5" fill="#4a90e2"/>
                                        <path d="M3 17l4-4 3 3 6-6 5 5v3H3v-1z" fill="#4a90e2"/>
                                    </svg>
                                    <span class="visual-name">${elementName}</span>
                                    <button class="visual-remove-btn" data-param="${param.name}" data-command-index="${index}" title="移除">×</button>
                                </div>
                            `;
                        } else {
                            // XML元素 - 显示{}图标
                            commandContent += `
                                <div class="param-visual-card" data-param="${param.name}" data-command-index="${index}">
                                    <svg width="16" height="16" viewBox="0 0 24 24" style="flex-shrink: 0;">
                                        <path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3"/>
                                    </svg>
                                    <span class="visual-name">${elementName}</span>
                                    <button class="visual-remove-btn" data-param="${param.name}" data-command-index="${index}" title="移除">×</button>
                                </div>
                            `;
                        }
                    } else {
                        // 未填入元素，显示普通输入框
                        commandContent += `
                            <input class="param-hole" id="${paramId}" type="text"
                                   data-param="${param.name}" data-command-index="${index}"
                                   data-param-type="element"
                                   placeholder="${param.placeholder}" value="${value}">
                        `;
                    }
                } else {
                    // 其他类型参数
                    commandContent += `
                        <input class="param-hole" id="${paramId}" type="${param.type === 'number' ? 'number' : 'text'}"
                               data-param="${param.name}" data-command-index="${index}"
                               placeholder="${param.placeholder}" value="${value}">
                    `;
                }
            });

            // 生成块HTML
            blocksHtml += `
                <div class="workspace-block command-block" data-index="${index}" data-type="${command.type}" draggable="true"
                     style="background: linear-gradient(135deg, ${category.color}ee, ${category.color}cc);">
                    <div class="command-content">${commandContent}</div>
                    <button class="block-delete" data-index="${index}" title="删除">×</button>
                </div>
            `;
        });

        // 插入按钮
        const finalInsertButton = `
            <div class="block-insert-area final" data-insert-index="${this.commands.length}">
                <button class="block-insert-btn" title="添加命令块">
                    <svg width="16" height="16" viewBox="0 0 16 16">
                        <path fill="currentColor" d="M8 2v12m-6-6h12" stroke="currentColor" stroke-width="2"/>
                    </svg>
                </button>
            </div>
        `;

        this.blocksContainer.innerHTML = blocksHtml + finalInsertButton;

        window.rLog(`渲染完成，命令数: ${this.commands.length}`);
    }

    /**
     * 渲染空状态页面 - 专门用于添加第一个命令块
     */
    renderEmptyState() {
        this.blocksContainer.innerHTML = `
            <div style="position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                        display: flex; flex-direction: column; align-items: center; justify-content: center;
                        padding: 40px; box-sizing: border-box;">
                <div style="text-align: center; margin-bottom: 32px;">
                    <p style="font-size: 16px; color: var(--text-secondary); margin: 0;">点击下方 ⊕ 按钮添加脚本块</p>
                </div>
                <button class="block-insert-btn" id="addFirstBlockBtn" title="添加脚本块">
                    <svg width="16" height="16" viewBox="0 0 16 16">
                        <path fill="currentColor" d="M8 2v12m-6-6h12" stroke="currentColor" stroke-width="2"/>
                    </svg>
                </button>
            </div>
        `;

        // 绑定按钮点击事件
        const btn = this.blocksContainer.querySelector('#addFirstBlockBtn');
        if (btn) {
            btn.addEventListener('click', () => {
                this.showFirstCommandMenu();
            });
        }

        window.rLog('渲染空状态页面');
    }

    /**
     * 显示第一个命令选择菜单
     */
    showFirstCommandMenu() {
        window.rLog('显示第一个命令选择菜单');

        // 创建菜单
        const menuItems = [];
        Object.entries(window.BlockDefinitions || {}).forEach(([categoryKey, category]) => {
            category.commands.forEach(cmd => {
                menuItems.push(`
                    <div class="command-menu-item" data-type="${cmd.type}">
                        <span class="menu-item-icon">${category.icon}</span>
                        <span class="menu-item-label">${cmd.label}</span>
                    </div>
                `);
            });
        });

        const menuHtml = `
            <div id="firstCommandMenu" style="position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
                    background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 8px;
                    box-shadow: 0 8px 24px rgba(0,0,0,0.2); z-index: 2000; min-width: 250px; max-height: 400px; overflow-y: auto;">
                ${menuItems.join('')}
            </div>
        `;

        document.body.insertAdjacentHTML('beforeend', menuHtml);
        const menu = document.querySelector('#firstCommandMenu');

        // 绑定菜单项点击
        menu.addEventListener('click', (e) => {
            const item = e.target.closest('.command-menu-item');
            if (item) {
                const commandType = item.dataset.type;
                this.insertCommand(commandType, 0);
                menu.remove();
            }
        });

        // 点击外部关闭
        setTimeout(() => {
            document.addEventListener('click', (e) => {
                if (!menu.contains(e.target)) {
                    menu.remove();
                }
            }, { once: true });
        }, 0);
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

        // 监听可视化元素移除按钮
        this.blocksContainer.querySelectorAll('.visual-remove-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const commandIndex = parseInt(btn.dataset.commandIndex);
                const paramName = btn.dataset.param;

                if (this.commands[commandIndex]) {
                    this.commands[commandIndex].params[paramName] = '';
                    this.renderBlocks();
                    this.setupBlockModeListeners();
                    this.triggerChange();
                }
            });
        });

        // 设置拖拽排序（脚本块之间）
        this.setupDragAndDrop();

        // 设置元素拖拽到参数孔
        this.setupElementDrop();

        // 设置插入按钮菜单
        this.setupInsertMenus();
    }

    /**
     * 设置拖拽排序（仅处理脚本块之间的拖拽）
     */
    setupDragAndDrop() {
        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');

        blocks.forEach(block => {
            block.addEventListener('dragstart', (e) => {
                block.classList.add('dragging');
                e.dataTransfer.effectAllowed = 'move';
                // 设置专门的类型标识：脚本块拖拽
                e.dataTransfer.setData('application/x-script-block', block.dataset.index);
                window.rLog('开始拖拽脚本块:', block.dataset.index);
            });

            block.addEventListener('dragend', (e) => {
                block.classList.remove('dragging');
                this.clearDragInsertIndicator();
            });
        });

        // 容器拖拽事件 - 只处理脚本块拖拽
        this.blocksContainer.addEventListener('dragover', (e) => {
            // 检查是否是脚本块拖拽
            const types = e.dataTransfer.types;
            if (!types.includes('application/x-script-block')) {
                // 不是脚本块拖拽，不处理（可能是元素拖拽）
                return;
            }

            e.preventDefault();
            const draggingBlock = this.blocksContainer.querySelector('.dragging');
            if (!draggingBlock) return;

            const afterElement = this.getDragAfterElement(e.clientY);

            // 只显示插入提示线，不实际移动DOM（避免频繁重渲染导致抖动）
            this.showDragInsertIndicatorAtTarget(afterElement);
        });

        this.blocksContainer.addEventListener('drop', (e) => {
            // 检查是否是脚本块拖拽
            const types = e.dataTransfer.types;
            if (!types.includes('application/x-script-block')) {
                // 不是脚本块拖拽，不处理
                return;
            }

            e.preventDefault();
            e.stopPropagation();
            this.clearDragInsertIndicator();

            const draggingBlock = this.blocksContainer.querySelector('.dragging');
            if (!draggingBlock) return;

            const fromIndex = parseInt(draggingBlock.dataset.index);
            const afterElement = this.getDragAfterElement(e.clientY);

            // 计算目标插入位置
            let toIndex;
            if (afterElement) {
                toIndex = parseInt(afterElement.dataset.index);
            } else {
                toIndex = this.commands.length;
            }

            // 调整索引（如果从前往后拖，需要减1）
            if (fromIndex < toIndex) {
                toIndex--;
            }

            // 移动命令
            if (fromIndex !== toIndex) {
                const [movedCommand] = this.commands.splice(fromIndex, 1);
                this.commands.splice(toIndex, 0, movedCommand);

                this.renderBlocks();
                this.setupBlockModeListeners();
                this.triggerChange();

                window.rLog(`脚本块从位置 ${fromIndex} 移动到 ${toIndex}`);
            }
        });
    }

    /**
     * 设置元素拖拽到参数孔（仅处理元素到参数的拖拽）
     */
    setupElementDrop() {
        // 为所有 element 类型的参数孔和可视化卡片设置拖拽接收
        const elementTargets = this.blocksContainer.querySelectorAll('.param-hole[data-param-type="element"], .param-visual-card');

        elementTargets.forEach(target => {
            // dragover - 允许放置
            target.addEventListener('dragover', (e) => {
                // 检查是否是元素拖拽（不是脚本块拖拽）
                const types = e.dataTransfer.types;
                if (types.includes('application/x-script-block')) {
                    // 脚本块拖拽，不处理
                    return;
                }

                // 检查是否是元素拖拽（图片或XML元素）
                if (types.includes('application/x-locator-image') || types.includes('application/x-locator-xml')) {
                    e.preventDefault();
                    e.stopPropagation();
                    target.classList.add('drag-over');
                }
            });

            // dragleave - 移除高亮
            target.addEventListener('dragleave', (e) => {
                target.classList.remove('drag-over');
            });

            // drop - 接收元素
            target.addEventListener('drop', (e) => {
                // 检查是否是脚本块拖拽
                const types = e.dataTransfer.types;
                if (types.includes('application/x-script-block')) {
                    // 脚本块拖拽，不处理
                    return;
                }

                // 检查是否是元素拖拽
                let elementData = null;
                let elementType = null;

                if (types.includes('application/x-locator-image')) {
                    elementData = e.dataTransfer.getData('application/x-locator-image');
                    elementType = 'image';
                } else if (types.includes('application/x-locator-xml')) {
                    elementData = e.dataTransfer.getData('application/x-locator-xml');
                    elementType = 'xml';
                }

                if (elementData) {
                    e.preventDefault();
                    e.stopPropagation();
                    target.classList.remove('drag-over');

                    // 更新参数值
                    const commandIndex = parseInt(target.dataset.commandIndex);
                    const paramName = target.dataset.param;

                    if (this.commands[commandIndex]) {
                        // 格式：图片用 @{name}，XML元素用 {name}
                        const value = elementType === 'image' ? `@{${elementData}}` : `{${elementData}}`;
                        this.commands[commandIndex].params[paramName] = value;

                        // 重新渲染并触发变化
                        this.renderBlocks();
                        this.setupBlockModeListeners();
                        this.triggerChange();

                        window.rLog(`元素已填入参数: ${value}`);
                    }
                }
            });
        });
    }

    /**
     * 显示拖拽插入提示线（根据目标位置）
     */
    showDragInsertIndicatorAtTarget(afterElement) {
        const containerRect = this.blocksContainer.getBoundingClientRect();
        let top;

        if (afterElement) {
            // 在afterElement上方显示
            const rect = afterElement.getBoundingClientRect();
            const prevElement = afterElement.previousElementSibling;

            if (prevElement && prevElement.classList.contains('command-block') && !prevElement.classList.contains('dragging')) {
                // 有上一个块（且不是正在拖拽的块），显示在中间
                const prevRect = prevElement.getBoundingClientRect();
                top = (prevRect.bottom + rect.top) / 2 - containerRect.top;
            } else {
                // 第一个位置
                top = rect.top - containerRect.top - 4;
            }
        } else {
            // 在最后显示
            const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block:not(.dragging)');
            if (blocks.length > 0) {
                const lastBlock = blocks[blocks.length - 1];
                const rect = lastBlock.getBoundingClientRect();
                top = rect.bottom - containerRect.top + 4;
            } else {
                top = 8;
            }
        }

        // 复用已存在的指示器，只更新位置（避免频繁创建/销毁DOM）
        let indicator = this.blocksContainer.querySelector('#drag-insert-indicator');
        if (!indicator) {
            indicator = document.createElement('div');
            indicator.className = 'drag-insert-indicator';
            indicator.id = 'drag-insert-indicator';
            this.blocksContainer.appendChild(indicator);
        }

        indicator.style.top = `${top}px`;
    }

    /**
     * 清除拖拽插入提示线
     */
    clearDragInsertIndicator() {
        const indicator = this.blocksContainer.querySelector('#drag-insert-indicator');
        if (indicator) {
            indicator.remove();
        }
    }

    /**
     * 获取拖拽后应该插入的位置
     */
    getDragAfterElement(y) {
        const draggableElements = [...this.blocksContainer.querySelectorAll('.workspace-block.command-block:not(.dragging)')];

        return draggableElements.reduce((closest, child) => {
            const box = child.getBoundingClientRect();
            const offset = y - box.top - box.height / 2;

            if (offset < 0 && offset > closest.offset) {
                return { offset: offset, element: child };
            } else {
                return closest;
            }
        }, { offset: Number.NEGATIVE_INFINITY }).element;
    }

    /**
     * 设置插入按钮菜单
     */
    setupInsertMenus() {
        this.blocksContainer.querySelectorAll('.block-insert-btn').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const insertArea = btn.closest('.block-insert-area');
                const insertIndex = parseInt(insertArea.dataset.insertIndex);
                this.showCommandMenu(insertArea, insertIndex);
            });
        });
    }

    /**
     * 显示命令菜单
     */
    showCommandMenu(insertArea, insertIndex) {
        window.rLog(`显示命令菜单，插入位置: ${insertIndex}`);

        // 创建菜单
        const menuItems = [];
        Object.entries(window.BlockDefinitions || {}).forEach(([categoryKey, category]) => {
            category.commands.forEach(cmd => {
                menuItems.push(`
                    <div class="command-menu-item" data-type="${cmd.type}">
                        <span class="menu-item-icon">${category.icon}</span>
                        <span class="menu-item-label">${cmd.label}</span>
                    </div>
                `);
            });
        });

        const menuHtml = `<div class="command-menu">${menuItems.join('')}</div>`;
        insertArea.insertAdjacentHTML('beforeend', menuHtml);

        const menu = insertArea.querySelector('.command-menu');

        // 点击菜单项
        menu.addEventListener('click', (e) => {
            const item = e.target.closest('.command-menu-item');
            if (item) {
                const commandType = item.dataset.type;
                this.insertCommand(commandType, insertIndex);
                menu.remove();
            }
        });

        // 点击外部关闭
        setTimeout(() => {
            document.addEventListener('click', () => menu.remove(), { once: true });
        }, 0);
    }

    /**
     * 插入命令
     */
    insertCommand(commandType, insertIndex) {
        const definition = window.CommandUtils?.findCommandDefinition(commandType);
        if (!definition) return;

        const newCommand = {
            type: commandType,
            params: {}
        };

        definition.params.forEach(param => {
            newCommand.params[param.name] = param.default || '';
        });

        this.commands.splice(insertIndex, 0, newCommand);
        this.renderBlocks();
        this.setupBlockModeListeners();
        this.triggerChange();
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
     * 触发变化事件 - 将块的修改同步到textEditor
     */
    triggerChange() {
        const newContent = this.toString();
        // 直接更新textEditor的内容
        this.textEditor.updateContent(newContent);
        window.rLog('块编辑器修改已同步到textEditor');
    }

    /**
     * 将命令数组转换为脚本文本
     */
    toString() {
        const lines = [...this.headerLines]; // 复制头部内容

        // 如果有命令但没有"步骤:"行,在末尾添加"步骤:"
        if (this.commands.length > 0) {
            const hasStepsLine = lines.some(line => line.trim() === '步骤:');
            if (!hasStepsLine) {
                lines.push('步骤:');
            }

            // 添加命令
            this.commands.forEach(command => {
                const definition = window.CommandUtils?.findCommandDefinition(command.type);
                if (definition) {
                    const tksCommand = definition.tksCommand || command.type;
                    const paramValues = definition.params.map(p => command.params[p.name] || '').filter(v => v);
                    const paramStr = paramValues.join(', ');
                    lines.push(`    ${tksCommand} [${paramStr}]`);
                }
            });
        }

        return lines.join('\n');
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
