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
        document.addEventListener('projectChanged', () => {
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
    async saveElementToLocator(index) {
        // 从当前UI元素列表获取元素
        let element;
        if (window.ElementsListPanel && window.ElementsListPanel.currentElements) {
            element = window.ElementsListPanel.currentElements[index];
        }
        
        if (!element) {
            window.NotificationModule.showNotification('元素不存在', 'error');
            return;
        }
        
        // 生成定位器名称
        const name = await this.promptForLocatorName(element);
        if (!name) return;
        
        // 创建定位器对象
        const locator = {
            type: 'xml',
            name: name,
            className: element.className || '',
            text: element.text || '',
            contentDesc: element.contentDesc || '',
            resourceId: element.resourceId || '',
            bounds: element.bounds || [],
            clickable: element.clickable || false,
            enabled: element.enabled || false,
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
        
        window.NotificationModule.showNotification(`定位器 "${name}" 已保存`, 'success');
    },
    
    // 提示输入定位器名称
    async promptForLocatorName(element) {
        // 生成默认名称
        const defaultName = element.text || element.contentDesc || 
                          element.className?.split('.').pop() || 
                          `element_${Date.now()}`;
        
        // 使用简单的prompt（实际可以用更好的模态框）
        const name = prompt('请输入定位器名称:', defaultName);
        
        if (!name) return null;
        
        // 检查是否已存在
        if (this.locators[name]) {
            const overwrite = confirm(`定位器 "${name}" 已存在，是否覆盖？`);
            if (!overwrite) return null;
        }
        
        return name;
    },
    
    // 渲染定位器列表
    renderLocators(filteredLocators = null) {
        const locatorList = document.getElementById('locatorList');
        if (!locatorList) return;
        
        const locatorsToRender = filteredLocators || Object.entries(this.locators);
        
        if (locatorsToRender.length === 0) {
            locatorList.innerHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">暂无保存的定位器</div>
                    <div class="empty-state-hint">在UI元素列表中点击"入库"保存定位器</div>
                </div>
            `;
            return;
        }
        
        const locatorsHTML = locatorsToRender.map(([name, locator]) => `
            <div class="locator-card ${locator.type}-type" data-name="${name}">
                <div class="locator-header">
                    <span class="locator-name">${this.escapeHtml(name)}</span>
                    <span class="locator-type">${locator.type === 'xml' ? 'XML' : '图像'}</span>
                </div>
                <div class="locator-info">
                    ${locator.type === 'xml' ? `
                        ${locator.className ? `<div class="info-item">类型: ${locator.className.split('.').pop()}</div>` : ''}
                        ${locator.resourceId ? `<div class="info-item">ID: ${locator.resourceId}</div>` : ''}
                        ${locator.text ? `<div class="info-item">文本: ${this.escapeHtml(locator.text)}</div>` : ''}
                        ${locator.contentDesc ? `<div class="info-item">描述: ${this.escapeHtml(locator.contentDesc)}</div>` : ''}
                    ` : `
                        <div class="info-item">图像定位器</div>
                        ${locator.threshold ? `<div class="info-item">阈值: ${locator.threshold}</div>` : ''}
                    `}
                </div>
                <div class="locator-actions">
                    <button class="btn-small" onclick="window.LocatorLibraryPanel.useLocator('${name}')">使用</button>
                    <button class="btn-small" onclick="window.LocatorLibraryPanel.deleteLocator('${name}')">删除</button>
                </div>
            </div>
        `).join('');
        
        locatorList.innerHTML = locatorsHTML;
    },
    
    // 使用定位器（插入到编辑器）
    useLocator(name) {
        const locator = this.locators[name];
        if (!locator) return;
        
        // 生成定位器代码
        let code = '';
        if (locator.type === 'xml') {
            if (locator.resourceId) {
                code = `click_element_by_id("${locator.resourceId}")`;
            } else if (locator.text) {
                code = `click_element_by_text("${locator.text}")`;
            } else if (locator.contentDesc) {
                code = `click_element_by_desc("${locator.contentDesc}")`;
            } else {
                code = `click_element_by_locator("${name}")`;
            }
        } else {
            code = `click_image("${name}")`;
        }
        
        // 如果有活动的编辑器，插入代码
        if (window.UnifiedEditorModule && window.UnifiedEditorModule.insertCode) {
            window.UnifiedEditorModule.insertCode(code);
            window.NotificationModule.showNotification('定位器已插入到编辑器', 'success');
        } else {
            // 复制到剪贴板
            navigator.clipboard.writeText(code);
            window.NotificationModule.showNotification('定位器代码已复制到剪贴板', 'success');
        }
    },
    
    // 删除定位器
    async deleteLocator(name) {
        if (!confirm(`确定要删除定位器 "${name}" 吗？`)) return;
        
        // 如果是图像定位器，删除图像文件
        if (this.locators[name].type === 'image') {
            try {
                const fs = window.nodeRequire('fs');
                const path = window.AppGlobals.path;
                const projectPath = window.AppGlobals.currentProject;
                const imgPath = path.join(projectPath, 'locator', 'img', `${name}.png`);
                
                if (fs.existsSync(imgPath)) {
                    fs.unlinkSync(imgPath);
                    window.rLog(`删除图像文件: ${imgPath}`);
                }
            } catch (error) {
                window.rError('删除图像文件失败:', error);
            }
        }
        
        delete this.locators[name];
        await this.saveLocators();
        this.renderLocators();
        
        window.NotificationModule.showNotification(`定位器 "${name}" 已删除`, 'info');
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
                   (locator.contentDesc && locator.contentDesc.toLowerCase().includes(searchLower)) ||
                   (locator.className && locator.className.toLowerCase().includes(searchLower));
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