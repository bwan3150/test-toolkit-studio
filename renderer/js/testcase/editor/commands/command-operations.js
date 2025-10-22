// 命令操作 - 负责命令的增删改查操作

const CommandOperations = {
    /**
     * 通过 TKEEditorBuffer 添加命令
     * @param {Object} command - 命令对象
     */
    async addCommandToBuffer(command) {
        if (!this.buffer) return;

        const tksLine = this.commandToTKSLine(command);
        if (!tksLine) return;

        window.rLog('添加TKS命令行:', tksLine);

        // 获取当前内容并添加新行
        const currentContent = this.buffer.getRawContent();
        const newContent = currentContent + '\n' + tksLine;

        // 更新缓冲区内容
        await this.buffer.updateContent(newContent);
    },

    /**
     * 通过 TKEEditorBuffer 插入命令
     * @param {Object} command - 命令对象
     * @param {number} index - 插入位置索引
     */
    async insertCommandToBuffer(command, index) {
        if (!this.buffer) return;

        const tksLine = this.commandToTKSLine(command);
        if (!tksLine) return;

        window.rLog(`插入TKS命令行到位置 ${index}:`, tksLine);

        // 获取当前内容并在指定位置插入
        const lines = this.buffer.getRawContent().split('\n');

        // 找到第 index 个命令行的位置
        let commandCount = 0;
        let insertPosition = lines.length; // 默认插入到末尾

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i].trim();
            if (this.isCommandLine(line)) {
                if (commandCount === index) {
                    insertPosition = i;
                    break;
                }
                commandCount++;
            }
        }

        // 在指定位置插入新命令行
        lines.splice(insertPosition, 0, tksLine);
        const newContent = lines.join('\n');

        // 更新缓冲区内容
        await this.buffer.updateContent(newContent);
    },

    /**
     * 添加命令（到末尾）
     * @param {string} type - 命令类型
     */
    addCommand(type) {
        const definition = window.CommandUtils.findCommandDefinition(type);
        if (!definition) return;

        const command = {
            type: type,
            params: {}
        };

        // 初始化参数
        definition.params.forEach(param => {
            command.params[param.name] = param.default || '';
        });

        // 通过 TKEEditorBuffer 添加命令
        this.addCommandToBuffer(command);
        this.triggerChange();
    },

    /**
     * 插入命令（在指定位置）
     * @param {string} type - 命令类型
     * @param {number} index - 插入位置索引
     */
    insertCommand(type, index) {
        const definition = window.CommandUtils.findCommandDefinition(type);
        if (!definition) return;

        const command = {
            type: type,
            params: {}
        };

        // 初始化参数
        definition.params.forEach(param => {
            command.params[param.name] = param.default || '';
        });

        // 通过 TKEEditorBuffer 插入命令
        this.insertCommandToBuffer(command, index);
        this.triggerChange();
    },

    /**
     * 重新排序命令
     * @param {number} fromIndex - 源索引
     * @param {number} toIndex - 目标索引
     */
    async reorderCommand(fromIndex, toIndex) {
        if (fromIndex === toIndex) {
            window.rLog(`⚠️ 跳过无效重排: 源索引 ${fromIndex} 与目标索引 ${toIndex} 相同`);
            return;
        }

        // 防止并发重排操作
        if (this._isReordering) {
            window.rLog(`⚠️ 重排操作正在进行中，跳过重复请求: ${fromIndex} -> ${toIndex}`);
            return;
        }

        this._isReordering = true;
        window.rLog(`开始移动命令：从位置 ${fromIndex} 到位置 ${toIndex}`);

        if (!this.buffer) {
            window.rError('TKE缓冲区未初始化，无法重排命令');
            this._isReordering = false;
            return;
        }

        try {
            // 获取当前TKS内容并分行
            const content = this.buffer.getRawContent();
            const lines = content.split('\n');

            // 找到步骤区域的起始行
            let stepsStartIndex = -1;
            for (let i = 0; i < lines.length; i++) {
                if (lines[i].trim() === '步骤:') {
                    stepsStartIndex = i + 1;
                    break;
                }
            }

            if (stepsStartIndex === -1) {
                window.rError('未找到步骤区域');
                this._isReordering = false;
                return;
            }

            // 收集所有命令行及其索引
            const commandLines = [];
            for (let i = stepsStartIndex; i < lines.length; i++) {
                const line = lines[i].trim();
                if (this.isCommandLine(line)) {
                    commandLines.push({ lineIndex: i, content: lines[i] });
                }
            }

            if (fromIndex >= commandLines.length || toIndex >= commandLines.length) {
                window.rError(`索引超出范围: fromIndex=${fromIndex}, toIndex=${toIndex}, 总命令数=${commandLines.length}`);
                this._isReordering = false;
                return;
            }

            // 调整索引以适应移动后的位置
            let adjustedToIndex = toIndex;
            if (fromIndex < toIndex) {
                adjustedToIndex = toIndex - 1;
            }

            // 移动命令行
            const movedCommand = commandLines.splice(fromIndex, 1)[0];
            commandLines.splice(adjustedToIndex, 0, movedCommand);

            // 重建lines数组，先移除所有旧的命令行
            const commandLineIndices = [];
            for (let i = stepsStartIndex; i < lines.length; i++) {
                const line = lines[i].trim();
                if (this.isCommandLine(line)) {
                    commandLineIndices.push(i);
                }
            }

            // 从后往前删除，避免索引变化
            for (let i = commandLineIndices.length - 1; i >= 0; i--) {
                lines.splice(commandLineIndices[i], 1);
            }

            // 插入重排后的命令行
            let insertIndex = stepsStartIndex;
            for (const cmd of commandLines) {
                lines.splice(insertIndex, 0, cmd.content);
                insertIndex++;
            }

            // 更新内容
            const newContent = lines.join('\n');
            await this.buffer.updateContent(newContent);

            window.rLog(`✅ 成功移动命令：从位置 ${fromIndex} 到位置 ${adjustedToIndex}`);
        } catch (error) {
            window.rError(`❌ 移动命令失败: ${error.message}`);
        } finally {
            this._isReordering = false;
        }
    },

    /**
     * 删除命令
     * @param {number} index - 命令索引
     */
    async removeCommand(index) {
        window.rLog(`开始删除命令，索引: ${index}`);
        const commandsBefore = this.getCommands().length;
        window.rLog(`删除前命令数量: ${commandsBefore}`);

        if (!this.buffer) {
            window.rError('TKE缓冲区未初始化，无法删除命令');
            return;
        }

        try {
            // 获取当前TKS内容并分行
            const content = this.buffer.getRawContent();
            const lines = content.split('\n');

            // 找到步骤区域的起始行
            let stepsStartIndex = -1;
            for (let i = 0; i < lines.length; i++) {
                if (lines[i].trim() === '步骤:') {
                    stepsStartIndex = i + 1;
                    break;
                }
            }

            if (stepsStartIndex === -1) {
                window.rError('未找到步骤区域');
                return;
            }

            // 找到命令行（跳过非命令行）
            let commandCount = -1;
            let targetLineIndex = -1;

            for (let i = stepsStartIndex; i < lines.length; i++) {
                const line = lines[i].trim();
                if (this.isCommandLine(line)) {
                    commandCount++;
                    if (commandCount === index) {
                        targetLineIndex = i;
                        break;
                    }
                }
            }

            if (targetLineIndex === -1) {
                window.rError(`未找到索引为 ${index} 的命令行`);
                return;
            }

            // 删除该行
            lines.splice(targetLineIndex, 1);

            // 更新内容
            const newContent = lines.join('\n');
            await this.buffer.updateContent(newContent);

            window.rLog(`✅ 成功删除命令，索引: ${index}`);
        } catch (error) {
            window.rError(`❌ 删除命令失败: ${error.message}`);
        }
    },

    /**
     * 更新命令参数
     * @param {number} index - 命令索引
     * @param {string} param - 参数名
     * @param {*} value - 参数值
     */
    updateCommandParam(index, param, value) {
        // TODO: 重构为通过 TKEEditorBuffer 操作
        // this.script.updateCommandParam(index, param, value);
        this.triggerChange();
    }
};

// 导出到全局
window.CommandOperations = CommandOperations;

if (window.rLog) {
    window.rLog('✅ CommandOperations 模块已加载');
}
