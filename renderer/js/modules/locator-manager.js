// Locator库管理模块

class LocatorManager {
    constructor() {
        this.locators = {};
        this.currentCasePath = null;
        this.initialized = false;
    }

    // 初始化Locator管理器
    async initialize() {
        if (this.initialized) return;
        
        // 设置拖放功能
        this.setupDragAndDrop();
        
        // 设置搜索功能
        this.setupSearch();
        
        // 加载当前case的locator
        await this.loadLocators();
        
        this.initialized = true;
    }

    // 设置当前case路径
    setCurrentCase(casePath) {
        this.currentCasePath = casePath;
        this.loadLocators();
    }

    // 推断当前case路径
    inferCurrentCasePath() {
        // 优先使用全局currentTab
        let tabData = window.AppGlobals.currentTab;
        if (!tabData) {
            // 备用方案：查找活动tab
            const activeTab = document.querySelector('.tab.active');
            if (activeTab) {
                tabData = window.AppGlobals.openTabs.find(tab => tab.id === activeTab.id);
            }
        }
        
        if (tabData && tabData.path) {
            console.log('Inferring case path from:', tabData.path);
            // 从脚本路径推断case路径
            const pathSep = window.AppGlobals.path.sep;
            const pathParts = tabData.path.split(pathSep);
            const caseIndex = pathParts.indexOf('cases');
            if (caseIndex !== -1 && caseIndex < pathParts.length - 2) {
                this.currentCasePath = pathParts.slice(0, caseIndex + 2).join(pathSep);
                console.log('Inferred case path:', this.currentCasePath);
                return this.currentCasePath;
            }
        }
        
        console.log('Could not infer case path');
        return null;
    }

    // 加载locator文件
    async loadLocators() {
        if (!this.currentCasePath) {
            this.inferCurrentCasePath();
        }

        if (!this.currentCasePath) {
            console.log('没有选中的测试用例');
            this.updateLocatorList({});
            return;
        }

        try {
            const { path, fs } = window.AppGlobals;
            const locatorPath = path.join(this.currentCasePath, 'locator', 'element.json');
            
            // 检查文件是否存在
            try {
                await fs.access(locatorPath);
                const content = await fs.readFile(locatorPath, 'utf8');
                this.locators = JSON.parse(content);
            } catch (err) {
                // 文件不存在，使用空对象
                this.locators = {};
            }
            
            this.updateLocatorList(this.locators);
        } catch (error) {
            console.error('加载locator失败:', error);
            window.NotificationModule.showNotification('加载Locator库失败', 'error');
        }
    }

    // 保存元素到locator库
    async saveElement(element, customName = null) {
        if (!this.currentCasePath) {
            // 尝试重新推断case路径
            this.inferCurrentCasePath();
        }
        
        if (!this.currentCasePath) {
            window.NotificationModule.showNotification('请先选择或打开一个测试用例', 'warning');
            return false;
        }

        try {
            // 如果没有提供自定义名称，弹出输入框
            if (!customName) {
                customName = await this.promptForElementName(element);
                if (!customName) return false; // 用户取消
            }

            // 构建locator对象
            const locatorData = {
                className: element.className,
                bounds: element.bounds,
                text: element.text || undefined,
                contentDesc: element.contentDesc || undefined,
                resourceId: element.resourceId || undefined,
                hint: element.hint || undefined,
                clickable: element.clickable,
                focusable: element.focusable,
                scrollable: element.scrollable,
                xpath: element.xpath || undefined,
                addedAt: new Date().toISOString(),
                description: element.toAiText()
            };

            // 添加到locators对象
            this.locators[customName] = locatorData;

            // 保存到文件
            await this.saveLocatorsToFile();

            // 更新UI
            this.updateLocatorList(this.locators);

            window.NotificationModule.showNotification(`元素 "${customName}" 已保存到Locator库`, 'success');
            
            // 切换到Locator库标签页
            this.switchToLocatorTab();
            
            return true;
        } catch (error) {
            console.error('保存元素失败:', error);
            window.NotificationModule.showNotification('保存元素失败: ' + error.message, 'error');
            return false;
        }
    }

