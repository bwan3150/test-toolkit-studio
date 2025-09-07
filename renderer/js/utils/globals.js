// 存储electron模块和其他依赖
const electron = window.nodeRequire ? window.nodeRequire('electron') : require('electron');
const { ipcRenderer } = electron;
const path = window.nodeRequire ? window.nodeRequire('path') : require('path');
const fs = (window.nodeRequire ? window.nodeRequire('fs') : require('fs')).promises;
const fsSync = window.nodeRequire ? window.nodeRequire('fs') : require('fs');
const yaml = window.nodeRequire ? window.nodeRequire('js-yaml') : require('js-yaml');
const { parse } = window.nodeRequire ? window.nodeRequire('csv-parse/sync') : require('csv-parse/sync');

// 全局变量
let currentProject = null;
let currentCase = null;
let currentTab = null;
let codeEditor = null;
let lineNumbers = null;
let syntaxHighlight = null;
let openTabs = [];
let deviceScreenInterval = null;

// 导出全局变量和模块
window.AppGlobals = {
    electron,
    ipcRenderer,
    path,
    fs,
    fsSync,
    yaml,
    parse,
    currentProject,
    currentCase,
    currentTab,
    codeEditor,
    lineNumbers,
    syntaxHighlight,
    openTabs,
    deviceScreenInterval,
    
    // 设置器函数
    setCurrentProject: (project) => { 
        currentProject = project; 
        window.AppGlobals.currentProject = project; 
        window.rLog(`✅ 项目路径已更新: ${project}`);
        
        // 触发项目变更事件
        document.dispatchEvent(new CustomEvent('project-changed', { detail: { projectPath: project } }));
        
        // 同时更新状态栏显示
        if (window.StatusBarModule) {
            window.StatusBarModule.updateProjectPath(project);
        }
    },
    setCurrentCase: (testCase) => { currentCase = testCase; window.AppGlobals.currentCase = testCase; },
    setCurrentTab: (tab) => { currentTab = tab; window.AppGlobals.currentTab = tab; },
    setCodeEditor: (editor) => { codeEditor = editor; window.AppGlobals.codeEditor = editor; },
    setLineNumbers: (ln) => { lineNumbers = ln; window.AppGlobals.lineNumbers = ln; },
    setSyntaxHighlight: (sh) => { syntaxHighlight = sh; window.AppGlobals.syntaxHighlight = sh; },
    setOpenTabs: (tabs) => { openTabs = tabs; window.AppGlobals.openTabs = tabs; },
    setDeviceScreenInterval: (interval) => { deviceScreenInterval = interval; window.AppGlobals.deviceScreenInterval = interval; },
    
    // 统一的项目路径获取函数
    getCurrentProjectPath: () => {
        const projectPath = window.AppGlobals.currentProject;
        if (!projectPath) {
            console.warn('⚠️  当前没有打开的项目，请先在Project页面打开或创建一个项目');
        }
        return projectPath;
    }
};