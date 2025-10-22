// 编辑器管理器 - 负责标签管理和编辑器实例协调
// 这是一个组合模块，导入所有管理器相关的子模块

// 注意：子模块已经通过 index.html 中的 <script> 标签加载
// 这里将它们组合成一个完整的 EditorManager 类

class EditorManager extends EditorManagerCore {
    // 继承核心类，并混入所有功能模块
}

// 混入所有功能模块
Object.assign(EditorManager.prototype, window.TabOperations);
Object.assign(EditorManager.prototype, window.ModeController);
Object.assign(EditorManager.prototype, window.EditorProxy);

// 创建全局实例
let editorManagerInstance = null;

/**
 * 初始化编辑器管理器
 * @returns {EditorManager} 编辑器管理器实例
 */
function initializeEditorManager() {
    if (!editorManagerInstance) {
        editorManagerInstance = new EditorManager();

        // 全局引用
        window.EditorManager = editorManagerInstance;

        // 更新全局AppGlobals的编辑器引用 - 直接使用EditorManager
        // 添加防御性检查
        if (window.AppGlobals && typeof window.AppGlobals.setCodeEditor === 'function') {
            window.AppGlobals.setCodeEditor(editorManagerInstance);
        } else {
            window.rError('❌ AppGlobals 未定义或 setCodeEditor 方法不存在', {
                hasAppGlobals: !!window.AppGlobals,
                AppGlobalsType: typeof window.AppGlobals,
                hasSetCodeEditor: window.AppGlobals ? typeof window.AppGlobals.setCodeEditor : 'N/A'
            });
            throw new Error('AppGlobals 未正确初始化');
        }

        window.rLog('编辑器管理器初始化完成');

        // 初始化后立即加载字体设置
        setTimeout(() => {
            if (window.SettingsModule && window.SettingsModule.loadEditorFontSettings) {
                window.SettingsModule.loadEditorFontSettings();
            }
        }, 100);
    }

    return editorManagerInstance;
}

// 导出初始化函数
window.initializeEditorManager = initializeEditorManager;

if (window.rLog) {
    window.rLog('✅ EditorManager 主模块已加载');
}
