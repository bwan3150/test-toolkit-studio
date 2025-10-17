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

            // 确保底部面板展开并切换到UI元素Tab
            if (window.BottomPanelManager) {
                window.BottomPanelManager.expand();
                window.BottomPanelManager.switchTab('elements-list');
            }
        });

        // 初始渲染空状态
        this.renderElements();

        window.rLog('✅ ElementsListPanel 初始化完成');
    },
    
    // 更新元素列表（由device-screen-manager调用）
    updateElements(elements) {
        this.currentElements = elements;
        this.renderElements();
    },
    
    // 渲染元素列表 - 全新的表格样式
    renderElements(filteredElements = null) {
        const container = document.getElementById('elementsListContent');
        if (!container) {
            window.rError('元素列表容器(elementsListContent)未找到');
            return;
        }

        const elements = filteredElements || this.currentElements;

        if (elements.length === 0) {
            // 空状态 - 显示表格结构但无数据
            container.innerHTML = `
                <div class="table-empty-state">
                    <table class="elements-table">
                        <thead>
                            <tr>
                                <th class="col-index">#</th>
                                <th class="col-type">类型</th>
                                <th class="col-content">内容</th>
                                <th class="col-bounds">位置</th>
                                <th class="col-size">尺寸</th>
                                <th class="col-actions">操作</th>
                            </tr>
                        </thead>
                    </table>
                    <div class="empty-message">
                        <div class="empty-icon">
                            <svg viewBox="0 0 48 48" width="48" height="48">
                                <rect x="8" y="12" width="32" height="24" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
                                <line x1="8" y1="18" x2="40" y2="18" stroke="currentColor" stroke-width="2"/>
                                <line x1="16" y1="24" x2="32" y2="24" stroke="currentColor" stroke-width="1.5"/>
                                <line x1="16" y1="28" x2="28" y2="28" stroke="currentColor" stroke-width="1.5"/>
                                <line x1="16" y1="32" x2="30" y2="32" stroke="currentColor" stroke-width="1.5"/>
                            </svg>
                        </div>
                        <div class="empty-title">No elements captured</div>
                    </div>
                </div>
            `;
            return;
        }

        // 计算元素的尺寸和中心点
        const elementsWithSize = elements.map(el => {
            const width = el.bounds.x2 - el.bounds.x1;
            const height = el.bounds.y2 - el.bounds.y1;
            const centerX = Math.round((el.bounds.x1 + el.bounds.x2) / 2);
            const centerY = Math.round((el.bounds.y1 + el.bounds.y2) / 2);
            return { ...el, width, height, centerX, centerY };
        });

        // 全新的表格式布局
        const tableHTML = `
            <table class="elements-table">
                <thead>
                    <tr>
                        <th class="col-index">#</th>
                        <th class="col-type">类型</th>
                        <th class="col-content">内容</th>
                        <th class="col-bounds">位置</th>
                        <th class="col-size">尺寸</th>
                        <th class="col-actions">操作</th>
                    </tr>
                </thead>
                <tbody>
                    ${elementsWithSize.map((el, idx) => `
                        <tr class="element-row" data-index="${idx}" onclick="window.ElementsListPanel.selectElement(${idx})">
                            <td class="col-index">
                                <span class="index-badge">${el.index}</span>
                            </td>
                            <td class="col-type">
                                <div class="type-cell">
                                    <span class="type-icon">${this.getTypeIcon(el)}</span>
                                    <span class="type-name">${el.className ? el.className.split('.').pop() : 'Unknown'}</span>
                                </div>
                            </td>
                            <td class="col-content">
                                <div class="content-cell">
                                    ${el.text ? `<div class="content-text" title="${this.escapeHtml(el.text)}">${this.escapeHtml(this.truncate(el.text, 30))}</div>` : ''}
                                    ${el.contentDesc ? `<div class="content-desc" title="${this.escapeHtml(el.contentDesc)}">desc: ${this.escapeHtml(this.truncate(el.contentDesc, 20))}</div>` : ''}
                                    ${!el.text && !el.contentDesc ? '<span class="text-muted">—</span>' : ''}
                                </div>
                            </td>
                            <td class="col-bounds">
                                <code class="bounds-code">(${el.centerX}, ${el.centerY})</code>
                            </td>
                            <td class="col-size">
                                <code class="size-code">${el.width}×${el.height}</code>
                            </td>
                            <td class="col-actions">
                                <button class="action-btn"
                                        onclick="event.stopPropagation(); window.LocatorLibraryPanel.saveElementToLocator(${el.index})"
                                        title="保存到元素库">
                                    <svg viewBox="0 0 16 16" width="14" height="14">
                                        <path fill="currentColor" d="M13.5 2h-11C1.67 2 1 2.67 1 3.5v9c0 .83.67 1.5 1.5 1.5h11c.83 0 1.5-.67 1.5-1.5v-9c0-.83-.67-1.5-1.5-1.5zM8 11.5c-1.38 0-2.5-1.12-2.5-2.5S6.62 6.5 8 6.5s2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5zM11 5H3V3h8v2z"/>
                                    </svg>
                                </button>
                            </td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        `;

        container.innerHTML = tableHTML;
        window.rLog(`✅ 已将 ${elements.length} 个UI元素显示在表格中`);
    },

    // 获取类型图标 - JetBrains风格SVG图标
    getTypeIcon(element) {
        const className = element.className?.toLowerCase() || '';
        if (className.includes('button')) {
            return '<svg class="type-icon-svg" viewBox="0 0 16 16"><rect x="2" y="5" width="12" height="6" rx="2" fill="currentColor"/></svg>';
        }
        if (className.includes('text') || className.includes('edit')) {
            return '<svg class="type-icon-svg" viewBox="0 0 16 16"><path d="M4 3h8v2H4V3zm0 4h8v2H4V7zm0 4h5v2H4v-2z" fill="currentColor"/></svg>';
        }
        if (className.includes('image')) {
            return '<svg class="type-icon-svg" viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="6" cy="6" r="1.5" fill="currentColor"/><path d="M2 12l3-3 2 2 4-4 3 3v2H2v-2z" fill="currentColor"/></svg>';
        }
        if (className.includes('view') || className.includes('layout')) {
            return '<svg class="type-icon-svg" viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>';
        }
        if (className.includes('list')) {
            return '<svg class="type-icon-svg" viewBox="0 0 16 16"><path d="M3 3h1v1H3V3zm2 0h8v1H5V3zM3 7h1v1H3V7zm2 0h8v1H5V7zm-2 4h1v1H3v-1zm2 0h8v1H5v-1z" fill="currentColor"/></svg>';
        }
        if (className.includes('scroll')) {
            return '<svg class="type-icon-svg" viewBox="0 0 16 16"><rect x="2" y="2" width="10" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="13" y="4" width="1" height="4" rx="0.5" fill="currentColor"/></svg>';
        }
        return '<svg class="type-icon-svg" viewBox="0 0 16 16"><rect x="6" y="6" width="4" height="4" fill="currentColor"/></svg>';
    },

    // 截断文本
    truncate(text, maxLength) {
        if (!text) return '';
        return text.length > maxLength ? text.substring(0, maxLength) + '...' : text;
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
