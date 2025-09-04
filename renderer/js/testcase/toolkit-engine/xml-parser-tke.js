// XML解析和UI元素管理模块 (TKE版本)
// 使用Rust TKE执行XML解析和UI元素提取功能

// UI元素类 - 保持与原版本相同的接口
class UIElement {
    constructor(data) {
        this.index = data.index || 0;
        this.className = data.className || '';
        this.bounds = data.bounds || [0, 0, 0, 0]; // [x1, y1, x2, y2]
        this.text = data.text || '';
        this.contentDesc = data.contentDesc || '';
        this.resourceId = data.resourceId || '';
        this.hint = data.hint || '';
        this.clickable = data.clickable || false;
        this.checkable = data.checkable || false;
        this.checked = data.checked || false;
        this.focusable = data.focusable || false;
        this.focused = data.focused || false;
        this.scrollable = data.scrollable || false;
        this.selected = data.selected || false;
        this.enabled = data.enabled !== false; // 默认为true
        this.xpath = data.xpath || '';
    }
    
    get centerX() { 
        return Math.floor((this.bounds[0] + this.bounds[2]) / 2); 
    }
    
    get centerY() { 
        return Math.floor((this.bounds[1] + this.bounds[3]) / 2); 
    }
    
    get width() { 
        return this.bounds[2] - this.bounds[0]; 
    }
    
    get height() { 
        return this.bounds[3] - this.bounds[1]; 
    }
    
    get isVisible() {
        return this.width > 0 && this.height > 0;
    }
    
    // 生成AI友好的文本描述
    toAiText() {
        // 类名简化
        const simpleClass = this.className.split('.').pop() || this.className;
        const attrs = [];
        
        // 按照原版本顺序添加有意义的文本属性
        if (this.text) attrs.push(`text=${this.text}`);
        if (this.hint) attrs.push(`hintText=${this.hint}`);
        if (this.contentDesc) attrs.push(`content-desc=${this.contentDesc}`);
        
        // 添加resource-id（简化显示，只取最后部分）
        if (this.resourceId) {
            const idPart = this.resourceId.includes(':') ? 
                this.resourceId.split(':').pop() : this.resourceId;
            if (idPart) {
                attrs.push(`id=${idPart}`);
            }
        }
        
        // 添加状态属性
        if (this.checked) attrs.push('checked=true');
        if (this.focused) attrs.push('focused=true');
        if (this.selected) attrs.push('selected=true');
        if (!this.enabled) attrs.push('enabled=false');
        
        // 组装结果
        return attrs.length > 0 ? `${simpleClass}(${attrs.join(', ')})` : `${simpleClass}()`;
    }
    
    // 检查元素是否匹配给定文本
    matchesText(searchText) {
        const text = searchText.toLowerCase();
        return (
            this.text.toLowerCase().includes(text) ||
            this.contentDesc.toLowerCase().includes(text) ||
            this.hint.toLowerCase().includes(text)
        );
    }
}

// XML解析器类 (TKE版本)
class XMLParserTKE {
    constructor() {
        this.tkeAdapter = null;
        this.screenWidth = 1080;
        this.screenHeight = 1920;
        this.initialized = false;
    }
    
    async init() {
        if (this.initialized) {
            return;
        }
        
        try {
            // 获取TKE适配器
            const tkeAdapter = await window.TKEAdapterModule.getTKEAdapter();
            // 创建XML解析相关的适配器
            this.scriptParserAdapter = new window.TKEAdapterModule.TKEScriptParserAdapter(tkeAdapter);
            this.locatorFetcherAdapter = new window.TKEAdapterModule.TKELocatorFetcherAdapter(tkeAdapter, '');
            this.initialized = true;
            window.rLog('XMLParser TKE版本已初始化');
        } catch (error) {
            window.rError('XMLParser TKE版本初始化失败:', error);
            throw error;
        }
    }
    
    setScreenSize(width, height) {
        this.screenWidth = width;
        this.screenHeight = height;
    }
    
