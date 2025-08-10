// 状态栏管理模块

function getGlobals() {
    return window.AppGlobals;
}

class StatusBarManager {
    constructor() {
        this.currentProjectElement = null;
        this.lastProject = null;
    }

    init() {
        this.currentProjectElement = document.getElementById('statusBarProjectPath');
        
        // 添加点击复制功能
        if (this.currentProjectElement) {
            this.currentProjectElement.parentElement.addEventListener('click', () => {
                this.copyProjectPath();
            });
        }
        
        // 立即尝试更新项目路径
        this.updateProjectPath();
        
        // 强制检查一次，延迟一秒确保项目已完全加载
        setTimeout(() => {
            const globals = getGlobals();
            if (globals && globals.currentProject) {
                // console.log('StatusBar: Force updating with project:', globals.currentProject); // 已禁用以减少日志
                this.updateProjectPath(globals.currentProject);
            }
        }, 1000);
        
        // 监听项目变化
        this.watchProjectChanges();
        
        // console.log('StatusBar initialized, current project:', getGlobals()?.currentProject); // 已禁用以减少日志
    }

    // 更新项目路径显示
    updateProjectPath(projectPath = null) {
        if (!this.currentProjectElement) {
            // console.log('StatusBar: currentProjectElement not found'); // 已禁用以减少日志
            return;
        }

        const globals = getGlobals();
        const currentPath = projectPath || (globals && globals.currentProject);
        
        // console.log('StatusBar: updating path', { projectPath, currentPath, globals: globals?.currentProject }); // 已禁用以减少日志
        
        if (currentPath) {
            // 只显示项目文件夹名称和上级目录
            const pathParts = currentPath.split(/[/\\]/);
            const projectName = pathParts[pathParts.length - 1];
            const parentDir = pathParts[pathParts.length - 2];
            
            let displayText;
            if (parentDir) {
                displayText = `${parentDir}/${projectName}`;
            } else {
                displayText = projectName;
            }
            
            this.currentProjectElement.textContent = displayText;
            
            // 设置tooltip显示完整路径和点击提示
            this.currentProjectElement.parentElement.title = `${currentPath}\n\nClick to copy full path to clipboard`;
            
            // 存储完整路径用于复制
            this.currentProjectElement.parentElement.dataset.fullPath = currentPath;
            
            // console.log('StatusBar: Updated to:', displayText); // 已禁用以减少日志
        } else {
            this.currentProjectElement.textContent = 'No project opened';
            this.currentProjectElement.parentElement.title = '';
            this.currentProjectElement.parentElement.dataset.fullPath = '';
            // console.log('StatusBar: No project found, showing default text'); // 已禁用以减少日志
        }
    }

    // 监听项目变化
    watchProjectChanges() {
        // 定期检查项目变化
        setInterval(() => {
            const globals = getGlobals();
            const currentProject = globals && globals.currentProject;
            if (currentProject !== this.lastProject) {
                this.lastProject = currentProject;
                this.updateProjectPath(currentProject);
            }
        }, 500);

        // 监听项目选择事件
        document.addEventListener('project-changed', (event) => {
            this.updateProjectPath(event.detail.projectPath);
        });
    }

    // 手动更新项目路径（供其他模块调用）
    setProjectPath(projectPath) {
        this.updateProjectPath(projectPath);
    }

    // 复制项目路径到剪贴板
    async copyProjectPath() {
        if (!this.currentProjectElement) return;

        const fullPath = this.currentProjectElement.parentElement.dataset.fullPath;
        if (!fullPath) {
            if (window.NotificationModule) {
                window.NotificationModule.showNotification('No project path to copy', 'warning');
            }
            return;
        }

        try {
            await navigator.clipboard.writeText(fullPath);
            if (window.NotificationModule) {
                window.NotificationModule.showNotification('Project path copied to clipboard', 'success');
            }
        } catch (error) {
            console.error('Failed to copy to clipboard:', error);
            
            // 备用方案：尝试使用选择+复制
            try {
                const textArea = document.createElement('textarea');
                textArea.value = fullPath;
                textArea.style.position = 'fixed';
                textArea.style.left = '-999999px';
                textArea.style.top = '-999999px';
                document.body.appendChild(textArea);
                textArea.focus();
                textArea.select();
                document.execCommand('copy');
                document.body.removeChild(textArea);
                
                if (window.NotificationModule) {
                    window.NotificationModule.showNotification('Project path copied to clipboard', 'success');
                }
            } catch (fallbackError) {
                console.error('Fallback copy method also failed:', fallbackError);
                if (window.NotificationModule) {
                    window.NotificationModule.showNotification('Failed to copy to clipboard', 'error');
                }
            }
        }
    }
}

// 创建状态栏管理器实例
const statusBarManager = new StatusBarManager();

// 导出模块
window.StatusBarModule = {
    updateProjectPath: (path) => statusBarManager.setProjectPath(path),
    init: () => statusBarManager.init()
};