/**
 * 单行命令执行器
 * 负责执行单个命令行，用于调试功能
 * 不改变UI状态，只执行命令并输出结果
 */
(window.rLog || console.log)('single-line-runner.js 开始加载');

class SingleLineRunner {
    constructor() {
        this.isExecuting = false;
    }

    /**
     * 执行单行命令
     * @param {number} lineNumber - 行号（1-based）
     * @param {string} commandLine - 命令内容
     */
    async executeLine(lineNumber, commandLine) {
        // 防止重复执行
        if (this.isExecuting) {
            window.AppNotifications?.warn('正在执行命令，请稍候');
            return;
        }

        // 获取设备ID
        const deviceId = document.getElementById('deviceSelect')?.value;
        if (!deviceId) {
            window.AppNotifications?.deviceRequired();
            return;
        }

        // 获取项目路径
        const projectPath = window.AppGlobals?.currentProject;
        if (!projectPath) {
            window.AppNotifications?.projectRequired();
            return;
        }

        // 检查命令内容
        const trimmed = commandLine.trim();
        if (!trimmed) {
            window.AppNotifications?.warn('该行为空，无法执行');
            return;
        }

        this.isExecuting = true;
        const startTime = Date.now();

        // 获取当前编辑器，用于高亮
        const editor = window.EditorManager?.getActiveEditor();

        try {
            // 输出开始执行
            window.ExecutionOutput?.log('info', `执行第 ${lineNumber} 行: ${trimmed}`);

            // 高亮当前执行行
            if (editor && editor.highlightExecutingLine) {
                editor.highlightExecutingLine(lineNumber);
            }

            // 执行命令
            const result = await window.AppGlobals.ipcRenderer.invoke(
                'tke-run-step',
                deviceId,
                projectPath,
                trimmed
            );

            const duration = Date.now() - startTime;

            if (!result.success) {
                // 执行失败
                if (editor && editor.highlightErrorLine) {
                    editor.highlightErrorLine(lineNumber);
                }
                window.ExecutionOutput?.log('error', `执行失败: ${result.error || '未知错误'}`);
                window.AppNotifications?.error('命令执行失败');
            } else {
                // 执行成功
                window.ExecutionOutput?.log('success', `执行成功 (耗时 ${duration}ms)`);

                // 成功后清除高亮
                setTimeout(() => {
                    if (editor && editor.setTestRunning) {
                        editor.setTestRunning(false, true);
                    }
                }, 500);
            }

            // 刷新设备截图和XML
            try {
                // 使用新架构刷新屏幕
                if (window.ScreenCapture && window.ScreenCapture.refreshDeviceScreen) {
                    await window.ScreenCapture.refreshDeviceScreen();

                    // 如果当前在 XML overlay 模式，重新激活以刷新覆盖层
                    if (window.ScreenCoordinator && window.ScreenCoordinator.getCurrentMode() === 'xml') {
                        window.rLog('🔄 重新激活 XML Overlay 以保持同步');
                        if (window.XmlOverlayMode && window.XmlOverlayMode.activate) {
                            await window.XmlOverlayMode.activate();
                        }
                    }
                }
            } catch (error) {
                // 截图刷新失败不影响执行结果
                window.rLog('截图刷新失败:', error);
            }

        } catch (error) {
            // 执行异常
            if (editor && editor.highlightErrorLine) {
                editor.highlightErrorLine(lineNumber);
            }
            window.ExecutionOutput?.log('error', `执行异常: ${error.message}`);
            window.AppNotifications?.error('命令执行异常');
        } finally {
            this.isExecuting = false;
        }
    }
}

// 创建全局单例
window.SingleLineRunner = new SingleLineRunner();
(window.rLog || console.log)('✅ SingleLineRunner 模块已加载');
