// UI元素列表面板管理器
// 负责显示当前页面的所有UI元素列表

const ElementsListPanel = {
    // 当前元素列表
    currentElements: [],
    
    // 初始化
    init() {
        // 监听UI元素更新事件
        document.addEventListener('uiElementsUpdated', (event) => {
            this.updateElements(event.detail.elements);
            
            // 确保底部面板可见并设置正确高度
            const bottomPanel = document.getElementById('uiElementsBottomPanel');
            if (bottomPanel) {
                bottomPanel.style.display = 'flex';
                bottomPanel.style.height = '300px'; // 设置合适的高度
                bottomPanel.classList.remove('collapsed');
                
                // 确保UI元素标签页处于激活状态
                const elementsTab = document.querySelector('.tab-btn[data-tab="elements-list"]');
                const elementsPane = document.getElementById('elementsListPane');
                if (elementsTab && !elementsTab.classList.contains('active')) {
                    elementsTab.click(); // 激活UI元素标签页
                } else if (elementsPane) {
                    // 如果已经激活，确保面板显示
                    elementsPane.style.display = 'flex';
                    elementsPane.classList.add('active');
                }
            }
        });
        
        // 绑定搜索功能
        const searchInput = document.querySelector('#elementsListPane .search-input');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterElements(e.target.value);
            });
        }
        
        // 初始渲染空状态
        this.renderElements();
    },
    
    // 更新元素列表（由device-screen-manager调用）
    updateElements(elements) {
        this.currentElements = elements;
        this.renderElements();
    },
    
    // 渲染元素列表
    renderElements(filteredElements = null) {
        const container = document.getElementById('elementsListContainer');
        if (!container) {
            window.rError('元素列表容器(elementsListContainer)未找到');
            return;
        }
        
        const elements = filteredElements || this.currentElements;
        
        if (elements.length === 0) {
            container.innerHTML = `
                <div class="empty-state">
                    <div class="empty-state-text">暂无UI元素</div>
                    <div class="empty-state-hint">点击上方"XML Overlay"按钮获取页面元素</div>
                </div>
            `;
            return;
        }
        
        // 计算元素的尺寸和中心点
        const elementsWithSize = elements.map(el => {
            const width = el.bounds ? (el.bounds[2] - el.bounds[0]) : (el.width || 0);
            const height = el.bounds ? (el.bounds[3] - el.bounds[1]) : (el.height || 0);
            const centerX = el.bounds ? Math.round((el.bounds[0] + el.bounds[2]) / 2) : (el.centerX || 0);
            const centerY = el.bounds ? Math.round((el.bounds[1] + el.bounds[3]) / 2) : (el.centerY || 0);
            
            return { ...el, width, height, centerX, centerY };
        });
        
        const elementsHTML = elementsWithSize.map((el, idx) => `
            <div class="element-item" data-index="${idx}">
                <div class="element-main" onclick="window.ElementsListPanel.selectElement(${idx})">
                    <div class="element-header">
                        <span class="element-index">[${el.index}]</span>
                        <span class="element-type">${el.className ? el.className.split('.').pop() : 'Unknown'}</span>
                    </div>
                    ${el.text ? `<div class="element-text">文本: ${this.escapeHtml(el.text)}</div>` : ''}
                    ${el.contentDesc ? `<div class="element-desc">描述: ${this.escapeHtml(el.contentDesc)}</div>` : ''}
                    ${el.hint ? `<div class="element-hint">提示: ${this.escapeHtml(el.hint)}</div>` : ''}
                    <div class="element-size">${el.width}×${el.height} @ (${el.centerX},${el.centerY})</div>
                </div>
                <div class="element-actions">
                    <button class="btn-icon-small save-to-locator-btn" 
                            onclick="event.stopPropagation(); window.LocatorLibraryPanel.saveElementToLocator(${idx})" 
                            title="入库"
                            style="background: transparent; border: none; padding: 4px;">
                        <svg viewBox="0 0 24 24" width="20" height="20">
                            <path fill="#FF9800" d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
                        </svg>
                    </button>
                </div>
            </div>
        `).join('');
        
        container.innerHTML = elementsHTML;
        window.rLog(`✅ 已将 ${elements.length} 个UI元素显示在UI库中`);
    },
    
    // 选择元素
    selectElement(index) {
        const element = this.currentElements[index];
        if (!element) return;
        
        // 调用全局的选择函数（由device-screen-manager导出）
        if (window.selectElementByIndex) {
            window.selectElementByIndex(index);
        }
        
        // 高亮列表中的元素
        const items = document.querySelectorAll('.element-item');
        items.forEach((item, i) => {
            if (i === index) {
                item.classList.add('selected');
                item.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            } else {
                item.classList.remove('selected');
            }
        });
    },
    
    // 筛选元素
    filterElements(searchText) {
        if (!searchText) {
            this.renderElements();
            return;
        }
        
        const searchLower = searchText.toLowerCase();
        const filtered = this.currentElements.filter(el => {
            return (el.text && el.text.toLowerCase().includes(searchLower)) ||
                   (el.contentDesc && el.contentDesc.toLowerCase().includes(searchLower)) ||
                   (el.className && el.className.toLowerCase().includes(searchLower)) ||
                   (el.resourceId && el.resourceId.toLowerCase().includes(searchLower)) ||
                   (el.hint && el.hint.toLowerCase().includes(searchLower));
        });
        
        this.renderElements(filtered);
    },
    
    // HTML转义
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
};

// 导出到全局
window.ElementsListPanel = ElementsListPanel;

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    ElementsListPanel.init();
});