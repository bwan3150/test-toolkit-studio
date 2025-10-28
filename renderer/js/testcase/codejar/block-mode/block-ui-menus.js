// 块UI菜单系统 - 负责命令选择菜单和右键菜单

const BlockUIMenus = {
    /**
     * 显示命令选择菜单
     * @param {HTMLElement} insertArea - 插入区域元素
     * @param {number} insertIndex - 插入位置索引
     */
    showCommandMenu(insertArea, insertIndex) {
        window.rLog(`showCommandMenu 被调用，插入位置: ${insertIndex}, 插入区域存在: ${!!insertArea}`);

        if (this.isTestRunning) {
            window.rLog('测试运行中，无法显示命令菜单');
            return;
        }

        if (!insertArea) {
            window.rError('插入区域不存在，无法显示命令菜单');
            return;
        }

        // 隐藏现有菜单
        this.hideCommandMenu();

        // 创建菜单HTML
        const menuItems = [];
        Object.entries(window.BlockDefinitions).forEach(([categoryKey, category]) => {
            category.commands.forEach(cmd => {
                menuItems.push(`
                    <div class="command-menu-item" data-type="${cmd.type}" data-insert-index="${insertIndex}">
                        <span class="menu-item-icon">${category.icon}</span>
                        <span class="menu-item-label">${cmd.label}</span>
                    </div>
                `);
            });
        });

        window.rLog(`创建了 ${menuItems.length} 个菜单项`);

        const menuHtml = `
            <div class="command-menu" id="commandMenu">
                ${menuItems.join('')}
            </div>
        `;

        // 插入菜单到插入区域
        window.rLog('将菜单HTML插入到插入区域');
        insertArea.insertAdjacentHTML('beforeend', menuHtml);
        this.currentMenu = insertArea.querySelector('.command-menu');

        if (this.currentMenu) {
            window.rLog('菜单元素创建成功，菜单项数量:', this.currentMenu.querySelectorAll('.command-menu-item').length);
            // 确保菜单可见
            this.currentMenu.style.display = 'block';
            this.currentMenu.style.visibility = 'visible';
            window.rLog('菜单样式:', window.getComputedStyle(this.currentMenu).display, window.getComputedStyle(this.currentMenu).visibility);
        } else {
            window.rError('菜单元素创建失败');
        }

        // 绑定菜单项点击事件
        this.currentMenu.addEventListener('click', (e) => {
            const menuItem = e.target.closest('.command-menu-item');
            if (menuItem) {
                const commandType = menuItem.dataset.type;
                const index = parseInt(menuItem.dataset.insertIndex);
                this.insertCommand(commandType, index);
                this.hideCommandMenu();
            }
        });
    },

    /**
     * 隐藏命令选择菜单
     */
    hideCommandMenu() {
        if (this.currentMenu) {
            this.currentMenu.remove();
            this.currentMenu = null;
        }
    },

    /**
     * 显示右键菜单
     * @param {number} x - X坐标
     * @param {number} y - Y坐标
     * @param {number} blockIndex - 块索引
     */
    showContextMenu(x, y, blockIndex) {
        window.rLog(`显示右键菜单，位置: (${x}, ${y}), 块索引: ${blockIndex}`);

        // 移除现有菜单
        this.hideContextMenu();

        const menuHtml = `
            <div class="context-menu" id="blockContextMenu" style="left: ${x}px; top: ${y}px;">
                <div class="context-menu-item" data-action="insert-below" data-index="${blockIndex}">
                    <span class="context-menu-item-icon">+</span>
                    在下方插入命令
                </div>
            </div>
        `;

        // 移除旧的菜单
        if (this.currentContextMenu) {
            this.currentContextMenu.remove();
        }

        const menuId = `${this.uniqueId}-context-menu`;
        const updatedMenuHtml = menuHtml.replace('id="blockContextMenu"', `id="${menuId}"`);
        document.body.insertAdjacentHTML('beforeend', updatedMenuHtml);
        this.currentContextMenu = document.querySelector(`#${menuId}`);

        window.rLog('右键菜单DOM元素已创建:', !!this.currentContextMenu);
        if (this.currentContextMenu) {
            window.rLog('菜单位置:', this.currentContextMenu.style.left, this.currentContextMenu.style.top);
            window.rLog('菜单尺寸:', this.currentContextMenu.offsetWidth, 'x', this.currentContextMenu.offsetHeight);
        }

        // 绑定菜单项点击事件
        this.currentContextMenu.addEventListener('click', (e) => {
            window.rLog('右键菜单项被点击');
            e.preventDefault();
            e.stopPropagation(); // 防止事件冒泡到document，导致菜单立即隐藏

            const menuItem = e.target.closest('.context-menu-item');
            if (menuItem) {
                const action = menuItem.dataset.action;
                const index = parseInt(menuItem.dataset.index);
                window.rLog(`菜单项动作: ${action}, 索引: ${index}`);

                if (action === 'insert-below') {
                    // 显示命令选择菜单在指定块下方
                    window.rLog(`尝试在块 ${index} 下方插入命令（插入位置: ${index + 1}）`);
                    this.hideContextMenu(); // 先隐藏右键菜单

                    // 使用 setTimeout 延迟执行，避免立即被全局点击事件隐藏
                    setTimeout(() => {
                        this.showInsertMenuAtBlock(index + 1);
                    }, 50);
                }
            }
        });

        // 点击其他地方关闭菜单
        setTimeout(() => {
            document.addEventListener('click', this.hideContextMenu.bind(this), { once: true });
        }, 0);
    },

    /**
     * 隐藏右键菜单
     */
    hideContextMenu() {
        if (this.currentContextMenu) {
            this.currentContextMenu.remove();
            this.currentContextMenu = null;
        }
    },

    /**
     * 在指定块下方显示插入菜单
     * @param {number} insertIndex - 插入位置索引
     */
    showInsertMenuAtBlock(insertIndex) {
        window.rLog(`showInsertMenuAtBlock 被调用，插入位置: ${insertIndex}`);

        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        window.rLog(`找到 ${blocks.length} 个命令块`);

        let targetBlock = null;

        if (insertIndex > 0 && insertIndex - 1 < blocks.length) {
            targetBlock = blocks[insertIndex - 1];
            window.rLog(`目标块索引: ${insertIndex - 1}, 找到目标块:`, !!targetBlock);
        }

        // 创建临时插入区域
        const tempInsertArea = document.createElement('div');
        tempInsertArea.className = 'block-insert-area temp-insert';
        tempInsertArea.dataset.insertIndex = insertIndex;
        tempInsertArea.innerHTML = `
            <button class="block-insert-btn temp" title="选择要插入的命令">
                <svg width="16" height="16" viewBox="0 0 16 16">
                    <path fill="currentColor" d="M8 2v12m-6-6h12"/>
                </svg>
            </button>
        `;

        window.rLog('临时插入区域已创建');

        // 插入临时区域
        if (targetBlock) {
            targetBlock.insertAdjacentElement('afterend', tempInsertArea);
            window.rLog('临时区域已插入到目标块后面');
        } else {
            this.blocksContainer.insertBefore(tempInsertArea, this.blocksContainer.firstChild);
            window.rLog('临时区域已插入到容器开头');
        }

        // 验证临时区域是否成功插入到DOM
        window.rLog('临时区域是否在DOM中:', document.contains(tempInsertArea));
        window.rLog('临时区域的父元素:', tempInsertArea.parentElement);
        window.rLog('临时区域位置:', tempInsertArea.getBoundingClientRect());

        // 立即显示菜单
        window.rLog('准备显示命令菜单');
        this.showCommandMenu(tempInsertArea, insertIndex);

        // 菜单关闭时移除临时区域
        const originalHideMenu = this.hideCommandMenu.bind(this);
        this.hideCommandMenu = () => {
            originalHideMenu();

            // 移除临时插入区域
            if (tempInsertArea && tempInsertArea.parentNode) {
                window.rLog('移除临时插入区域');
                tempInsertArea.remove();
            }

            // 恢复原来的 hideCommandMenu 方法
            this.hideCommandMenu = originalHideMenu;
        };
    },

    /**
     * 设置菜单功能
     */
    setupMenus() {
        window.rLog('设置块菜单功能...');

        // 为所有插入按钮添加点击事件
        const buttons = this.blocksContainer.querySelectorAll('.block-insert-btn');
        window.rLog(`找到 ${buttons.length} 个插入按钮`);

        buttons.forEach((btn, index) => {
            const insertArea = btn.closest('.block-insert-area');
            window.rLog(`按钮 ${index}:`, {
                hasInsertArea: !!insertArea,
                insertIndex: insertArea?.dataset.insertIndex
            });

            if (insertArea) {
                btn.addEventListener('click', (e) => {
                    e.stopPropagation();
                    const insertIndex = parseInt(insertArea.dataset.insertIndex);
                    window.rLog(`点击插入按钮，位置: ${insertIndex}`);
                    this.showCommandMenu(insertArea, insertIndex);
                });
            }
        });

        window.rLog('菜单功能设置完成');
    },

    /**
     * 显示策略选择菜单（调用独立模块）
     * @param {number} x - X坐标
     * @param {number} y - Y坐标
     * @param {number} commandIndex - 命令索引
     * @param {string} paramName - 参数名
     * @param {string} currentStrategy - 当前策略
     */
    showStrategyMenu(x, y, commandIndex, paramName, currentStrategy) {
        if (!window.BlockUIStrategyMenu) {
            window.rError('BlockUIStrategyMenu 模块未加载');
            return;
        }

        // 调用策略菜单模块显示菜单
        window.BlockUIStrategyMenu.show(x, y, commandIndex, paramName, currentStrategy,
            (cmdIndex, param, strategy) => {
                // 策略选择后的回调
                this.applyStrategy(cmdIndex, param, strategy);
            }
        );
    },

    /**
     * 应用策略到元素
     * @param {number} commandIndex - 命令索引
     * @param {string} paramName - 参数名
     * @param {string} strategy - 策略（空字符串表示无策略）
     */
    applyStrategy(commandIndex, paramName, strategy) {
        window.rLog(`应用策略: ${strategy || '默认'}, 命令: ${commandIndex}, 参数: ${paramName}`);

        const commands = this.getCommands();
        const command = commands[commandIndex];

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
    },

    /**
     * 插入命令
     * @param {string} commandType - 命令类型
     * @param {number} insertIndex - 插入位置
     */
    insertCommand(commandType, insertIndex) {
        window.rLog(`插入命令: ${commandType}, 位置: ${insertIndex}`);

        // 查找命令定义
        const definition = window.CommandUtils?.findCommandDefinition(commandType);
        if (!definition) {
            window.rError(`未找到命令定义: ${commandType}`);
            return;
        }

        // 创建新命令
        const newCommand = {
            type: commandType,
            params: {}
        };

        // 初始化参数为默认值
        definition.params.forEach(param => {
            newCommand.params[param.name] = param.default || '';
        });

        // 插入到commands数组
        const commands = this.getCommands();
        commands.splice(insertIndex, 0, newCommand);

        // 重新渲染
        this.renderBlocks();
        this.triggerChange();

        window.rLog(`命令已插入，当前命令数: ${commands.length}`);
    }
};

// 导出到全局
window.BlockUIMenus = BlockUIMenus;

if (window.rLog) {
    window.rLog('✅ BlockUIMenus 模块已加载');
}
