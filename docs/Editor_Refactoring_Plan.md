# 编辑器模块重构方案

## 当前状态分析

### 文件列表及大小

| 文件名 | 行数 | 大小 | 状态 |
|--------|------|------|------|
| editor-tab.js | 1939 | 76K | ⚠️ 需要拆分 |
| editor-manager.js | 493 | 16K | ⚠️ 需要拆分 |
| editor-highlighting.js | 439 | 20K | ⚠️ 需要拆分 |
| script-model.js | 380 | 14K | ❌ 待删除（已被 TKEEditorBuffer 替代） |
| editor-syntax-highlighter.js | 293 | 8.7K | ✅ 保留 |
| editor-block-parser.js | 286 | 7.0K | ✅ 保留 |
| editor-buffer.js | 253 | 7.9K | ❌ 待删除（未被使用） |
| editor-drag-drop.js | 232 | 9.5K | ✅ 保留 |
| tke-editor-buffer.js | 127 | 3.4K | ✅ 保留 |
| editor-line-mapping.js | 84 | 3.5K | ✅ 保留 |
| editor-cursor.js | 81 | 2.2K | ✅ 保留 |
| tks-syntax.js | 64 | 1.5K | ✅ 保留 |
| editor-font-settings.js | 22 | 848B | ✅ 保留 |

## 重构目标

1. ✅ 删除不再使用的旧文件
2. ✅ 将超过 400 行的文件拆分为单一职责模块
3. ✅ 创建清晰的文件夹结构
4. ✅ 每个文件只负责一件事
5. ✅ 提高代码可维护性

## 第一阶段：删除不使用的文件

### 待删除文件

**1. `editor-buffer.js` (253行)**
- **原因**: 未被任何文件使用
- **检查**: `index.html` 中未引用
- **替代**: 已被 `tke-editor-buffer.js` 替代

**2. `script-model.js` (380行)**
- **原因**: 根据重构文档，已被 `TKEEditorBuffer` 完全替代
- **检查**: 虽然 `index.html` 中有引用，但代码中只有注释提到
- **替代**: `tke-editor-buffer.js` + `editor-block-parser.js`

### 删除步骤

```bash
# 1. 备份文件
mkdir -p /tmp/editor-backup
cp renderer/js/testcase/editor/editor-buffer.js /tmp/editor-backup/
cp renderer/js/testcase/editor/script-model.js /tmp/editor-backup/

# 2. 从 index.html 中移除引用
# 删除这行: <script src="../js/testcase/editor/script-model.js"></script>

# 3. 删除文件
rm renderer/js/testcase/editor/editor-buffer.js
rm renderer/js/testcase/editor/script-model.js
```

## 第二阶段：重新组织文件结构

### 新的文件夹结构

```
renderer/js/testcase/editor/
├── core/                           # 核心模块
│   ├── editor-tab-core.js         # EditorTab 核心类（构造函数、基础属性、生命周期）
│   ├── editor-mode-switcher.js    # 模式切换逻辑
│   ├── editor-renderer.js         # 渲染逻辑（文本模式、块模式）
│   └── editor-event-hub.js        # 事件系统
│
├── commands/                       # 命令相关
│   ├── command-definitions.js     # blockDefinitions（命令定义）
│   ├── command-parser.js          # TKS命令解析
│   ├── command-converter.js       # 命令格式转换
│   └── command-operations.js      # 命令增删改查
│
├── ui/                            # UI 组件
│   ├── block-ui-builder.js        # 块编辑器UI构建
│   ├── text-ui-builder.js         # 文本编辑器UI构建
│   ├── line-numbers.js            # 行号显示
│   └── placeholder.js             # 占位符
│
├── input/                         # 输入处理
│   ├── text-input-handler.js      # 文本模式输入处理
│   ├── block-input-handler.js     # 块模式输入处理
│   └── ime-handler.js             # 输入法处理
│
├── syntax/                        # 语法相关（保留现有文件）
│   ├── tks-syntax.js              # TKS 语法定义
│   ├── editor-syntax-highlighter.js  # 语法高亮器
│   └── editor-block-parser.js     # 块解析器
│
├── utils/                         # 工具函数（保留现有文件）
│   ├── editor-cursor.js           # 光标管理
│   ├── editor-line-mapping.js     # 行号映射
│   ├── editor-font-settings.js    # 字体设置
│   └── editor-drag-drop.js        # 拖放功能
│
├── buffer/                        # 缓冲区（保留现有文件）
│   └── tke-editor-buffer.js       # TKE 编辑器缓冲区
│
├── highlighting/                  # 高亮功能（需拆分）
│   ├── execution-highlighter.js   # 执行高亮
│   ├── error-highlighter.js       # 错误高亮
│   └── syntax-highlighter.js      # 语法高亮（从 editor-highlighting.js 拆分）
│
├── manager/                       # 管理器（需拆分）
│   ├── editor-manager-core.js     # 编辑器管理器核心
│   ├── tab-manager.js             # 标签管理
│   ├── mode-manager.js            # 模式管理
│   └── keyboard-handler.js        # 键盘快捷键
│
└── editor-tab.js                  # 主入口（引入所有模块）
    editor-manager.js              # 管理器入口（引入管理器模块）
```