    // 弹出输入框获取元素名称
    async promptForElementName(element) {
        return new Promise((resolve) => {
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
                background: #2d2d2d;
                border-radius: 8px;
                padding: 20px;
                width: 400px;
                box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
            `;

            // 生成默认名称建议
            let defaultName = '';
            if (element.text) {
                defaultName = element.text.replace(/[^\u4e00-\u9fa5a-zA-Z0-9]/g, '');
            } else if (element.contentDesc) {
                defaultName = element.contentDesc.replace(/[^\u4e00-\u9fa5a-zA-Z0-9]/g, '');
            } else if (element.resourceId) {
                defaultName = element.resourceId.split('/').pop().replace(/[_-]/g, '');
            } else {
                defaultName = element.className.split('.').pop() + element.index;
            }

            dialog.innerHTML = `
                <h3 style="margin: 0 0 15px 0; color: #fff;">保存元素到Locator库</h3>
                <div style="margin-bottom: 15px; color: #aaa; font-size: 12px;">
                    ${element.toAiText()}
                </div>
                <input type="text" id="elementNameInput" 
                    placeholder="请输入元素名称（例如：登录按钮）" 
                    value="${defaultName}"
                    style="width: 100%; padding: 8px; background: #1e1e1e; border: 1px solid #444; 
                           color: #fff; border-radius: 4px; margin-bottom: 15px;">
                <div style="display: flex; justify-content: flex-end; gap: 10px;">
                    <button id="cancelBtn" style="padding: 6px 16px; background: #444; color: #fff; 
                            border: none; border-radius: 4px; cursor: pointer;">取消</button>
                    <button id="saveBtn" style="padding: 6px 16px; background: #4CAF50; color: #fff; 
                            border: none; border-radius: 4px; cursor: pointer;">保存</button>
                </div>
            `;

            modal.appendChild(dialog);
            document.body.appendChild(modal);

            const input = dialog.querySelector('#elementNameInput');
            const saveBtn = dialog.querySelector('#saveBtn');
            const cancelBtn = dialog.querySelector('#cancelBtn');

            // 自动聚焦并选中文本
            input.focus();
            input.select();

            // 保存处理
            const handleSave = () => {
                const name = input.value.trim();
                if (name) {
                    document.body.removeChild(modal);
                    resolve(name);
                } else {
                    input.style.borderColor = '#f44336';
                    input.placeholder = '名称不能为空';
                }
            };

            // 事件监听
            saveBtn.addEventListener('click', handleSave);
            cancelBtn.addEventListener('click', () => {
                document.body.removeChild(modal);
                resolve(null);
            });
            input.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') handleSave();
                if (e.key === 'Escape') {
                    document.body.removeChild(modal);
                    resolve(null);
                }
            });
        });
    }

    // 保存locators到文件
    async saveLocatorsToFile() {
        const { path, fs } = window.AppGlobals;
        const locatorDir = path.join(this.currentCasePath, 'locator');
        const locatorPath = path.join(locatorDir, 'element.json');

        // 确保locator目录存在
        try {
            await fs.mkdir(locatorDir, { recursive: true });
        } catch (err) {
            // 目录可能已存在
        }

        // 保存JSON文件
        await fs.writeFile(locatorPath, JSON.stringify(this.locators, null, 2), 'utf8');
    }

    // 更新Locator列表UI
    updateLocatorList(locators) {
        const locatorList = document.getElementById('locatorList');
        if (!locatorList) return;

        const entries = Object.entries(locators);
        
        if (entries.length === 0) {
            locatorList.innerHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">暂无保存的元素</div>
                    <div class="empty-state-hint">在元素属性页面点击"入库"保存元素</div>
                </div>
            `;
            return;
        }

        const listHTML = entries.map(([name, data]) => `
            <div class="locator-item" draggable="true" data-name="${name}">
                <div class="locator-header">
                    <span class="locator-name">${name}</span>
                    <div class="locator-actions">
                        <button class="btn-icon-small" onclick="LocatorManager.editLocator('${name}')" title="编辑">
                            <svg viewBox="0 0 24 24" width="16" height="16">
                                <path fill="currentColor" d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                            </svg>
                        </button>
                        <button class="btn-icon-small" onclick="LocatorManager.deleteLocator('${name}')" title="删除">
                            <svg viewBox="0 0 24 24" width="16" height="16">
                                <path fill="currentColor" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                            </svg>
                        </button>
                    </div>
                </div>
                <div class="locator-details">
                    ${data.text ? `<div class="locator-text">文本: ${data.text}</div>` : ''}
                    ${data.resourceId ? `<div class="locator-id">ID: ${data.resourceId.split('/').pop()}</div>` : ''}
                    <div class="locator-type">${data.className.split('.').pop()}</div>
                </div>
            </div>
        `).join('');

        locatorList.innerHTML = listHTML;

        // 为每个元素添加拖动事件
        locatorList.querySelectorAll('.locator-item').forEach(item => {
            this.setupItemDragEvents(item);
        });
    }

    // 设置拖放功能
    setupDragAndDrop() {
        // 设置编辑器为放置目标
        document.addEventListener('DOMContentLoaded', () => {
            const editorTextarea = document.getElementById('editorTextarea');
            if (editorTextarea) {
                editorTextarea.addEventListener('dragover', (e) => {
                    e.preventDefault();
                    e.dataTransfer.dropEffect = 'copy';
                });

                editorTextarea.addEventListener('drop', (e) => {
                    e.preventDefault();
                    const elementName = e.dataTransfer.getData('text/plain');
                    if (elementName && elementName.startsWith('[') && elementName.endsWith(']')) {
                        // 在光标位置插入文本
                        const start = editorTextarea.selectionStart;
                        const end = editorTextarea.selectionEnd;
                        const text = editorTextarea.value;
                        const before = text.substring(0, start);
                        const after = text.substring(end);
                        
                        editorTextarea.value = before + elementName + after;
                        
                        // 设置光标位置
                        const newPos = start + elementName.length;
                        editorTextarea.selectionStart = newPos;
                        editorTextarea.selectionEnd = newPos;
                        
                        // 触发input事件以更新编辑器
                        editorTextarea.dispatchEvent(new Event('input'));
                        
                        window.NotificationModule.showNotification(`已插入元素引用: ${elementName}`, 'success');
                    }
                });
            }
        });
    }

    // 设置单个元素的拖动事件
    setupItemDragEvents(item) {
        item.addEventListener('dragstart', (e) => {
            const name = item.dataset.name;
            e.dataTransfer.effectAllowed = 'copy';
            e.dataTransfer.setData('text/plain', `[${name}]`);
            item.style.opacity = '0.5';
        });

        item.addEventListener('dragend', (e) => {
            item.style.opacity = '';
        });
    }

    // 设置搜索功能
    setupSearch() {
        const searchInput = document.getElementById('locatorSearchInput');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterLocators(e.target.value);
            });
        }
    }

    // 过滤locator列表
    filterLocators(searchTerm) {
        const items = document.querySelectorAll('.locator-item');
        const term = searchTerm.toLowerCase();

        items.forEach(item => {
            const name = item.dataset.name.toLowerCase();
            const text = item.textContent.toLowerCase();
            
            if (name.includes(term) || text.includes(term)) {
                item.style.display = '';
            } else {
                item.style.display = 'none';
            }
        });

        // 如果没有匹配项，显示提示
        const visibleItems = Array.from(items).filter(item => item.style.display !== 'none');
        const locatorList = document.getElementById('locatorList');
        
        if (visibleItems.length === 0 && items.length > 0) {
            if (!locatorList.querySelector('.no-results')) {
                const noResults = document.createElement('div');
                noResults.className = 'no-results empty-state';
                noResults.innerHTML = `<div class="empty-state-text">没有找到匹配的元素</div>`;
                locatorList.appendChild(noResults);
            }
        } else {
            const noResults = locatorList.querySelector('.no-results');
            if (noResults) noResults.remove();
        }
    }

    // 删除locator
    async deleteLocator(name) {
        if (!confirm(`确定要删除元素 "${name}" 吗？`)) return;

        delete this.locators[name];
        await this.saveLocatorsToFile();
        this.updateLocatorList(this.locators);
        
        window.NotificationModule.showNotification(`元素 "${name}" 已删除`, 'info');
    }

    // 编辑locator名称
    async editLocator(oldName) {
        const newName = await this.promptForNewName(oldName);
        if (!newName || newName === oldName) return;

        if (this.locators[newName]) {
            window.NotificationModule.showNotification(`名称 "${newName}" 已存在`, 'warning');
            return;
        }

        this.locators[newName] = this.locators[oldName];
        delete this.locators[oldName];
        
        await this.saveLocatorsToFile();
        this.updateLocatorList(this.locators);
        
        window.NotificationModule.showNotification(`元素已重命名为 "${newName}"`, 'success');
    }

    // 获取新名称
    async promptForNewName(oldName) {
        return new Promise((resolve) => {
            const newName = prompt(`重命名元素 "${oldName}"`, oldName);
            resolve(newName ? newName.trim() : null);
        });
    }

    // 切换到Locator库标签页
    switchToLocatorTab() {
        const locatorTab = document.getElementById('locatorLibTab');
        if (locatorTab) {
            locatorTab.click();
        }
    }
}

