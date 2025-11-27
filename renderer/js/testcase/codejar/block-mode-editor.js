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
        this.isTestRunning = false; // 是否正在运行测试
        this.currentHighlightedBlock = null; // 当前高亮的块索引（0-based）
        this.highlightType = null; // 高亮类型：'executing' 或 'error'

        // DOM元素
        this.blocksContainer = null;
        this.blockNumberController = null; // 块号控制器

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

        // 6. 初始化块号控制器
        if (window.BlockNumberController && typeof window.BlockNumberController === 'function') {
            try {
                this.blockNumberController = new window.BlockNumberController(
                    this.blocksContainer,
                    this
                );
                window.rLog('✅ BlockNumberController 创建成功');
            } catch (error) {
                window.rError('❌ BlockNumberController 创建失败:', error);
            }
        } else {
            window.rError('❌ BlockNumberController 未正确加载');
        }

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
                    // 统一格式：{元素名}&策略
                    const elementMatch = value.match(/^\{(.+?)\}(?:&(\w+))?$/);

                    // 检查是否是坐标格式 {数字, 数字}
                    const isCoordinate = /^\{\s*\d+\s*,\s*\d+\s*\}$/.test(value);

                    if (elementMatch && !isCoordinate) {
                        // 已填入元素，显示可视化卡片
                        const elementName = elementMatch[1];
                        const strategy = elementMatch[2] || ''; // 提取策略
                        const isImageStrategy = ['img', 'image'].includes(strategy.toLowerCase());

                        if (isImageStrategy) {
                            // 图片元素 - 显示图片预览，失败时显示图标
                            const { path: PathModule } = window.AppGlobals;
                            const projectPath = window.AppGlobals.currentProject;
                            const imagePath = projectPath ? PathModule.join(projectPath, 'locator/img', `${elementName}.png`) : '';

                            commandContent += `
                                <div class="param-visual-card param-image-card"
                                     data-param="${param.name}"
                                     data-command-index="${index}"
                                     data-strategy="${strategy}">
                                    <div class="visual-image-preview">
                                        <img src="${imagePath}" alt="${elementName}"
                                             onerror="this.style.display='none'; this.nextElementSibling.style.display='flex';">
                                        <svg width="16" height="16" viewBox="0 0 24 24" style="display: none; flex-shrink: 0;">
                                            <rect x="3" y="3" width="18" height="18" fill="#4a90e2" opacity="0.2" rx="2"/>
                                            <circle cx="8.5" cy="8.5" r="1.5" fill="#4a90e2"/>
                                            <path d="M3 17l4-4 3 3 6-6 5 5v3H3v-1z" fill="#4a90e2"/>
                                        </svg>
                                    </div>
                                    <span class="visual-name">${elementName}</span>
                                    <button class="visual-remove-btn" data-param="${param.name}" data-command-index="${index}" title="移除">×</button>
                                </div>
                            `;
                        } else {
                            // 元素卡片 - 根据策略显示不同图标
                            const iconHtml = window.BlockUIStrategyMenu
                                ? window.BlockUIStrategyMenu.getStrategyIcon(strategy, 16)
                                : `<svg width="16" height="16" viewBox="0 0 24 24" style="flex-shrink: 0;"><path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3"/></svg>`;

                            commandContent += `
                                <div class="param-visual-card visual-xml-card"
                                     data-param="${param.name}"
                                     data-command-index="${index}"
                                     data-strategy="${strategy}">
                                    ${iconHtml}
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

        // 为所有元素卡片添加点击事件以显示策略菜单（包括 XML 卡片和图片卡片）
        this.blocksContainer.querySelectorAll('.visual-xml-card, .param-image-card').forEach(card => {
            card.addEventListener('click', (e) => {
                // 如果点击的是移除按钮，不处理
                if (e.target.closest('.visual-remove-btn')) {
                    return;
                }

                e.stopPropagation();

                const commandIndex = parseInt(card.dataset.commandIndex);
                const paramName = card.dataset.param;
                const currentStrategy = card.dataset.strategy || '';

                // 获取元素名称
                const command = this.getCommands()[commandIndex];
                const paramValue = command?.params[paramName] || '';
                const elementName = paramValue.match(/^\{(.+?)\}/)?.[1] || '';

                // 计算菜单位置（在卡片下方）
                const rect = card.getBoundingClientRect();
                const x = rect.left;
                const y = rect.bottom + 4;

                window.rLog(`点击元素卡片，命令: ${commandIndex}, 参数: ${paramName}, 元素: ${elementName}, 策略: ${currentStrategy}`);

                // 使用策略菜单模块显示菜单
                if (window.BlockUIStrategyMenu && typeof window.BlockUIStrategyMenu.show === 'function') {
                    window.BlockUIStrategyMenu.show(x, y, commandIndex, paramName, elementName, currentStrategy,
                        (cmdIndex, param, strategy) => {
                            // 策略选择后的回调
                            this.applyStrategy(cmdIndex, param, strategy);
                        }
                    );
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

                // 检查是否是元素拖拽（统一格式）
                if (types.includes('application/x-locator') || types.includes('application/x-locator-image') || types.includes('application/x-locator-xml')) {
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
                let hasImgPath = false;

                // 优先使用新的统一格式
                if (types.includes('application/x-locator')) {
                    elementData = e.dataTransfer.getData('application/x-locator');
                    // 尝试获取 JSON 数据来判断是否有图片路径
                    try {
                        const jsonData = e.dataTransfer.getData('application/json');
                        if (jsonData) {
                            const parsed = JSON.parse(jsonData);
                            hasImgPath = parsed.hasImgPath === true;
                        }
                    } catch (err) {
                        // 忽略解析错误
                    }
                } else if (types.includes('application/x-locator-image')) {
                    // 向后兼容：旧的图片格式
                    elementData = e.dataTransfer.getData('application/x-locator-image');
                    hasImgPath = true;
                } else if (types.includes('application/x-locator-xml')) {
                    // 向后兼容：旧的 XML 格式
                    elementData = e.dataTransfer.getData('application/x-locator-xml');
                    hasImgPath = false;
                }

                if (elementData) {
                    e.preventDefault();
                    e.stopPropagation();
                    target.classList.remove('drag-over');

                    // 更新参数值
                    const commandIndex = parseInt(target.dataset.commandIndex);
                    const paramName = target.dataset.param;

                    if (this.commands[commandIndex]) {
                        // 统一格式：{元素名} 或 {元素名}&img
                        const value = hasImgPath ? `{${elementData}}&img` : `{${elementData}}`;
                        this.commands[commandIndex].params[paramName] = value;

                        // 重新渲染并触发变化
                        this.renderBlocks();
                        this.setupBlockModeListeners();
                        this.triggerChange();

                        window.rLog(`元素已填入参数: ${value}`);

                        // 如果不是图片元素，自动弹出策略选择菜单
                        if (!hasImgPath) {
                            // 使用 setTimeout 确保渲染完成后再查找元素
                            setTimeout(() => {
                                // 查找刚刚填入的可视化卡片
                                const visualCards = this.blocksContainer.querySelectorAll('.visual-xml-card');
                                let targetCard = null;

                                // 找到对应命令和参数的卡片
                                visualCards.forEach(card => {
                                    if (parseInt(card.dataset.commandIndex) === commandIndex &&
                                        card.dataset.param === paramName) {
                                        targetCard = card;
                                    }
                                });

                                if (targetCard) {
                                    // 获取元素名称
                                    const command = this.getCommands()[commandIndex];
                                    const paramValue = command?.params[paramName] || '';
                                    const elementName = paramValue.match(/^\{(.+?)\}/)?.[1] || '';

                                    const rect = targetCard.getBoundingClientRect();
                                    const x = rect.left;
                                    const y = rect.bottom + 4;

                                    window.rLog(`自动显示策略菜单，命令: ${commandIndex}, 参数: ${paramName}, 元素: ${elementName}`);

                                    // 使用策略菜单模块显示菜单（当前策略为空，表示默认）
                                    if (window.BlockUIStrategyMenu && typeof window.BlockUIStrategyMenu.show === 'function') {
                                        window.BlockUIStrategyMenu.show(x, y, commandIndex, paramName, elementName, '',
                                            (cmdIndex, param, strategy) => {
                                                // 策略选择后的回调
                                                this.applyStrategy(cmdIndex, param, strategy);
                                            }
                                        );
                                    }
                                } else {
                                    window.rError(`未找到目标卡片: commandIndex=${commandIndex}, paramName=${paramName}`);
                                }
                            }, 150);
                        }
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
     * 应用策略到元素
     * @param {number} commandIndex - 命令索引
     * @param {string} paramName - 参数名
     * @param {string} strategy - 策略（空字符串表示无策略）
     */
    applyStrategy(commandIndex, paramName, strategy) {
        window.rLog(`应用策略: ${strategy || '默认'}, 命令: ${commandIndex}, 参数: ${paramName}`);

        const command = this.commands[commandIndex];
        if (!command) {
            window.rError(`未找到命令: ${commandIndex}`);
            return;
        }

        const currentValue = command.params[paramName];
        if (!currentValue) {
            window.rError(`参数值为空: ${paramName}`);
            return;
        }

        // 使用策略菜单模块的工具函数应用策略
        if (!window.BlockUIStrategyMenu) {
            window.rError('BlockUIStrategyMenu 模块未加载');
            return;
        }

        const newValue = window.BlockUIStrategyMenu.applyStrategyToValue(currentValue, strategy);
        if (!newValue) {
            return;
        }

        window.rLog(`更新参数值: ${currentValue} -> ${newValue}`);
        command.params[paramName] = newValue;

        // 重新渲染
        this.renderBlocks();
        this.setupBlockModeListeners();
        this.triggerChange();
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
            this.blocksContainer.classList.add('locked');
            this.blocksContainer.style.pointerEvents = 'none';
        }
        window.rLog('块编辑器已锁定');
    }

    /**
     * 解锁编辑器（允许编辑）
     */
    unlock() {
        if (this.blocksContainer) {
            this.blocksContainer.classList.remove('locked');
            this.blocksContainer.style.pointerEvents = 'auto';
        }
        window.rLog('块编辑器已解锁');
    }

    /**
     * 高亮正在执行的行
     * @param {number} lineNumber - 行号（1-based，在完整脚本中的行号）
     */
    highlightExecutingLine(lineNumber) {
        window.rLog(`🔆 块模式高亮执行行: ${lineNumber}`);

        // 计算块索引
        const blockIndex = this.lineNumberToBlockIndex(lineNumber);

        if (blockIndex === -1) {
            window.rLog('行号不对应任何命令块');
            return;
        }

        // 保存当前高亮状态
        this.currentHighlightedBlock = blockIndex;
        this.highlightType = 'executing';

        // 应用高亮
        this.applyBlockHighlight(blockIndex, 'executing');
    }

    /**
     * 高亮错误行
     * @param {number} lineNumber - 行号（1-based）
     */
    highlightErrorLine(lineNumber) {
        window.rLog(`❌ 块模式高亮错误行: ${lineNumber}`);

        const blockIndex = this.lineNumberToBlockIndex(lineNumber);

        if (blockIndex === -1) {
            window.rLog('行号不对应任何命令块');
            return;
        }

        // 保存当前高亮状态
        this.currentHighlightedBlock = blockIndex;
        this.highlightType = 'error';

        // 应用高亮
        this.applyBlockHighlight(blockIndex, 'error');
    }

    /**
     * 将行号转换为块索引
     * @param {number} lineNumber - 行号（1-based）
     * @returns {number} 块索引（0-based），如果不是命令行则返回-1
     */
    lineNumberToBlockIndex(lineNumber) {
        // 找到"步骤:"所在的行号
        let stepsLineNumber = -1;
        for (let i = 0; i < this.headerLines.length; i++) {
            if (this.headerLines[i].trim() === '步骤:') {
                stepsLineNumber = i + 1; // 1-based
                break;
            }
        }

        if (stepsLineNumber === -1) {
            window.rLog('未找到"步骤:"行');
            return -1;
        }

        // 命令行从"步骤:"的下一行开始
        const commandStartLine = stepsLineNumber + 1;
        if (lineNumber < commandStartLine) {
            return -1; // 在"步骤:"之前
        }

        const blockIndex = lineNumber - commandStartLine;

        if (blockIndex >= 0 && blockIndex < this.commands.length) {
            return blockIndex;
        }

        return -1;
    }

    /**
     * 应用块高亮
     * @param {number} blockIndex - 块索引（0-based）
     * @param {string} type - 高亮类型：'executing' 或 'error'
     */
    applyBlockHighlight(blockIndex, type) {
        if (!this.blocksContainer) return;

        // 移除之前的高亮
        this.blocksContainer.querySelectorAll('.executing-block, .error-block').forEach(el => {
            el.classList.remove('executing-block', 'error-block');
        });

        // 添加新高亮
        const blockEl = this.blocksContainer.querySelector(`.command-block[data-index="${blockIndex}"]`);
        if (blockEl) {
            const className = type === 'executing' ? 'executing-block' : 'error-block';
            blockEl.classList.add(className);

            // 滚动到视图中心
            blockEl.scrollIntoView({ behavior: 'smooth', block: 'center' });

            window.rLog(`✅ 块 ${blockIndex} 已高亮为 ${type}`);
        } else {
            window.rError(`未找到块元素: data-index="${blockIndex}"`);
        }
    }

    /**
     * 设置测试运行状态
     * @param {boolean} isRunning - 是否正在运行
     * @param {boolean} clearHighlight - 是否清除高亮
     */
    setTestRunning(isRunning, clearHighlight) {
        window.rLog(`🎯 块模式设置测试运行状态: ${isRunning}, 清除高亮: ${clearHighlight}`);

        this.isTestRunning = isRunning;

        if (clearHighlight && this.blocksContainer) {
            this.blocksContainer.querySelectorAll('.executing-block, .error-block').forEach(el => {
                el.classList.remove('executing-block', 'error-block');
            });
            this.currentHighlightedBlock = null;
            this.highlightType = null;
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

        if (this.blockNumberController) {
            this.blockNumberController.destroy();
            this.blockNumberController = null;
        }

        this.container.innerHTML = '';
        this.eventHandlers.clear();
    }
}

// 导出到全局
window.BlockModeEditor = BlockModeEditor;
(window.rLog || console.log)('BlockModeEditor 模块已加载');
