// TKS 编辑器缓冲区 - 简化版
// 纯文本存储，不依赖 TKE parser

class TKEEditorBuffer {
    constructor(filePath) {
        this.filePath = filePath;
        this.rawContent = '';
        this.isDirty = false;
        this.listeners = new Set();
        this.saveTimeout = null;

        window.rLog(`📝 TKEEditorBuffer初始化: ${filePath}`);
    }

    // 从文件加载内容
    async loadFromFile() {
        try {
            const fs = window.nodeRequire('fs').promises;
            this.rawContent = await fs.readFile(this.filePath, 'utf8');

            this.isDirty = false;
            this.notifyListeners('loaded', { content: this.rawContent });
            window.rLog(`📖 文件加载成功: ${this.filePath}`);
        } catch (error) {
            window.rError(`❌ 加载文件失败: ${error.message}`);
            throw error;
        }
    }

    getRawContent() {
        return this.rawContent;
    }

    async updateFromText(newContent) {
        if (this.rawContent === newContent) return;

        this.rawContent = newContent;
        this.markDirty();
        this.notifyListeners('content-changed', {
            source: 'text',
            content: this.rawContent
        });
    }

    async updateContent(newContent) {
        this.rawContent = newContent;
        this.markDirty();
        this.notifyListeners('content-changed', {
            source: 'direct',
            content: this.rawContent
        });
    }

    markDirty() {
        this.isDirty = true;
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => this.save(), 1000);
    }

    async save() {
        if (!this.isDirty) return;

        try {
            const fs = window.nodeRequire('fs').promises;
            await fs.writeFile(this.filePath, this.rawContent, 'utf8');
            this.isDirty = false;
            window.rLog(`💾 文件保存成功: ${this.filePath}`);
            this.notifyListeners('saved', { filePath: this.filePath });
        } catch (error) {
            window.rError(`❌ 文件保存失败: ${error.message}`);
            throw error;
        }
    }

    async forceSave() {
        this.isDirty = true;
        return this.save();
    }

    on(eventType, callback) {
        const wrapped = (type, data) => {
            if (type === eventType) callback(data);
        };
        this.listeners.add(wrapped);
        return wrapped;
    }

    off(callback) {
        this.listeners.delete(callback);
    }

    notifyListeners(eventType, data) {
        this.listeners.forEach(callback => {
            try {
                callback(eventType, data);
            } catch (error) {
                window.rError(`❌ 监听器回调失败: ${error.message}`);
            }
        });
    }

    dispose() {
        clearTimeout(this.saveTimeout);
        this.listeners.clear();
    }
}

window.TKEEditorBuffer = TKEEditorBuffer;