// 创建全局实例
const locatorManager = new LocatorManager();

// 确保函数在DOM加载完成后绑定
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        // 导出全局函数供HTML调用
        window.saveElementToLocator = async function(index) {
            console.log('saveElementToLocator called with index:', index);
            const element = window.currentUIElements?.find(el => el.index === index);
            if (!element) {
                console.error('Element not found:', index, window.currentUIElements);
                window.NotificationModule.showNotification('元素未找到', 'error');
                return;
            }
            
            console.log('Saving element:', element);
            await locatorManager.saveElement(element);
        };
    });
} else {
    // 导出全局函数供HTML调用
    window.saveElementToLocator = async function(index) {
        console.log('saveElementToLocator called with index:', index);
        const element = window.currentUIElements?.find(el => el.index === index);
        if (!element) {
            console.error('Element not found:', index, window.currentUIElements);
            window.NotificationModule.showNotification('元素未找到', 'error');
            return;
        }
        
        console.log('Saving element:', element);
        await locatorManager.saveElement(element);
    };
}

// 静态方法供HTML调用
window.LocatorManager = {
    instance: locatorManager,
    
    deleteLocator: (name) => locatorManager.deleteLocator(name),
    editLocator: (name) => locatorManager.editLocator(name),
    
    // 初始化方法
    initialize: async () => {
        await locatorManager.initialize();
        
        // 监听文件打开事件，自动加载对应的locator
        document.addEventListener('fileOpened', (e) => {
            if (e.detail && e.detail.path) {
                const pathParts = e.detail.path.split(window.AppGlobals.path.sep);
                const caseIndex = pathParts.indexOf('cases');
                if (caseIndex !== -1 && caseIndex < pathParts.length - 2) {
                    const casePath = pathParts.slice(0, caseIndex + 2).join(window.AppGlobals.path.sep);
                    locatorManager.setCurrentCase(casePath);
                }
            }
        });
    }
};

// 导出模块
window.LocatorManagerModule = window.LocatorManager;