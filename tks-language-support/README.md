# TKS Language Support

ToolkitScript (.tks) 语法高亮支持包 - 适用于 VSCode、nvim 等编辑器

## 功能特性

- ✅ 完整的 TextMate Grammar 定义
- ✅ 支持所有 TKS 命令关键字
- ✅ 定位器语法高亮（坐标、XML、图片）
- ✅ 注释、运算符、常量高亮
- ✅ 括号自动匹配
- ✅ 导出通用语法规则（可用于自定义编辑器）

## VSCode 安装

### 方式 1: 从 VSIX 安装

```bash
cd tks-language-support
npm install
npm run package
# 会生成 tks-language-support-1.0.0.vsix

# 在 VSCode 中安装
code --install-extension tks-language-support-1.0.0.vsix
```

### 方式 2: 从源码开发模式

```bash
cd tks-language-support
# 创建符号链接到 VSCode 扩展目录
ln -s $(pwd) ~/.vscode/extensions/tks-language-support
```

重启 VSCode，打开 .tks 文件即可看到语法高亮。

## 发布到 VSCode Marketplace

```bash
# 1. 安装 vsce
npm install -g vsce

# 2. 登录（需要 Azure DevOps Personal Access Token）
vsce login your-publisher-name

# 3. 发布
cd tks-language-support
vsce publish
```

## 在 Web 编辑器中使用

```javascript
// 引入语法定义
const TKSSyntax = require('./src/browser.js');

// 获取所有命令关键字
const commands = TKSSyntax.getAllCommands();
console.log(commands); // ['启动', '关闭', '点击', ...]

// 使用正则表达式匹配
const code = '点击 [{相机按钮}]';
const hasCommand = TKSSyntax.patterns.locator.test(code);

// 使用主题颜色
const darkTheme = TKSSyntax.themes.dark;
```

## 语法规则说明

### 支持的命令

- **进程控制**: 启动、关闭
- **普通交互**: 点击、按压、滑动、拖动、定向拖动
- **输入框交互**: 输入、清理、隐藏键盘
- **时间管理**: 等待
- **页面控制**: 返回
- **断言**: 断言、读取

### 定位器类型

- **坐标**: `{200, 400}`
- **XML 元素**: `{相机按钮}`
- **图片**: `@{加号图标}`

### 示例代码

```tks
# 这是注释
步骤:

启动 [com.example.app, .MainActivity]

点击 [{200, 400}]
点击 [{相机按钮}]
点击 [@{加号图标}]

输入 [{搜索框}, 测试文本]

断言 [{相机按钮}, 存在]

等待 [1000]

返回
```

## 目录结构

```
tks-language-support/
├── grammars/
│   └── tks.tmLanguage.json    # TextMate Grammar 定义
├── src/
│   ├── index.js               # Node.js 导出
│   └── browser.js             # 浏览器导出
├── language-configuration.json # 语言配置（括号匹配等）
├── package.json               # VSCode 扩展配置
└── README.md
```

## 开发

### 测试语法高亮

1. 在 VSCode 中按 F5 启动调试
2. 在新窗口中打开 .tks 文件
3. 修改 `grammars/tks.tmLanguage.json` 后重新加载窗口

### 添加新的语法规则

编辑 `grammars/tks.tmLanguage.json`，添加新的 pattern：

```json
{
  "name": "keyword.new.tks",
  "match": "\\b(新命令)\\b"
}
```
