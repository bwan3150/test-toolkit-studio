// Locator库面板管理器
// 负责管理保存的元素定位器（从项目数据库读取）

const LocatorLibraryPanel = {
    // 保存的定位器对象
    locators: {},

    // 初始化
    init() {
        // 加载项目的定位器
        this.loadLocators();

        // 绑定搜索功能
        const searchInput = document.getElementById('locatorSearchInput');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterLocators(e.target.value);
            });
        } else {
            window.rWarn('搜索输入框未找到: #locatorSearchInput');
        }

        // 监听项目变更事件
        document.addEventListener('project-changed', () => {
            window.rLog('项目变更事件触发，重新加载locators');
            this.loadLocators();
        });
    },

    // 从数据库加载定位器
    async loadLocators() {
        try {
            const projectPath = window.AppGlobals.currentProject;
            if (!projectPath) {
                window.rLog('没有打开的项目，跳过加载定位器');
                this.locators = {};
                this.renderLocators();
                return;
            }

            const result = await window.AppGlobals.ipcRenderer.invoke('db-locator-getAll', projectPath);

            if (result.success) {
                this.locators = result.data || {};
                window.rLog(`加载了 ${Object.keys(this.locators).length} 个定位器`);
            } else {
                window.rError('加载定位器失败:', result.error);
                this.locators = {};
            }

            this.renderLocators();
        } catch (error) {
            window.rError('加载定位器失败:', error);
            this.locators = {};
            this.renderLocators();
        }
    },

    // 保存单个定位器到数据库
    async saveLocator(locator) {
        try {
            const projectPath = window.AppGlobals.currentProject;
            if (!projectPath) {
                window.rError('没有打开的项目，无法保存');
                return { success: false, error: '没有打开的项目' };
            }

            const result = await window.AppGlobals.ipcRenderer.invoke('db-locator-save', projectPath, locator);

            if (result.success) {
                this.locators[locator.name] = locator;
                window.rLog('定位器已保存:', locator.name);
            }

            return result;
        } catch (error) {
            window.rError('保存定位器失败:', error);
            return { success: false, error: error.message };
        }
    },

    // 保存元素到定位器库
    async saveElementToLocator(elementIndex) {
        // 从当前UI元素列表获取元素
        let element;
        if (window.ElementsListPanel && window.ElementsListPanel.currentElements) {
            element = window.ElementsListPanel.currentElements.find(el => el.index === elementIndex);
        }

        if (!element) {
            window.rError(`无法找到index为${elementIndex}的元素`);
            window.AppNotifications?.error('元素不存在');
            return;
        }

        // 准备元素数据
        const elementData = {
            xpath: element.xpath || null,
            resource_id: element.resource_id || null,
            text: element.text || null,
            content_desc: element.content_desc || null,
            class_name: element.class_name || '',
            bounds: element.bounds || null,
            clickable: element.clickable || false,
            enabled: element.enabled || false
        };

        // 生成默认名称
        const defaultName = element.text || element.content_desc ||
                          element.class_name?.split('.').pop() ||
                          `element_${Date.now()}`;

        // 检查 ElementSaveModal 是否可用
        if (!window.ElementSaveModal) {
            window.rError('ElementSaveModal 未加载');
            window.AppNotifications?.error('保存模态框未加载，请刷新页面');
            return;
        }

        try {
            const result = await window.ElementSaveModal.show({
                title: '保存 XML 元素',
                saveType: 'xml',
                elementData: elementData,
                defaultName: defaultName
            });

            if (!result) {
                window.rLog('用户取消了保存');
                return;
            }

            if (result.action === 'new') {
                await this._saveNewElement(result.name, result.note, elementData);
            } else if (result.action === 'merge') {
                await this._mergeToElement(result.targetName, elementData);
            }
        } catch (error) {
            window.rError('保存元素失败:', error);
            window.AppNotifications?.error('保存失败: ' + error.message);
        }
    },

    // 保存为新元素
    async _saveNewElement(name, note, elementData) {
        const locator = {
            name: name,
            note: note || '',
            xpath: elementData.xpath,
            resource_id: elementData.resource_id,
            text: elementData.text,
            content_desc: elementData.content_desc,
            class_name: elementData.class_name,
            bounds: elementData.bounds,
            clickable: elementData.clickable,
            enabled: elementData.enabled,
            img_path: elementData.img_path || null
        };

        const result = await this.saveLocator(locator);

        if (result.success) {
            this.renderLocators();
            this._switchToLocatorTab();
            window.AppNotifications?.success(`元素 "${name}" 已保存`);
        } else {
            window.AppNotifications?.error('保存失败: ' + result.error);
        }
    },

    // 合并到已有元素
    async _mergeToElement(targetName, elementData) {
        const existingLocator = this.locators[targetName];

        if (!existingLocator) {
            window.AppNotifications?.error('目标元素不存在');
            return;
        }

        // 合并字段
        const updatedLocator = { ...existingLocator };
        if (elementData.xpath) updatedLocator.xpath = elementData.xpath;
        if (elementData.resource_id) updatedLocator.resource_id = elementData.resource_id;
        if (elementData.text) updatedLocator.text = elementData.text;
        if (elementData.content_desc) updatedLocator.content_desc = elementData.content_desc;
        if (elementData.class_name) updatedLocator.class_name = elementData.class_name;
        if (elementData.bounds) updatedLocator.bounds = elementData.bounds;
        if (elementData.clickable !== undefined) updatedLocator.clickable = elementData.clickable;
        if (elementData.enabled !== undefined) updatedLocator.enabled = elementData.enabled;

        const result = await this.saveLocator(updatedLocator);

        if (result.success) {
            this.renderLocators();
            this._switchToLocatorTab();
            window.AppNotifications?.success(`XML 属性已合并到元素 "${targetName}"`);
        } else {
            window.AppNotifications?.error('合并失败: ' + result.error);
        }
    },

    // 切换到 Locator 库标签
    _switchToLocatorTab() {
        const locatorTab = document.getElementById('locatorLibTab');
        if (locatorTab) locatorTab.click();
    },

    // 渲染定位器列表
    renderLocators(filteredLocators = null) {
        const container = document.getElementById('locatorLibContent');
        if (!container) return;

        const locatorsToRender = filteredLocators || Object.entries(this.locators);

        if (locatorsToRender.length === 0) {
            container.classList.remove('with-cards');
            container.innerHTML = `
                <div class="locator-empty-state">
                    <div class="empty-icon">
                        <svg viewBox="0 0 48 48" width="48" height="48">
                            <rect x="8" y="8" width="32" height="32" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
                            <circle cx="16" cy="16" r="2" fill="currentColor"/>
                            <circle cx="24" cy="24" r="2" fill="currentColor"/>
                            <circle cx="32" cy="32" r="2" fill="currentColor"/>
                            <path d="M14,24 L10,20 M26,32 L22,28" stroke="currentColor" stroke-width="1.5"/>
                        </svg>
                    </div>
                    <div class="empty-title">No locators saved</div>
                </div>
            `;
            return;
        }

        container.classList.add('with-cards');

        const cardsHTML = locatorsToRender.map(([name, locator]) => {
            const isImageType = !!locator.img_path;
            const note = locator.note || '';

            let iconHTML = '';
            if (isImageType) {
                const projectPath = window.AppGlobals.currentProject;
                // 新路径：img/{name}.png
                const imagePath = locator.img_path ?
                    (projectPath ? `${projectPath}/${locator.img_path}` : locator.img_path) : '';

                if (imagePath) {
                    iconHTML = `<img src="${imagePath}" alt="${this.escapeHtml(name)}">`;
                } else {
                    iconHTML = `<svg viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="2" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="8" cy="8" r="2" fill="currentColor"/><path d="M2 18l5-5 3 3 6-6 6 6v4H2v-2z" fill="currentColor"/></svg>`;
                }
            } else {
                iconHTML = `<svg viewBox="0 0 24 24"><path d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3z" fill="currentColor"/></svg>`;
            }

            return `
                <div class="locator-card"
                     draggable="true"
                     data-name="${this.escapeHtml(name)}"
                     data-has-img="${isImageType}"
                     oncontextmenu="window.LocatorLibraryPanel.showContextMenu(event, '${this.escapeHtml(name)}'); return false;">
                    <div class="locator-card-icon">
                        ${iconHTML}
                    </div>
                    <div class="locator-card-content">
                        <div class="locator-card-name">${this.escapeHtml(name)}</div>
                        ${note ? `<div class="locator-card-note">${this.escapeHtml(note)}</div>` : ''}
                    </div>
                </div>
            `;
        }).join('');

        container.innerHTML = cardsHTML;

        // 绑定拖拽事件
        container.querySelectorAll('.locator-card').forEach(card => {
            this.setupCardDragEvents(card);
        });

        window.rLog(`✅ 已显示 ${locatorsToRender.length} 个定位器`);
    },

    // 设置卡片拖拽事件
    setupCardDragEvents(card) {
        card.addEventListener('dragstart', (e) => {
            const name = card.dataset.name;
            const hasImg = card.dataset.hasImg === 'true';
            e.dataTransfer.effectAllowed = 'copy';

            if (hasImg) {
                e.dataTransfer.setData('text/plain', `{${name}}&img`);
            } else {
                e.dataTransfer.setData('text/plain', `{${name}}`);
            }

            e.dataTransfer.setData('application/x-locator', name);
            e.dataTransfer.setData('application/json', JSON.stringify({
                type: 'locator',
                name: name,
                hasImgPath: hasImg,
                data: this.locators[name]
            }));

            card.style.opacity = '0.5';
            window.rLog(`开始拖拽元素: ${name}${hasImg ? ' (图片)' : ''}`);
        });

        card.addEventListener('dragend', () => {
            card.style.opacity = '';
        });
    },

    // 显示右键菜单
    showContextMenu(event, name) {
        const existingMenu = document.querySelector('.locator-context-menu');
        if (existingMenu) existingMenu.remove();

        const menu = document.createElement('div');
        menu.className = 'locator-context-menu';
        menu.style.cssText = `
            position: fixed;
            top: ${event.clientY}px;
            left: ${event.clientX}px;
            z-index: 10000;
        `;

        menu.innerHTML = `
            <div class="context-menu-item" data-action="edit">
                <svg viewBox="0 0 24 24" width="16" height="16">
                    <path fill="currentColor" d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                </svg>
                编辑
            </div>
            <div class="context-menu-item" data-action="delete">
                <svg viewBox="0 0 24 24" width="16" height="16">
                    <path fill="currentColor" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                </svg>
                删除
            </div>
        `;

        menu.addEventListener('click', async (e) => {
            const action = e.target.closest('.context-menu-item')?.dataset.action;
            menu.remove();
            if (action === 'edit') {
                await this.editLocator(name);
            } else if (action === 'delete') {
                await this.deleteLocator(name);
            }
        });

        const closeMenu = (e) => {
            if (!menu.contains(e.target)) {
                menu.remove();
                document.removeEventListener('click', closeMenu);
            }
        };
        setTimeout(() => document.addEventListener('click', closeMenu), 0);

        document.body.appendChild(menu);
    },

    // 编辑定位器
    async editLocator(name) {
        if (!window.ElementEditModal) {
            window.rError('ElementEditModal 未加载');
            window.AppNotifications?.error('编辑模态框未加载');
            return;
        }

        try {
            await window.ElementEditModal.show(name);
        } catch (error) {
            window.rError('编辑元素失败:', error);
            window.AppNotifications?.error('编辑失败: ' + error.message);
        }
    },

    // 使用定位器
    useLocator(name) {
        const locator = this.locators[name];
        if (!locator) return;

        let code = '';
        if (locator.img_path) {
            code = `点击 {${name}}&img`;
        } else {
            code = `点击 {${name}}`;
        }

        if (window.UnifiedEditorModule && window.UnifiedEditorModule.insertCode) {
            window.UnifiedEditorModule.insertCode(code);
            window.AppNotifications?.success('定位器已插入到编辑器');
        } else {
            navigator.clipboard.writeText(code);
            window.AppNotifications?.success('定位器代码已复制到剪贴板');
        }
    },

    // 删除定位器
    async deleteLocator(name) {
        if (!confirm(`确定要删除定位器 "${name}" 吗？`)) return;

        try {
            const projectPath = window.AppGlobals.currentProject;
            const locator = this.locators[name];

            // 如果有图像路径，删除图像文件
            if (locator && locator.img_path) {
                try {
                    const fs = window.nodeRequire('fs');
                    const imgPath = `${projectPath}/${locator.img_path}`;

                    if (fs.existsSync(imgPath)) {
                        fs.unlinkSync(imgPath);
                        window.rLog(`删除图像文件: ${imgPath}`);
                    }
                } catch (error) {
                    window.rError('删除图像文件失败:', error);
                }
            }

            // 从数据库删除
            const result = await window.AppGlobals.ipcRenderer.invoke('db-locator-delete', projectPath, name);

            if (result.success) {
                delete this.locators[name];
                this.renderLocators();
                window.AppNotifications?.info(`定位器 "${name}" 已删除`);
            } else {
                window.AppNotifications?.error('删除失败: ' + result.error);
            }
        } catch (error) {
            window.rError('删除定位器失败:', error);
            window.AppNotifications?.error('删除失败: ' + error.message);
        }
    },

    // 筛选定位器
    filterLocators(searchText) {
        if (!searchText) {
            this.renderLocators();
            return;
        }

        const searchLower = searchText.toLowerCase();
        const filtered = Object.entries(this.locators).filter(([name, locator]) => {
            return name.toLowerCase().includes(searchLower) ||
                   (locator.note && locator.note.toLowerCase().includes(searchLower)) ||
                   (locator.text && locator.text.toLowerCase().includes(searchLower)) ||
                   (locator.content_desc && locator.content_desc.toLowerCase().includes(searchLower)) ||
                   (locator.class_name && locator.class_name.toLowerCase().includes(searchLower));
        });

        window.rLog(`🔍 搜索 "${searchText}": 找到 ${filtered.length}/${Object.keys(this.locators).length} 个匹配项`);
        this.renderLocators(filtered);
    },

    // HTML转义
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
};

// 导出到全局
window.LocatorLibraryPanel = LocatorLibraryPanel;

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    LocatorLibraryPanel.init();
});
