// 统一的脚本数据模型
class ScriptModel {
    constructor() {
        this.header = {
            name: '',
            scriptName: '',
            details: {
                appPackage: '',
                appActivity: ''
            }
        };
        this.commands = [];
    }
    
    fromTKSCode(tksCode) {
        this.commands = [];
        this.originalLines = tksCode.split('\n'); // 保留原始行
        this.lineToCommandMap = []; // 行号到命令的映射
        
        let commandIndex = 0;
        
        this.originalLines.forEach((line, lineIndex) => {
            const trimmed = line.trim();
            
            // 跳过头部信息
            if (!trimmed || trimmed.startsWith('#') || 
                trimmed.startsWith('用例:') || trimmed.startsWith('脚本名:') ||
                trimmed === '详情:' || trimmed === '步骤:' ||
                trimmed.includes('appPackage:') || trimmed.includes('appActivity:')) {
                this.lineToCommandMap.push(null); // 非命令行
                return;
            }
            
            const command = this.parseTKSLine(trimmed);
            if (command) {
                this.commands.push(command);
                this.lineToCommandMap.push(commandIndex); // 映射到命令索引
                commandIndex++;
            } else {
                this.lineToCommandMap.push(null); // 无效命令行
            }
        });
        
        window.rLog('脚本解析完成:', {
            totalLines: this.originalLines.length,
            commands: this.commands.length,
            lineMapping: this.lineToCommandMap
        });
    }
    
