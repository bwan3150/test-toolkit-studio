# JS模块化重构说明

## 概述
原本的 `app.js` 文件包含1732行代码，现已按功能模块进行拆分，便于后续维护和开发。

## 目录结构
```
renderer/js/
├── app.js                      # 主应用入口文件（模块化版本）
├── app-original.js             # 原始app.js备份
├── login.js                    # 登录页面（未修改）
├── core/                       # 核心模块
│   └── globals.js              # 全局变量和依赖管理
├── modules/                    # 业务功能模块
│   ├── device-manager.js       # 设备管理功能
│   ├── project-manager.js      # 项目管理功能
│   └── testcase-manager.js     # 测试用例管理功能
├── ui/                         # 用户界面模块
│   ├── editor.js               # 本地代码编辑器（支持离线使用）
│   ├── navigation.js           # 页面导航管理
│   ├── notifications.js        # 通知系统
│   ├── resizable-panels.js     # 可调整大小面板
│   └── settings.js             # 设置页面管理
└── utils/                      # 工具模块
    ├── ipc-handlers.js         # IPC消息处理
    └── keyboard-shortcuts.js   # 键盘快捷键
```

## 模块功能说明

### 核心模块 (core/)
- **globals.js**: 管理全局变量、electron模块引用和依赖项

### 业务模块 (modules/)
- **project-manager.js**: 项目的创建、打开、历史管理、CSV导入和测试用例管理
- **testcase-manager.js**: 测试用例的创建、显示、执行、文件树管理和脚本编辑
- **device-manager.js**: 设备的添加、编辑、删除、ADB设备扫描和连接管理

### 用户界面模块 (ui/)
- **editor.js**: 本地代码编辑器功能（替代Monaco，支持离线使用）
  - 语法高亮、tab管理、文件操作
- **navigation.js**: 页面间的导航逻辑
- **notifications.js**: 通知系统和控制台日志管理
- **settings.js**: 用户设置、SDK状态检查、用户信息加载
- **resizable-panels.js**: 可调整大小的面板功能

### 工具模块 (utils/)
- **keyboard-shortcuts.js**: 键盘快捷键处理（Ctrl+S保存、Ctrl+W关闭tab等）
- **ipc-handlers.js**: 处理来自主进程的IPC消息

## 模块加载顺序
1. core/globals.js - 全局变量和依赖项
2. ui/notifications.js - 通知系统
3. ui/navigation.js - 导航管理
4. ui/editor.js - 编辑器功能
5. ui/settings.js - 设置页面
6. ui/resizable-panels.js - 可调整面板
7. modules/project-manager.js - 项目管理
8. modules/testcase-manager.js - 测试用例管理
9. modules/device-manager.js - 设备管理
10. utils/keyboard-shortcuts.js - 键盘快捷键
11. utils/ipc-handlers.js - IPC消息处理

## 主要改进

### 1. 模块化架构
- 将1732行的单一文件拆分为11个功能模块
- 每个模块职责单一，便于维护和测试
- 模块间通过window对象进行通信

### 2. 离线支持
- 使用本地简单编辑器替代Monaco编辑器
- 支持YAML语法高亮和代码格式化
- 无需网络连接即可正常使用

### 3. 更好的代码组织
- 按功能域分组（UI、业务、工具、核心）
- 清晰的模块依赖关系
- 统一的模块导出方式

### 4. 便于扩展
- 新功能可以作为独立模块添加
- 模块间松耦合，便于单独测试
- 清晰的接口定义

## 使用方式

### 开发调试
可以通过浏览器控制台使用以下调试工具：
```javascript
// 获取当前项目
window.AppDebug.getCurrentProject()

// 获取打开的标签页
window.AppDebug.getOpenTabs()

// 获取编辑器实例
window.AppDebug.getCodeEditor()
```

### 添加新模块
1. 在对应目录创建新的.js文件
2. 使用统一的导出格式：`window.ModuleName = { ... }`
3. 在app.js中添加模块加载和初始化代码

## 兼容性
- 保持了所有原有功能
- 全局函数和变量通过window对象访问
- 与现有HTML页面完全兼容

## 维护建议
1. 每个模块保持功能单一性
2. 避免模块间的循环依赖
3. 新功能优先考虑作为独立模块实现
4. 定期检查和优化模块间的接口