// 元素属性面板管理器
// 负责显示选中元素的详细属性信息

const ElementPropertiesPanel = {
    // 初始化
    init() {
        // 监听元素选择事件
        document.addEventListener('elementSelected', (event) => {
            this.showElementProperties(event.detail.element);
        });
    },
    
    // 显示元素属性
    showElementProperties(element) {
        const elementPropsTab = document.getElementById('elementPropsTab');
        const elementPropsPane = document.getElementById('elementPropsPane');
        const elementPropsContainer = document.getElementById('elementPropsContainer');
        const elementsListTab = document.querySelector('.tab-btn[data-tab="elements-list"]');
        const elementsListPane = document.getElementById('elementsListPane');
        
        if (!elementPropsTab || !elementPropsPane || !elementPropsContainer) {
            window.rError('元素属性面板组件未找到');
            return;
        }
        
        // 生成属性面板HTML
        const propertiesHTML = `
            <div class="element-details">
                <div class="prop-group">
                    <h4 class="prop-title">基本信息</h4>
                    <div class="prop-item"><strong>索引:</strong> [${element.index}]</div>
                    <div class="prop-item"><strong>类型:</strong> ${element.className || 'Unknown'}</div>
                    <div class="prop-item"><strong>包名:</strong> ${element.package || 'N/A'}</div>
                </div>
                
                <div class="prop-group">
                    <h4 class="prop-title">位置信息</h4>
                    <div class="prop-item"><strong>中心点:</strong> (${element.centerX || 0}, ${element.centerY || 0})</div>
                    <div class="prop-item"><strong>边界:</strong> [${element.bounds ? element.bounds.join(', ') : 'N/A'}]</div>
                    <div class="prop-item"><strong>尺寸:</strong> ${element.width || 0} × ${element.height || 0}</div>
                </div>
                
                ${element.text || element.contentDesc || element.hint ? `
                    <div class="prop-group">
                        <h4 class="prop-title">文本信息</h4>
                        ${element.text ? `<div class="prop-item"><strong>文本:</strong> ${element.text}</div>` : ''}
                        ${element.contentDesc ? `<div class="prop-item"><strong>描述:</strong> ${element.contentDesc}</div>` : ''}
                        ${element.hint ? `<div class="prop-item"><strong>提示:</strong> ${element.hint}</div>` : ''}
                    </div>
                ` : ''}
                
                ${element.resourceId ? `
                    <div class="prop-group">
                        <h4 class="prop-title">资源信息</h4>
                        <div class="prop-item"><strong>资源ID:</strong> ${element.resourceId}</div>
                    </div>
                ` : ''}
                
                <div class="prop-group">
                    <h4 class="prop-title">状态</h4>
                    <div class="prop-item">
                        <span class="status-item ${element.clickable ? 'status-true' : 'status-false'}">
                            ${element.clickable ? '✓' : '✗'} 可点击
                        </span>
                        <span class="status-item ${element.enabled ? 'status-true' : 'status-false'}">
                            ${element.enabled ? '✓' : '✗'} 已启用
                        </span>
                    </div>
                    <div class="prop-item">
                        <span class="status-item ${element.focusable ? 'status-true' : 'status-false'}">
                            ${element.focusable ? '✓' : '✗'} 可获焦点
                        </span>
                        <span class="status-item ${element.scrollable ? 'status-true' : 'status-false'}">
                            ${element.scrollable ? '✓' : '✗'} 可滚动
                        </span>
                    </div>
                    <div class="prop-item">
                        <span class="status-item ${element.checkable ? 'status-true' : 'status-false'}">
                            ${element.checkable ? '✓' : '✗'} 可勾选
                        </span>
                        <span class="status-item ${element.checked ? 'status-true' : 'status-false'}">
                            ${element.checked ? '✓' : '✗'} 已勾选
                        </span>
                    </div>
                </div>
                
                <div class="prop-group">
                    <h4 class="prop-title">操作</h4>
                    <div class="action-buttons">
                        <button class="btn btn-primary" onclick="window.LocatorLibraryPanel.saveElementToLocator(${element.index})">
                            <svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;">
                                <path fill="currentColor" d="M17 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V7l-4-4zm-5 16c-1.66 0-3-1.34-3-3s1.34-3 3-3 3 1.34 3 3-1.34 3-3 3zm3-10H5V5h10v4z"/>
                            </svg>
                            入库
                        </button>
                        <button class="btn btn-secondary" onclick="window.ElementPropertiesPanel.copyElementInfo(${element.index})">
                            <svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;">
                                <path fill="currentColor" d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                            </svg>
                            复制
                        </button>
                    </div>
                </div>
            </div>
        `;
        
        // 更新属性容器内容
        elementPropsContainer.innerHTML = propertiesHTML;
        
        // 切换到属性标签页
        elementsListTab.classList.remove('active');
        elementPropsTab.classList.add('active');
        elementsListPane.style.display = 'none';
        elementsListPane.classList.remove('active');
        elementPropsPane.style.display = 'block';
        elementPropsPane.classList.add('active');
        
        window.rLog(`显示元素 [${element.index}] 的属性`);
    },
    
    // 复制元素信息到剪贴板
    async copyElementInfo(elementIndex) {
        // 从ElementsListPanel获取元素，使用元素的index属性而不是数组索引
        const element = window.ElementsListPanel?.currentElements.find(el => el.index === elementIndex);
        if (!element) {
            window.rError(`无法获取index为${elementIndex}的元素信息`);
            return;
        }
        
        const info = `元素 [${element.index}]
类型: ${element.className || 'Unknown'}
位置: [${element.bounds ? element.bounds.join(', ') : 'N/A'}]
文本: ${element.text || 'N/A'}
描述: ${element.contentDesc || 'N/A'}
可点击: ${element.clickable ? '是' : '否'}`;
        
        try {
            await navigator.clipboard.writeText(info);
            window.NotificationModule.showNotification('元素信息已复制到剪贴板', 'success');
        } catch (err) {
            window.rError('复制失败:', err);
            window.NotificationModule.showNotification('复制失败', 'error');
        }
    },
    
    // 清空属性面板
    clear() {
        const elementPropsContainer = document.getElementById('elementPropsContainer');
        if (elementPropsContainer) {
            elementPropsContainer.innerHTML = '<div class="empty-state"><div class="empty-state-text">请选择一个元素查看属性</div></div>';
        }
    }
};

// 导出到全局
window.ElementPropertiesPanel = ElementPropertiesPanel;

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    ElementPropertiesPanel.init();
});