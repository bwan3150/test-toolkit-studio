// Locator库管理模块
// 通过IPC调用主进程的TKE handlers进行元素查找

class LocatorManagerTKE {
    constructor() {
        this.locators = {};
        this.currentCasePath = null;
        this.initialized = false;
        this.ipcRenderer = require('electron').ipcRenderer;
    }

    // 初始化Locator管理器
    async initialize() {
        if (this.initialized) return;

        try {
            this.setupDragAndDrop();
            this.setupSearch();
            await this.loadLocators();

            this.initialized = true;
            console.log('LocatorManagerTKE已初始化');
        } catch (error) {
            console.error('LocatorManagerTKE初始化失败:', error);
            throw error;
        }
    }

    setCurrentCase(casePath) {
        this.currentCasePath = casePath;
        this.loadLocators();
    }

    inferCurrentCasePath() {
        let tabData = window.AppGlobals.currentTab;
        if (!tabData) {
            const activeTab = document.querySelector('.tab.active');
            if (activeTab) {
                tabData = window.AppGlobals.openTabs.find(tab => tab.id === activeTab.id);
            }
        }

        if (tabData && tabData.path) {
            const pathSep = window.AppGlobals.path.sep;
            const pathParts = tabData.path.split(pathSep);
            const caseIndex = pathParts.indexOf('cases');
            if (caseIndex !== -1 && caseIndex < pathParts.length - 2) {
                const caseFolderName = pathParts[caseIndex + 1];
                const projectPath = pathParts.slice(0, caseIndex).join(pathSep);
                return window.AppGlobals.path.join(projectPath, 'cases', caseFolderName);
            }
        }

        const projectPath = window.AppGlobals.getCurrentProjectPath();
        if (projectPath) {
            return window.AppGlobals.path.join(projectPath, 'cases', 'temp');
        }

        return null;
    }