    // 从XML中推断屏幕尺寸 - 使用TKE
    async inferScreenSizeFromXML(xmlString) {
        try {
            await this.init();
            
            // 调用TKE的XML解析功能来推断屏幕尺寸
            const result = await this.tkeAdapter.callTKE('script-parser', ['infer-screen-size'], xmlString);
            
            if (result.success && result.data) {
                const { width, height } = result.data;
                window.rLog(`从XML推断的屏幕尺寸: ${width}x${height}`);
                return { width, height };
            }
            
            return null;
        } catch (error) {
            window.rError('推断屏幕尺寸失败:', error);
            return null;
        }
    }
    
    // 主要的UI树优化方法 - 使用TKE
    async optimizeUITree(xmlString) {
        try {
            if (!xmlString || xmlString.trim() === '') {
                window.rError('XML字符串为空');
                return null;
            }
            
            await this.init();
            
            window.rLog('开始使用TKE优化UI树...');
            
            // 调用TKE的XML解析优化功能
            const result = await this.tkeAdapter.callTKE('script-parser', ['optimize-ui-tree'], xmlString);
            
            if (result.success && result.data) {
                window.rLog('TKE UI树优化成功');
                return result.data; // TKE返回的优化后的树结构
            } else {
                window.rError('TKE UI树优化失败:', result.error);
                return null;
            }
        } catch (error) {
            window.rError('XML优化异常:', error);
            return null;
        }
    }
    
    // 从优化后的树中提取UI元素列表 - 使用TKE
    async extractUIElements(optimizedTree, rawXmlString = null) {
        try {
            await this.init();
            
            let xmlInput = optimizedTree;
            
            // 如果没有优化树但有原始XML，使用原始XML
            if (!optimizedTree && rawXmlString) {
                xmlInput = rawXmlString;
            }
            
            if (!xmlInput) {
                window.rLog('extractUIElements: 没有可用的XML输入，返回空数组');
                return [];
            }
            
            // 调用TKE的UI元素提取功能
            const result = await this.tkeAdapter.callTKE('script-parser', ['extract-ui-elements'], 
                JSON.stringify({ xml: xmlInput, screenWidth: this.screenWidth, screenHeight: this.screenHeight }));
            
            if (result.success && result.data && Array.isArray(result.data.elements)) {
                const elements = result.data.elements.map((elementData, index) => {
                    return new UIElement({
                        ...elementData,
                        index: index
                    });
                });
                
                window.rLog(`TKE提取到 ${elements.length} 个UI元素`);
                return elements;
            } else {
                window.rError('TKE UI元素提取失败:', result.error);
                return [];
            }
        } catch (error) {
            window.rError('提取UI元素异常:', error);
            return [];
        }
    }
    
    // 生成元素列表的文本描述
    getElementListText(elements) {
        return elements.map(element => `[${element.index}] ${element.toAiText()}`).join('\n');
    }
    
    // 通过文本查找元素
    findElementByText(elements, text) {
        // 精确匹配
        for (const element of elements) {
            if (element.text === text || 
                element.contentDesc === text || 
                element.hint === text) {
                return element;
            }
        }
        
        // 包含匹配
        const searchText = text.toLowerCase();
        for (const element of elements) {
            if (element.matchesText(searchText)) {
                return element;
            }
        }
        
        return null;
    }
    
    // 通过索引查找元素
    findElementByIndex(elements, index) {
        return elements.find(element => element.index === index) || null;
    }
    
    // 生成树形结构的字符串表示 - 使用TKE
    async getTreeString(nodeOrXml, depth = 0) {
        try {
            await this.init();
            
            // 调用TKE生成树形字符串
            const result = await this.tkeAdapter.callTKE('script-parser', ['generate-tree-string'], 
                JSON.stringify({ input: nodeOrXml, depth: depth }));
            
            if (result.success && result.data && result.data.treeString) {
                return result.data.treeString;
            } else {
                window.rError('TKE生成树形字符串失败:', result.error);
                return '';
            }
        } catch (error) {
            window.rError('生成树形字符串异常:', error);
            return '';
        }
    }
}

// 导出到全局
window.UIElement = UIElement;
window.XMLParserTKE = XMLParserTKE;

// 导出模块
window.XMLParserTKEModule = {
    UIElement,
    XMLParserTKE,
    createParser: () => new XMLParserTKE()
};

window.rLog('XMLParser TKE版本模块已加载');