// 元素编辑模态框控制器
const ElementEditModal = {
    _elementName: null,
    _elementData: null,
    _resolvePromise: null,

    // 可编辑字段配置
    _editableFields: [
        { key: 'name', label: '名称', type: 'text' },
        { key: 'note', label: '备注', type: 'text' },
        { key: 'text', label: 'text', type: 'text' },
        { key: 'content_desc', label: 'content_desc', type: 'text' },
        { key: 'resource_id', label: 'resource_id', type: 'text' },
        { key: 'class_name', label: 'class_name', type: 'text' },
        { key: 'xpath', label: 'xpath', type: 'textarea' },
        { key: 'img_path', label: '截图', type: 'image' }
    ],

    init() {
        this._bindEvents();
        window.rLog('✅ ElementEditModal 初始化完成');
    },

    // 显示编辑模态框
    show(name) {
        const locators = window.LocatorLibraryPanel?.locators || {};
        const elementData = locators[name];

        if (!elementData) {
            window.AppNotifications?.error('元素不存在');
            return Promise.reject(new Error('Element not found'));
        }

        this._elementName = name;
        this._elementData = { ...elementData, name };

        const modal = document.getElementById('elementEditModal');
        if (!modal) return Promise.reject(new Error('Modal not found'));

        // 设置标题
        const titleEl = document.getElementById('elementEditModalTitle');
        if (titleEl) titleEl.textContent = `编辑 - ${name}`;

        // 渲染属性列表
        this._renderProps();

        modal.style.display = 'flex';

        return new Promise(resolve => { this._resolvePromise = resolve; });
    },

    hide() {
        const modal = document.getElementById('elementEditModal');
        if (modal) modal.style.display = 'none';
    },

    _bindEvents() {
        // 关闭
        document.getElementById('closeElementEditModal')?.addEventListener('click', () => this._cancel());
        document.getElementById('cancelElementEdit')?.addEventListener('click', () => this._cancel());
        document.getElementById('elementEditModal')?.addEventListener('click', e => {
            if (e.target.id === 'elementEditModal') this._cancel();
        });

        // 确认
        document.getElementById('confirmElementEdit')?.addEventListener('click', () => this._confirm());

        // ESC关闭
        document.addEventListener('keydown', e => {
            if (e.key === 'Escape') {
                const modal = document.getElementById('elementEditModal');
                if (modal?.style.display !== 'none') this._cancel();
            }
        });

        // 图片预览关闭
        document.getElementById('closeImagePreview')?.addEventListener('click', () => this._hideImagePreview());
        document.getElementById('imagePreviewModal')?.addEventListener('click', e => {
            if (e.target.id === 'imagePreviewModal') this._hideImagePreview();
        });
    },

    _renderProps() {
        const container = document.getElementById('elementEditProps');
        if (!container) return;

        const data = this._elementData;
        let html = '';

        this._editableFields.forEach(field => {
            const value = data[field.key] || '';

            if (field.type === 'image') {
                html += this._renderImageField(field, value);
            } else if (field.type === 'textarea') {
                html += `
                    <div class="eem-prop">
                        <label class="eem-label">${field.label}</label>
                        <textarea class="eem-textarea" data-field="${field.key}">${this._esc(value)}</textarea>
                    </div>
                `;
            } else {
                html += `
                    <div class="eem-prop">
                        <label class="eem-label">${field.label}</label>
                        <input class="eem-input" type="text" data-field="${field.key}" value="${this._esc(value)}">
                    </div>
                `;
            }
        });

        container.innerHTML = html;

        // 绑定图片点击事件
        container.querySelectorAll('.eem-img-thumb').forEach(img => {
            img.addEventListener('click', () => this._showImagePreview(img.src));
        });
    },

    _renderImageField(field, value) {
        if (!value) {
            return `
                <div class="eem-prop">
                    <label class="eem-label">${field.label}</label>
                    <div class="eem-img-empty">无截图</div>
                </div>
            `;
        }

        // 构建完整图片路径 (新路径: img/{name}.png)
        const projectPath = window.AppGlobals?.currentProject || '';
        const fullPath = projectPath ? `${projectPath}/${value}` : value;
        const imgSrc = fullPath.startsWith('file://') ? fullPath : `file://${fullPath}`;

        return `
            <div class="eem-prop">
                <label class="eem-label">${field.label}</label>
                <div class="eem-img-container">
                    <img class="eem-img-thumb" src="${imgSrc}" alt="截图">
                </div>
            </div>
        `;
    },

    _showImagePreview(src) {
        const modal = document.getElementById('imagePreviewModal');
        const img = document.getElementById('imagePreviewContent');
        if (!modal || !img) return;

        img.src = src;
        modal.style.display = 'flex';
    },

    _hideImagePreview() {
        const modal = document.getElementById('imagePreviewModal');
        if (modal) modal.style.display = 'none';
    },

    _cancel() {
        this.hide();
        if (this._resolvePromise) {
            this._resolvePromise(null);
            this._resolvePromise = null;
        }
    },

    async _confirm() {
        const container = document.getElementById('elementEditProps');
        if (!container) return;

        // 收集所有字段值
        const newData = {};
        container.querySelectorAll('[data-field]').forEach(el => {
            const field = el.dataset.field;
            newData[field] = el.value?.trim() || '';
        });

        // 验证名称
        const newName = newData.name;
        if (!newName) {
            window.AppNotifications?.warn('名称不能为空');
            return;
        }

        const oldName = this._elementName;
        const projectPath = window.AppGlobals?.currentProject;

        if (!projectPath) {
            window.AppNotifications?.error('没有打开的项目');
            return;
        }

        // 如果名称变更，检查是否冲突
        if (newName !== oldName) {
            const locators = window.LocatorLibraryPanel?.locators || {};
            if (locators[newName]) {
                window.AppNotifications?.warn('该名称已存在');
                return;
            }
        }

        // 构建更新后的 locator 对象
        const updatedLocator = { ...this._elementData };
        this._editableFields.forEach(field => {
            if (field.type !== 'image' && newData[field.key] !== undefined) {
                updatedLocator[field.key] = newData[field.key] || null;
            }
        });
        updatedLocator.name = newName;

        try {
            // 如果名称变更，需要先删除旧的
            if (newName !== oldName) {
                await window.AppGlobals.ipcRenderer.invoke('db-locator-delete', projectPath, oldName);
            }

            // 保存新数据
            const result = await window.AppGlobals.ipcRenderer.invoke('db-locator-save', projectPath, updatedLocator);

            if (result.success) {
                // 更新本地缓存
                if (newName !== oldName) {
                    delete window.LocatorLibraryPanel.locators[oldName];
                }
                window.LocatorLibraryPanel.locators[newName] = updatedLocator;

                // 重新渲染列表
                window.LocatorLibraryPanel?.renderLocators();

                this.hide();
                window.AppNotifications?.success('保存成功');

                if (this._resolvePromise) {
                    this._resolvePromise({ name: newName, data: updatedLocator });
                    this._resolvePromise = null;
                }
            } else {
                window.AppNotifications?.error('保存失败: ' + result.error);
            }
        } catch (error) {
            window.rError('保存元素失败:', error);
            window.AppNotifications?.error('保存失败: ' + error.message);
        }
    },

    _esc(text) {
        const div = document.createElement('div');
        div.textContent = text || '';
        return div.innerHTML;
    }
};

window.ElementEditModal = ElementEditModal;
document.addEventListener('DOMContentLoaded', () => ElementEditModal.init());
