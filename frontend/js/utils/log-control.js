// 日志控制模块 - 完全禁用用户控制台日志
// 只保留错误和警告信息，隐藏所有运行逻辑

(function() {
    'use strict';
    
    // 保存原始的console方法
    const originalLog = console.log;
    const originalInfo = console.info;
    const originalDebug = console.debug;
    
    // 完全禁用console.log, console.info, console.debug
    console.log = function() {
        // 完全静默，不输出任何信息
    };
    
    console.info = function() {
        // 完全静默，不输出任何信息  
    };
    
    console.debug = function() {
        // 完全静默，不输出任何信息
    };
    
    // 保留console.error和console.warn用于调试关键问题
    // console.error和console.warn保持原样
    
    // 为开发者提供一个恢复日志的方法（仅供调试时使用）
    window.restoreConsoleLog = function() {
        console.log = originalLog;
        console.info = originalInfo;
        console.debug = originalDebug;
        console.log('Console logging restored for debugging');
    };
    
    // 提供一个完全静默的方法
    window.silenceConsole = function() {
        console.log = function() {};
        console.info = function() {};
        console.debug = function() {};
    };
    
})();

console.log('日志控制系统已激活 - 用户日志已禁用');