## 第三阶段：拆分 editor-tab.js (1939行)

### 功能分组

**1. `core/editor-tab-core.js` (~200行)**
- 构造函数
- 基础属性初始化
- 生命周期方法 (init, destroy, dispose)
- 模块混入逻辑 (ensureModulesMixed)

**2. `commands/command-definitions.js` (~300行)**
- blockDefinitions 完整定义
- 只包含命令配置数据

**3. `commands/command-parser.js` (~150行)**
- `parseTKSCommandText()` - 解析 TKS 命令文本
- `tksCommandToType()` - TKS 命令转类型
- `isCommandLine()` - 判断是否为命令行

**4. `commands/command-converter.js` (~100行)**
- `commandToTKSLine()` - 命令转 TKS 行
- `getTKSCode()` - 获取 TKS 代码
- `convertBlockParamsToEditorFormat()` - 参数格式转换

**5. `commands/command-operations.js` (~100行)**
- `getCommands()` - 获取命令列表
- `addCommand()` - 添加命令
- `updateCommand()` - 更新命令
- `deleteCommand()` - 删除命令
- `findCommandLineIndex()` - 查找命令行索引

**6. `core/editor-mode-switcher.js` (~150行)**
- `toggleMode()` - 切换模式
- `switchToTextMode()` - 切换到文本模式
- `switchToBlockMode()` - 切换到块模式
- 模式切换时的状态保存和恢复

**7. `core/editor-renderer.js` (~200行)**
- `render()` - 主渲染方法
- `renderTextMode()` - 渲染文本模式
- `renderBlockMode()` - 渲染块模式
- `renderPlaceholder()` - 渲染占位符

**8. `ui/block-ui-builder.js` (~300行)**
- `renderCommandBlocks()` - 渲染命令块
- `createBlockHTML()` - 创建块 HTML
- `getCommandDefinition()` - 获取命令定义
- 块编辑器相关的 UI 构建

**9. `ui/text-ui-builder.js` (~100行)**
- 文本编辑器 UI 构建
- 行号显示
- 高亮显示

**10. `input/text-input-handler.js` (~150行)**
- `setupTextModeListeners()` - 设置文本模式监听器
- `updateEditorHighlight()` - 更新编辑器高亮
- IME 输入法处理

**11. `input/block-input-handler.js` (~150行)**
- `setupBlockModeListeners()` - 设置块模式监听器
- 块编辑器的各种事件处理

**12. `core/editor-event-hub.js` (~50行)**
- `on()` - 添加事件监听
- `off()` - 移除事件监听
- `triggerChange()` - 触发变化事件
- 事件系统

**13. `editor-tab.js` (主入口, ~100行)**
- 导入所有模块
- 组合成完整的 EditorTab 类
- 导出到全局

## 第四阶段：拆分 editor-manager.js (493行)

### 功能分组

**1. `manager/editor-manager-core.js` (~100行)**
- 构造函数
- 基础属性
- 初始化逻辑

**2. `manager/tab-manager.js` (~200行)**
- `createTab()` - 创建标签
- `selectTab()` - 选择标签
- `closeTab()` - 关闭标签
- `createTabElement()` - 创建标签 DOM
- `createEditorTabContainer()` - 创建编辑器容器

**3. `manager/mode-manager.js` (~100行)**
- `setGlobalEditMode()` - 设置全局编辑模式
- `getGlobalEditMode()` - 获取全局编辑模式
- `switchAllEditorsMode()` - 切换所有编辑器模式
- 全局模式切换快捷键处理

**4. `manager/keyboard-handler.js` (~50行)**
- 键盘快捷键处理
- Tab 切换快捷键
- 保存快捷键

**5. `editor-manager.js` (主入口, ~50行)**
- 导入所有模块
- 组合成完整的 EditorManager 类
- 导出到全局

## 第五阶段：拆分 editor-highlighting.js (439行)

### 功能分组

