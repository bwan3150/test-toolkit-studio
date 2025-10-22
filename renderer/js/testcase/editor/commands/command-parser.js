// 命令解析器 - 负责TKS代码解析和命令格式转换

const CommandParser = {
    /**
     * 获取命令列表 - 使用 TKSBlockParser
     * @returns {Array} 命令数组
     */
    getCommands() {
        if (!this.buffer) {
            return [];
        }

        // 获取原始 TKS 代码
        const tksCode = this.buffer.getRawContent();

        // 使用 TKSBlockParser 解析
        const blocks = window.TKSBlockParser.parse(tksCode);

        // 转换为编辑器期望的格式
        return blocks.map(block => {
            return {
                type: block.command,
                params: this.convertBlockParamsToEditorFormat(block.params, block.command),
                lineNumber: block.lineNumber,
                raw: block.raw
            };
        });
    },

    /**
     * 将块参数转换为编辑器格式
     * @param {Array} params - 参数数组
     * @param {string} command - 命令名称
     * @returns {Object} 参数对象
     */
    convertBlockParamsToEditorFormat(params, command) {
        const result = {};
        const paramsDef = window.TKSBlockParser.getCommandParamsDef(command);

        params.forEach((param, index) => {
            const def = paramsDef[index];
            if (!def) return;

            // 根据参数类型处理值
            if (param.type === 'coordinate') {
                result[def.name] = `{${param.value.join(',')}}`;
            } else if (param.type === 'image-locator') {
                result[def.name] = `@{${param.value}}`;
            } else if (param.type === 'locator') {
                result[def.name] = `{${param.value}}`;
            } else {
                result[def.name] = param.value.toString();
            }
        });

        return result;
    },

    /**
     * 解析TKS命令文本，提取命令和参数
     * @param {string} commandText - 命令文本
     * @returns {Object} 包含type和params的对象
     */
    parseTKSCommandText(commandText) {
        // 去除首尾空白
        commandText = commandText.trim();

        // 解析命令名（第一个单词）
        const parts = commandText.split(/\s+/);
        const commandName = parts[0];

        // 映射到类型
        const type = this.tksCommandToType(commandName);

        // 解析参数
        const params = {};

        // 解析方括号中的参数 [param1, param2]
        const bracketMatch = commandText.match(/\[([^\]]*)\]/);
        if (bracketMatch) {
            const bracketContent = bracketMatch[1];
            const bracketParams = bracketContent.split(',').map(p => p.trim());

            // 根据命令类型分配参数
            const definition = window.CommandUtils.findCommandDefinition(type);
            if (definition && definition.params) {
                definition.params.forEach((paramDef, index) => {
                    if (bracketParams[index]) {
                        params[paramDef.name] = bracketParams[index];
                    }
                });
            }
        }

        // 解析图片引用 @{imageName}
        const imageMatch = commandText.match(/@\{([^}]+)\}/);
        if (imageMatch) {
            params.target = `@{${imageMatch[1]}}`;
        }

        // 解析XML元素引用 {elementName}
        const xmlMatch = commandText.match(/(?<!@)\{([^}]+)\}/);
        if (xmlMatch && !imageMatch) {
            params.target = `{${xmlMatch[1]}}`;
        }

        // 解析坐标 {x,y}
        const coordMatch = commandText.match(/\{(\d+\s*,\s*\d+)\}/);
        if (coordMatch) {
            // 坐标可能用于不同参数，根据命令类型判断
            if (type === 'swipe') {
                // 滑动命令可能有起点和终点坐标
                params.startPoint = `{${coordMatch[1]}}`;
            } else {
                params.target = `{${coordMatch[1]}}`;
            }
        }

        return { type, params };
    },

    /**
     * TKS命令名到类型的映射
     * @param {string} tksCommand - TKS命令名
     * @returns {string} 命令类型
     */
    tksCommandToType(tksCommand) {
        const mapping = {
            '启动': 'launch',
            '关闭': 'close',
            '点击': 'click',
            '按压': 'press',
            '滑动': 'swipe',
            '拖动': 'drag',
            '定向拖动': 'directional_drag',
            '输入': 'input',
            '清理': 'clear',
            '隐藏键盘': 'hide_keyboard',
            '等待': 'wait',
            '返回': 'back',
            '断言': 'assert',
            '读取': 'read'
        };

        return mapping[tksCommand] || 'unknown';
    },

    /**
     * 临时适配方法：获取TKS代码
     * @returns {string} TKS代码
     */
    getTKSCode() {
        return this.buffer ? this.buffer.getRawContent() : '';
    },

    /**
     * 将命令对象转换为TKS文本行
     * @param {Object} command - 命令对象
     * @returns {string|null} TKS命令行
     */
    commandToTKSLine(command) {
        const definition = window.CommandUtils.findCommandDefinition(command.type);
        if (!definition) return null;

        const commandName = definition.tksCommand;
        const params = [];

        // 根据命令类型构造参数
        definition.params.forEach(param => {
            const value = command.params[param.name];
            if (value) {
                params.push(value);
            }
        });

        // 构造TKS命令行
        if (params.length > 0) {
            return `    ${commandName} [${params.join(', ')}]`;
        } else {
            return `    ${commandName}`;
        }
    },

    /**
     * 判断是否是命令行（与 TKEEditorBuffer 保持一致）
     * @param {string} line - 文本行
     * @returns {boolean} 是否是命令行
     */
    isCommandLine(line) {
        if (!line || line.startsWith('#') || line.startsWith('用例:') ||
            line.startsWith('脚本名:') || line === '详情:' || line === '步骤:' ||
            line.includes('appPackage:') || line.includes('appActivity:')) {
            return false;
        }
        return true;
    }
};

// 导出到全局
window.CommandParser = CommandParser;

if (window.rLog) {
    window.rLog('✅ CommandParser 模块已加载');
}
