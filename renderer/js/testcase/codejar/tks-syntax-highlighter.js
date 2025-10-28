/**
 * TKS 语法高亮器 - Web 编辑器适配器
 * 基于 tks-language-support 的语法定义
 */

const TKSSyntaxHighlighter = {
  /**
   * Token 类型定义
   */
  TOKEN_TYPES: {
    COMMENT: 'comment',
    SECTION: 'section',
    COMMAND: 'command',
    IMAGE_LOCATOR: 'image-locator',
    COORDINATE: 'coordinate',
    LOCATOR: 'locator',
    LOCATOR_STRATEGY: 'locator-strategy',  // 新增：定位器策略 (#resourceId, #text 等)
    DIRECTION: 'direction',
    ASSERTION_STATE: 'assertion-state',
    OPERATOR: 'operator',
    NUMBER: 'number',
    BRACKET: 'bracket',
    COMMA: 'comma',
    TEXT: 'text'
  },

  /**
   * 词法分析 - 将文本转换为 tokens
   * @param {string} text - 要分析的文本
   * @returns {Array} tokens 数组
   */
  tokenize(text) {
    const tokens = [];
    const lines = text.split('\n');

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
      const line = lines[lineIndex];
      const lineTokens = this.tokenizeLine(line);

      tokens.push(...lineTokens);

      // 添加换行符（除了最后一行）
      if (lineIndex < lines.length - 1) {
        tokens.push({ type: 'newline', value: '\n' });
      }
    }

    return tokens;
  },

  /**
   * 分析单行文本
   */
  tokenizeLine(line) {
    const tokens = [];
    let remaining = line;
    let position = 0;

    // 1. 检查注释（整行）
    if (/#/.test(remaining)) {
      const commentMatch = remaining.match(/^([^#]*)(#.*)$/);
      if (commentMatch) {
        const [, before, comment] = commentMatch;
        if (before) {
          // 递归处理注释前的部分
          tokens.push(...this.tokenizeLine(before));
        }
        tokens.push({ type: this.TOKEN_TYPES.COMMENT, value: comment });
        return tokens;
      }
    }

    // 2. 检查步骤标记
    const sectionMatch = remaining.match(/^(\s*)(步骤)(\s*):(.*)$/);
    if (sectionMatch) {
      const [, leadingSpace, keyword, trailingSpace, rest] = sectionMatch;
      if (leadingSpace) tokens.push({ type: this.TOKEN_TYPES.TEXT, value: leadingSpace });
      tokens.push({ type: this.TOKEN_TYPES.SECTION, value: keyword });
      if (trailingSpace) tokens.push({ type: this.TOKEN_TYPES.TEXT, value: trailingSpace });
      tokens.push({ type: this.TOKEN_TYPES.TEXT, value: ':' });
      if (rest) tokens.push({ type: this.TOKEN_TYPES.TEXT, value: rest });
      return tokens;
    }

    // 3. 逐字符扫描，识别其他 tokens
    while (remaining.length > 0) {
      let matched = false;

      // 检查图片定位器 @{...}
      if (remaining.startsWith('@{')) {
        const endIndex = remaining.indexOf('}', 2);
        if (endIndex !== -1) {
          const imageLocator = remaining.substring(0, endIndex + 1);
          tokens.push({ type: this.TOKEN_TYPES.IMAGE_LOCATOR, value: imageLocator });
          remaining = remaining.substring(endIndex + 1);
          matched = true;
          continue;
        }
      }

      // 检查普通定位器 {...} (需要区分坐标和XML)
      if (remaining.startsWith('{') && !remaining.startsWith('@{')) {
        const endIndex = remaining.indexOf('}', 1);
        if (endIndex !== -1) {
          const content = remaining.substring(1, endIndex);
          const fullLocator = remaining.substring(0, endIndex + 1);

          // 检查是否是坐标
          if (/^\s*\d+\s*,\s*\d+\s*$/.test(content)) {
            tokens.push({ type: this.TOKEN_TYPES.COORDINATE, value: fullLocator });
          } else {
            tokens.push({ type: this.TOKEN_TYPES.LOCATOR, value: fullLocator });
          }

          remaining = remaining.substring(endIndex + 1);

          // 🔥 检查是否紧跟着策略标记 &resourceId, &text, &className, &contentDesc, &xpath
          const strategyMatch = remaining.match(/^&(resourceId|text|className|contentDesc|xpath)(?=\s|,|\]|$)/);
          if (strategyMatch) {
            tokens.push({ type: this.TOKEN_TYPES.LOCATOR_STRATEGY, value: strategyMatch[0] });
            remaining = remaining.substring(strategyMatch[0].length);
          }

          matched = true;
          continue;
        }
      }

      // 检查命令关键字
      const commandMatch = remaining.match(/^(启动|关闭|点击|按压|滑动|拖动|定向拖动|输入|清理|隐藏键盘|等待|返回|断言|读取)(?=\s|$|\[)/);
      if (commandMatch) {
        tokens.push({ type: this.TOKEN_TYPES.COMMAND, value: commandMatch[1] });
        remaining = remaining.substring(commandMatch[1].length);
        matched = true;
        continue;
      }

      // 检查方向常量
      const directionMatch = remaining.match(/^(up|down|left|right)(?=\s|,|\]|$)/);
      if (directionMatch) {
        tokens.push({ type: this.TOKEN_TYPES.DIRECTION, value: directionMatch[1] });
        remaining = remaining.substring(directionMatch[1].length);
        matched = true;
        continue;
      }

      // 检查断言状态
      const assertionMatch = remaining.match(/^(存在|不存在|可见|不可见)(?=\s|,|\]|$)/);
      if (assertionMatch) {
        tokens.push({ type: this.TOKEN_TYPES.ASSERTION_STATE, value: assertionMatch[1] });
        remaining = remaining.substring(assertionMatch[1].length);
        matched = true;
        continue;
      }

      // 检查运算符
      if (remaining.startsWith('==')) {
        tokens.push({ type: this.TOKEN_TYPES.OPERATOR, value: '==' });
        remaining = remaining.substring(2);
        matched = true;
        continue;
      }

      // 检查括号
      if (remaining.startsWith('[')) {
        tokens.push({ type: this.TOKEN_TYPES.BRACKET, value: '[' });
        remaining = remaining.substring(1);
        matched = true;
        continue;
      }

      if (remaining.startsWith(']')) {
        tokens.push({ type: this.TOKEN_TYPES.BRACKET, value: ']' });
        remaining = remaining.substring(1);
        matched = true;
        continue;
      }

      // 检查逗号
      if (remaining.startsWith(',')) {
        tokens.push({ type: this.TOKEN_TYPES.COMMA, value: ',' });
        remaining = remaining.substring(1);
        matched = true;
        continue;
      }

      // 检查数字
      const numberMatch = remaining.match(/^\d+/);
      if (numberMatch) {
        tokens.push({ type: this.TOKEN_TYPES.NUMBER, value: numberMatch[0] });
        remaining = remaining.substring(numberMatch[0].length);
        matched = true;
        continue;
      }

      // 如果都不匹配，作为普通文本
      if (!matched) {
        tokens.push({ type: this.TOKEN_TYPES.TEXT, value: remaining[0] });
        remaining = remaining.substring(1);
      }
    }

    return tokens;
  },

  /**
   * 将 tokens 渲染为 HTML
   * @param {Array} tokens - token 数组
   * @returns {string} HTML 字符串
   */
  tokensToHTML(tokens) {
    let html = '';

    for (const token of tokens) {
      const escapedValue = this.escapeHTML(token.value);

      switch (token.type) {
        case 'newline':
          html += '\n';
          break;
        case this.TOKEN_TYPES.COMMENT:
          html += `<span class="tks-comment">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.SECTION:
          html += `<span class="tks-section">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.COMMAND:
          html += `<span class="tks-command">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.IMAGE_LOCATOR:
          html += `<span class="tks-image-locator">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.COORDINATE:
          html += `<span class="tks-coordinate">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.LOCATOR:
          html += `<span class="tks-locator">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.LOCATOR_STRATEGY:
          html += `<span class="tks-locator-strategy">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.DIRECTION:
          html += `<span class="tks-direction">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.ASSERTION_STATE:
          html += `<span class="tks-assertion-state">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.OPERATOR:
          html += `<span class="tks-operator">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.NUMBER:
          html += `<span class="tks-number">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.BRACKET:
          html += `<span class="tks-bracket">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.COMMA:
          html += `<span class="tks-comma">${escapedValue}</span>`;
          break;
        case this.TOKEN_TYPES.TEXT:
        default:
          html += escapedValue;
          break;
      }
    }

    return html;
  },

  /**
   * 主入口：高亮文本
   * @param {string} text - 要高亮的文本
   * @returns {string} 高亮后的 HTML
   */
  highlight(text) {
    if (!text) return '';

    const tokens = this.tokenize(text);
    return this.tokensToHTML(tokens);
  },

  /**
   * 转义 HTML 特殊字符
   */
  escapeHTML(text) {
    const map = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#039;'
    };
    return text.replace(/[&<>"']/g, m => map[m]);
  }
};

// 导出到全局
if (typeof window !== 'undefined') {
  window.TKSSyntaxHighlighter = TKSSyntaxHighlighter;
}

// 记录加载
if (window.rLog) {
  window.rLog('✅ TKSSyntaxHighlighter 模块已加载');
}
