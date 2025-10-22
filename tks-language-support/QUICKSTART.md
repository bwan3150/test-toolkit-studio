# TKS Language Support - 快速开始

## 这是什么？

**tks-language-support** 是 ToolkitScript (.tks) 的语法高亮支持包，可以：

- ✅ 在 VSCode 中提供语法高亮
- ✅ 在 nvim 中使用（通过 TextMate Grammar）
- ✅ 为 Web 编辑器提供语法定义

## 5 分钟上手

### 1. VSCode 快速安装

```bash
# 进入目录
cd tks-language-support

# 创建符号链接
ln -s $(pwd) ~/.vscode/extensions/tks-language-support

# 重启 VSCode
```

打开任何 `.tks` 文件即可看到语法高亮！

### 2. 打包为 VSIX

```bash
# 安装依赖
npm install

# 打包
npm run package

# 会生成 tks-language-support-1.0.0.vsix
# 在 VSCode 中: Extensions -> Install from VSIX
```

### 3. 发布到 VSCode Marketplace

```bash
# 安装 vsce
npm install -g vsce

# 登录 (需要 Azure DevOps Token)
vsce login your-publisher-name

# 发布
vsce publish
```

## 项目结构

```
tks-language-support/
├── grammars/
│   └── tks.tmLanguage.json    # TextMate Grammar (VSCode/nvim 用)
├── src/
│   ├── index.js               # Node.js 导出
│   └── browser.js             # 浏览器导出 (Web 编辑器用)
├── examples/
│   └── sample.tks             # 示例文件
├── package.json               # VSCode 扩展配置
├── language-configuration.json # 语言配置（括号匹配等）
└── README.md
```

## 在 Web 编辑器中使用

```javascript
// 1. 引入语法定义
import TKSSyntax from './src/browser.js';

// 2. 获取所有命令
const commands = TKSSyntax.getAllCommands();
// ['启动', '关闭', '点击', '按压', ...]

// 3. 使用正则匹配
const code = '点击 [{相机按钮}]';
TKSSyntax.patterns.locator.test(code); // true

// 4. 获取主题颜色
const colors = TKSSyntax.themes.dark;
```

## 支持的语法

### 命令

```tks
启动 [...]       # 进程控制
关闭 [...]
点击 [...]       # 交互操作
按压 [...]
滑动 [...]
拖动 [...]
定向拖动 [...]
输入 [...]       # 输入操作
清理 [...]
隐藏键盘
等待 [...]       # 时间控制
返回            # 导航
断言 [...]       # 断言
读取 [...]
```

### 定位器

```tks
{200, 400}       # 坐标
{相机按钮}        # XML 元素
@{加号图标}       # 图片
```

### 示例

```tks
# 这是注释
步骤:

启动 [com.example.app, .MainActivity]
点击 [{200, 400}]
点击 [{相机按钮}]
点击 [@{加号图标}]
输入 [{搜索框}, 测试文本]
断言 [{按钮}, 存在]
等待 [1000]
返回
```

## 自定义配置

### 修改颜色

编辑 `src/browser.js`:

```javascript
themes: {
  dark: {
    command: '#569cd6',  // 命令颜色
    locator: '#ce9178',  // 定位器颜色
    // ...
  }
}
```

### 添加新命令

1. 编辑 `grammars/tks.tmLanguage.json`
2. 添加到 `src/browser.js` 的 `commands` 对象
3. 重启 VSCode

## 常见问题

**Q: VSCode 不显示高亮？**
- 确保文件扩展名是 `.tks`
- 重启 VSCode
- 检查 Extensions 面板是否已安装

**Q: 如何调试 Grammar？**
- F5 启动调试
- 在新窗口打开 `.tks` 文件
- 修改 `tks.tmLanguage.json` 后重新加载窗口

**Q: 如何支持其他编辑器？**
- **Sublime Text**: 直接使用 `.tmLanguage` 文件
- **Atom**: 支持 TextMate Grammar
- **nvim**: 转换为 Tree-sitter 或使用 vim-polyglot

## 下一步

- 📖 查看完整文档: [TKS_Syntax_Highlighting.md](../docs/TKS_Syntax_Highlighting.md)
- 🎨 自定义主题
- 🚀 发布到 VSCode Marketplace
- 🔧 集成 LSP (语义分析、智能补全)

