// 编辑器缓冲区管理器
// 统一管理文件内容，解耦块编辑器和文本编辑器的展示逻辑

class EditorBuffer {
    constructor(filePath, initialContent = '') {
        this.filePath = filePath;
        this.content = initialContent; // 当前内容（TKS格式）
        this.isDirty = false; // 是否有未保存的更改
        this.listeners = new Set(); // 内容变更监听器
        this.saveTimeout = null;
        
        // 解析初始内容
        this.parseContent(initialContent);
        
        window.rLog(`📝 EditorBuffer 初始化: ${filePath}`);
    }
    
    // 解析TKS内容为结构化数据
    parseContent(tksContent) {
        try {
            // 这里需要实现TKS解析逻辑，暂时简单处理
            this.commands = this.parseTKSToCommands(tksContent);
            this.content = tksContent;
        } catch (error) {
            window.rError(`解析TKS内容失败: ${error.message}`);
            this.commands = [];
            this.content = tksContent; // 保留原始内容
        }
    }
    
    // 简单的TKS解析（这里需要根据具体的TKS语法实现）
    parseTKSToCommands(tksContent) {
        const lines = tksContent.split('\n');
        const commands = [];
        let inStepsSection = false;
        
        for (let line of lines) {
            line = line.trim();
            if (!line) continue;
            
            if (line === '步骤:') {
                inStepsSection = true;
                continue;
            }
            
            if (inStepsSection && !line.startsWith('#')) {
                // 简单的命令解析
                commands.push({
                    type: this.detectCommandType(line),
                    raw: line,
                    params: this.parseCommandParams(line)
                });
            }
        }
        
        return commands;
    }
    
    // 检测命令类型
    detectCommandType(line) {
        if (line.includes('click')) return 'click';
        if (line.includes('input') || line.includes('type')) return 'input';
        if (line.includes('wait')) return 'wait';
        if (line.includes('swipe')) return 'swipe';
        return 'unknown';
    }
    
    // 解析命令参数
    parseCommandParams(line) {
        const params = {};
        
        // 提取花括号中的参数
        const bracketMatches = line.match(/\{([^}]+)\}/g);
        if (bracketMatches) {
            bracketMatches.forEach((match, index) => {
                const content = match.slice(1, -1); // 移除花括号
                params[`param${index}`] = content;
            });
        }
        
        // 提取引号中的参数
        const quoteMatches = line.match(/"([^"]+)"/g);
        if (quoteMatches) {
            quoteMatches.forEach((match, index) => {
                const content = match.slice(1, -1); // 移除引号
                params[`text${index}`] = content;
            });
        }
        
        return params;
    }
    
    // 从命令结构生成TKS内容
    commandsToTKS() {
        let tksContent = '步骤:\n';
        
        this.commands.forEach(command => {
            if (command.raw) {
                tksContent += command.raw + '\n';
            }
        });
        
        return tksContent;
    }
    
    // 获取当前内容
    getContent() {
        return this.content;
    }
    
    // 获取命令列表（用于块编辑器）
    getCommands() {
        return this.commands;
    }
    
    // 设置内容（从文本编辑器）
    setContentFromText(textContent) {
        const oldContent = this.content;
        this.parseContent(textContent);
        
        if (oldContent !== this.content) {
            this.markDirty();
            this.notifyListeners('content-changed', { source: 'text', content: this.content });
        }
    }
    
    // 设置命令（从块编辑器）
    setCommands(commands) {
        this.commands = commands;
        const newContent = this.commandsToTKS();
        
        if (this.content !== newContent) {
            this.content = newContent;
            this.markDirty();
            this.notifyListeners('content-changed', { source: 'block', content: this.content });
        }
    }
    
    // 更新单个命令（从块编辑器拖拽）
    updateCommand(index, updatedCommand) {
        if (index >= 0 && index < this.commands.length) {
            this.commands[index] = { ...this.commands[index], ...updatedCommand };
            const newContent = this.commandsToTKS();
            
            if (this.content !== newContent) {
                this.content = newContent;
                this.markDirty();
                this.notifyListeners('command-updated', { index, command: this.commands[index], source: 'block' });
                this.notifyListeners('content-changed', { source: 'block', content: this.content });
            }
        }
    }
    
    // 添加新命令
    addCommand(command) {
        this.commands.push(command);
        const newContent = this.commandsToTKS();
        
        if (this.content !== newContent) {
            this.content = newContent;
            this.markDirty();
            this.notifyListeners('command-added', { command, source: 'block' });
            this.notifyListeners('content-changed', { source: 'block', content: this.content });
        }
    }
    
    // 删除命令
    removeCommand(index) {
        if (index >= 0 && index < this.commands.length) {
            const removedCommand = this.commands.splice(index, 1)[0];
            const newContent = this.commandsToTKS();
            
            if (this.content !== newContent) {
                this.content = newContent;
                this.markDirty();
                this.notifyListeners('command-removed', { index, command: removedCommand, source: 'block' });
                this.notifyListeners('content-changed', { source: 'block', content: this.content });
            }
        }
    }
    
    // 标记为脏数据
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
            if (window.EditorManager && window.EditorManager.saveFileContent) {
                await window.EditorManager.saveFileContent(this.filePath, this.content);
                this.isDirty = false;
                window.rLog(`💾 Buffer保存成功: ${this.filePath}`);
                this.notifyListeners('saved', { filePath: this.filePath, content: this.content });
            } else {
                window.rError('❌ EditorManager.saveFileContent 不可用');
            }
        } catch (error) {
            window.rError(`💾 Buffer保存失败: ${error.message}`);
            throw error;
        }
    }
    
    // 强制保存
    async forceSave() {
        this.isDirty = true;
        return this.save();
    }
    
    // 添加变更监听器
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
                window.rError(`监听器回调失败: ${error.message}`);
            }
        });
    }
    
    // 清理资源
    dispose() {
        clearTimeout(this.saveTimeout);
        this.listeners.clear();
        window.rLog(`🗑️ EditorBuffer已释放: ${this.filePath}`);
    }
}

// 导出到全局
window.EditorBuffer = EditorBuffer;