    parseTKSLine(line) {
        // 新TKS语法解析: 命令名 [参数1, 参数2, ...]
        const match = line.match(/^(\S+)(?:\s+\[(.*?)\])?$/);
        if (!match) return null;
        
        const commandName = match[1];
        const params = match[2] || '';
        
        // 根据命令名称确定类型
        const typeMap = {
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
        
        const type = typeMap[commandName];
        if (!type) return null;
        
        const command = { type, params: {} };
        
        // 解析参数
        if (params) {
            const paramValues = this.parseParams(params);
            
            // 根据新TKS语法规则分配参数
            switch (type) {
                case 'launch':
                    // 启动 [包名, Activity名字]
                    command.params.package = paramValues[0] || '';
                    command.params.activity = paramValues[1] || '';
                    break;
                case 'close':
                    // 关闭 [包名, Activity名字]
                    command.params.package = paramValues[0] || '';
                    command.params.activity = paramValues[1] || '';
                    break;
                case 'click':
                    // 点击 [坐标/XML/图片元素]
                    command.params.target = paramValues[0] || '';
                    break;
                case 'press':
                    // 按压 [坐标/XML/图片元素, 持续时长/ms]
                    command.params.target = paramValues[0] || '';
                    command.params.duration = paramValues[1] || '1000';
                    break;
                case 'swipe':
                    // 滑动 [起点坐标, 终点坐标, 持续时长/ms]
                    command.params.startPoint = paramValues[0] || '';
                    command.params.endPoint = paramValues[1] || '';
                    command.params.duration = paramValues[2] || '1000';
                    break;
                case 'drag':
                    // 拖动 [起点, 终点坐标, 持续时长]
                    command.params.target = paramValues[0] || '';
                    command.params.endPoint = paramValues[1] || '';
                    command.params.duration = paramValues[2] || '1000';
                    break;
                case 'directional_drag':
                    // 定向拖动 [元素, 方向, 距离, 持续时长]
                    command.params.target = paramValues[0] || '';
                    command.params.direction = paramValues[1] || 'up';
                    command.params.distance = paramValues[2] || '300';
                    command.params.duration = paramValues[3] || '1000';
                    break;
                case 'input':
                    // 输入 [坐标/XML元素, 文本内容]
                    command.params.target = paramValues[0] || '';
                    command.params.text = paramValues[1] || '';
                    break;
                case 'clear':
                    // 清理 [坐标/XML元素]
                    command.params.target = paramValues[0] || '';
                    break;
                case 'wait':
                    // 等待 [等待时长/ms]
                    command.params.duration = paramValues[0] || '1000';
                    break;
                case 'assert':
                    // 断言 [XML/图片元素, 存在/不存在/可见/不可见]
                    command.params.target = paramValues[0] || '';
                    command.params.condition = paramValues[1] || '存在';
                    break;
                case 'read':
                    // 读取 [坐标/XML元素, 左右扩展, 上下扩展] 或 读取 [XML元素]
                    command.params.target = paramValues[0] || '';
                    command.params.leftRight = paramValues[1] || '';
                    command.params.upDown = paramValues[2] || '';
                    break;
            }
        }
        
        return command;
    }
    
    parseParams(paramsStr) {
        if (!paramsStr) return [];
        
        const parts = [];
        let current = '';
        let braceDepth = 0;
        let inString = false;
        
        for (let i = 0; i < paramsStr.length; i++) {
            const char = paramsStr[i];
            const nextChar = paramsStr[i + 1];
            
            // 处理花括号嵌套 (坐标、XML元素、图片元素)
            if (char === '{') {
                braceDepth++;
            } else if (char === '}') {
                braceDepth--;
            }
            
            // 检测字符串内容 (简单处理，假设没有嵌套引号)
            if (char === '"' || char === "'") {
                inString = !inString;
            }
            
            // 只在非嵌套状态下分割参数
            if (char === ',' && braceDepth === 0 && !inString) {
                parts.push(current.trim());
                current = '';
            } else {
                current += char;
            }
        }
        
        if (current.trim()) {
            parts.push(current.trim());
        }
        
        return parts;
    }
    
    toTKSCode() {
        if (this.commands.length === 0) {
            return `用例: 新测试用例
脚本名: new_test
详情:
    appPackage: com.example.app
    appActivity: .MainActivity
步骤:
    启动 [com.example.app, .MainActivity]
    等待 [2000]
    点击 [{示例按钮}]
    断言 [{示例元素}, 存在]`;
        }
        
        const commandLines = this.commands.map(command => {
            const commandName = this.getCommandName(command.type);
            const params = this.getCommandParams(command);
            
            if (params.length > 0) {
                return `    ${commandName} [${params.join(', ')}]`;
            } else {
                return `    ${commandName}`;
            }
        }).join('\n');
        
        return `用例: 测试用例
脚本名: test_script
详情:
    appPackage: com.example.app
    appActivity: .MainActivity
步骤:
${commandLines}`;
    }
    
    getCommandName(type) {
        const nameMap = {
            'launch': '启动',
            'close': '关闭',
            'click': '点击',
            'press': '按压',
            'swipe': '滑动',
            'drag': '拖动',
            'directional_drag': '定向拖动',
            'input': '输入',
            'clear': '清理',
            'hide_keyboard': '隐藏键盘',
            'wait': '等待',
            'back': '返回',
            'assert': '断言',
            'read': '读取'
        };
        return nameMap[type] || type;
    }
    
    getCommandParams(command) {
        const params = [];
        
        switch (command.type) {
            case 'launch':
                // 启动 [包名, Activity名字]
                if (command.params.package) params.push(command.params.package);
                if (command.params.activity) params.push(command.params.activity);
                break;
            case 'close':
                // 关闭 [包名, Activity名字]
                if (command.params.package) params.push(command.params.package);
                if (command.params.activity) params.push(command.params.activity);
                break;
            case 'click':
            case 'clear':
                // 点击/清理 [坐标/XML/图片元素]
                if (command.params.target) params.push(command.params.target);
                break;
            case 'press':
                // 按压 [坐标/XML/图片元素, 持续时长/ms]
                if (command.params.target) params.push(command.params.target);
                if (command.params.duration && command.params.duration !== '1000') {
                    params.push(command.params.duration);
                }
                break;
            case 'swipe':
                // 滑动 [起点坐标, 终点坐标, 持续时长/ms]
                if (command.params.startPoint) params.push(command.params.startPoint);
                if (command.params.endPoint) params.push(command.params.endPoint);
                if (command.params.duration && command.params.duration !== '1000') {
                    params.push(command.params.duration);
                }
                break;
            case 'drag':
                // 拖动 [起点, 终点坐标, 持续时长]
                if (command.params.target) params.push(command.params.target);
                if (command.params.endPoint) params.push(command.params.endPoint);
                if (command.params.duration && command.params.duration !== '1000') {
                    params.push(command.params.duration);
                }
                break;
            case 'directional_drag':
                // 定向拖动 [元素, 方向, 距离, 持续时长]
                if (command.params.target) params.push(command.params.target);
                if (command.params.direction) params.push(command.params.direction);
                if (command.params.distance) params.push(command.params.distance);
                if (command.params.duration && command.params.duration !== '1000') {
                    params.push(command.params.duration);
                }
                break;
            case 'input':
                // 输入 [坐标/XML元素, 文本内容]
                if (command.params.target) params.push(command.params.target);
                if (command.params.text) params.push(command.params.text);
                break;
            case 'wait':
                // 等待 [等待时长/ms]
                if (command.params.duration && command.params.duration !== '1000') {
                    params.push(command.params.duration);
                }
                break;
            case 'assert':
                // 断言 [XML/图片元素, 存在/不存在/可见/不可见]
                if (command.params.target) params.push(command.params.target);
                if (command.params.condition) {
                    params.push(command.params.condition);
                } else {
                    params.push('存在'); // 默认条件
                }
                break;
            case 'read':
                // 读取 [坐标/XML元素, 左右扩展, 上下扩展] 或 读取 [XML元素]
                if (command.params.target) params.push(command.params.target);
                if (command.params.leftRight) params.push(command.params.leftRight);
                if (command.params.upDown) params.push(command.params.upDown);
                break;
        }
        
        return params;
    }
    
    getCommands() {
        return this.commands;
    }
    
    addCommand(command) {
        this.commands.push(command);
    }
    
    insertCommand(command, index) {
        this.commands.splice(index, 0, command);
    }
    
    reorderCommand(fromIndex, toIndex) {
        if (fromIndex < 0 || fromIndex >= this.commands.length) return;
        if (toIndex < 0 || toIndex >= this.commands.length) return;
        if (fromIndex === toIndex) return;
        
        // 移除原位置的命令
        const [command] = this.commands.splice(fromIndex, 1);
        // 插入到新位置
        this.commands.splice(toIndex, 0, command);
    }
    
    removeCommand(index) {
        window.rLog(`ScriptModel.removeCommand: 尝试删除索引 ${index}`);
        window.rLog(`当前commands数组长度: ${this.commands.length}`);
        
        if (index < 0 || index >= this.commands.length) {
            window.rError(`索引无效: ${index}, 数组长度: ${this.commands.length}`);
            return;
        }
        
        window.rLog(`删除的命令: ${JSON.stringify(this.commands[index])}`);
        this.commands.splice(index, 1);
        window.rLog(`删除后数组长度: ${this.commands.length}`);
    }
    
    clearCommands() {
        this.commands = [];
    }
    
    updateCommandParam(index, param, value) {
        if (this.commands[index]) {
            this.commands[index].params[param] = value;
        }
    }
}

// 导出到全局
window.ScriptModel = ScriptModel;