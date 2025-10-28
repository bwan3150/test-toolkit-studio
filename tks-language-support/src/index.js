/**
 * TKS Language Support - 通用语法定义
 * 可被 VSCode、Web 编辑器等使用
 */

const fs = require('fs');
const path = require('path');

// 导出语法规则（从 TextMate Grammar 读取）
function getGrammar() {
  const grammarPath = path.join(__dirname, '../grammars/tks.tmLanguage.json');
  const grammar = JSON.parse(fs.readFileSync(grammarPath, 'utf8'));
  return grammar;
}

// 导出简化的规则定义（方便 Web 编辑器使用）
const rules = {
  // 命令关键字
  commands: {
    process: ['启动', '关闭'],
    interaction: ['点击', '按压', '滑动', '拖动', '定向拖动'],
    input: ['输入', '清理', '隐藏键盘'],
    timing: ['等待'],
    navigation: ['返回'],
    assertion: ['断言', '读取']
  },

  // 常量
  constants: {
    direction: ['up', 'down', 'left', 'right'],
    assertionState: ['存在', '不存在', '可见', '不可见']
  },

  // 正则表达式模式
  patterns: {
    comment: /#.*$/,
    section: /^\s*(步骤)\s*:/,
    imageLocator: /@\{[^}]+\}/,
    locator: /\{[^}]+\}/,
    locatorStrategy: /(?<=\})&(resourceId|text|className|xpath)\b/,  // v0.5.12-beta 新增
    coordinate: /\{\s*\d+\s*,\s*\d+\s*\}/,
    parameter: /\[[^\]]*\]/,
    operator: /==/,
    number: /\b\d+\b/
  }
};

// 主题颜色定义
const themes = {
  dark: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    comment: '#6a9955',
    section: '#c586c0',
    command: '#569cd6',
    locator: '#ce9178',
    locatorStrategy: '#f0a070',  // v0.5.12-beta 新增：策略标记颜色
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
    locatorStrategy: '#d2691e',  // v0.5.12-beta 新增：策略标记颜色
    imageLocator: '#267f99',
    coordinate: '#098658',
    direction: '#0070c1',
    assertionState: '#795e26',
    operator: '#000000',
    number: '#098658',
    bracket: '#ffd700',
    comma: '#000000'
  }
};

module.exports = {
  getGrammar,
  rules,
  themes
};
