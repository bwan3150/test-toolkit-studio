// 块UI构建器 - 负责渲染块模式的UI元素

const BlockUIBuilder = {
    /**
     * 渲染所有命令块
     */
    renderBlocks() {
        // 获取命令
        const commands = this.getCommands();

        let blocksHtml = '';

        // 为每个命令块生成HTML，包括块间的插入按钮
        commands.forEach((command, index) => {
            const definition = window.CommandUtils.findCommandDefinition(command.type);
            const category = window.CommandUtils.findCommandCategory(command.type);

            if (!definition || !category) return;

            // 创建带参数孔的指令块 - 混合文本和输入框
            let commandContent = `<span class="block-icon">${category.icon}</span><span class="command-label">${definition.label}</span>`;

            // 为每个参数创建输入框，并整合到命令中
            if (definition.params.length > 0) {
                definition.params.forEach((param, paramIndex) => {
                    const value = command.params[param.name] || param.default || '';
                    const paramId = `param-${index}-${param.name}`;

                    if (param.type === 'select') {
                        const optionsHtml = param.options.map(opt =>
                            `<option value="${opt}" ${value === opt ? 'selected' : ''}>${opt}</option>`
                        ).join('');
                        commandContent += `
                            <select class="param-hole"
                                    id="${paramId}"
                                    data-param="${param.name}"
                                    data-command-index="${index}"
                                    title="${param.placeholder}">
                                ${optionsHtml}
                            </select>
                        `;
                    } else {
                        // 检查参数类型是否为element，以支持可视化渲染
                        if (param.type === 'element') {
                            // 检查是否是坐标格式 {数字, 数字}
                            const isCoordinate = value && /^\{\s*\d+\s*,\s*\d+\s*\}$/.test(value);

                            if (value && !isCoordinate && (value.match(/^@\{(.+)\}$/) || value.match(/^\{(.+?)\}(?:&(?:resourceId|text|className|xpath))?$/))) {
                                // 检查值是否为图片引用格式 @{name} 或 XML元素引用格式 {name}&strategy
                                const imageMatch = value.match(/^@\{(.+)\}$/);
                                const xmlMatch = value.match(/^\{(.+)\}$/);

                                // 创建一个容器用于显示可视化元素
                                commandContent += `
                                    <div class="param-hole-container"
                                         data-param="${param.name}"
                                         data-command-index="${index}"
                                         data-type="element">
                                        <input class="param-hole hidden-input"
                                               id="${paramId}"
                                               type="hidden"
                                               data-param="${param.name}"
                                               data-command-index="${index}"
                                               value="${value}">
                                        <div class="param-visual-element"
                                             id="visual-${paramId}"
                                             data-param="${param.name}"
                                             data-command-index="${index}">
                                            <!-- 可视化内容将在渲染后动态添加 -->
                                        </div>
                                    </div>
                                `;
                            } else {
                                // element 类型的普通文本输入框（无论是否有值）
                                commandContent += `
                                    <input class="param-hole"
                                           id="${paramId}"
                                           type="text"
                                           data-param="${param.name}"
                                           data-command-index="${index}"
                                           data-param-type="element"
                                           placeholder="${param.placeholder}"
                                           title="${param.placeholder}"
                                           value="${value}">
                                `;
                            }
                        } else {
                            // 非locator类型的普通输入框
                            commandContent += `
                                <input class="param-hole"
                                       id="${paramId}"
                                       type="${param.type === 'number' ? 'number' : 'text'}"
                                       data-param="${param.name}"
                                       data-command-index="${index}"
                                       placeholder="${param.placeholder}"
                                       title="${param.placeholder}"
                                       value="${value}">
                            `;
                        }
                    }
                });
            }

            // 单行命令块 - 保持原有颜色背景
            const blockHtml = `
                <div class="workspace-block command-block"
                     data-index="${index}"
                     data-type="${command.type}"
                     draggable="true"
                     style="background: linear-gradient(135deg, ${category.color}ee, ${category.color}cc);">
                    <div class="command-content">
                        ${commandContent}
                    </div>
                    <button class="block-delete" data-index="${index}" title="删除">×</button>
                </div>
            `;

            blocksHtml += blockHtml;
        });

        // 最后添加一个插入按钮
        const finalInsertButton = `
            <div class="block-insert-area final" data-insert-index="${commands.length}">
                <button class="block-insert-btn" title="添加命令块">
                    <svg width="16" height="16" viewBox="0 0 16 16">
                        <path fill="currentColor" d="M8 2v12m-6-6h12"/>
                    </svg>
                </button>
            </div>
        `;

        if (commands.length === 0) {
            // 空状态
            this.blocksContainer.innerHTML = `
                <div class="empty-state">
                    <svg width="48" height="48" viewBox="0 0 48 48" opacity="0.3">
                        <path fill="currentColor" d="M38 8H10c-2.21 0-4 1.79-4 4v24c0 2.21 1.79 4 4 4h28c2.21 0 4-1.79 4-4V12c0-2.21-1.79-4-4-4z"/>
                    </svg>
                    <p>点击下方 ⊕ 按钮添加一个脚本块</p>
                </div>
                ${finalInsertButton}
            `;
        } else {
            this.blocksContainer.innerHTML = blocksHtml + finalInsertButton;
        }

        // 渲染完成后，处理可视化元素
        this.renderVisualElements();
    },

    /**
     * 渲染可视化元素（图片和XML元素卡片）
     */
    renderVisualElements() {
        const visualElements = this.blocksContainer.querySelectorAll('.param-visual-element');

        visualElements.forEach(element => {
            const commandIndex = parseInt(element.dataset.commandIndex);
            const paramName = element.dataset.param;
            const command = this.getCommands()[commandIndex];

            if (command && command.params[paramName]) {
                const value = command.params[paramName];

                const imageMatch = value.match(/^@\{(.+)\}$/);
                const xmlMatch = value.match(/^\{(.+?)\}(?:&(resourceId|text|className|xpath))?$/);

                if (imageMatch) {
                    // 渲染图片元素
                    const imageName = imageMatch[1];
                    // 获取项目路径
                    const { path: PathModule } = window.AppGlobals;
                    const projectPath = window.AppGlobals.currentProject;
                    const imagePath = projectPath ? PathModule.join(projectPath, 'locator/img', `${imageName}.png`) : '';

                    element.innerHTML = `
                        <div class="visual-image-card">
                            <img src="${imagePath}" alt="${imageName}" onerror="this.style.display='none'; this.nextElementSibling.style.display='block';">
                            <div class="image-fallback" style="display:none;">
                                <svg width="24" height="24" viewBox="0 0 24 24">
                                    <path fill="currentColor" d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
                                </svg>
                            </div>
                            <span class="visual-name">${imageName}</span>
                            <button class="visual-remove" data-command-index="${commandIndex}" data-param="${paramName}">×</button>
                        </div>
                    `;
                } else if (xmlMatch) {
                    // 渲染XML元素卡片
                    const elementName = xmlMatch[1];
                    const strategy = xmlMatch[2]; // 可能是 undefined

                    // 根据策略获取对应的图标
                    const iconHtml = window.BlockUIStrategyMenu
                        ? window.BlockUIStrategyMenu.getStrategyIcon(strategy || '', 20)
                        : `<svg width="20" height="20" viewBox="0 0 24 24"><path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3"/></svg>`;

                    element.innerHTML = `
                        <div class="visual-xml-card" data-strategy="${strategy || ''}">
                            ${iconHtml}
                            <span class="visual-name">${elementName}</span>
                            <button class="visual-remove" data-command-index="${commandIndex}" data-param="${paramName}">×</button>
                        </div>
                    `;
                }
            }
        });

        // 为移除按钮添加事件监听
        this.blocksContainer.querySelectorAll('.visual-remove').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const commandIndex = parseInt(btn.dataset.commandIndex);
                const paramName = btn.dataset.param;

                // 清空参数值
                const command = this.getCommands()[commandIndex];
                if (command) {
                    command.params[paramName] = '';
                    this.renderBlocks();
                    this.setupBlockModeListeners();
                    this.triggerChange();
                }
            });
        });

        // 为 XML 卡片添加点击事件以显示策略菜单
        this.blocksContainer.querySelectorAll('.visual-xml-card').forEach(card => {
            card.addEventListener('click', (e) => {
                // 如果点击的是移除按钮，不处理
                if (e.target.closest('.visual-remove')) {
                    return;
                }

                e.stopPropagation();

                const visualElement = card.closest('.param-visual-element');
                if (!visualElement) return;

                const commandIndex = parseInt(visualElement.dataset.commandIndex);
                const paramName = visualElement.dataset.param;
                const currentStrategy = card.dataset.strategy || '';

                // 计算菜单位置（在卡片下方）
                const rect = card.getBoundingClientRect();
                const x = rect.left;
                const y = rect.bottom + 4;

                window.rLog(`点击 XML 卡片，命令: ${commandIndex}, 参数: ${paramName}, 策略: ${currentStrategy}`);

                // 使用策略菜单模块显示菜单
                if (window.BlockUIStrategyMenu && typeof window.BlockUIStrategyMenu.show === 'function') {
                    window.BlockUIStrategyMenu.show(x, y, commandIndex, paramName, currentStrategy,
                        (cmdIndex, param, strategy) => {
                            // 策略选择后的回调 - 通过 BlockUIMenus 应用策略
                            if (window.BlockUIMenus && typeof window.BlockUIMenus.applyStrategy === 'function') {
                                window.BlockUIMenus.applyStrategy(cmdIndex, param, strategy);
                            }
                        }
                    );
                }
            });
        });
    }
};

// 导出到全局
window.BlockUIBuilder = BlockUIBuilder;

if (window.rLog) {
    window.rLog('✅ BlockUIBuilder 模块已加载');
}
