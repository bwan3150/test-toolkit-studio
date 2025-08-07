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
    codeEditor,
    lineNumbers,
    syntaxHighlight,
    openTabs,
    deviceScreenInterval,
    
    // 设置器函数
    setCurrentProject: (project) => { currentProject = project; window.AppGlobals.currentProject = project; },
    setCurrentCase: (testCase) => { currentCase = testCase; window.AppGlobals.currentCase = testCase; },
    setCodeEditor: (editor) => { codeEditor = editor; window.AppGlobals.codeEditor = editor; },
    setLineNumbers: (ln) => { lineNumbers = ln; window.AppGlobals.lineNumbers = ln; },
    setSyntaxHighlight: (sh) => { syntaxHighlight = sh; window.AppGlobals.syntaxHighlight = sh; },
    setOpenTabs: (tabs) => { openTabs = tabs; window.AppGlobals.openTabs = tabs; },
    setDeviceScreenInterval: (interval) => { deviceScreenInterval = interval; window.AppGlobals.deviceScreenInterval = interval; }
};