**1. `highlighting/execution-highlighter.js` (~150行)**
- `setTestRunning()` - 设置测试运行状态
- `highlightExecutingLine()` - 高亮执行行
- `clearExecutionHighlight()` - 清除执行高亮
- `updateExecutionHighlight()` - 更新执行高亮

**2. `highlighting/error-highlighter.js` (~100行)**
- `highlightErrorLine()` - 高亮错误行
- `clearErrorHighlight()` - 清除错误高亮
- 错误显示逻辑

**3. `highlighting/syntax-highlighter.js` (~100行)**
- `highlightTKSSyntax()` - TKS 语法高亮
- `updateLineNumbers()` - 更新行号
- 与 `editor-syntax-highlighter.js` 集成

**4. `editor-highlighting.js` (主入口, ~50行)**
- 导入所有模块
- 组合高亮功能
- 作为 mixin 导出

## 实施步骤

### 步骤 1: 删除旧文件

```bash
# 1. 从 index.html 移除 script-model.js 引用
# 2. 删除文件
rm renderer/js/testcase/editor/editor-buffer.js
rm renderer/js/testcase/editor/script-model.js
```

### 步骤 2: 创建新文件夹结构

```bash
cd renderer/js/testcase/editor/
mkdir -p core commands ui input syntax utils buffer highlighting manager
```

### 步骤 3: 移动现有文件到新结构

```bash
# 语法相关
mv tks-syntax.js syntax/
mv editor-syntax-highlighter.js syntax/
mv editor-block-parser.js syntax/

# 工具函数
mv editor-cursor.js utils/
mv editor-line-mapping.js utils/
mv editor-font-settings.js utils/
mv editor-drag-drop.js utils/

# 缓冲区
mv tke-editor-buffer.js buffer/
```

### 步骤 4: 拆分 editor-tab.js

按照上述功能分组，逐个创建新文件并迁移代码。

### 步骤 5: 拆分 editor-manager.js

按照上述功能分组，逐个创建新文件并迁移代码。

### 步骤 6: 拆分 editor-highlighting.js

按照上述功能分组，逐个创建新文件并迁移代码。

### 步骤 7: 更新 index.html 引用

按照新的文件结构更新所有 `<script>` 标签引用。

### 步骤 8: 测试

全面测试所有编辑器功能。

## 重构优先级

### 高优先级（立即执行）
1. ✅ 删除 `editor-buffer.js` 和 `script-model.js`
2. ✅ 创建新文件夹结构
3. ✅ 移动现有小文件到新位置

### 中优先级（分步执行）
4. 拆分 `editor-tab.js` - 最复杂，分多次完成
5. 拆分 `editor-manager.js`
6. 拆分 `editor-highlighting.js`

### 低优先级（优化）
7. 代码优化和清理
8. 添加注释和文档
9. 性能优化

## 预期收益

1. **可维护性提升**: 每个文件职责单一，易于理解和修改
2. **代码复用**: 模块化后可以在其他地方复用
3. **团队协作**: 多人可以同时修改不同模块，减少冲突
4. **测试友好**: 单一职责的模块更容易编写单元测试
5. **性能优化**: 可以按需加载模块
6. **调试简化**: 问题更容易定位到具体模块

## 风险评估

### 低风险
- 删除未使用的文件
- 移动现有小文件
- 创建文件夹结构

### 中风险
- 拆分 editor-manager.js 和 editor-highlighting.js
- 更新 index.html 引用

### 高风险
- 拆分 editor-tab.js（1939行，功能复杂，相互依赖多）

### 风险缓解
1. 每次修改后立即测试
2. 使用 Git 创建分支，随时可以回滚
3. 分步骤进行，不要一次性修改太多
4. 保持功能不变，只重构结构
5. 保留完整的备份

## 测试清单

重构后需要测试的功能：

- [ ] 文本编辑模式
  - [ ] 语法高亮
  - [ ] 中文输入
  - [ ] 光标位置保存和恢复
  - [ ] 行号显示

- [ ] 块编辑模式
  - [ ] 块显示
  - [ ] 块编辑
  - [ ] 拖拽元素到块
  - [ ] 删除块

- [ ] 模式切换
  - [ ] 文本 ↔ 块模式切换
  - [ ] 全局模式切换快捷键
  - [ ] 内容不丢失

- [ ] 标签管理
  - [ ] 创建标签
  - [ ] 切换标签
  - [ ] 关闭标签

- [ ] 文件操作
  - [ ] 打开文件
  - [ ] 保存文件
  - [ ] 自动保存

- [ ] 执行高亮
  - [ ] 运行时高亮
  - [ ] 错误高亮

- [ ] 拖放功能
  - [ ] 拖拽定位器到编辑器
