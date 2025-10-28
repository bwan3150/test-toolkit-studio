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
        const searchInput = document.querySelector('#locatorLibPane .search-input');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterLocators(e.target.value);
            });
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
        
        // 生成定位器名称和备注
        const result = await this.promptForLocatorName(element);
        if (!result) return;

        const { name, note } = result;

        // 创建定位器对象，兼容toolkit-engine格式
        const locator = {
            type: 'xml',
            locator_type: 'XML',  // 兼容toolkit-engine
            name: name,
            note: note || '',  // 添加备注字段
            class_name: element.className || '',  // 使用下划线格式兼容toolkit-engine
            text: element.text || null,
            content_desc: element.contentDesc || null,  // 使用下划线格式
            resource_id: element.resourceId || null,
            bounds: element.bounds || [],
            clickable: element.clickable || false,
            enabled: element.enabled || false,
            xpath: element.xpath || null,  // 添加xpath支持
            match_strategy: null,  // 可选的匹配策略
            createdAt: new Date().toISOString()
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
        
        window.AppNotifications?.success(`定位器 "${name}" 已保存`);
    },
    
    // 提示输入定位器名称
    async promptForLocatorName(element) {
        return new Promise((resolve) => {
            // 生成默认名称
            const defaultName = element.text || element.contentDesc || 
                              element.className?.split('.').pop() || 
                              `element_${Date.now()}`;
            
            // 创建模态框
            const modal = document.createElement('div');
            modal.className = 'modal-overlay';
            modal.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0, 0, 0, 0.5);
                display: flex;
                align-items: center;
                justify-content: center;
                z-index: 10000;
            `;

            const dialog = document.createElement('div');
            dialog.className = 'modal-dialog';
            dialog.style.cssText = `
                background: var(--bg-secondary);
                border-radius: 8px;
                padding: 20px;
                width: 400px;
                box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
            `;

            dialog.innerHTML = `
                <h3 style="margin: 0 0 15px 0; color: var(--text-primary);">保存元素到定位器库</h3>
                <div style="margin-bottom: 15px;">
                    <label style="display: block; margin-bottom: 5px; color: var(--text-secondary); font-size: 12px;">定位器名称：</label>
                    <input type="text" id="locator-name-input" value="${this.escapeHtml(defaultName)}"
                           style="width: 100%; padding: 8px; background: var(--bg-primary);
                                  border: 1px solid var(--border-color); color: var(--text-primary);
                                  border-radius: 4px; font-size: 13px;">
                </div>
                <div style="margin-bottom: 15px;">
                    <label style="display: block; margin-bottom: 5px; color: var(--text-secondary); font-size: 12px;">备注 (可选)：</label>
                    <input type="text" id="locator-note-input" placeholder="添加备注说明..."
                           style="width: 100%; padding: 8px; background: var(--bg-primary);
                                  border: 1px solid var(--border-color); color: var(--text-primary);
                                  border-radius: 4px; font-size: 13px;">
                </div>
                ${element.text ? `<div style="margin-bottom: 5px; color: var(--text-secondary); font-size: 11px;">文本: ${this.escapeHtml(element.text)}</div>` : ''}
                ${element.contentDesc ? `<div style="margin-bottom: 5px; color: var(--text-secondary); font-size: 11px;">描述: ${this.escapeHtml(element.contentDesc)}</div>` : ''}
                ${element.className ? `<div style="margin-bottom: 15px; color: var(--text-secondary); font-size: 11px;">类型: ${this.escapeHtml(element.className)}</div>` : ''}
                ${element.resourceId ? `<div style="margin-bottom: 5px; color: var(--text-secondary); font-size: 11px;">资源ID: ${this.escapeHtml(element.resourceId)}</div>` : ''}
                <div style="display: flex; gap: 10px; justify-content: flex-end;">
                    <button id="cancel-btn" style="padding: 6px 16px; background: var(--bg-tertiary);
                                                   border: 1px solid var(--border-color); color: var(--text-primary);
                                                   border-radius: 4px; cursor: pointer;">取消</button>
                    <button id="save-btn" style="padding: 6px 16px; background: var(--accent-primary);
                                                border: none; color: white; border-radius: 4px; cursor: pointer;">保存</button>
                </div>
            `;

            modal.appendChild(dialog);
            document.body.appendChild(modal);

            // 自动聚焦输入框并选中文本
            const input = dialog.querySelector('#locator-name-input');
            setTimeout(() => {
                input.focus();
                input.select();
            }, 100);

            // 事件处理
            const handleSave = async () => {
                const nameInput = dialog.querySelector('#locator-name-input');
                const noteInput = dialog.querySelector('#locator-note-input');
                const name = nameInput.value.trim();
                const note = noteInput.value.trim();

                if (!name) {
                    window.AppNotifications?.warn('请输入定位器名称');
                    return;
                }

                // 检查是否已存在
                if (this.locators[name]) {
                    if (!confirm(`定位器 "${name}" 已存在，是否覆盖？`)) {
                        return;
                    }
                }

                document.body.removeChild(modal);
                resolve({ name, note });
            };

            const handleCancel = () => {
                document.body.removeChild(modal);
                resolve(null);
            };

            dialog.querySelector('#save-btn').addEventListener('click', handleSave);
            dialog.querySelector('#cancel-btn').addEventListener('click', handleCancel);
            
            // 回车保存，ESC取消
            input.addEventListener('keydown', (e) => {
                if (e.key === 'Enter') {
                    handleSave();
                } else if (e.key === 'Escape') {
                    handleCancel();
                }
            });
            
            // 点击模态框外部取消
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    handleCancel();
                }
            });
        });
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
            const type = locator.type || 'xml';
            const note = locator.note || '';

            // 图标HTML
            let iconHTML = '';
            if (type === 'image') {
                // 如果是图片类型，显示图片
                const { path: PathModule } = window.AppGlobals;
                const projectPath = window.AppGlobals.currentProject;
                const imagePath = locator.path ? (projectPath ? PathModule.join(projectPath, locator.path) : locator.path) : '';

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
                     data-type="${type}"
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
            const type = card.dataset.type || 'xml';
            e.dataTransfer.effectAllowed = 'copy';

            // 设置拖拽数据，支持新TKS语法
            if (type === 'image') {
                // 图片元素：@{图片名称}
                e.dataTransfer.setData('text/plain', `@{${name}}`);
                // 设置专门的类型标识用于块编辑器识别
                e.dataTransfer.setData('application/x-locator-image', name);
            } else {
                // XML元素：{元素名称}
                e.dataTransfer.setData('text/plain', `{${name}}`);
                // 设置专门的类型标识用于块编辑器识别
                e.dataTransfer.setData('application/x-locator-xml', name);
            }

            // 设置JSON格式数据供编辑器使用
            e.dataTransfer.setData('application/json', JSON.stringify({
                type: 'locator',
                name: name,
                locatorType: type,
                data: this.locators[name]
            }));

            card.style.opacity = '0.5';
            window.rLog(`开始拖拽${type === 'image' ? '图片' : 'XML'}元素: ${name}`);
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
        
        // 生成定位器代码
        let code = '';
        if (locator.type === 'xml') {
            const resourceId = locator.resource_id || locator.resourceId;
            const contentDesc = locator.content_desc || locator.contentDesc;
            
            if (resourceId) {
                code = `click_element_by_id("${resourceId}")`;
            } else if (locator.text) {
                code = `click_element_by_text("${locator.text}")`;
            } else if (contentDesc) {
                code = `click_element_by_desc("${contentDesc}")`;
            } else {
                code = `click_element_by_locator("${name}")`;
            }
        } else {
            code = `click_image("${name}")`;
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
        
        // 如果是图像定位器，删除图像文件
        if (locator && locator.type === 'image') {
            try {
                const fs = window.nodeRequire('fs');
                const path = window.AppGlobals.path;
                const projectPath = window.AppGlobals.currentProject;
                
                // 使用定位器中的路径信息，或者默认路径
                let imgPath;
                if (locator.path) {
                    imgPath = path.join(projectPath, locator.path);
                } else {
                    imgPath = path.join(projectPath, 'locator', 'img', `${name}.png`);
                }
                
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
                   (locator.text && locator.text.toLowerCase().includes(searchLower)) ||
                   ((locator.content_desc || locator.contentDesc) && (locator.content_desc || locator.contentDesc).toLowerCase().includes(searchLower)) ||
                   ((locator.class_name || locator.className) && (locator.class_name || locator.className).toLowerCase().includes(searchLower));
        });
        
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