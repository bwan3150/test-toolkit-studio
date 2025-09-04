// Locator库管理模块 (TKE版本)
// 使用TKE进行XML元素获取和图像匹配

class LocatorManagerTKE {
    constructor() {
        this.locators = {};
        this.currentCasePath = null;
        this.initialized = false;
        this.tkeAdapter = null;
        this.fetcherAdapter = null;
        this.recognizerAdapter = null;
    }

    // 初始化Locator管理器
    async initialize() {
        if (this.initialized) return;
        
        try {
            // 初始化TKE适配器
            this.tkeAdapter = await window.TKEAdapterModule.getTKEAdapter();
            console.log('LocatorManagerTKE: TKE适配器已初始化');
            
            // 设置拖放功能
            this.setupDragAndDrop();
            
            // 设置搜索功能
            this.setupSearch();
            
            // 加载当前case的locator
            await this.loadLocators();
            
            this.initialized = true;
            console.log('LocatorManagerTKE已初始化');
        } catch (error) {
            console.error('LocatorManagerTKE初始化失败:', error);
            throw error;
        }
    }

    // 设置当前case路径
    setCurrentCase(casePath) {
        this.currentCasePath = casePath;
        this.loadLocators();
    }

    // 推断当前case路径
    inferCurrentCasePath() {
        // 优先使用全局currentTab
        let tabData = window.AppGlobals.currentTab;
        if (!tabData) {
            // 备用方案：查找活动tab
            const activeTab = document.querySelector('.tab.active');
            if (activeTab) {
                tabData = window.AppGlobals.openTabs.find(tab => tab.id === activeTab.id);
            }
        }
        
        if (tabData && tabData.path) {
            console.log('Inferring case path from:', tabData.path);
            // 从脚本路径推断case路径
            const pathSep = window.AppGlobals.path.sep;
            const pathParts = tabData.path.split(pathSep);
            const caseIndex = pathParts.indexOf('cases');
            if (caseIndex !== -1 && caseIndex < pathParts.length - 2) {
                const caseFolderName = pathParts[caseIndex + 1];
                const projectPath = pathParts.slice(0, caseIndex).join(pathSep);
                const casePath = window.AppGlobals.path.join(projectPath, 'cases', caseFolderName);
                console.log('Inferred case path:', casePath);
                return casePath;
            }
        }
        
        // 尝试从状态栏获取项目路径
        const projectPath = window.AppGlobals.getCurrentProjectPath();
        if (projectPath) {
            // 默认使用第一个case，或者创建一个临时case
            const defaultCasePath = window.AppGlobals.path.join(projectPath, 'cases', 'temp');
            console.log('Using default case path:', defaultCasePath);
            return defaultCasePath;
        }
        
        console.warn('无法推断case路径');
        return null;
    }

