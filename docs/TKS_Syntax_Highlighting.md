# TKS 语法高亮系统

## 概述

TKS 语法高亮系统由两部分组成：

1. **tks-language-support** - 独立的语法定义包（可发布到 VSCode Marketplace）
2. **Web 编辑器适配器** - Electron app 中的集成实现

## 架构设计

```
┌─────────────────────────────────────────┐
│  tks-language-support (独立包)          │
│  ├─ grammars/tks.tmLanguage.json       │  ← VSCode/nvim 使用
│  ├─ src/browser.js                     │  ← 语法规则定义
│  └─ package.json                       │  ← VSCode 扩展配置
└─────────────────────────────────────────┘
                  │
                  │ 导出语法规则
                  ▼
┌─────────────────────────────────────────┐
│  Electron App (Web 编辑器)              │
│  ├─ tks-syntax.js                      │  ← 导入语法定义
│  ├─ editor-syntax-highlighter.js      │  ← 词法分析器
│  ├─ editor-tab.js                     │  ← 使用高亮器
│  └─ syntax-highlight.css              │  ← 样式定义
└─────────────────────────────────────────┘
```

## 1. VSCode 扩展使用

### 安装方式

#### 方式 1: 本地开发

```bash
cd tks-language-support

# 创建符号链接到 VSCode 扩展目录
# macOS/Linux
ln -s $(pwd) ~/.vscode/extensions/tks-language-support

# Windows
mklink /D "%USERPROFILE%\.vscode\extensions\tks-language-support" "%CD%"

# 重启 VSCode
```

#### 方式 2: 打包安装

```bash
cd tks-language-support

# 安装依赖
npm install

# 打包
npm run package
# 生成 tks-language-support-1.0.0.vsix

# 在 VSCode 中安装
code --install-extension tks-language-support-1.0.0.vsix
```

#### 方式 3: 发布到 Marketplace

```bash
# 1. 安装 vsce
npm install -g vsce

# 2. 创建 Azure DevOps Personal Access Token
# 访问: https://dev.azure.com/

# 3. 登录
vsce login your-publisher-name

# 4. 发布
cd tks-language-support
vsce publish
```

### 效果预览

打开任何 `.tks` 文件，即可看到语法高亮：

```tks
# 这是注释 - 绿色斜体
步骤:  - 紫色粗体

启动 [com.example.app, .MainActivity]  # "启动" 蓝色粗体

点击 [{200, 400}]           # "点击" 蓝色，{200, 400} 浅绿色
点击 [{相机按钮}]           # {相机按钮} 橙色
点击 [@{加号图标}]          # @{加号图标} 青色粗体

等待 [1000]                 # 1000 浅绿色
```

## 2. Web 编辑器使用

### 文件说明

#### `tks-syntax.js`
- 语法定义（从 tks-language-support 导出）
- 命令关键字列表
- 正则表达式模式
- 主题颜色

#### `editor-syntax-highlighter.js`
- 词法分析器（Tokenizer）
- 将文本分解为 tokens
- 生成带语法高亮的 HTML

#### `editor-tab.js`
- 集成点：`highlightTKSSyntax(text)` 方法
- 调用 `TKSSyntaxHighlighter.highlight(text)`

### 工作流程

```javascript
// 1. 用户输入 TKS 代码
const tksCode = "点击 [{相机按钮}]";

// 2. 词法分析
const tokens = TKSSyntaxHighlighter.tokenize(tksCode);
// [
//   { type: 'command', value: '点击' },
//   { type: 'text', value: ' ' },
//   { type: 'bracket', value: '[' },
//   { type: 'locator', value: '{相机按钮}' },
//   { type: 'bracket', value: ']' }
// ]

// 3. 转换为 HTML
const html = TKSSyntaxHighlighter.tokensToHTML(tokens);
// <span class="tks-command">点击</span> ...

// 4. 渲染到编辑器
textContentEl.innerHTML = html;
```

### 词法分析规则优先级

1. **注释** - `#` 开头的整行
2. **步骤标记** - `步骤:`
3. **图片定位器** - `@{...}`
4. **坐标定位器** - `{数字,数字}`
5. **XML定位器** - `{...}`
6. **命令关键字** - 启动、点击等
7. **方向常量** - up、down、left、right
8. **断言状态** - 存在、不存在、可见、不可见
9. **运算符** - `==`
10. **括号** - `[` `]`
11. **逗号** - `,`
12. **数字** - `\d+`
13. **文本** - 其他所有字符

