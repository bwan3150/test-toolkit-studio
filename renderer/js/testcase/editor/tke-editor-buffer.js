// 基于TKE的编辑器缓冲区
// 通过IPC调用主进程的TKE handlers进行解析

class TKEEditorBuffer {
    constructor(filePath) {
        this.filePath = filePath;
        this.rawContent = '';
        this.parsedStructure = null;
        this.isDirty = false;
        this.listeners = new Set();
        this.saveTimeout = null;
        this.ipcRenderer = require('electron').ipcRenderer;

        window.rLog(`📝 TKEEditorBuffer初始化: ${filePath}`);
    }

    // 从文件加载内容
    async loadFromFile() {
        try {
            const fs = window.nodeRequire('fs').promises;
            this.rawContent = await fs.readFile(this.filePath, 'utf8');

            // 使用TKE解析文件结构
            await this.parseWithTKE();

            this.isDirty = false;
            this.notifyListeners('loaded', { content: this.rawContent });
            window.rLog(`📖 文件加载成功: ${this.filePath}`);
        } catch (error) {
            window.rError(`❌ 加载文件失败: ${error.message}`);
            throw error;
        }
    }

    // 使用TKE解析文件结构（通过IPC）
    async parseWithTKE() {
        try {
            window.rLog(`🚀 调用TKE解析文件（IPC）...`);

            const result = await this.ipcRenderer.invoke(
                'tke-parser-parse',
                null,
                null,
                this.filePath
            );

            if (!result.success) {
                throw new Error(result.error || 'TKE解析失败');
            }

            const jsonResult = JSON.parse(result.output);

            this.parsedStructure = {
                success: jsonResult.success,
                caseId: jsonResult.case_id,
                scriptName: jsonResult.script_name,
                detailsCount: Object.keys(jsonResult.details || {}).length,
                stepsCount: jsonResult.steps ? jsonResult.steps.length : 0,
                steps: jsonResult.steps ? jsonResult.steps.map((step, index) => ({
                    index: index,
                    command: step.command,
                    lineNumber: step.line_number,
                    commandType: step.command_type,
                    params: step.params
                })) : []
            };

            window.rLog(`🔍 TKE解析完成: ${this.parsedStructure.stepsCount}个步骤`);
            this.notifyListeners('parsed', { structure: this.parsedStructure });
        } catch (error) {
            window.rError(`❌ TKE解析失败: ${error.message}`);
            this.parsedStructure = null;
        }
    }

    getRawContent() {
        return this.rawContent;
    }

    getParsedStructure() {
        return this.parsedStructure;
    }

    async updateFromText(newContent) {
        if (this.rawContent === newContent) return;

        this.rawContent = newContent;
        await this.parseWithTKE();
        this.markDirty();
        this.notifyListeners('content-changed', {
            source: 'text',
            content: this.rawContent,
            structure: this.parsedStructure
        });
    }

    async updateContent(newContent) {
        this.rawContent = newContent;
        await this.parseWithTKE();
        this.markDirty();
        this.notifyListeners('content-changed', {
            source: 'direct',
            content: this.rawContent,
            structure: this.parsedStructure
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

    async validateScript() {
        try {
            const result = await this.ipcRenderer.invoke(
                'tke-parser-validate',
                null,
                null,
                this.filePath
            );

            if (!result.success) {
                return { valid: false, error: result.error };
            }

            return { valid: true, output: result.output };
        } catch (error) {
            return { valid: false, error: error.message };
        }
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
