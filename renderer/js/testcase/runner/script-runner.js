// 脚本运行器模块 - 逐行执行.tks脚本
// 负责调用 tke run step 逐步执行脚本,并与编辑器高亮功能配合

class ScriptRunner {
    constructor() {
        this.isRunning = false;
        this.shouldStop = false;
        this.currentLineIndex = 0;
    }

    /**
     * 运行当前编辑器中的脚本
     */
    async runCurrentScript() {
        // 防止重复点击
        if (this.isRunning) {
            return;
        }

        // 获取当前活动编辑器
        const editor = window.EditorManager?.getActiveEditor();
        if (!editor) {
            window.AppNotifications?.error('没有活动的编辑器');
            return;
        }

        // 获取设备ID和项目路径
        const deviceId = document.getElementById('deviceSelect')?.value;
        if (!deviceId) {
            window.AppNotifications?.deviceRequired();
            return;
        }

        // 获取项目路径 (统一使用 AppGlobals)
        const projectPath = window.AppGlobals?.currentProject;
        if (!projectPath) {
            window.AppNotifications?.projectRequired();
            return;
        }

        // 获取脚本内容
        const scriptContent = editor.buffer?.getRawContent();
        if (!scriptContent) {
            window.AppNotifications?.error('脚本内容为空');
            return;
        }

        // 解析脚本获取所有行
        const lines = scriptContent.split('\n');

        // 提取命令行(跳过头部和空行)
        const commandLines = [];
        const lineNumberMap = []; // 记录每个命令行对应的原始行号(1-based)

        let inStepsSection = false;
        lines.forEach((line, index) => {
            const trimmed = line.trim();

            // 检测步骤部分
            if (trimmed === '步骤:') {
                inStepsSection = true;
                return;
            }

            // 跳过非步骤部分
            if (!inStepsSection) {
                return;
            }

            // 跳过空行和注释
            if (!trimmed || trimmed.startsWith('#')) {
                return;
            }

            // 这是一个命令行
            commandLines.push(trimmed);
            lineNumberMap.push(index + 1); // 存储原始行号(1-based)
        });

        if (commandLines.length === 0) {
            window.AppNotifications?.error('脚本中没有可执行的命令');
            return;
        }

        // 获取脚本名称(用于日志)
        const scriptName = editor.filePath ? editor.filePath.split('/').pop() : '未命名脚本';

        // 设置运行状态
        this.isRunning = true;
        this.shouldStop = false;
        this.currentLineIndex = 0;
        const startTime = Date.now();

        // 输出脚本开始执行
        window.ExecutionOutput?.scriptStart(scriptName);

        // 设置编辑器为运行状态
        editor.setTestRunning?.(true, false);

        try {
            // 逐行执行命令
            for (let i = 0; i < commandLines.length; i++) {
                if (this.shouldStop) {
                    window.ExecutionOutput?.warn('测试被用户中止');
                    break;
                }

                this.currentLineIndex = i;
                const commandLine = commandLines[i];
                const originalLineNumber = lineNumberMap[i];
                const stepIndex = i + 1;

                // 输出步骤开始
                window.ExecutionOutput?.stepStart(stepIndex, commandLines.length, commandLine);

                // 在执行命令前刷新设备截图
                try {
                    if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                        await window.DeviceScreenManagerModule.refreshDeviceScreen();
                    }
                } catch (error) {
                    // 截图失败不影响脚本执行
                }

                // 高亮当前执行行
                editor.highlightExecutingLine?.(originalLineNumber);

                // 执行命令
                const stepStartTime = Date.now();
                try {
                    const result = await window.AppGlobals.ipcRenderer.invoke(
                        'tke-run-step',
                        deviceId,
                        projectPath,
                        commandLine
                    );

                    const stepDuration = Date.now() - stepStartTime;

                    if (!result.success) {
                        // 执行失败
                        editor.highlightErrorLine?.(originalLineNumber);
                        window.ExecutionOutput?.stepFailed(stepIndex, result.error || '未知错误');
                        window.AppNotifications?.error('执行失败');

                        // 停止执行
                        break;
                    }

                    // 执行成功
                    window.ExecutionOutput?.stepSuccess(stepIndex, stepDuration);

                    // 短暂延迟
                    await this.sleep(300);

                } catch (error) {
                    editor.highlightErrorLine?.(originalLineNumber);
                    window.ExecutionOutput?.stepFailed(stepIndex, error.message);
                    window.AppNotifications?.error('执行异常');

                    // 停止执行
                    break;
                }
            }

            // 检查是否全部成功
            if (!this.shouldStop && this.currentLineIndex === commandLines.length - 1) {
                // 最后一步执行完后,刷新设备截图显示最终状态
                try {
                    if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                        await window.DeviceScreenManagerModule.refreshDeviceScreen();
                    }
                } catch (error) {
                    // 截图失败不影响成功提示
                }

                const totalDuration = Date.now() - startTime;
                window.ExecutionOutput?.scriptSuccess(scriptName, totalDuration);
                // 不需要Toast - 控制台已经显示了执行成功

                // 成功完成时清除高亮
                editor.setTestRunning?.(false, true);
            } else {
                // 失败或中止
                const totalDuration = Date.now() - startTime;
                if (this.shouldStop) {
                    window.ExecutionOutput?.warn(`脚本执行已中止 (耗时 ${(totalDuration / 1000).toFixed(2)}s)`);
                } else {
                    window.ExecutionOutput?.scriptFailed(scriptName, '执行过程中出现错误');
                }

                // 失败或中止时保持高亮
                editor.setTestRunning?.(false, false);
            }

        } finally {
            // 重置运行状态
            this.isRunning = false;
            this.shouldStop = false;
        }
    }

    /**
     * 停止当前运行
     */
    stop() {
        if (this.isRunning) {
            window.rLog('请求停止脚本执行');
            this.shouldStop = true;
        }
    }

    /**
     * 辅助延迟函数
     */
    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}

// 导出全局单例
window.ScriptRunner = new ScriptRunner();

// 记录模块加载
window.rLog('✅ ScriptRunner 模块已加载');
