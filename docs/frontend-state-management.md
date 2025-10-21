# 前端状态管理规范

## 全局状态变量

### 1. 项目路径
**正确用法:**
```javascript
const projectPath = window.AppGlobals.currentProject;
```

**设置方法:**
```javascript
window.AppGlobals.setCurrentProject(projectPath);
```

**位置:** 由 `project-manager.js` 在加载项目时设置

---

### 2. 当前活动编辑器标签

**EditorManager 内部:**
```javascript
// EditorManager 类中
this.activeTabId  // 当前活动标签的 ID
```

**获取活动编辑器:**
```javascript
const editor = window.EditorManager?.getActiveEditor();
```

**返回:** `EditorTab` 实例或 `null`

---

### 3. 编辑器实例集合

**EditorManager 内部:**
```javascript
// EditorManager 类中
this.editors  // Map<tabId, EditorTab>
```

**获取特定编辑器:**
```javascript
const editor = window.EditorManager.editors.get(tabId);
```

---

### 4. 打开的标签列表

**位置:**
```javascript
window.AppGlobals.openTabs  // Array<TabData>
```

**TabData 结构:**
```javascript
{
  id: 'tab-xxx',
  filePath: '/path/to/file.tks',
  fileName: 'file.tks',
  type: 'script'
}
```

---

### 5. 当前选中的设备

**位置:**
```javascript
const deviceId = document.getElementById('deviceSelect')?.value;
```

**存储:** 由 `device-manager.js` 管理

---

## 脚本执行流程的状态获取

### RunTest 按钮点击时需要的信息:

```javascript
// 1. 获取当前编辑器
const editor = window.EditorManager?.getActiveEditor();

// 2. 获取脚本内容
const scriptContent = editor.buffer?.getRawContent();

// 3. 获取项目路径
const projectPath = window.AppGlobals.currentProject;

// 4. 获取设备ID
const deviceId = document.getElementById('deviceSelect')?.value;

// 5. 获取脚本文件路径(可选,用于显示)
const activeTabId = window.EditorManager.activeTabId;
const tabData = window.AppGlobals.openTabs.find(t => t.id === activeTabId);
const scriptPath = tabData?.filePath;
```

---

## 注意事项

1. **不要使用的变量:**
   - ❌ `window.ProjectManagerModule?.currentProjectPath` (不存在)
   - ❌ `window.currentProject` (不推荐)
   - ❌ `console.log()` (项目禁止,使用 `window.rLog()` 代替)

2. **推荐的模式:**
   - ✅ 始终通过 `window.AppGlobals` 访问全局状态
   - ✅ 通过 `window.EditorManager` 访问编辑器相关状态
   - ✅ 使用 `?.` 可选链操作符防止空指针
   - ✅ 使用 `window.rLog()` / `window.rError()` 输出日志

3. **日志输出规范:**
```javascript
// ✅ 正确
window.rLog('信息:', data);
window.rError('错误:', error);

// ❌ 错误
console.log('信息:', data);
console.error('错误:', error);
```
