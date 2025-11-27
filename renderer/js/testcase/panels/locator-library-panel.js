// Locator库面板管理器
// 负责管理保存的元素定位器（从项目的locator文件夹读取）

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
    
    // 从文件系统加载定位器
    async loadLocators() {
        try {
            const projectPath = window.AppGlobals.currentProject;
            if (!projectPath) {
                window.rLog('没有打开的项目，跳过加载定位器');
                this.locators = {};
                this.renderLocators();
                return;
            }
            
            const fs = window.nodeRequire('fs');
            const path = window.AppGlobals.path;
            const locatorFile = path.join(projectPath, 'locator', 'element.json');
            
            if (fs.existsSync(locatorFile)) {
                const content = fs.readFileSync(locatorFile, 'utf8');
                this.locators = JSON.parse(content);
                window.rLog(`加载了 ${Object.keys(this.locators).length} 个定位器`);
            } else {
                window.rLog('定位器文件不存在，初始化为空');
                this.locators = {};
            }
            
            this.renderLocators();
        } catch (error) {
            window.rError('加载定位器失败:', error);
            this.locators = {};
            this.renderLocators();
        }
    },
    
    // 保存定位器到文件
    async saveLocators() {
        try {
            const projectPath = window.AppGlobals.currentProject;
            if (!projectPath) {
                window.rError('没有打开的项目，无法保存');
                return;
            }
            
            const fs = window.nodeRequire('fs');
            const path = window.AppGlobals.path;
            const locatorDir = path.join(projectPath, 'locator');
            const locatorFile = path.join(locatorDir, 'element.json');
            
            // 确保locator目录存在
            if (!fs.existsSync(locatorDir)) {
                fs.mkdirSync(locatorDir, { recursive: true });
            }
            
            // 保存文件
            fs.writeFileSync(locatorFile, JSON.stringify(this.locators, null, 2));
            window.rLog('定位器已保存到文件');
        } catch (error) {
            window.rError('保存定位器失败:', error);
        }
    },
    
    // 保存元素到定位器库
    async saveElementToLocator(elementIndex) {
        // 从当前UI元素列表获取元素，使用元素的index属性而不是数组索引
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
                // 新建元素
                await this._saveNewElement(result.name, result.note, elementData);
            } else if (result.action === 'merge') {
                // 合并到已有元素
                await this._mergeToElement(result.targetName, elementData);
            }
        } catch (error) {
            window.rError('保存元素失败:', error);
            window.AppNotifications?.error('保存失败: ' + error.message);
        }
    },

    // 保存为新元素
    async _saveNewElement(name, note, elementData) {
        // 创建定位器对象 - 统一格式（无 type 区分）
        const locator = {
            name: name,
            note: note || '',
            // XML 定位字段
            xpath: elementData.xpath,
            resource_id: elementData.resource_id,
            text: elementData.text,
            content_desc: elementData.content_desc,
            class_name: elementData.class_name,
            // 通用字段
            bounds: elementData.bounds,
            clickable: elementData.clickable,
            enabled: elementData.enabled,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
        };

        // 保存到locators对象
        this.locators[name] = locator;

        // 保存到文件
        await this.saveLocators();

        // 重新渲染列表
        this.renderLocators();

        // 切换到Locator库标签
        const locatorTab = document.getElementById('locatorLibTab');
        if (locatorTab) {
            locatorTab.click();
        }

        window.AppNotifications?.success(`元素 "${name}" 已保存`);
    },

    // 合并到已有元素
    async _mergeToElement(targetName, elementData) {
        const existingLocator = this.locators[targetName];

        if (!existingLocator) {
            window.AppNotifications?.error('目标元素不存在');
            return;
        }

        // 合并 XML 字段（覆盖非空值）
        if (elementData.xpath) existingLocator.xpath = elementData.xpath;
        if (elementData.resource_id) existingLocator.resource_id = elementData.resource_id;
        if (elementData.text) existingLocator.text = elementData.text;
        if (elementData.content_desc) existingLocator.content_desc = elementData.content_desc;
        if (elementData.class_name) existingLocator.class_name = elementData.class_name;
        if (elementData.bounds) existingLocator.bounds = elementData.bounds;
        if (elementData.clickable !== undefined) existingLocator.clickable = elementData.clickable;
        if (elementData.enabled !== undefined) existingLocator.enabled = elementData.enabled;

        existingLocator.updated_at = new Date().toISOString();

        // 保存到文件
        await this.saveLocators();

        // 重新渲染列表
        this.renderLocators();

        // 切换到Locator库标签
        const locatorTab = document.getElementById('locatorLibTab');
        if (locatorTab) {
            locatorTab.click();
        }

        window.AppNotifications?.success(`XML 属性已合并到元素 "${targetName}"`);
    },

    // 渲染定位器列表 - 卡片布局
    renderLocators(filteredLocators = null) {
        const container = document.getElementById('locatorLibContent');
        if (!container) return;

        const locatorsToRender = filteredLocators || Object.entries(this.locators);

        if (locatorsToRender.length === 0) {
            // 显示统一的空状态提示
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

        // 有卡片：添加 with-cards 类，显示padding和gap
        container.classList.add('with-cards');

        // 卡片布局
        const cardsHTML = locatorsToRender.map(([name, locator]) => {
            // 根据字段判断类型：有 img_path 是图片类型，否则是 XML 类型
            const isImageType = !!locator.img_path;
            const note = locator.note || '';

            // 图标HTML
            let iconHTML = '';
            if (isImageType) {
                // 如果是图片类型，显示图片
                const { path: PathModule } = window.AppGlobals;
                const projectPath = window.AppGlobals.currentProject;
                const imagePath = locator.img_path ? (projectPath ? PathModule.join(projectPath, locator.img_path) : locator.img_path) : '';

                if (imagePath) {
                    iconHTML = `<img src="${imagePath}" alt="${this.escapeHtml(name)}">`;
                } else {
                    iconHTML = `<svg viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="2" fill="none" stroke="currentColor" stroke-width="2"/><circle cx="8" cy="8" r="2" fill="currentColor"/><path d="M2 18l5-5 3 3 6-6 6 6v4H2v-2z" fill="currentColor"/></svg>`;
                }
            } else {
                // XML类型，显示花括号图标
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

        // 为每个卡片添加拖拽事件
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

            // 统一语法：{元素名}&策略
            // 图片元素默认使用 img 策略，XML元素默认使用 auto 策略
            if (hasImg) {
                // 图片元素：{元素名}&img
                e.dataTransfer.setData('text/plain', `{${name}}&img`);
            } else {
                // XML元素：{元素名} (auto 策略可省略)
                e.dataTransfer.setData('text/plain', `{${name}}`);
            }

            // 设置专门的类型标识用于块编辑器识别
            e.dataTransfer.setData('application/x-locator', name);

            // 设置JSON格式数据供编辑器使用
            e.dataTransfer.setData('application/json', JSON.stringify({
                type: 'locator',
                name: name,
                hasImgPath: hasImg,
                data: this.locators[name]
            }));

            card.style.opacity = '0.5';
            window.rLog(`开始拖拽元素: ${name}${hasImg ? ' (图片)' : ''}`);
        });

        card.addEventListener('dragend', (e) => {
            card.style.opacity = '';
        });
    },
    
    // 显示右键菜单
    showContextMenu(event, name) {
        // 移除已存在的菜单
        const existingMenu = document.querySelector('.locator-context-menu');
        if (existingMenu) {
            existingMenu.remove();
        }
        
        // 创建菜单
        const menu = document.createElement('div');
        menu.className = 'locator-context-menu';
        menu.style.cssText = `
            position: fixed;
            top: ${event.clientY}px;
            left: ${event.clientX}px;
            z-index: 10000;
        `;
        
        menu.innerHTML = `
            <div class="context-menu-item" data-action="rename">
                <svg viewBox="0 0 24 24" width="16" height="16">
                    <path fill="currentColor" d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                </svg>
                重命名
            </div>
            <div class="context-menu-item" data-action="delete">
                <svg viewBox="0 0 24 24" width="16" height="16">
                    <path fill="currentColor" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                </svg>
                删除
            </div>
        `;
        
        // 添加点击事件
        menu.addEventListener('click', async (e) => {
            const action = e.target.closest('.context-menu-item')?.dataset.action;
            if (action === 'rename') {
                await this.renameLocator(name);
            } else if (action === 'delete') {
                await this.deleteLocator(name);
            }
            menu.remove();
        });
        
        // 点击其他地方关闭菜单
        const closeMenu = (e) => {
            if (!menu.contains(e.target)) {
                menu.remove();
                document.removeEventListener('click', closeMenu);
            }
        };
        setTimeout(() => document.addEventListener('click', closeMenu), 0);
        
        document.body.appendChild(menu);
    },
    
    // 重命名定位器
    async renameLocator(oldName) {
        const newName = prompt(`重命名定位器 "${oldName}"`, oldName);
        if (!newName || newName === oldName) return;
        
        if (this.locators[newName]) {
            window.AppNotifications?.error('该名称已存在');
            return;
        }
        
        this.locators[newName] = this.locators[oldName];
        delete this.locators[oldName];
        
        await this.saveLocators();
        this.renderLocators();
        window.AppNotifications?.success('重命名成功');
    },
    
    // 使用定位器（插入到编辑器）
    useLocator(name) {
        const locator = this.locators[name];
        if (!locator) return;

        // 生成定位器代码 - 使用统一语法
        let code = '';
        if (locator.img_path) {
            // 有图片路径，使用 img 策略
            code = `点击 {${name}}&img`;
        } else {
            // 无图片路径，使用 auto 策略（可省略）
            code = `点击 {${name}}`;
        }
        
        // 如果有活动的编辑器，插入代码
        if (window.UnifiedEditorModule && window.UnifiedEditorModule.insertCode) {
            window.UnifiedEditorModule.insertCode(code);
            window.AppNotifications?.success('定位器已插入到编辑器');
        } else {
            // 复制到剪贴板
            navigator.clipboard.writeText(code);
            window.AppNotifications?.success('定位器代码已复制到剪贴板');
        }
    },
    
    // 删除定位器
    async deleteLocator(name) {
        if (!confirm(`确定要删除定位器 "${name}" 吗？`)) return;

        const locator = this.locators[name];

        // 如果有图像路径，删除图像文件
        if (locator && locator.img_path) {
            try {
                const fs = window.nodeRequire('fs');
                const path = window.AppGlobals.path;
                const projectPath = window.AppGlobals.currentProject;

                const imgPath = path.join(projectPath, locator.img_path);

                if (fs.existsSync(imgPath)) {
                    fs.unlinkSync(imgPath);
                    window.rLog(`删除图像文件: ${imgPath}`);
                } else {
                    window.rLog(`图像文件不存在，跳过删除: ${imgPath}`);
                }
            } catch (error) {
                window.rError('删除图像文件失败:', error);
            }
        }
        
        delete this.locators[name];
        await this.saveLocators();
        this.renderLocators();
        
        window.AppNotifications?.info(`定位器 "${name}" 已删除`);
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