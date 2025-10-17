// 元素属性面板管理器
// 负责显示选中元素的详细属性信息

const ElementPropertiesPanel = {
    // 初始化
    init() {
        // 监听元素选择事件
        document.addEventListener('elementSelected', (event) => {
            this.showElementProperties(event.detail.element);
        });

        // 初始化显示空状态
        this.clear();

        window.rLog('✅ ElementPropertiesPanel 初始化完成');
    },
    
    // 显示元素属性 - IntelliJ风格属性表格
    showElementProperties(element) {
        const elementPropsContainer = document.getElementById('elementPropsContainer');

        if (!elementPropsContainer) {
            window.rError('元素属性容器未找到: elementPropsContainer');
            return;
        }

        // IntelliJ风格的属性表格
        const propertiesHTML = `
            <div class="properties-panel">
                <!-- 工具栏 -->
                <div class="properties-toolbar">
                    <div class="toolbar-left">
                        <span class="element-id">[${element.index}] ${element.className ? element.className.split('.').pop() : 'Element'}</span>
                    </div>
                    <div class="toolbar-right">
                        <button class="tool-btn" onclick="window.LocatorLibraryPanel.saveElementToLocator(${element.index})" title="保存到元素库">
                            <svg viewBox="0 0 16 16" width="16" height="16">
                                <path fill="currentColor" d="M13.5 2h-11C1.67 2 1 2.67 1 3.5v9c0 .83.67 1.5 1.5 1.5h11c.83 0 1.5-.67 1.5-1.5v-9c0-.83-.67-1.5-1.5-1.5zM8 11.5c-1.38 0-2.5-1.12-2.5-2.5S6.62 6.5 8 6.5s2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5zM11 5H3V3h8v2z"/>
                            </svg>
                        </button>
                        <button class="tool-btn" onclick="window.ElementPropertiesPanel.copyElementInfo(${element.index})" title="复制属性">
                            <svg viewBox="0 0 16 16" width="16" height="16">
                                <path fill="currentColor" d="M11 1H3c-.55 0-1 .45-1 1v10h1V2h8V1zm2 3H6c-.55 0-1 .45-1 1v10c0 .55.45 1 1 1h7c.55 0 1-.45 1-1V5c0-.55-.45-1-1-1zm0 11H6V5h7v10z"/>
                            </svg>
                        </button>
                    </div>
                </div>

                <!-- 属性表格 -->
                <table class="properties-table">
                    <tbody>
                        <tr class="prop-category">
                            <td colspan="2">General</td>
                        </tr>
                        <tr>
                            <td class="prop-key">index</td>
                            <td class="prop-value">${element.index}</td>
                        </tr>
                        <tr>
                            <td class="prop-key">className</td>
                            <td class="prop-value"><code>${element.className || 'N/A'}</code></td>
                        </tr>
                        <tr>
                            <td class="prop-key">package</td>
                            <td class="prop-value"><code>${element.package || 'N/A'}</code></td>
                        </tr>
                        ${element.resourceId ? `
                        <tr>
                            <td class="prop-key">resourceId</td>
                            <td class="prop-value"><code>${element.resourceId}</code></td>
                        </tr>` : ''}

                        <tr class="prop-category">
                            <td colspan="2">Bounds & Size</td>
                        </tr>
                        <tr>
                            <td class="prop-key">center</td>
                            <td class="prop-value"><code>(${element.centerX || 0}, ${element.centerY || 0})</code></td>
                        </tr>
                        <tr>
                            <td class="prop-key">bounds</td>
                            <td class="prop-value"><code>[${element.bounds ? `${element.bounds.x1}, ${element.bounds.y1}, ${element.bounds.x2}, ${element.bounds.y2}` : 'N/A'}]</code></td>
                        </tr>
                        <tr>
                            <td class="prop-key">size</td>
                            <td class="prop-value"><code>${element.width || 0} × ${element.height || 0}</code></td>
                        </tr>

                        ${element.text || element.contentDesc || element.hint ? `
                        <tr class="prop-category">
                            <td colspan="2">Text Content</td>
                        </tr>
                        ${element.text ? `
                        <tr>
                            <td class="prop-key">text</td>
                            <td class="prop-value">"${this.escapeHtml(element.text)}"</td>
                        </tr>` : ''}
                        ${element.contentDesc ? `
                        <tr>
                            <td class="prop-key">contentDesc</td>
                            <td class="prop-value">"${this.escapeHtml(element.contentDesc)}"</td>
                        </tr>` : ''}
                        ${element.hint ? `
                        <tr>
                            <td class="prop-key">hint</td>
                            <td class="prop-value">"${this.escapeHtml(element.hint)}"</td>
                        </tr>` : ''}
                        ` : ''}

                        <tr class="prop-category">
                            <td colspan="2">State Flags</td>
                        </tr>
                        <tr>
                            <td class="prop-key">clickable</td>
                            <td class="prop-value"><span class="bool-value ${element.clickable ? 'bool-true' : 'bool-false'}">${element.clickable}</span></td>
                        </tr>
                        <tr>
                            <td class="prop-key">enabled</td>
                            <td class="prop-value"><span class="bool-value ${element.enabled ? 'bool-true' : 'bool-false'}">${element.enabled}</span></td>
                        </tr>
                        <tr>
                            <td class="prop-key">focusable</td>
                            <td class="prop-value"><span class="bool-value ${element.focusable ? 'bool-true' : 'bool-false'}">${element.focusable}</span></td>
                        </tr>
                        <tr>
                            <td class="prop-key">scrollable</td>
                            <td class="prop-value"><span class="bool-value ${element.scrollable ? 'bool-true' : 'bool-false'}">${element.scrollable}</span></td>
                        </tr>
                        <tr>
                            <td class="prop-key">checkable</td>
                            <td class="prop-value"><span class="bool-value ${element.checkable ? 'bool-true' : 'bool-false'}">${element.checkable}</span></td>
                        </tr>
                        <tr>
                            <td class="prop-key">checked</td>
                            <td class="prop-value"><span class="bool-value ${element.checked ? 'bool-true' : 'bool-false'}">${element.checked}</span></td>
                        </tr>
                    </tbody>
                </table>
            </div>
        `;

        // 更新属性容器内容
        elementPropsContainer.innerHTML = propertiesHTML;

        // 使用BottomPanelManager切换到属性Tab
        if (window.BottomPanelManager) {
            window.BottomPanelManager.switchTab('element-props');
        }

        window.rLog(`显示元素 [${element.index}] 的属性`);
    },

    // HTML转义
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    },
    
    // 复制元素信息到剪贴板
    async copyElementInfo(elementIndex) {
        // 从ElementsListPanel获取元素，使用元素的index属性而不是数组索引
        const element = window.ElementsListPanel?.currentElements.find(el => el.index === elementIndex);
        if (!element) {
            window.rError(`无法获取index为${elementIndex}的元素信息`);
            return;
        }
        
        const boundsStr = element.bounds ?
            `[${element.bounds.x1}, ${element.bounds.y1}, ${element.bounds.x2}, ${element.bounds.y2}]` :
            'N/A';

        const info = `元素 [${element.index}]
类型: ${element.className || 'Unknown'}
位置: ${boundsStr}
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
            elementPropsContainer.innerHTML = `
                <div class="properties-empty-state">
                    <div class="empty-icon">
                        <svg viewBox="0 0 48 48" width="48" height="48">
                            <rect x="10" y="8" width="28" height="32" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
                            <line x1="16" y1="16" x2="32" y2="16" stroke="currentColor" stroke-width="1.5"/>
                            <line x1="16" y1="21" x2="28" y2="21" stroke="currentColor" stroke-width="1.5"/>
                            <line x1="16" y1="26" x2="30" y2="26" stroke="currentColor" stroke-width="1.5"/>
                            <line x1="16" y1="31" x2="26" y2="31" stroke="currentColor" stroke-width="1.5"/>
                        </svg>
                    </div>
                    <div class="empty-title">No element selected</div>
                </div>
            `;
        }
    }
};

// 导出到全局
window.ElementPropertiesPanel = ElementPropertiesPanel;

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    ElementPropertiesPanel.init();
});