    // 加载locator定义
    async loadLocators() {
        try {
            const casePath = this.currentCasePath || this.inferCurrentCasePath();
            if (!casePath) {
                console.warn('无法获取case路径，跳过locator加载');
                return;
            }

            // 获取项目路径
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) {
                console.warn('无法获取项目路径，跳过locator加载');
                return;
            }

            // 使用TKE读取locator文件
            const fs = require('fs');
            const path = window.AppGlobals.path;
            const locatorFile = path.join(projectPath, 'locator', 'element.json');
            
            if (fs.existsSync(locatorFile)) {
                const content = fs.readFileSync(locatorFile, 'utf8');
                this.locators = JSON.parse(content);
                console.log(`加载了 ${Object.keys(this.locators).length} 个locator定义`);
                this.updateLocatorList();
            } else {
                console.log('Locator文件不存在，创建空文件:', locatorFile);
                this.locators = {};
                this.ensureLocatorFile();
            }
        } catch (error) {
            console.error('加载locator失败:', error);
            this.locators = {};
        }
    }

    // 确保locator文件存在
    async ensureLocatorFile() {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) return;

            const fs = require('fs');
            const path = window.AppGlobals.path;
            const locatorDir = path.join(projectPath, 'locator');
            const locatorFile = path.join(locatorDir, 'element.json');
            const imgDir = path.join(locatorDir, 'img');

            // 创建目录
            if (!fs.existsSync(locatorDir)) {
                fs.mkdirSync(locatorDir, { recursive: true });
            }
            if (!fs.existsSync(imgDir)) {
                fs.mkdirSync(imgDir, { recursive: true });
            }

            // 创建空的locator文件
            if (!fs.existsSync(locatorFile)) {
                fs.writeFileSync(locatorFile, '{}', 'utf8');
            }
        } catch (error) {
            console.error('创建locator文件失败:', error);
        }
    }

    // 获取当前UI元素（使用TKE）
    async getCurrentUIElements() {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) {
                throw new Error('无法获取项目路径');
            }

            // 创建fetcherAdapter
            if (!this.fetcherAdapter) {
                this.fetcherAdapter = new window.TKEAdapterModule.TKELocatorFetcherAdapter(
                    this.tkeAdapter, 
                    projectPath
                );
            }

            // 获取当前UI元素
            return await this.fetcherAdapter.getCurrentElements();
        } catch (error) {
            console.error('获取UI元素失败:', error);
            throw error;
        }
    }

    // 获取可交互元素（使用TKE）
    async getInteractiveElements() {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) {
                throw new Error('无法获取项目路径');
            }

            if (!this.fetcherAdapter) {
                this.fetcherAdapter = new window.TKEAdapterModule.TKELocatorFetcherAdapter(
                    this.tkeAdapter, 
                    projectPath
                );
            }

            return await this.fetcherAdapter.getInteractiveElements();
        } catch (error) {
            console.error('获取可交互元素失败:', error);
            throw error;
        }
    }

    // 图像匹配（使用TKE）
    async findImageElement(locatorName) {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) {
                throw new Error('无法获取项目路径');
            }

            if (!this.recognizerAdapter) {
                this.recognizerAdapter = new window.TKEAdapterModule.TKERecognizerAdapter(
                    this.tkeAdapter, 
                    projectPath
                );
            }

            return await this.recognizerAdapter.findImageElement(locatorName);
        } catch (error) {
            console.error('图像元素查找失败:', error);
            throw error;
        }
    }

    // XML元素匹配（使用TKE）
    async findXmlElement(locatorName) {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) {
                throw new Error('无法获取项目路径');
            }

            if (!this.recognizerAdapter) {
                this.recognizerAdapter = new window.TKEAdapterModule.TKERecognizerAdapter(
                    this.tkeAdapter, 
                    projectPath
                );
            }

            return await this.recognizerAdapter.findXmlElement(locatorName);
        } catch (error) {
            console.error('XML元素查找失败:', error);
            throw error;
        }
    }

    // 添加XML locator
    async addXmlLocator(name, elementData) {
        try {
            // 创建XML类型的locator
            const locator = {
                type: 'xml',
                className: elementData.className || '',
                bounds: elementData.bounds || null,
                text: elementData.text || '',
                contentDesc: elementData.contentDesc || '',
                resourceId: elementData.resourceId || '',
                hint: elementData.hint || '',
                description: `XML元素: ${name}`,
                addedAt: new Date().toISOString()
            };

            this.locators[name] = locator;
            await this.saveLocators();
            this.updateLocatorList();

            console.log(`已添加XML locator: ${name}`);
            return true;
        } catch (error) {
            console.error('添加XML locator失败:', error);
            return false;
        }
    }

    // 添加图像locator
    async addImageLocator(name, imagePath) {
        try {
            // 创建图像类型的locator
            const locator = {
                type: 'image',
                path: `locator/img/${name}.png`,
                description: `图像元素: ${name}`,
                addedAt: new Date().toISOString()
            };

            this.locators[name] = locator;
            await this.saveLocators();
            this.updateLocatorList();

            console.log(`已添加图像locator: ${name}`);
            return true;
        } catch (error) {
            console.error('添加图像locator失败:', error);
            return false;
        }
    }

    // 保存locator定义
    async saveLocators() {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) return;

            const fs = require('fs');
            const path = window.AppGlobals.path;
            const locatorFile = path.join(projectPath, 'locator', 'element.json');
            
            // 确保目录存在
            await this.ensureLocatorFile();
            
            // 保存文件
            fs.writeFileSync(locatorFile, JSON.stringify(this.locators, null, 2), 'utf8');
            console.log('Locator定义已保存');
        } catch (error) {
            console.error('保存locator失败:', error);
        }
    }

    // 删除locator
    async deleteLocator(name) {
        try {
            if (this.locators[name]) {
                // 如果是图像locator，同时删除图像文件
                if (this.locators[name].type === 'image' && this.locators[name].path) {
                    const projectPath = window.AppGlobals.getCurrentProjectPath();
                    if (projectPath) {
                        const fs = require('fs');
                        const path = window.AppGlobals.path;
                        const imagePath = path.join(projectPath, this.locators[name].path);
                        
                        if (fs.existsSync(imagePath)) {
                            fs.unlinkSync(imagePath);
                            console.log(`已删除图像文件: ${imagePath}`);
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

    // 更新locator列表显示
    updateLocatorList() {
        const locatorList = document.getElementById('locatorList');
        if (!locatorList) return;

        locatorList.innerHTML = '';
        
        for (const [name, locator] of Object.entries(this.locators)) {
            const locatorItem = document.createElement('div');
            locatorItem.className = 'locator-item';
            
            const typeIcon = locator.type === 'image' ? '🖼️' : '📄';
            
            locatorItem.innerHTML = `
                <div class="locator-info">
                    <span class="locator-type">${typeIcon}</span>
                    <span class="locator-name">${name}</span>
                    <span class="locator-desc">${locator.description || ''}</span>
                </div>
                <div class="locator-actions">
                    <button class="btn btn-sm btn-outline-primary" onclick="locatorManagerTKE.testLocator('${name}')">测试</button>
                    <button class="btn btn-sm btn-outline-danger" onclick="locatorManagerTKE.deleteLocator('${name}')">删除</button>
                </div>
            `;
            
            locatorList.appendChild(locatorItem);
        }
    }

    // 测试locator
    async testLocator(name) {
        try {
            const locator = this.locators[name];
            if (!locator) {
                throw new Error(`Locator '${name}' 不存在`);
            }

            console.log(`测试locator: ${name} (类型: ${locator.type})`);
            
            let result;
            if (locator.type === 'xml') {
                result = await this.findXmlElement(name);
            } else if (locator.type === 'image') {
                result = await this.findImageElement(name);
            } else {
                throw new Error(`不支持的locator类型: ${locator.type}`);
            }
            
            console.log(`Locator测试成功: ${name}`, result);
            window.NotificationModule.showNotification(
                `Locator '${name}' 测试成功，位置: (${result.x}, ${result.y})`, 
                'success'
            );
            
            return result;
        } catch (error) {
            console.error(`Locator测试失败: ${name}`, error);
            window.NotificationModule.showNotification(
                `Locator '${name}' 测试失败: ${error.message}`, 
                'error'
            );
            throw error;
        }
    }

    // 设置拖放功能
    setupDragAndDrop() {
        const locatorList = document.getElementById('locatorList');
        if (locatorList) {
            locatorList.addEventListener('dragover', (e) => {
                e.preventDefault();
            });

            locatorList.addEventListener('drop', (e) => {
                e.preventDefault();
                this.handleDrop(e);
            });
        }
    }

    // 处理拖放
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
            window.NotificationModule.showNotification('处理拖放文件失败', 'error');
        }
    }

    // 处理图像拖放
    async handleImageDrop(file) {
        try {
            const projectPath = window.AppGlobals.getCurrentProjectPath();
            if (!projectPath) {
                throw new Error('无法获取项目路径');
            }

            const fs = require('fs');
            const path = window.AppGlobals.path;
            
            // 生成locator名称
            const baseName = path.basename(file.name, path.extname(file.name));
            const locatorName = baseName.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '_');
            
            // 保存图像文件
            const imgDir = path.join(projectPath, 'locator', 'img');
            const imgPath = path.join(imgDir, `${locatorName}.png`);
            
            // 确保目录存在
            if (!fs.existsSync(imgDir)) {
                fs.mkdirSync(imgDir, { recursive: true });
            }
            
            // 读取文件并保存
            const reader = new FileReader();
            reader.onload = async (event) => {
                try {
                    const buffer = Buffer.from(event.target.result);
                    fs.writeFileSync(imgPath, buffer);
                    
                    // 添加到locator列表
                    await this.addImageLocator(locatorName, imgPath);
                    
                    window.NotificationModule.showNotification(
                        `已添加图像locator: ${locatorName}`, 
                        'success'
                    );
                } catch (error) {
                    console.error('保存图像文件失败:', error);
                    window.NotificationModule.showNotification('保存图像文件失败', 'error');
                }
            };
            
            reader.readAsArrayBuffer(file);
        } catch (error) {
            console.error('处理图像拖放失败:', error);
            throw error;
        }
    }

    // 设置搜索功能
    setupSearch() {
        const searchInput = document.getElementById('locatorSearch');
        if (searchInput) {
            searchInput.addEventListener('input', (e) => {
                this.filterLocators(e.target.value);
            });
        }
    }

    // 过滤locator
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

// 创建全局实例
let locatorManagerTKE = null;

// 初始化函数
async function initializeLocatorManagerTKE() {
    if (locatorManagerTKE) {
        console.log('LocatorManagerTKE已经初始化过，跳过重复初始化');
        return locatorManagerTKE;
    }
    
    try {
        locatorManagerTKE = new LocatorManagerTKE();
        await locatorManagerTKE.initialize();
        
        // 导出到全局
        window.locatorManagerTKE = locatorManagerTKE;
        
        return locatorManagerTKE;
    } catch (error) {
        console.error('LocatorManagerTKE初始化失败:', error);
        throw error;
    }
}

// 导出模块
window.LocatorManagerTKEModule = {
    LocatorManagerTKE,
    initializeLocatorManagerTKE,
    getInstance: () => locatorManagerTKE
};