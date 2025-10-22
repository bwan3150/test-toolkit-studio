/**
 * TKS Language Support - 浏览器版本
 * 用于 Web 编辑器
 */

// 导出简化的规则定义
const TKSSyntax = {
  // 命令关键字
  commands: {
    process: ['启动', '关闭'],
    interaction: ['点击', '按压', '滑动', '拖动', '定向拖动'],
    input: ['输入', '清理', '隐藏键盘'],
    timing: ['等待'],
    navigation: ['返回'],
    assertion: ['断言', '读取']
  },

  // 获取所有命令关键字（平铺数组）
  getAllCommands() {
    return Object.values(this.commands).flat();
  },

  // 常量
  constants: {
    direction: ['up', 'down', 'left', 'right'],
    assertionState: ['存在', '不存在', '可见', '不可见']
  },

  // 正则表达式模式
  patterns: {
    comment: /#.*$/gm,
    section: /^\s*(步骤)\s*:/gm,
    imageLocator: /@\{[^}]+\}/g,
    locator: /(?<!@)\{[^}]+\}/g,
    coordinate: /\{\s*\d+\s*,\s*\d+\s*\}/g,
    parameter: /\[[^\]]*\]/g,
    operator: /==/g,
    number: /\b\d+\b/g
  },

  // 主题颜色定义
  themes: {
    dark: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      comment: '#6a9955',
      section: '#c586c0',
      command: '#569cd6',
      locator: '#ce9178',
      imageLocator: '#4ec9b0',
      coordinate: '#b5cea8',
      direction: '#4fc1ff',
      assertionState: '#dcdcaa',
      operator: '#d4d4d4',
      number: '#b5cea8',
      bracket: '#ffd700',
      comma: '#d4d4d4'
    },
    light: {
      background: '#ffffff',
      foreground: '#000000',
      comment: '#008000',
      section: '#af00db',
      command: '#0000ff',
      locator: '#a31515',
      imageLocator: '#267f99',
      coordinate: '#098658',
      direction: '#0070c1',
      assertionState: '#795e26',
      operator: '#000000',
      number: '#098658',
      bracket: '#ffd700',
      comma: '#000000'
    }
  }
};

// 导出到全局（浏览器环境）
if (typeof window !== 'undefined') {
  window.TKSSyntax = TKSSyntax;
}

// 导出到 CommonJS（Node.js 环境）
if (typeof module !== 'undefined' && module.exports) {
  module.exports = TKSSyntax;
}