    async loadLocators() {
        try {
            const casePath = this.currentCasePath || this.inferCurrentCasePath();
            if (!casePath) return;

            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) return;

            const fs = require('fs');
            const path = window.AppGlobals.path;
            const locatorFile = path.join(projectPath, 'locator', 'element.json');

            if (fs.existsSync(locatorFile)) {
                const content = fs.readFileSync(locatorFile, 'utf8');
                this.locators = JSON.parse(content);
                console.log(`加载了 ${Object.keys(this.locators).length} 个locator定义`);
                this.updateLocatorList();
            } else {
                this.locators = {};
                this.ensureLocatorFile();
            }
        } catch (error) {
            console.error('加载locator失败:', error);
            this.locators = {};
        }
    }

    async ensureLocatorFile() {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) return;

            const fs = require('fs');
            const path = window.AppGlobals.path;
            const locatorDir = path.join(projectPath, 'locator');
            const locatorFile = path.join(locatorDir, 'element.json');
            const imgDir = path.join(locatorDir, 'img');

            if (!fs.existsSync(locatorDir)) {
                fs.mkdirSync(locatorDir, { recursive: true });
            }
            if (!fs.existsSync(imgDir)) {
                fs.mkdirSync(imgDir, { recursive: true });
            }
            if (!fs.existsSync(locatorFile)) {
                fs.writeFileSync(locatorFile, '{}', 'utf8');
            }
        } catch (error) {
            console.error('创建locator文件失败:', error);
        }
    }

    // 统一元素查找（通过IPC）
    async findElement(locatorName, strategy = 'auto', threshold = 0.6) {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) throw new Error('无法获取项目路径');

            const deviceId = window.AppGlobals.getCurrentDeviceId();
            if (!deviceId) throw new Error('无法获取设备ID');

            const result = await this.ipcRenderer.invoke(
                'tke-recognizer-find',
                deviceId,
                projectPath,
                locatorName,
                strategy,
                threshold
            );

            if (!result.success) {
                throw new Error(result.error);
            }

            return JSON.parse(result.output);
        } catch (error) {
            console.error('元素查找失败:', error);
            throw error;
        }
    }

    // 向后兼容：XML元素查找
    async findXmlElement(locatorName) {
        return this.findElement(locatorName, 'auto');
    }

    // 向后兼容：图像元素查找
    async findImageElement(locatorName, threshold = 0.6) {
        return this.findElement(locatorName, 'img', threshold);
    }

    async addXmlLocator(name, elementData) {
        try {
            // 统一格式 - 无 type 字段
            const locator = {
                name: name,
                note: elementData.note || '',
                // XML 定位字段
                xpath: elementData.xpath || null,
                resource_id: elementData.resourceId || elementData.resource_id || null,
                text: elementData.text || null,
                content_desc: elementData.contentDesc || elementData.content_desc || null,
                class_name: elementData.className || elementData.class_name || '',
                // 通用字段
                bounds: elementData.bounds || null,
                clickable: elementData.clickable || false,
                enabled: elementData.enabled || false,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString()
            };

            this.locators[name] = locator;
            await this.saveLocators();
            this.updateLocatorList();

            console.log(`已添加 locator: ${name}`);
            return true;
        } catch (error) {
            console.error('添加 locator 失败:', error);
            return false;
        }
    }

    async addImageLocator(name, imagePath) {
        try {
            // 统一格式 - 使用 img_path 字段
            const locator = {
                name: name,
                note: '',
                // 图片定位字段
                img_path: `locator/img/${name}.png`,
                // 通用字段
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString()
            };

            this.locators[name] = locator;
            await this.saveLocators();
            this.updateLocatorList();

            console.log(`已添加图像 locator: ${name}`);
            return true;
        } catch (error) {
            console.error('添加图像 locator 失败:', error);
            return false;
        }
    }

    async saveLocators() {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) return;

            const fs = require('fs');
            const path = window.AppGlobals.path;
            const locatorFile = path.join(projectPath, 'locator', 'element.json');

            await this.ensureLocatorFile();
            fs.writeFileSync(locatorFile, JSON.stringify(this.locators, null, 2), 'utf8');
            console.log('Locator定义已保存');
        } catch (error) {
            console.error('保存locator失败:', error);
        }
    }

    async deleteLocator(name) {
        try {
            if (this.locators[name]) {
                // 如果有图像路径，删除图像文件
                if (this.locators[name].img_path) {
                    const projectPath = window.AppGlobals.getCurrentProjectPath();
                    if (projectPath) {
                        const fs = require('fs');
                        const path = window.AppGlobals.path;
                        const imagePath = path.join(projectPath, this.locators[name].img_path);

                        if (fs.existsSync(imagePath)) {
                            fs.unlinkSync(imagePath);
                        }
                    }
                }

                delete this.locators[name];
                await this.saveLocators();
                this.updateLocatorList();

                console.log(`已删除locator: ${name}`);
                return true;
            }
        } catch (error) {
            console.error('删除locator失败:', error);
            return false;
        }
    }

    updateLocatorList() {
        const locatorLibContent = document.getElementById('locatorLibContent');
        if (!locatorLibContent) return;

        locatorLibContent.innerHTML = '';

        for (const [name, locator] of Object.entries(this.locators)) {
            const locatorItem = document.createElement('div');
            locatorItem.className = 'locator-item';

            // 根据 img_path 判断类型
            const isImageType = !!locator.img_path;
            const typeIcon = isImageType ? '🖼️' : '📄';

            locatorItem.innerHTML = `
                <div class="locator-info">
                    <span class="locator-type">${typeIcon}</span>
                    <span class="locator-name">${name}</span>
                    <span class="locator-desc">${locator.note || ''}</span>
                </div>
                <div class="locator-actions">
                    <button class="btn btn-sm btn-outline-primary" onclick="locatorManagerTKE.testLocator('${name}')">测试</button>
                    <button class="btn btn-sm btn-outline-danger" onclick="locatorManagerTKE.deleteLocator('${name}')">删除</button>
                </div>
            `;

            locatorLibContent.appendChild(locatorItem);
        }
    }

    async testLocator(name) {
        try {
            const locator = this.locators[name];
            if (!locator) throw new Error(`Locator '${name}' 不存在`);

            // 根据 img_path 判断使用哪种策略
            const strategy = locator.img_path ? 'img' : 'auto';
            const result = await this.findElement(name, strategy);

            window.AppNotifications?.success(`Locator '${name}' 测试成功，位置: (${result.x}, ${result.y})`);

            return result;
        } catch (error) {
            window.AppNotifications?.error(`Locator '${name}' 测试失败: ${error.message}`);
            throw error;
        }
    }

    setupDragAndDrop() {
        const locatorLibContent = document.getElementById('locatorLibContent');
        if (locatorLibContent) {
            locatorLibContent.addEventListener('dragover', (e) => e.preventDefault());
            locatorLibContent.addEventListener('drop', (e) => {
                e.preventDefault();
                this.handleDrop(e);
            });
        }
    }

    async handleDrop(e) {
        try {
            const files = e.dataTransfer.files;
            for (const file of files) {
                if (file.type.startsWith('image/')) {
                    await this.handleImageDrop(file);
                }
            }
        } catch (error) {
            console.error('处理拖放失败:', error);
            window.AppNotifications?.error('处理拖放文件失败');
        }
    }

    async handleImageDrop(file) {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) throw new Error('无法获取项目路径');

            const fs = require('fs');
            const path = window.AppGlobals.path;

            const baseName = path.basename(file.name, path.extname(file.name));
            const locatorName = baseName.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '_');

            const imgDir = path.join(projectPath, 'locator', 'img');
            const imgPath = path.join(imgDir, `${locatorName}.png`);

            if (!fs.existsSync(imgDir)) {
                fs.mkdirSync(imgDir, { recursive: true });
            }

            const reader = new FileReader();
            reader.onload = async (event) => {
                try {
                    const buffer = Buffer.from(event.target.result);
                    fs.writeFileSync(imgPath, buffer);
                    await this.addImageLocator(locatorName, imgPath);

                    window.AppNotifications?.success(`已添加图像locator: ${locatorName}`);
                } catch (error) {
                    window.AppNotifications?.error('保存图像文件失败');
                }
            };

            reader.readAsArrayBuffer(file);
        } catch (error) {
            console.error('处理图像拖放失败:', error);
            throw error;
        }
    }

    setupSearch() {
        const searchInput = document.getElementById('locatorSearch');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterLocators(e.target.value);
            });
        }
    }

    filterLocators(searchText) {
        const locatorItems = document.querySelectorAll('.locator-item');
        const searchLower = searchText.toLowerCase();

        locatorItems.forEach(item => {
            const name = item.querySelector('.locator-name').textContent.toLowerCase();
            const desc = item.querySelector('.locator-desc').textContent.toLowerCase();

            if (name.includes(searchLower) || desc.includes(searchLower)) {
                item.style.display = 'flex';
            } else {
                item.style.display = 'none';
            }
        });
    }
}

let locatorManagerTKE = null;

async function initializeLocatorManagerTKE() {
    if (locatorManagerTKE) return locatorManagerTKE;

    try {
        locatorManagerTKE = new LocatorManagerTKE();
        await locatorManagerTKE.initialize();
        window.locatorManagerTKE = locatorManagerTKE;
        return locatorManagerTKE;
    } catch (error) {
        console.error('LocatorManagerTKE初始化失败:', error);
        throw error;
    }
}

window.LocatorManagerTKEModule = {
    LocatorManagerTKE,
    initializeLocatorManagerTKE,
    getInstance: () => locatorManagerTKE
};
