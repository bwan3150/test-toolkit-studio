// 简单的组件加载器
class ComponentLoader {
    constructor() {
        this.loadedComponents = new Set();
    }

    async loadComponent(componentName, containerId) {
        if (this.loadedComponents.has(componentName)) {
            return true;
        }

        const container = document.getElementById(containerId);
        if (!container) {
            console.error(`容器 ${containerId} 未找到`);
            return false;
        }

        try {
            const response = await fetch(`components/${componentName}.html`);
            if (response.ok) {
                const html = await response.text();
                container.innerHTML = html;
                this.loadedComponents.add(componentName);
                return true;
            } else {
                console.error(`组件 ${componentName} 加载失败:`, response.status);
            }
        } catch (error) {
            console.error(`组件 ${componentName} 加载错误:`, error);
        }
        return false;
    }

    async loadComponents(components) {
        const promises = components.map(component => 
            this.loadComponent(component.name, component.container)
        );
        return Promise.all(promises);
    }
}

// 创建全局实例
window.ComponentLoader = new ComponentLoader();