/**
 * TKS 脚本块解析器
 * 将 TKS 脚本解析为可编辑的块
 * 不依赖 tke parser，完全基于 tokenizer
 */

const TKSBlockParser = {
  /**
   * 解析 TKS 脚本为命令块列表
   * @param {string} tksCode - TKS 脚本代码
   * @returns {Array} 命令块列表
   */
  parse(tksCode) {
    const blocks = [];
    const lines = tksCode.split('\n');
    let inStepsSection = false;
    let currentLineNumber = 1;

    for (const line of lines) {
      const trimmed = line.trim();

      // 检测"步骤:"标记
      if (trimmed === '步骤:') {
        inStepsSection = true;
        currentLineNumber++;
        continue;
      }

      // 只解析步骤部分
      if (!inStepsSection) {
        currentLineNumber++;
        continue;
      }

      // 跳过空行和注释
      if (!trimmed || trimmed.startsWith('#')) {
        currentLineNumber++;
        continue;
      }

      // 解析命令块
      const block = this.parseCommandLine(line, currentLineNumber);
      if (block) {
        blocks.push(block);
      }

      currentLineNumber++;
    }

    return blocks;
  },

  /**
   * 解析单行命令
   * @param {string} line - 命令行
   * @param {number} lineNumber - 行号
   * @returns {object|null} 命令块对象
   */
  parseCommandLine(line, lineNumber) {
    const trimmed = line.trim();

    // 识别命令关键字
    const commandMatch = trimmed.match(/^(启动|关闭|点击|按压|滑动|拖动|定向拖动|输入|清理|隐藏键盘|等待|返回|断言|读取)/);
    if (!commandMatch) {
      return null;
    }

    const command = commandMatch[1];

    // 提取参数列表 [...]
    const paramsMatch = trimmed.match(/\[(.*)\]$/);
    const paramsString = paramsMatch ? paramsMatch[1] : '';

    // 解析参数
    const params = this.parseParameters(paramsString, command);

    return {
      lineNumber,
      command,
      params,
      raw: trimmed
    };
  },

  /**
   * 解析参数字符串
   * @param {string} paramsString - 参数字符串（不含括号）
   * @param {string} command - 命令名称
   * @returns {Array} 参数列表
   */
  parseParameters(paramsString, command) {
    if (!paramsString) {
      return [];
    }

    const params = [];
    let current = '';
    let inLocator = false;
    let depth = 0;

    for (let i = 0; i < paramsString.length; i++) {
      const char = paramsString[i];

      if (char === '{') {
        inLocator = true;
        depth++;
        current += char;
      } else if (char === '}') {
        depth--;
        current += char;
        if (depth === 0) {
          inLocator = false;
        }
      } else if (char === ',' && !inLocator) {
        // 遇到逗号且不在定位器内，分隔参数
        if (current.trim()) {
          params.push(this.parseParameter(current.trim()));
        }
        current = '';
      } else {
        current += char;
      }
    }

    // 添加最后一个参数
    if (current.trim()) {
      params.push(this.parseParameter(current.trim()));
    }

    return params;
  },

  /**
   * 解析单个参数
   * @param {string} param - 参数字符串
   * @returns {object} 参数对象
   */
  parseParameter(param) {
    const trimmed = param.trim();

    // 图片定位器 @{...}
    if (/^@\{.*\}$/.test(trimmed)) {
      return {
        type: 'image-locator',
        value: trimmed.slice(2, -1), // 去掉 @{ 和 }
        raw: trimmed
      };
    }

    // 坐标 {数字,数字}
    if (/^\{\s*\d+\s*,\s*\d+\s*\}$/.test(trimmed)) {
      const coords = trimmed.slice(1, -1).split(',').map(s => parseInt(s.trim()));
      return {
        type: 'coordinate',
        value: coords,
        raw: trimmed
      };
    }

    // XML 定位器 {...}
    if (/^\{.*\}$/.test(trimmed)) {
      return {
        type: 'locator',
        value: trimmed.slice(1, -1), // 去掉 { 和 }
        raw: trimmed
      };
    }

    // 数字
    if (/^\d+$/.test(trimmed)) {
      return {
        type: 'number',
        value: parseInt(trimmed),
        raw: trimmed
      };
    }

    // 方向
    if (/^(up|down|left|right)$/.test(trimmed)) {
      return {
        type: 'direction',
        value: trimmed,
        raw: trimmed
      };
    }

    // 断言状态
    if (/^(存在|不存在|可见|不可见)$/.test(trimmed)) {
      return {
        type: 'assertion-state',
        value: trimmed,
        raw: trimmed
      };
    }

    // 包名/Activity（带点的字符串）
    if (/^[a-zA-Z][a-zA-Z0-9_.]*$/.test(trimmed)) {
      return {
        type: 'package',
        value: trimmed,
        raw: trimmed
      };
    }

    // 普通文本
    return {
      type: 'text',
      value: trimmed,
      raw: trimmed
    };
  },

  /**
   * 获取命令的参数定义（用于 UI 显示）
   * @param {string} command - 命令名称
   * @returns {Array} 参数定义
   */
  getCommandParamsDef(command) {
    const defs = {
      '启动': [
        { name: '包名', type: 'package', required: true },
        { name: 'Activity', type: 'package', required: true }
      ],
      '关闭': [
        { name: '包名', type: 'package', required: true },
        { name: 'Activity', type: 'package', required: true }
      ],
      '点击': [
        { name: '元素', type: 'locator', required: true }
      ],
      '按压': [
        { name: '元素', type: 'locator', required: true },
        { name: '时长/ms', type: 'number', required: false }
      ],
      '滑动': [
        { name: '起点', type: 'coordinate', required: true },
        { name: '终点', type: 'coordinate', required: true },
        { name: '时长/ms', type: 'number', required: false }
      ],
      '拖动': [
        { name: '元素', type: 'locator', required: true },
        { name: '终点', type: 'coordinate', required: true },
        { name: '时长/ms', type: 'number', required: false }
      ],
      '定向拖动': [
        { name: '元素', type: 'locator', required: true },
        { name: '方向', type: 'direction', required: true },
        { name: '距离', type: 'number', required: true },
        { name: '时长/ms', type: 'number', required: false }
      ],
      '输入': [
        { name: '输入框', type: 'locator', required: true },
        { name: '文本', type: 'text', required: true }
      ],
      '清理': [
        { name: '输入框', type: 'locator', required: true }
      ],
      '隐藏键盘': [],
      '等待': [
        { name: '时长/ms', type: 'number', required: false }
      ],
      '返回': [],
      '断言': [
        { name: '元素', type: 'locator', required: true },
        { name: '状态', type: 'assertion-state', required: true }
      ],
      '读取': [
        { name: '中心坐标或元素', type: 'locator', required: true },
        { name: '宽度', type: 'number', required: false },
        { name: '高度', type: 'number', required: false }
      ]
    };

    return defs[command] || [];
  }
};

// 导出到全局
if (typeof window !== 'undefined') {
  window.TKSBlockParser = TKSBlockParser;
}

// 记录加载
if (window.rLog) {
  window.rLog('✅ TKSBlockParser 模块已加载');
}