## 3. 样式自定义

### CSS 变量（推荐）

修改 `syntax-highlight.css`：

```css
:root {
  --tks-comment: #6a9955;
  --tks-command: #569cd6;
  --tks-locator: #ce9178;
  /* ... */
}

.tks-comment {
  color: var(--tks-comment);
}
```

### 主题切换

```javascript
// 在 tks-syntax.js 中定义多个主题
const themes = {
  dark: { /* ... */ },
  light: { /* ... */ },
  monokai: { /* ... */ }
};

// 切换主题
function applyTheme(themeName) {
  const theme = TKSSyntax.themes[themeName];
  document.documentElement.style.setProperty('--tks-command', theme.command);
  // ...
}
```

## 4. 扩展语法

### 添加新命令

#### 1. 在 `tks-language-support/grammars/tks.tmLanguage.json` 中：

```json
{
  "name": "keyword.control.new-category.tks",
  "match": "\\b(新命令)\\b"
}
```

#### 2. 在 `tks-syntax.js` 中：

```javascript
commands: {
  // ...
  newCategory: ['新命令']
}
```

#### 3. 在 `editor-syntax-highlighter.js` 中：

```javascript
// 在 tokenizeLine 方法中添加
const newCommandMatch = remaining.match(/^(新命令)(?=\s|$|\[)/);
if (newCommandMatch) {
  tokens.push({ type: this.TOKEN_TYPES.COMMAND, value: newCommandMatch[1] });
  // ...
}
```

#### 4. 在 `syntax-highlight.css` 中（可选）：

```css
.tks-new-category {
  color: #ff0000;
  font-weight: bold;
}
```

## 5. 性能优化

### 增量更新（已实现）

编辑器只在用户输入时重新高亮，不是每帧都更新。

### 虚拟滚动（可选）

对于超长文件（>1000行），可以实现虚拟滚动：

```javascript
// 只渲染可见区域的代码
function renderVisibleLines(startLine, endLine) {
  const visibleCode = allLines.slice(startLine, endLine).join('\n');
  return TKSSyntaxHighlighter.highlight(visibleCode);
}
```

## 6. 调试

### 查看 Tokens

```javascript
const tokens = TKSSyntaxHighlighter.tokenize("点击 [{相机按钮}]");
console.table(tokens);
```

### 测试高亮器

```javascript
// 在浏览器控制台
const testCode = `
步骤:
点击 [{200, 400}]
点击 [@{图标}]
`;

const html = TKSSyntaxHighlighter.highlight(testCode);
console.log(html);
```

## 7. 常见问题

### Q: 高亮不生效？
A: 检查脚本加载顺序，确保 `tks-syntax.js` 和 `editor-syntax-highlighter.js` 在 `editor-tab.js` 之前加载。

### Q: 某些语法没有高亮？
A: 检查 `tokenizeLine` 方法中的正则表达式是否匹配。

### Q: VSCode 扩展不工作？
A:
1. 检查 `package.json` 中的 `contributes` 配置
2. 重启 VSCode
3. 查看 VSCode 开发者工具的控制台错误

### Q: 如何支持更多编辑器？
A:
- **nvim**: 使用 Tree-sitter 或转换 TextMate Grammar
- **Sublime Text**: 支持 `.tmLanguage` 文件
- **Atom**: 支持 TextMate Grammar
- **Emacs**: 需要编写 major mode

## 8. 版本迁移

### 从旧的高亮系统迁移

旧代码（已删除）：
```javascript
// ❌ 旧方式 - 多次正则替换，容易冲突
.replace(/(点击)/g, '<span>$1</span>')
.replace(/\{([^}]+)\}/g, '{<span>$1</span>}')
```

新代码：
```javascript
// ✅ 新方式 - 词法分析，精确控制
TKSSyntaxHighlighter.highlight(text);
```

优势：
- ✅ 无正则冲突
- ✅ 性能更好
- ✅ 更易维护
- ✅ 更易扩展

## 9. 未来计划

- [ ] LSP 支持（语义分析、智能补全）
- [ ] 错误诊断（实时语法检查）
- [ ] 代码折叠
- [ ] 自动缩进
- [ ] 代码片段（Snippets）

## 参考资料

- [TextMate Language Grammars](https://macromates.com/manual/en/language_grammars)
- [VSCode Extension API](https://code.visualstudio.com/api)
- [Monaco Editor](https://microsoft.github.io/monaco-editor/)
