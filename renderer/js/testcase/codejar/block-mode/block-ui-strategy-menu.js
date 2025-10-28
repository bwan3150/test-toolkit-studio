// 块UI策略选择菜单 - 专门处理元素识别策略的选择和切换

const BlockUIStrategyMenu = {
    currentStrategyMenu: null,

    /**
     * 显示策略选择菜单
     * @param {number} x - X坐标
     * @param {number} y - Y坐标
     * @param {number} commandIndex - 命令索引
     * @param {string} paramName - 参数名
     * @param {string} elementName - 元素名称
     * @param {string} currentStrategy - 当前策略
     * @param {Function} onStrategySelected - 策略选择后的回调函数
     */
    show(x, y, commandIndex, paramName, elementName, currentStrategy, onStrategySelected) {
        window.rLog(`显示策略菜单，位置: (${x}, ${y}), 命令索引: ${commandIndex}, 参数: ${paramName}, 元素: ${elementName}, 当前策略: ${currentStrategy}`);

        // 移除现有菜单
        this.hide();

        // 获取元素定义以检查哪些字段有值
        const elementDef = this.getElementDefinition(elementName);
        window.rLog(`元素定义:`, elementDef);

        const strategies = [
            {
                value: '',
                label: '默认（全精确匹配）',
                icon: '<svg width="18" height="18" viewBox="0 0 24 24"><path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3"/></svg>',
                description: '所有字段全匹配',
                field: null // 默认策略不需要检查特定字段
            },
            {
                value: 'resourceId',
                label: 'Resource ID',
                icon: '<svg width="18" height="18" viewBox="0 0 24 24"><path fill="#4caf50" d="M5.5 7A1.5 1.5 0 0 0 4 8.5v7A1.5 1.5 0 0 0 5.5 17h7a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 12.5 7h-7m0 1.5h7v7h-7v-7M15 11v2h2v4h2v-4h2v-2h-6Z"/></svg>',
                description: '仅匹配 resourceId',
                field: 'resource_id'
            },
            {
                value: 'text',
                label: 'Text',
                icon: '<svg width="18" height="18" viewBox="0 0 24 24"><path fill="#00897b" d="M9.6 14L12 7.7L14.4 14M11 5L5.5 19h2.25l1.12-3h6.25l1.12 3h2.25L13 5h-2Z"/></svg>',
                description: '仅匹配文本内容',
                field: 'text'
            },
            {
                value: 'className',
                label: 'Class Name',
                icon: '<svg width="18" height="18" viewBox="0 0 24 24"><path fill="#9c27b0" d="M10 4H4c-1.11 0-2 .89-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-8l-2-2Z"/></svg>',
                description: '仅匹配类名',
                field: 'class_name'
            },
            {
                value: 'contentDesc',
                label: 'Content Desc',
                icon: '<svg width="18" height="18" viewBox="0 0 24 24"><path fill="#2196f3" d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/></svg>',
                description: '仅匹配 content description',
                field: 'content_desc'
            },
            {
                value: 'xpath',
                label: 'XPath',
                icon: '<svg width="18" height="18" viewBox="0 0 24 24"><path fill="#ff9800" d="M10 20v-6h4v6h5v-8h3L12 3L2 12h3v8h5Z"/></svg>',
                description: '使用 XPath 路径',
                field: 'xpath'
            }
        ];

        const menuItems = strategies.map(strategy => {
            const isActive = currentStrategy === strategy.value;
            const activeClass = isActive ? 'active' : '';

            // 检查策略是否可用（字段是否有值）
            let isDisabled = false;
            let disabledReason = '';
            if (strategy.field && elementDef) {
                const fieldValue = elementDef[strategy.field];
                if (!fieldValue || fieldValue === '' || fieldValue === null) {
                    isDisabled = true;
                    disabledReason = `元素定义中缺少 ${strategy.field} 字段`;
                }
            }

            const disabledClass = isDisabled ? 'disabled' : '';
            const title = isDisabled ? disabledReason : strategy.description;

            return `
                <div class="strategy-menu-item ${activeClass} ${disabledClass}"
                     data-strategy="${strategy.value}"
                     data-command-index="${commandIndex}"
                     data-param="${paramName}"
                     data-disabled="${isDisabled}"
                     title="${title}">
                    <div class="strategy-icon">${strategy.icon}</div>
                    <span class="strategy-label">${strategy.label}</span>
                    ${isActive ? '<svg class="strategy-check" width="14" height="14" viewBox="0 0 24 24"><path fill="#4caf50" d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg>' : ''}
                </div>
            `;
        }).join('');

        const menuHtml = `
            <div class="strategy-menu" id="strategyMenu" style="left: ${x}px; top: ${y}px;">
                <div class="strategy-menu-header">选择识别策略</div>
                ${menuItems}
            </div>
        `;

        document.body.insertAdjacentHTML('beforeend', menuHtml);
        this.currentStrategyMenu = document.querySelector('#strategyMenu');

        if (!this.currentStrategyMenu) {
            window.rError('策略菜单DOM元素创建失败');
            return;
        }

        window.rLog('策略菜单DOM元素已创建');

        // 确保菜单不会超出视口
        this.adjustMenuPosition();

        // 绑定菜单项点击事件
        this.currentStrategyMenu.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();

            const menuItem = e.target.closest('.strategy-menu-item');
            if (menuItem) {
                // 检查是否禁用
                const isDisabled = menuItem.dataset.disabled === 'true';
                if (isDisabled) {
                    window.rLog('策略已禁用，无法选择');
                    return;
                }

                const strategy = menuItem.dataset.strategy;
                const cmdIndex = parseInt(menuItem.dataset.commandIndex);
                const param = menuItem.dataset.param;
                window.rLog(`选择策略: ${strategy || '默认'}, 命令: ${cmdIndex}, 参数: ${param}`);

                // 调用回调函数
                if (typeof onStrategySelected === 'function') {
                    onStrategySelected(cmdIndex, param, strategy);
                }

                this.hide();
            }
        });

        // 点击其他地方关闭菜单
        setTimeout(() => {
            document.addEventListener('click', this.hide.bind(this), { once: true });
        }, 0);
    },

    /**
     * 调整菜单位置，确保不超出视口
     */
    adjustMenuPosition() {
        if (!this.currentStrategyMenu) return;

        const rect = this.currentStrategyMenu.getBoundingClientRect();
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        let x = parseFloat(this.currentStrategyMenu.style.left);
        let y = parseFloat(this.currentStrategyMenu.style.top);

        // 如果右侧超出视口，向左调整
        if (rect.right > viewportWidth) {
            x = viewportWidth - rect.width - 10;
            this.currentStrategyMenu.style.left = `${x}px`;
        }

        // 如果底部超出视口，向上调整
        if (rect.bottom > viewportHeight) {
            y = viewportHeight - rect.height - 10;
            this.currentStrategyMenu.style.top = `${y}px`;
        }

        // 确保不超出左侧和顶部
        if (x < 10) {
            this.currentStrategyMenu.style.left = '10px';
        }
        if (y < 10) {
            this.currentStrategyMenu.style.top = '10px';
        }
    },

    /**
     * 隐藏策略选择菜单
     */
    hide() {
        if (this.currentStrategyMenu) {
            this.currentStrategyMenu.remove();
            this.currentStrategyMenu = null;
            window.rLog('策略菜单已隐藏');
        }
    },

    /**
     * 应用策略到元素值
     * @param {string} currentValue - 当前参数值 (如 {登录按钮} 或 {登录按钮}&text)
     * @param {string} newStrategy - 新策略 (空字符串表示无策略)
     * @returns {string|null} 新的参数值，如果格式无效则返回 null
     */
    applyStrategyToValue(currentValue, newStrategy) {
        if (!currentValue) {
            window.rError('参数值为空');
            return null;
        }

        // 解析当前值，提取元素名（去掉旧的策略后缀）
        const xmlMatch = currentValue.match(/^\{(.+?)\}(?:&(?:resourceId|text|className|xpath))?$/);
        if (!xmlMatch) {
            window.rError(`无效的元素格式: ${currentValue}`);
            return null;
        }

        const elementName = xmlMatch[1];
        // 构建新值
        const newValue = newStrategy ? `{${elementName}}&${newStrategy}` : `{${elementName}}`;

        window.rLog(`策略应用: ${currentValue} -> ${newValue}`);
        return newValue;
    },

    /**
     * 从元素值中提取当前策略
     * @param {string} value - 元素值 (如 {登录按钮}&text)
     * @returns {string} 策略名称，如果没有策略则返回空字符串
     */
    extractStrategy(value) {
        if (!value) return '';

        const match = value.match(/^\{.+?\}&(resourceId|text|className|contentDesc|xpath)$/);
        return match ? match[1] : '';
    },

    /**
     * 获取策略对应的 SVG 图标
     * @param {string} strategy - 策略名称（空字符串表示默认）
     * @param {number} size - 图标大小，默认 16
     * @returns {string} SVG 图标的 HTML 字符串
     */
    getStrategyIcon(strategy, size = 16) {
        const icons = {
            '': `<svg width="${size}" height="${size}" viewBox="0 0 24 24"><path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3"/></svg>`,
            'resourceId': `<svg width="${size}" height="${size}" viewBox="0 0 24 24"><path fill="#4caf50" d="M5.5 7A1.5 1.5 0 0 0 4 8.5v7A1.5 1.5 0 0 0 5.5 17h7a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 12.5 7h-7m0 1.5h7v7h-7v-7M15 11v2h2v4h2v-4h2v-2h-6Z"/></svg>`,
            'text': `<svg width="${size}" height="${size}" viewBox="0 0 24 24"><path fill="#00897b" d="M9.6 14L12 7.7L14.4 14M11 5L5.5 19h2.25l1.12-3h6.25l1.12 3h2.25L13 5h-2Z"/></svg>`,
            'className': `<svg width="${size}" height="${size}" viewBox="0 0 24 24"><path fill="#9c27b0" d="M10 4H4c-1.11 0-2 .89-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-8l-2-2Z"/></svg>`,
            'contentDesc': `<svg width="${size}" height="${size}" viewBox="0 0 24 24"><path fill="#2196f3" d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/></svg>`,
            'xpath': `<svg width="${size}" height="${size}" viewBox="0 0 24 24"><path fill="#ff9800" d="M10 20v-6h4v6h5v-8h3L12 3L2 12h3v8h5Z"/></svg>`
        };
        return icons[strategy] || icons[''];
    },

    /**
     * 获取元素定义
     * @param {string} elementName - 元素名称
     * @returns {Object|null} 元素定义对象，如果未找到则返回 null
     */
    getElementDefinition(elementName) {
        if (!elementName) return null;

        // 从 LocatorLibraryPanel 获取元素定义
        if (window.LocatorLibraryPanel && window.LocatorLibraryPanel.locators) {
            return window.LocatorLibraryPanel.locators[elementName] || null;
        }

        return null;
    }
};

// 导出到全局
window.BlockUIStrategyMenu = BlockUIStrategyMenu;

if (window.rLog) {
    window.rLog('✅ BlockUIStrategyMenu 模块已加载');
}
