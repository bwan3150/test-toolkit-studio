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
            window.rLog('脚本正在运行中,忽略重复点击');
            return;
        }

        window.rLog('开始运行当前脚本');

        // 获取当前活动编辑器
        const editor = window.EditorManager?.getActiveEditor();
        if (!editor) {
            window.rError('没有活动的编辑器');
            window.notifications?.show('没有活动的编辑器', 'error');
            return;
        }

        // 获取设备ID和项目路径
        const deviceId = document.getElementById('deviceSelect')?.value;
        if (!deviceId) {
            window.rError('请先选择设备');
            window.notifications?.show('请先选择设备', 'error');
            return;
        }

        // 获取项目路径 (统一使用 AppGlobals)
        const projectPath = window.AppGlobals?.currentProject;
        if (!projectPath) {
            window.rError('没有打开的项目');
            window.notifications?.show('没有打开的项目', 'error');
            return;
        }

        // 获取脚本内容
        const scriptContent = editor.buffer?.getRawContent();
        if (!scriptContent) {
            window.rError('脚本内容为空');
            window.notifications?.show('脚本内容为空', 'error');
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
            window.rError('脚本中没有可执行的命令');
            window.notifications?.show('脚本中没有可执行的命令', 'error');
            return;
        }

        window.rLog(`找到 ${commandLines.length} 个命令行`);

        // 设置运行状态
        this.isRunning = true;
        this.shouldStop = false;
        this.currentLineIndex = 0;

        // 设置编辑器为运行状态
        editor.setTestRunning?.(true, false);

        try {
            // 逐行执行命令
            for (let i = 0; i < commandLines.length; i++) {
                if (this.shouldStop) {
                    window.rLog('测试被用户中止');
                    window.notifications?.show('测试已中止', 'warning');
                    break;
                }

                this.currentLineIndex = i;
                const commandLine = commandLines[i];
                const originalLineNumber = lineNumberMap[i];

                window.rLog(`执行第 ${i + 1}/${commandLines.length} 个命令 (原始行号: ${originalLineNumber}): ${commandLine}`);

                // 在执行命令前刷新设备截图,让用户看到手机当前状态
                try {
                    if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                        window.rLog('🔄 刷新设备截图...');
                        await window.DeviceScreenManagerModule.refreshDeviceScreen();
                    }
                } catch (error) {
                    window.rWarn('截图刷新失败:', error);
                    // 截图失败不影响脚本执行,继续
                }

                // 高亮当前执行行
                editor.highlightExecutingLine?.(originalLineNumber);

                // 执行命令
                try {
                    const result = await window.AppGlobals.ipcRenderer.invoke(
                        'tke-run-step',
                        deviceId,
                        projectPath,
                        commandLine
                    );

                    window.rLog('命令执行结果:', result);

                    if (!result.success) {
                        // 执行失败,高亮错误行
                        window.rError(`命令执行失败 (行 ${originalLineNumber}): ${result.error}`);
                        editor.highlightErrorLine?.(originalLineNumber);
                        window.notifications?.show(`执行失败: ${result.error}`, 'error');

                        // 停止执行
                        break;
                    }

                    // 短暂延迟,让高亮效果更明显
                    await this.sleep(300);

                } catch (error) {
                    window.rError(`命令执行异常 (行 ${originalLineNumber}):`, error);
                    editor.highlightErrorLine?.(originalLineNumber);
                    window.notifications?.show(`执行异常: ${error.message}`, 'error');

                    // 停止执行
                    break;
                }
            }

            // 检查是否全部成功
            if (!this.shouldStop && this.currentLineIndex === commandLines.length - 1) {
                window.rLog('脚本执行成功完成');

                // 最后一步执行完后,刷新设备截图显示最终状态
                try {
                    if (window.DeviceScreenManagerModule && window.DeviceScreenManagerModule.refreshDeviceScreen) {
                        window.rLog('🔄 刷新设备截图显示最终状态...');
                        await window.DeviceScreenManagerModule.refreshDeviceScreen();
                    }
                } catch (error) {
                    window.rWarn('最终截图刷新失败:', error);
                    // 截图失败不影响成功提示
                }

                window.notifications?.show('脚本执行成功', 'success');

                // 成功完成时清除高亮
                editor.setTestRunning?.(false, true);
            } else {
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
