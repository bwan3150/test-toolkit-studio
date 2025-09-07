// 基于TKE的编辑器缓冲区
// TKE负责所有.tks文件的解析、验证和处理
// JS只负责UI展示和用户交互

class TKEEditorBuffer {
    constructor(filePath) {
        this.filePath = filePath;
        this.rawContent = ''; // 原始TKS文件内容
        this.parsedStructure = null; // TKE解析后的结构
        this.isDirty = false;
        this.listeners = new Set();
        this.saveTimeout = null;
        this.tkeAdapter = null;
        this.scriptParser = null;
        
        window.rLog(`📝 TKEEditorBuffer初始化: ${filePath}`);
    }
    
    // 初始化TKE连接
    async initialize() {
        try {
            this.tkeAdapter = await window.TKEAdapterModule.getTKEAdapter();
            this.scriptParser = new window.TKEAdapterModule.TKEScriptParserAdapter(this.tkeAdapter);
            window.rLog('✅ TKE连接已建立');
        } catch (error) {
            window.rError(`❌ TKE连接失败: ${error.message}`);
            throw error;
        }
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
            window.rLog(`📖 文件加载成功: ${this.filePath}, 长度: ${this.rawContent.length}`);
        } catch (error) {
            window.rError(`❌ 加载文件失败: ${error.message}`);
            throw error;
        }
    }
    
    // 使用TKE解析文件结构
    async parseWithTKE() {
        if (!this.scriptParser) {
            window.rError('TKE ScriptParser未初始化');
            return;
        }
        
        try {
            // 先保存原始内容到临时文件，让TKE解析
            const tempFile = await this.saveToTempFile();
            
            // 调用TKE解析
            this.parsedStructure = await this.scriptParser.parseScriptFile(tempFile);
            
            window.rLog(`🔍 TKE解析完成: ${this.parsedStructure.stepsCount}个步骤`);
            
            // 清理临时文件
            await this.cleanupTempFile(tempFile);
            
            this.notifyListeners('parsed', { structure: this.parsedStructure });
        } catch (error) {
            window.rError(`❌ TKE解析失败: ${error.message}`);
            this.parsedStructure = null;
        }
    }
    
    // 创建临时文件用于TKE解析
    async saveToTempFile() {
        const fs = window.nodeRequire('fs').promises;
        const path = window.nodeRequire('path');
        const os = window.nodeRequire('os');
        
        const tempFile = path.join(os.tmpdir(), `tke_temp_${Date.now()}.tks`);
        await fs.writeFile(tempFile, this.rawContent, 'utf8');
        return tempFile;
    }
    
    // 清理临时文件
    async cleanupTempFile(tempFile) {
        try {
            const fs = window.nodeRequire('fs').promises;
            await fs.unlink(tempFile);
        } catch (error) {
            // 忽略清理错误
            window.rLog(`⚠️ 清理临时文件失败: ${error.message}`);
        }
    }
    
    // 获取原始内容（用于文本编辑器）
    getRawContent() {
        return this.rawContent;
    }
    
    // 获取解析后的结构（用于块编辑器）
    getParsedStructure() {
        return this.parsedStructure;
    }
    
    // 从文本编辑器更新内容
    async updateFromText(newContent) {
        if (this.rawContent === newContent) {
            return; // 内容未变更
        }
        
        this.rawContent = newContent;
        
        // 重新解析
        await this.parseWithTKE();
        
        this.markDirty();
        this.notifyListeners('content-changed', { 
            source: 'text', 
            content: this.rawContent,
            structure: this.parsedStructure
        });
    }
    
    // 从块编辑器更新内容
    async updateFromBlocks(blockUpdates) {
        // 这里我们需要重新构造TKS内容
        // 但是为了保持架构纯净，我们让TKE来做这个工作
        
        try {
            // 调用TKE的脚本生成功能（如果有的话）
            // 目前暂时用简单的字符串拼接，后续可以让TKE提供生成接口
            this.rawContent = this.reconstructTKSFromBlocks(blockUpdates);
            
            // 重新解析验证
            await this.parseWithTKE();
            
            this.markDirty();
            this.notifyListeners('content-changed', {
                source: 'blocks',
                content: this.rawContent,
                structure: this.parsedStructure
            });
            
            window.rLog('🧩 从块编辑器更新内容成功');
        } catch (error) {
            window.rError(`❌ 从块编辑器更新失败: ${error.message}`);
        }
    }
    
    // 临时的TKS重构方法（理想情况下应该由TKE提供）
    reconstructTKSFromBlocks(blockUpdates) {
        // 这是一个临时实现，真正的生成应该由TKE负责
        const lines = this.rawContent.split('\n');
        
        // 更新特定的命令行
        if (blockUpdates.commandIndex !== undefined && blockUpdates.updatedLine) {
            // 找到对应的命令行并更新
            let commandCount = -1;
            for (let i = 0; i < lines.length; i++) {
                const line = lines[i].trim();
                if (this.isCommandLine(line)) {
                    commandCount++;
                    if (commandCount === blockUpdates.commandIndex) {
                        lines[i] = blockUpdates.updatedLine;
                        break;
                    }
                }
            }
        }
        
        return lines.join('\n');
    }
    
    // 判断是否是命令行（临时实现）
    isCommandLine(line) {
        if (!line || line.startsWith('#') || line.startsWith('用例:') || 
            line.startsWith('脚本名:') || line === '详情:' || line === '步骤:' ||
            line.includes('appPackage:') || line.includes('appActivity:')) {
            return false;
        }
        return true;
    }
    
    // 标记为需要保存
    markDirty() {
        this.isDirty = true;
        this.scheduleAutoSave();
    }
    
    // 计划自动保存
    scheduleAutoSave() {
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            this.save();
        }, 1000);
    }
    
    // 保存到文件
    async save() {
        if (!this.isDirty) {
            window.rLog('📁 Buffer无变更，跳过保存');
            return;
        }
        
        try {
            const fs = window.nodeRequire('fs').promises;
            await fs.writeFile(this.filePath, this.rawContent, 'utf8');
            
            this.isDirty = false;
            
            window.rLog(`💾 文件保存成功: ${this.filePath}`);
            this.notifyListeners('saved', { 
                filePath: this.filePath, 
                content: this.rawContent 
            });
        } catch (error) {
            window.rError(`❌ 文件保存失败: ${error.message}`);
            throw error;
        }
    }
    
    // 强制保存
    async forceSave() {
        this.isDirty = true;
        return this.save();
    }
    
    // 验证脚本
    async validateScript() {
        if (!this.scriptParser) {
            window.rError('TKE ScriptParser未初始化');
            return { valid: false, error: 'TKE未初始化' };
        }
        
        try {
            const tempFile = await this.saveToTempFile();
            const result = await this.scriptParser.validateScript(tempFile);
            await this.cleanupTempFile(tempFile);
            
            window.rLog(`✅ 脚本验证完成: ${result.valid ? '有效' : '无效'}`);
            return result;
        } catch (error) {
            window.rError(`❌ 脚本验证失败: ${error.message}`);
            return { valid: false, error: error.message };
        }
    }
    
    // 添加监听器
    addListener(callback) {
        this.listeners.add(callback);
    }
    
    // 移除监听器
    removeListener(callback) {
        this.listeners.delete(callback);
    }
    
    // 通知监听器
    notifyListeners(eventType, data) {
        this.listeners.forEach(callback => {
            try {
                callback(eventType, data);
            } catch (error) {
                window.rError(`❌ 监听器回调失败: ${error.message}`);
            }
        });
    }
    
    // 清理资源
    dispose() {
        clearTimeout(this.saveTimeout);
        this.listeners.clear();
        window.rLog(`🗑️ TKEEditorBuffer已释放: ${this.filePath}`);
    }
}

// 导出到全局
window.TKEEditorBuffer = TKEEditorBuffer;