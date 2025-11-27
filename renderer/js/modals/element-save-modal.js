// 元素保存模态框控制器
// 支持新建元素和合并到已有元素

const ElementSaveModal = {
    // 状态
    _currentTab: 'new',
    _selectedElement: null,
    _elementData: null,
    _saveType: null, // 'img' 或 'xml'
    _resolvePromise: null,
    _rejectPromise: null,

    /**
     * 初始化模态框
     */
    init() {
        this._bindEvents();
        window.rLog('✅ ElementSaveModal 初始化完成');
    },

    /**
     * 显示模态框
     * @param {Object} options - 配置选项
     * @param {string} options.title - 模态框标题
     * @param {string} options.saveType - 保存类型: 'img' (图片) 或 'xml' (XML元素)
     * @param {Object} options.elementData - 要保存的元素数据
     * @param {string} options.defaultName - 默认名称
     * @returns {Promise<Object|null>} 返回保存结果或 null（取消）
     */
    show(options) {
        const { title, saveType, elementData, defaultName } = options;

        this._saveType = saveType;
        this._elementData = elementData;
        this._currentTab = 'new';
        this._selectedElement = null;

        const modal = document.getElementById('elementSaveModal');
        if (!modal) {
            window.rError('ElementSaveModal: 模态框元素未找到');
            return Promise.reject(new Error('Modal not found'));
        }

        // 设置标题
        const titleEl = document.getElementById('elementSaveModalTitle');
        if (titleEl) titleEl.textContent = title || '保存元素';

        // 设置默认名称
        const nameInput = document.getElementById('newElementName');
        if (nameInput) {
            nameInput.value = defaultName || '';
        }

        // 清空备注
        const noteInput = document.getElementById('newElementNote');
        if (noteInput) noteInput.value = '';

        // 渲染元素预览
        this._renderElementPreview();

        // 渲染可合并的元素列表
        this._renderMergeList();

        // 切换到新建 tab
        this._switchTab('new');

        // 更新确认按钮文字
        this._updateConfirmButton();

        // 显示模态框
        modal.style.display = 'flex';

        // 聚焦输入框
        setTimeout(() => {
            if (nameInput) {
                nameInput.focus();
                nameInput.select();
            }
        }, 100);

        // 返回 Promise
        return new Promise((resolve, reject) => {
            this._resolvePromise = resolve;
            this._rejectPromise = reject;
        });
    },

    /**
     * 隐藏模态框
     */
    hide() {
        const modal = document.getElementById('elementSaveModal');
        if (modal) {
            modal.style.display = 'none';
        }
    },

    /**
     * 绑定事件
     */
    _bindEvents() {
        // Tab 切换
        document.addEventListener('click', (e) => {
            const tab = e.target.closest('.element-save-tab');
            if (tab) {
                const tabName = tab.dataset.tab;
                this._switchTab(tabName);
            }
        });

        // 关闭按钮
        const closeBtn = document.getElementById('closeElementSaveModal');
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this._handleCancel());
        }

        // 取消按钮
        const cancelBtn = document.getElementById('cancelElementSave');
        if (cancelBtn) {
            cancelBtn.addEventListener('click', () => this._handleCancel());
        }

        // 确认按钮
        const confirmBtn = document.getElementById('confirmElementSave');
        if (confirmBtn) {
            confirmBtn.addEventListener('click', () => this._handleConfirm());
        }

        // 搜索框
        const searchInput = document.getElementById('mergeElementSearch');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this._filterMergeList(e.target.value);
            });
        }

        // 元素列表点击
        const listContainer = document.getElementById('mergeElementList');
        if (listContainer) {
            listContainer.addEventListener('click', (e) => {
                const item = e.target.closest('.merge-element-item');
                if (item) {
                    this._selectElement(item.dataset.name);
                }
            });
        }

        // 点击背景关闭
        const modal = document.getElementById('elementSaveModal');
        if (modal) {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    this._handleCancel();
                }
            });
        }

        // ESC 关闭
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                const modal = document.getElementById('elementSaveModal');
                if (modal && modal.style.display !== 'none') {
                    this._handleCancel();
                }
            }
        });

        // Enter 确认（在新建 tab 时）
        const nameInput = document.getElementById('newElementName');
        if (nameInput) {
            nameInput.addEventListener('keydown', (e) => {
                if (e.key === 'Enter') {
                    this._handleConfirm();
                }
            });
        }

        // 覆盖确认对话框事件
        this._bindOverwriteEvents();
    },

    /**
     * 绑定覆盖确认对话框事件
     */
    _bindOverwriteEvents() {
        const closeBtn = document.getElementById('closeOverwriteModal');
        const cancelBtn = document.getElementById('cancelOverwrite');
        const confirmBtn = document.getElementById('confirmOverwrite');

        if (closeBtn) {
            closeBtn.addEventListener('click', () => this._hideOverwriteModal());
        }
        if (cancelBtn) {
            cancelBtn.addEventListener('click', () => this._hideOverwriteModal());
        }
        if (confirmBtn) {
            confirmBtn.addEventListener('click', () => this._confirmOverwrite());
        }
    },

    /**
     * 切换 Tab
     */
    _switchTab(tabName) {
        this._currentTab = tabName;

        // 更新 tab 样式
        document.querySelectorAll('.element-save-tab').forEach(tab => {
            tab.classList.toggle('active', tab.dataset.tab === tabName);
        });

        // 更新 panel 显示
        document.querySelectorAll('.element-save-panel').forEach(panel => {
            panel.classList.toggle('active', panel.dataset.panel === tabName);
        });

        // 更新确认按钮
        this._updateConfirmButton();

        // 如果切换到合并 tab，清空搜索
        if (tabName === 'merge') {
            const searchInput = document.getElementById('mergeElementSearch');
            if (searchInput) {
                searchInput.value = '';
                searchInput.focus();
            }
            this._filterMergeList('');
        }
    },

    /**
     * 更新确认按钮
     */
    _updateConfirmButton() {
        const btnText = document.getElementById('confirmBtnText');
        if (btnText) {
            if (this._currentTab === 'new') {
                btnText.textContent = '保存';
            } else {
                btnText.textContent = this._selectedElement ? '并入' : '选择元素';
            }
        }
    },

    /**
     * 渲染元素预览
     */
    _renderElementPreview() {
        const container = document.getElementById('elementPreviewContent');
        if (!container || !this._elementData) return;

        const data = this._elementData;
        let html = '';

        if (this._saveType === 'img') {
            html = `<div class="preview-item"><span class="preview-label">类型:</span><span class="preview-value">图片截图</span></div>`;
            if (data.img_path) {
                html += `<div class="preview-item"><span class="preview-label">路径:</span><span class="preview-value">${this._escapeHtml(data.img_path)}</span></div>`;
            }
        } else {
            // XML 元素
            if (data.text) {
                html += `<div class="preview-item"><span class="preview-label">文本:</span><span class="preview-value">${this._escapeHtml(data.text)}</span></div>`;
            }
            if (data.content_desc) {
                html += `<div class="preview-item"><span class="preview-label">描述:</span><span class="preview-value">${this._escapeHtml(data.content_desc)}</span></div>`;
            }
            if (data.resource_id) {
                html += `<div class="preview-item"><span class="preview-label">资源ID:</span><span class="preview-value">${this._escapeHtml(data.resource_id)}</span></div>`;
            }
            if (data.class_name) {
                html += `<div class="preview-item"><span class="preview-label">类名:</span><span class="preview-value">${this._escapeHtml(data.class_name)}</span></div>`;
            }
        }

        container.innerHTML = html || '<div class="preview-empty">无预览信息</div>';
    },

    /**
     * 渲染合并列表
     */
    _renderMergeList() {
        const container = document.getElementById('mergeElementList');
        if (!container) return;

        const locators = window.LocatorLibraryPanel?.locators || {};
        const entries = Object.entries(locators);

        if (entries.length === 0) {
            container.innerHTML = `
                <div class="merge-list-empty">
                    <svg viewBox="0 0 24 24" width="48" height="48" opacity="0.3">
                        <path fill="currentColor" d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14z"/>
                    </svg>
                    <p>暂无已保存的元素</p>
                </div>
            `;
            return;
        }

        const html = entries.map(([name, locator]) => {
            const hasImg = !!locator.img_path;
            const hasXml = !!(locator.xpath || locator.resource_id || locator.text || locator.content_desc);

            // 标签
            const tags = [];
            if (hasXml) tags.push('<span class="element-tag tag-xml">XML</span>');
            if (hasImg) tags.push('<span class="element-tag tag-img">IMG</span>');

            // 图标
            let iconHtml = '';
            if (hasImg && hasXml) {
                iconHtml = `<svg viewBox="0 0 24 24" width="24" height="24"><path fill="#9b59b6" d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zm-5-7l-3 3.72L9 13l-3 4h12l-4-5z"/></svg>`;
            } else if (hasImg) {
                iconHtml = `<svg viewBox="0 0 24 24" width="24" height="24"><path fill="#e67e22" d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14zm-5-7l-3 3.72L9 13l-3 4h12l-4-5z"/></svg>`;
            } else {
                iconHtml = `<svg viewBox="0 0 24 24" width="24" height="24"><path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3z"/></svg>`;
            }

            // 描述信息
            let desc = '';
            if (locator.note) {
                desc = locator.note;
            } else if (locator.text) {
                desc = `文本: ${locator.text}`;
            } else if (locator.content_desc) {
                desc = `描述: ${locator.content_desc}`;
            } else if (locator.resource_id) {
                desc = `ID: ${locator.resource_id}`;
            }

            return `
                <div class="merge-element-item" data-name="${this._escapeHtml(name)}">
                    <div class="element-icon">${iconHtml}</div>
                    <div class="element-content">
                        <div class="element-name">${this._escapeHtml(name)}</div>
                        ${desc ? `<div class="element-desc">${this._escapeHtml(desc)}</div>` : ''}
                    </div>
                    <div class="element-tags">${tags.join('')}</div>
                </div>
            `;
        }).join('');

        container.innerHTML = html;
    },

    /**
     * 筛选合并列表
     */
    _filterMergeList(keyword) {
        const container = document.getElementById('mergeElementList');
        if (!container) return;

        const items = container.querySelectorAll('.merge-element-item');
        const lowerKeyword = keyword.toLowerCase();

        items.forEach(item => {
            const name = item.dataset.name.toLowerCase();
            const desc = item.querySelector('.element-desc')?.textContent?.toLowerCase() || '';
            const match = name.includes(lowerKeyword) || desc.includes(lowerKeyword);
            item.style.display = match ? 'flex' : 'none';
        });
    },

    /**
     * 选择元素
     */
    _selectElement(name) {
        this._selectedElement = name;

        // 更新选中状态
        const container = document.getElementById('mergeElementList');
        if (container) {
            container.querySelectorAll('.merge-element-item').forEach(item => {
                item.classList.toggle('selected', item.dataset.name === name);
            });
        }

        // 更新合并预览
        this._renderMergePreview(name);

        // 更新确认按钮
        this._updateConfirmButton();
    },

    /**
     * 渲染合并预览
     */
    _renderMergePreview(name) {
        const previewContainer = document.getElementById('mergePreview');
        const contentContainer = document.getElementById('mergePreviewContent');
        const targetNameEl = document.getElementById('mergeTargetName');

        if (!previewContainer || !contentContainer) return;

        const locators = window.LocatorLibraryPanel?.locators || {};
        const existingLocator = locators[name];

        if (!existingLocator) {
            previewContainer.style.display = 'none';
            return;
        }

        previewContainer.style.display = 'block';
        if (targetNameEl) targetNameEl.textContent = name;

        // 显示将要添加/覆盖的字段
        let html = '';
        const newData = this._elementData;

        if (this._saveType === 'img') {
            // 添加图片
            const hasExisting = !!existingLocator.img_path;
            html += `
                <div class="merge-field ${hasExisting ? 'conflict' : 'new'}">
                    <span class="field-label">img_path:</span>
                    <span class="field-value">${this._escapeHtml(newData.img_path || '(新增)')}</span>
                    ${hasExisting ? `<span class="field-status">将覆盖: ${this._escapeHtml(existingLocator.img_path)}</span>` : '<span class="field-status add">新增</span>'}
                </div>
            `;
        } else {
            // 添加 XML 属性
            const xmlFields = ['xpath', 'resource_id', 'text', 'content_desc', 'class_name', 'bounds'];
            xmlFields.forEach(field => {
                if (newData[field]) {
                    const hasExisting = !!existingLocator[field];
                    html += `
                        <div class="merge-field ${hasExisting ? 'conflict' : 'new'}">
                            <span class="field-label">${field}:</span>
                            <span class="field-value">${this._escapeHtml(String(newData[field]).substring(0, 50))}</span>
                            ${hasExisting ? `<span class="field-status">将覆盖</span>` : '<span class="field-status add">新增</span>'}
                        </div>
                    `;
                }
            });
        }

        contentContainer.innerHTML = html || '<div class="preview-empty">无变更</div>';
    },

    /**
     * 处理取消
     */
    _handleCancel() {
        this.hide();
        if (this._resolvePromise) {
            this._resolvePromise(null);
            this._resolvePromise = null;
        }
    },

    /**
     * 处理确认
     */
    async _handleConfirm() {
        if (this._currentTab === 'new') {
            // 新建元素
            const nameInput = document.getElementById('newElementName');
            const noteInput = document.getElementById('newElementNote');
            const name = nameInput?.value?.trim();
            const note = noteInput?.value?.trim();

            if (!name) {
                window.AppNotifications?.warn('请输入元素名称');
                nameInput?.focus();
                return;
            }

            // 检查是否已存在
            const locators = window.LocatorLibraryPanel?.locators || {};
            if (locators[name]) {
                const confirm = await this._showConfirmDialog(`元素 "${name}" 已存在，是否覆盖？`);
                if (!confirm) return;
            }

            this.hide();
            if (this._resolvePromise) {
                this._resolvePromise({
                    action: 'new',
                    name: name,
                    note: note,
                    data: this._elementData
                });
                this._resolvePromise = null;
            }
        } else {
            // 合并到已有元素
            if (!this._selectedElement) {
                window.AppNotifications?.warn('请选择要合并的元素');
                return;
            }

            // 检查是否有冲突字段
            const conflicts = this._checkConflicts();
            if (conflicts.length > 0) {
                this._showOverwriteModal(conflicts);
                return;
            }

            // 无冲突，直接合并
            this._doMerge();
        }
    },

    /**
     * 检查冲突字段
     */
    _checkConflicts() {
        const locators = window.LocatorLibraryPanel?.locators || {};
        const existingLocator = locators[this._selectedElement];
        if (!existingLocator) return [];

        const conflicts = [];
        const newData = this._elementData;

        if (this._saveType === 'img') {
            if (existingLocator.img_path && newData.img_path) {
                conflicts.push({
                    field: 'img_path',
                    oldValue: existingLocator.img_path,
                    newValue: newData.img_path
                });
            }
        } else {
            const xmlFields = ['xpath', 'resource_id', 'text', 'content_desc', 'class_name'];
            xmlFields.forEach(field => {
                if (existingLocator[field] && newData[field]) {
                    conflicts.push({
                        field: field,
                        oldValue: existingLocator[field],
                        newValue: newData[field]
                    });
                }
            });
        }

        return conflicts;
    },

    /**
     * 显示覆盖确认对话框
     */
    _showOverwriteModal(conflicts) {
        const modal = document.getElementById('overwriteConfirmModal');
        const listContainer = document.getElementById('overwriteFieldsList');

        if (!modal || !listContainer) return;

        const html = conflicts.map(conflict => `
            <div class="overwrite-field">
                <div class="field-name">${conflict.field}</div>
                <div class="field-values">
                    <div class="old-value">
                        <span class="value-label">当前值:</span>
                        <span class="value-content">${this._escapeHtml(String(conflict.oldValue).substring(0, 80))}</span>
                    </div>
                    <div class="new-value">
                        <span class="value-label">新值:</span>
                        <span class="value-content">${this._escapeHtml(String(conflict.newValue).substring(0, 80))}</span>
                    </div>
                </div>
            </div>
        `).join('');

        listContainer.innerHTML = html;
        modal.style.display = 'flex';
    },

    /**
     * 隐藏覆盖确认对话框
     */
    _hideOverwriteModal() {
        const modal = document.getElementById('overwriteConfirmModal');
        if (modal) {
            modal.style.display = 'none';
        }
    },

    /**
     * 确认覆盖
     */
    _confirmOverwrite() {
        this._hideOverwriteModal();
        this._doMerge();
    },

    /**
     * 执行合并
     */
    _doMerge() {
        this.hide();
        if (this._resolvePromise) {
            this._resolvePromise({
                action: 'merge',
                targetName: this._selectedElement,
                data: this._elementData
            });
            this._resolvePromise = null;
        }
    },

    /**
     * 显示确认对话框
     */
    _showConfirmDialog(message) {
        return new Promise((resolve) => {
            resolve(window.confirm(message));
        });
    },

    /**
     * HTML 转义
     */
    _escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text || '';
        return div.innerHTML;
    }
};

// 导出到全局
window.ElementSaveModal = ElementSaveModal;

// DOM 加载后初始化
document.addEventListener('DOMContentLoaded', () => {
    ElementSaveModal.init();
});

if (window.rLog) {
    window.rLog('✅ ElementSaveModal 模块已加载');
}
