// 行高亮功能模块 - 作为EditorTab的扩展方法
// 这是一个组合模块，导入所有高亮相关的子模块

// 注意：子模块已经通过 index.html 中的 <script> 标签加载
// 这里只是将它们组合成一个统一的接口

const EditorHighlighting = {
    // 从 ExecutionHighlighter 导入方法
    ...window.ExecutionHighlighter,

    // 从 ErrorHighlighter 导入方法
    ...window.ErrorHighlighter,

    // 从 HighlightUtils 导入方法
    ...window.HighlightUtils
};

// 导出到全局
window.EditorHighlighting = EditorHighlighting;

// 记录模块加载（延迟到 rLog 可用时）
if (window.rLog) {
    window.rLog('✅ EditorHighlighting 模块已加载，包含方法:', Object.keys(EditorHighlighting));
    window.rLog('检查 setTestRunning 方法:', {
        exists: 'setTestRunning' in EditorHighlighting,
        type: typeof EditorHighlighting.setTestRunning
    });
} else {
    // 如果 rLog 还不可用，使用 console.log
    console.log('✅ EditorHighlighting 模块已加载，包含方法:', Object.keys(EditorHighlighting));
    console.log('检查 setTestRunning 方法:', {
        exists: 'setTestRunning' in EditorHighlighting,
        type: typeof EditorHighlighting.setTestRunning
    });
}
