   
    // 计算显示行号：将TKS引擎的原始行号转换为编辑器中显示的行号
    calculateDisplayLineNumber(tksOriginalLineNumber) {
        console.log('计算显示行号 - TKS引擎原始行号:', tksOriginalLineNumber);
        console.log('脚本模型信息:', {
            originalLines: this.script.originalLines ? this.script.originalLines.length : '无',
            commands: this.script.commands.length,
            mapping: this.script.lineToCommandMap ? this.script.lineToCommandMap.length : '无'
        });
        
        if (!this.textContentEl || !this.script.originalLines) {
            console.warn('缺少必要的数据');
            return -1;
        }
        
        // TKS引擎报告的是基于原始文本的行号(1基索引)，需要转换为0基索引
        const originalLineIndex = tksOriginalLineNumber - 1;
        
        // 检查原始行号是否有效
        if (originalLineIndex < 0 || originalLineIndex >= this.script.originalLines.length) {
            console.warn('TKS原始行号超出范围:', tksOriginalLineNumber, '有效范围: 1-' + this.script.originalLines.length);
            return -1;
        }
        
        // 从映射中查找对应的命令索引
        const commandIndex = this.script.lineToCommandMap[originalLineIndex];
        console.log('原始行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);
        
        if (commandIndex === null) {
            console.warn('原始行号不是命令行:', tksOriginalLineNumber);
            return -1;
        }
        
        // 现在需要在显示的文本中找到这个命令对应的行
        const lines = this.textContentEl.textContent.split('\n');
        let stepsStartLine = -1;
        
        // 找到"步骤:"行
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].trim() === '步骤:') {
                stepsStartLine = i + 1;
                break;
            }
        }
        
        if (stepsStartLine === -1) {
            console.warn('未找到步骤行');
            return -1;
        }
        
        // 在步骤区域中找到第N个命令行
        let foundCommandCount = 0;
        for (let i = stepsStartLine; i < lines.length; i++) {
            const line = lines[i].trim();
            if (!line || line.startsWith('#') || 
                line.startsWith('用例:') || line.startsWith('脚本名:') ||
                line === '详情:' || line === '步骤:' ||
                line.includes('appPackage:') || line.includes('appActivity:')) {
                continue;
            }
            
            // 这是一个命令行
            if (foundCommandCount === commandIndex) {
                const displayLine = i + 1;
                console.log('找到显示行号:', displayLine, '(原始行号:', tksOriginalLineNumber, '命令索引:', commandIndex, ')');
                return displayLine;
            }
            foundCommandCount++;
        }
        
        console.warn('未找到对应的显示行号');
        return -1;
    }
 