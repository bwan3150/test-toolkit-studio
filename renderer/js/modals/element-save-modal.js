// 元素保存模态框控制器 - 重写版
const ElementSaveModal = {
    _currentTab: 'new',
    _selectedElement: null,
    _elementData: null,
    _saveType: null,
    _resolvePromise: null,

    init() {
        this._bindEvents();
        window.rLog('✅ ElementSaveModal 初始化完成');
    },

    show(options) {
        const { title, saveType, elementData, defaultName } = options;
        this._saveType = saveType;
        this._elementData = elementData;
        this._currentTab = 'new';
        this._selectedElement = null;

        const modal = document.getElementById('elementSaveModal');
        if (!modal) return Promise.reject(new Error('Modal not found'));

        // 设置标题
        const titleEl = document.getElementById('elementSaveModalTitle');
        if (titleEl) titleEl.textContent = title || '保存元素';

        // 设置默认名称
        const nameInput = document.getElementById('newElementName');
        if (nameInput) nameInput.value = defaultName || '';

        const noteInput = document.getElementById('newElementNote');
        if (noteInput) noteInput.value = '';

        // 渲染
        this._renderPreview();
        this._renderList();
        this._switchTab('new');

        modal.style.display = 'flex';
        setTimeout(() => nameInput?.focus(), 100);

        return new Promise(resolve => { this._resolvePromise = resolve; });
    },

    hide() {
        const modal = document.getElementById('elementSaveModal');
        if (modal) modal.style.display = 'none';
    },

    _bindEvents() {
        // Tab切换
        document.querySelectorAll('.esm-tab').forEach(tab => {
            tab.addEventListener('click', () => this._switchTab(tab.dataset.tab));
        });

        // 关闭
        document.getElementById('closeElementSaveModal')?.addEventListener('click', () => this._cancel());
        document.getElementById('cancelElementSave')?.addEventListener('click', () => this._cancel());
        document.getElementById('elementSaveModal')?.addEventListener('click', e => {
            if (e.target.id === 'elementSaveModal') this._cancel();
        });

        // 确认
        document.getElementById('confirmElementSave')?.addEventListener('click', () => this._confirm());

        // 搜索
        document.getElementById('mergeElementSearch')?.addEventListener('input', e => this._filterList(e.target.value));

        // 列表点击
        document.getElementById('mergeElementList')?.addEventListener('click', e => {
            const item = e.target.closest('.esm-item');
            if (item) this._selectElement(item.dataset.name);
        });

        // Enter确认
        document.getElementById('newElementName')?.addEventListener('keydown', e => {
            if (e.key === 'Enter') this._confirm();
        });

        // ESC关闭
        document.addEventListener('keydown', e => {
            if (e.key === 'Escape') {
                const modal = document.getElementById('elementSaveModal');
                if (modal?.style.display !== 'none') this._cancel();
            }
        });

        // 覆盖确认
        document.getElementById('closeOverwriteModal')?.addEventListener('click', () => this._hideOverwrite());
        document.getElementById('cancelOverwrite')?.addEventListener('click', () => this._hideOverwrite());
        document.getElementById('confirmOverwrite')?.addEventListener('click', () => this._doMerge());
    },

    _switchTab(tab) {
        this._currentTab = tab;
        document.querySelectorAll('.esm-tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
        document.querySelectorAll('.esm-panel').forEach(p => p.classList.toggle('active', p.dataset.panel === tab));

        const btn = document.getElementById('confirmElementSave');
        if (btn) btn.textContent = tab === 'new' ? '保存' : (this._selectedElement ? '并入' : '选择元素');

        if (tab === 'merge') {
            document.getElementById('mergeElementSearch')?.focus();
        }
    },

    _renderPreview() {
        const container = document.getElementById('elementPreviewContent');
        if (!container || !this._elementData) return;

        const data = this._elementData;
        let rows = '';

        if (this._saveType === 'img') {
            // 图片类型 - 显示图片预览
            const imgHtml = data.img_path
                ? `<img class="esm-img-preview" src="file://${this._esc(data.img_path)}" alt="截图">`
                : '<span class="esm-empty">(待保存)</span>';
            rows = `<tr><td>截图</td><td>${imgHtml}</td></tr>`;
        } else {
            const fields = ['text', 'content_desc', 'resource_id', 'class_name', 'xpath', 'bounds'];
            fields.forEach(f => {
                if (data[f]) rows += `<tr><td>${f}</td><td title="${this._esc(data[f])}">${this._esc(data[f])}</td></tr>`;
            });
        }

        if (!rows) {
            container.innerHTML = '<div style="padding:12px;text-align:center;color:var(--text-tertiary)">无数据</div>';
            return;
        }

        container.innerHTML = `<table class="esm-table"><thead><tr><th>字段</th><th>值</th></tr></thead><tbody>${rows}</tbody></table>`;
    },

    _renderList() {
        const container = document.getElementById('mergeElementList');
        if (!container) return;

        const locators = window.LocatorLibraryPanel?.locators || {};
        const entries = Object.entries(locators);

        if (entries.length === 0) {
            container.innerHTML = '<div class="esm-list-empty">暂无已保存的元素</div>';
            return;
        }

        container.innerHTML = entries.map(([name, loc]) => {
            const hasXml = !!(loc.xpath || loc.resource_id || loc.text || loc.content_desc);
            const hasImg = !!loc.img_path;
            const tags = (hasXml ? '<span class="esm-item-tag xml">XML</span>' : '') +
                        (hasImg ? '<span class="esm-item-tag img">IMG</span>' : '');
            return `<div class="esm-item" data-name="${this._esc(name)}"><span class="esm-item-name">${this._esc(name)}</span>${tags}</div>`;
        }).join('');
    },

    _filterList(keyword) {
        const items = document.querySelectorAll('#mergeElementList .esm-item');
        const kw = keyword.toLowerCase();
        items.forEach(item => {
            item.style.display = item.dataset.name.toLowerCase().includes(kw) ? 'flex' : 'none';
        });
    },

    _selectElement(name) {
        this._selectedElement = name;
        document.querySelectorAll('#mergeElementList .esm-item').forEach(item => {
            item.classList.toggle('selected', item.dataset.name === name);
        });
        this._renderMergePreview(name);
        const btn = document.getElementById('confirmElementSave');
        if (btn) btn.textContent = '并入';
    },

    _renderMergePreview(name) {
        const preview = document.getElementById('mergePreview');
        const container = document.getElementById('mergePreviewContent');
        const targetEl = document.getElementById('mergeTargetName');
        if (!preview || !container) return;

        const locators = window.LocatorLibraryPanel?.locators || {};
        const existing = locators[name];
        if (!existing) { preview.style.display = 'none'; return; }

        preview.style.display = 'block';
        if (targetEl) targetEl.textContent = name;

        const newData = this._elementData;
        let rows = '';

        if (this._saveType === 'img') {
            // 图片类型 - 显示图片预览
            const hasOld = !!existing.img_path;
            const status = hasOld ? '<span class="esm-status overwrite">覆盖</span>' : '<span class="esm-status new">新增</span>';
            const newImgHtml = newData.img_path
                ? `<img class="esm-img-preview" src="file://${this._esc(newData.img_path)}" alt="新截图">`
                : '<span class="esm-empty">(待保存)</span>';
            const oldImgHtml = hasOld
                ? `<img class="esm-img-preview" src="file://${this._esc(existing.img_path)}" alt="原截图">`
                : '<span class="esm-empty">无</span>';
            rows = `<tr><td>截图</td><td>${status}</td><td>${newImgHtml}</td><td>${oldImgHtml}</td></tr>`;
            // 图片模式始终显示4列
            container.innerHTML = `<table class="esm-table"><thead><tr><th>字段</th><th>状态</th><th>新值</th><th>原值</th></tr></thead><tbody>${rows}</tbody></table>`;
        } else {
            // XML类型 - 始终显示4列，空值显示占位符
            const fields = ['text', 'content_desc', 'resource_id', 'class_name', 'xpath', 'bounds'];
            fields.forEach(f => {
                if (newData[f]) {
                    const hasOld = !!existing[f];
                    const status = hasOld ? '<span class="esm-status overwrite">覆盖</span>' : '<span class="esm-status new">新增</span>';
                    const oldVal = hasOld ? this._esc(existing[f]) : '<span class="esm-empty">无</span>';
                    rows += `<tr><td>${f}</td><td>${status}</td><td title="${this._esc(newData[f])}">${this._esc(newData[f])}</td><td class="${hasOld ? 'esm-old-value' : ''}" title="${hasOld ? this._esc(existing[f]) : ''}">${oldVal}</td></tr>`;
                }
            });
            container.innerHTML = `<table class="esm-table"><thead><tr><th>字段</th><th>状态</th><th>新值</th><th>原值</th></tr></thead><tbody>${rows}</tbody></table>`;
        }
    },

    _cancel() {
        this.hide();
        if (this._resolvePromise) {
            this._resolvePromise(null);
            this._resolvePromise = null;
        }
    },

    async _confirm() {
        if (this._currentTab === 'new') {
            const name = document.getElementById('newElementName')?.value?.trim();
            const note = document.getElementById('newElementNote')?.value?.trim();
            if (!name) {
                window.AppNotifications?.warn('请输入元素名称');
                return;
            }
            const locators = window.LocatorLibraryPanel?.locators || {};
            if (locators[name] && !confirm(`元素 "${name}" 已存在，是否覆盖？`)) return;

            this.hide();
            this._resolvePromise?.({ action: 'new', name, note, data: this._elementData });
            this._resolvePromise = null;
        } else {
            if (!this._selectedElement) {
                window.AppNotifications?.warn('请选择要合并的元素');
                return;
            }
            const conflicts = this._checkConflicts();
            if (conflicts.length > 0) {
                this._showOverwrite(conflicts);
            } else {
                this._doMerge();
            }
        }
    },

    _checkConflicts() {
        const locators = window.LocatorLibraryPanel?.locators || {};
        const existing = locators[this._selectedElement];
        if (!existing) return [];

        const conflicts = [];
        const newData = this._elementData;

        if (this._saveType === 'img') {
            if (existing.img_path && newData.img_path) {
                conflicts.push({ field: 'img_path', oldValue: existing.img_path, newValue: newData.img_path });
            }
        } else {
            ['xpath', 'resource_id', 'text', 'content_desc', 'class_name'].forEach(f => {
                if (existing[f] && newData[f]) {
                    conflicts.push({ field: f, oldValue: existing[f], newValue: newData[f] });
                }
            });
        }
        return conflicts;
    },

    _showOverwrite(conflicts) {
        const modal = document.getElementById('overwriteConfirmModal');
        const list = document.getElementById('overwriteFieldsList');
        if (!modal || !list) return;

        list.innerHTML = conflicts.map(c => `
            <div style="margin-bottom:8px;padding:8px;background:var(--bg-primary);border-radius:4px;">
                <div style="font-size:12px;color:var(--text-secondary);margin-bottom:4px;">${c.field}</div>
                <div style="font-size:11px;color:#e74c3c;text-decoration:line-through;">${this._esc(String(c.oldValue).substring(0,60))}</div>
                <div style="font-size:11px;color:#2ecc71;">${this._esc(String(c.newValue).substring(0,60))}</div>
            </div>
        `).join('');
        modal.style.display = 'flex';
    },

    _hideOverwrite() {
        const modal = document.getElementById('overwriteConfirmModal');
        if (modal) modal.style.display = 'none';
    },

    _doMerge() {
        this._hideOverwrite();
        this.hide();
        this._resolvePromise?.({ action: 'merge', targetName: this._selectedElement, data: this._elementData });
        this._resolvePromise = null;
    },

    _esc(text) {
        const div = document.createElement('div');
        div.textContent = text || '';
        return div.innerHTML;
    }
};

window.ElementSaveModal = ElementSaveModal;
document.addEventListener('DOMContentLoaded', () => ElementSaveModal.init());
