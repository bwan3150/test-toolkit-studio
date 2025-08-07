// XML解析和UI元素管理模块
// 基于Arbigent项目的UI识别逻辑移植

// UI元素类
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
    
    // 生成AI友好的文本描述（完全按照Arbigent逻辑）
    toAiText() {
        // 类名简化
        const simpleClass = this.className.split('.').pop() || this.className;
        const attrs = [];
        
        // 按照Arbigent顺序添加有意义的文本属性
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

// XML解析器类
class XMLParser {
    constructor() {
        // 有意义的属性定义（完全匹配Arbigent原版）
        this.meaningfulAttributes = new Set([
            'text',
            'content-desc',
            'hint',
            'hintText',
            'title',
            'accessibilityText'
        ]);
        
        // 有意义的布尔属性
        this.meaningfulBoolAttributes = new Set([
            'clickable',
            'checkable',
            'focusable',
            'selectable'
        ]);
        
        // 需要过滤的系统UI元素
        this.filteredResourceIds = [
            'status_bar_container',
            'status_bar_launch_animation_container',
            'navigationBarBackground',
            'navigation_bar'
        ];
        
        this.screenWidth = 1080;
        this.screenHeight = 1920;
    }
    
    setScreenSize(width, height) {
        this.screenWidth = width;
        this.screenHeight = height;
    }
    
    // 从XML中推断屏幕尺寸
    inferScreenSizeFromXML(xmlString) {
        try {
            const parser = new DOMParser();
            const doc = parser.parseFromString(xmlString, 'text/xml');
            const root = doc.documentElement;
            
            if (!root) return null;
            
            let maxX = 0, maxY = 0;
            
            // 递归查找所有元素的bounds属性
            const findMaxBounds = (node) => {
                if (node.nodeType === Node.ELEMENT_NODE) {
                    const boundsStr = node.getAttribute('bounds');
                    if (boundsStr) {
                        const bounds = this._parseBounds(boundsStr);
                        if (bounds[2] > maxX) maxX = bounds[2];
                        if (bounds[3] > maxY) maxY = bounds[3];
                    }
                }
                
                for (const child of node.children || []) {
                    findMaxBounds(child);
                }
            };
            
            findMaxBounds(root);
            
            if (maxX > 0 && maxY > 0) {
                console.log('从XML推断的屏幕尺寸:', maxX, 'x', maxY);
                return { width: maxX, height: maxY };
            }
            
            return null;
        } catch (error) {
            console.error('推断屏幕尺寸失败:', error);
            return null;
        }
    }
    
    // 主要的UI树优化方法
    optimizeUITree(xmlString) {
        try {
            if (!xmlString || xmlString.trim() === '') {
                console.error('XML字符串为空');
                return null;
            }
            
            // 清理XML字符串（移除可能的控制字符）
            const cleanedXml = xmlString.replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, '').trim();
            
            console.log('清理后的XML前100字符:', cleanedXml.substring(0, 100));
            
            const parser = new DOMParser();
            const doc = parser.parseFromString(cleanedXml, 'text/xml');
            
            // 检查是否解析成功
            const parseErrors = doc.getElementsByTagName('parsererror');
            if (parseErrors.length > 0) {
                console.error('XML解析失败，错误信息:', parseErrors[0].textContent);
                return null;
            }
            
            const root = doc.documentElement;
            if (!root || root.nodeName === 'parsererror') {
                console.error('XML根元素无效');
                return null;
            }
            
            console.log('XML解析成功，根元素:', root.nodeName);
            console.log('根元素属性数:', root.attributes ? root.attributes.length : 0);
            console.log('根元素子节点数:', root.children ? root.children.length : 0);
            
            const optimizedRoot = this._optimizeNode(root);
            console.log('优化后的树:', optimizedRoot ? '成功' : '失败');
            
            return optimizedRoot;
        } catch (error) {
            console.error('XML优化异常:', error);
            console.error('错误堆栈:', error.stack);
            return null;
        }
    }
    
    // 递归优化节点（基于Arbigent的optimizeTree2算法）
    _optimizeNode(node) {
        if (!node || node.nodeType !== Node.ELEMENT_NODE) {
            // console.debug('跳过非元素节点:', node?.nodeName);
            return null;
        }
        
        // console.debug('处理节点:', node.nodeName, node.getAttribute('class'));
        
        // 如果节点不应该包含，返回null
        if (!this._shouldIncludeNode(node)) {
            // console.debug('节点被过滤:', node.getAttribute('class'));
            return null;
        }
        
        // 如果节点及其子孙都没有意义，返回null
        if (!this._hasMeaningfulDescendants(node)) {
            // console.debug('节点无意义子孙:', node.getAttribute('class'));
            return null;
        }
        
        // 递归优化子节点
        const optimizedChildren = [];
        if (node.children && node.children.length > 0) {
            for (const child of Array.from(node.children)) {
                const optimizedChild = this._optimizeNode(child);
                if (optimizedChild) {
                    optimizedChildren.push(optimizedChild);
                }
            }
        }
        
        // 创建新节点并复制属性
        const newNode = node.cloneNode(false);
        // console.debug('创建新节点:', newNode.nodeName, '优化后子节点数:', optimizedChildren.length);
        
        // 如果当前节点有意义，保留它和它的子节点
        if (this._isMeaningfulNode(node)) {
            optimizedChildren.forEach(child => newNode.appendChild(child));
            // console.debug('保留有意义节点:', node.getAttribute('class'));
            return newNode;
        }
        
        // 如果当前节点无意义但有多个子节点，保留结构
        if (optimizedChildren.length > 1) {
            optimizedChildren.forEach(child => newNode.appendChild(child));
            // console.debug('保留结构节点:', node.getAttribute('class'));
            return newNode;
        }
        
        // 如果当前节点无意义且只有一个子节点，提升子节点
        if (optimizedChildren.length === 1) {
            // console.debug('提升子节点:', optimizedChildren[0].getAttribute?.('class'));
            return optimizedChildren[0];
        }
        
        // 如果没有子节点但当前节点本身有意义，返回节点
        if (this._isMeaningfulNode(node)) {
            // console.debug('保留叶子节点:', node.getAttribute('class'));
            return newNode;
        }
        
        // console.debug('丢弃节点:', node.getAttribute('class'));
        // 如果没有子节点且无意义，返回null
        return null;
    }
    
    // 判断节点是否有意义（完全匹配Arbigent的isMeaningfulView逻辑）
    _isMeaningfulNode(node) {
        // 检查有意义的文本属性
        for (const attr of this.meaningfulAttributes) {
            const value = node.getAttribute(attr);
            if (value && value.trim()) {
                return true;
            }
        }
        
        // 检查有意义的布尔属性
        for (const attr of this.meaningfulBoolAttributes) {
            if (node.getAttribute(attr) === 'true') {
                return true;
            }
        }
        
        return false;
    }
    
    // 判断节点是否应该包含在优化后的树中
    _shouldIncludeNode(node) {
        // 检查是否是需要过滤的系统UI
        const resourceId = node.getAttribute('resource-id') || '';
        for (const filteredId of this.filteredResourceIds) {
            if (resourceId.includes(filteredId)) {
                return false;
            }
        }
        
        // 检查是否可见
        const bounds = this._parseBounds(node.getAttribute('bounds') || '');
        if (bounds[2] - bounds[0] <= 0 || bounds[3] - bounds[1] <= 0) {
            return false;
        }
        
        // 检查是否在屏幕范围内
        if (bounds[0] >= this.screenWidth || bounds[1] >= this.screenHeight) {
            return false;
        }
        if (bounds[2] <= 0 || bounds[3] <= 0) {
            return false;
        }
        
        return true;
    }
    
    // 递归检查节点是否有有意义的子孙节点
    _hasMeaningfulDescendants(node) {
        if (this._isMeaningfulNode(node)) {
            return true;
        }
        
        for (const child of node.children) {
            if (this._hasMeaningfulDescendants(child)) {
                return true;
            }
        }
        
        return false;
    }
    
    // 解析bounds字符串
    _parseBounds(boundsStr) {
        try {
            // 格式: "[0,0][1080,1920]"
            const boundsStr_clean = boundsStr.replace(/\]\[/g, ',').replace(/[\[\]]/g, '');
            const coords = boundsStr_clean.split(',').map(x => parseInt(x.trim()));
            if (coords.length === 4) {
                return coords;
            }
        } catch (error) {
            console.debug('解析bounds失败:', boundsStr, error);
        }
        return [0, 0, 0, 0];
    }
    
    // 从优化后的树中提取UI元素列表
    extractUIElements(optimizedTree) {
        if (!optimizedTree) return [];
        
        const elements = [];
        const indexCounter = {};
        
        // 如果是原始XML根节点，使用简化提取
        if (optimizedTree.nodeName === 'hierarchy' && optimizedTree.children.length > 0) {
            console.log('检测到原始XML hierarchy，使用简化提取');
            this._extractElementsFromRawXML(optimizedTree, elements);
        } else {
            this._extractElementsFromNode(optimizedTree, elements, indexCounter, '');
        }
        
        // 重新分配连续索引
        elements.forEach((element, index) => {
            element.index = index;
        });
        
        console.log(`最终提取到 ${elements.length} 个UI元素`);
        return elements;
    }
    
    // 从原始XML直接提取元素（备用方案）
    _extractElementsFromRawXML(hierarchyNode, elements) {
        const traverse = (node) => {
            if (node.nodeType === Node.ELEMENT_NODE && node.nodeName === 'node') {
                // 检查是否为可交互元素
                const clickable = node.getAttribute('clickable') === 'true';
                const focusable = node.getAttribute('focusable') === 'true';
                const hasText = node.getAttribute('text')?.trim();
                const hasContentDesc = node.getAttribute('content-desc')?.trim();
                const hasHint = node.getAttribute('hint')?.trim();
                
                // 更宽松的条件：任何有文本、可点击、可聚焦的元素都包含
                if (clickable || focusable || hasText || hasContentDesc || hasHint) {
                    const bounds = this._parseBounds(node.getAttribute('bounds') || '');
                    if (bounds[2] > bounds[0] && bounds[3] > bounds[1]) { // 确保有有效尺寸
                        const element = new UIElement({
                            index: elements.length,
                            className: node.getAttribute('class') || '',
                            bounds: bounds,
                            text: node.getAttribute('text') || '',
                            contentDesc: node.getAttribute('content-desc') || '',
                            resourceId: node.getAttribute('resource-id') || '',
                            hint: node.getAttribute('hint') || '',
                            clickable: clickable,
                            checkable: node.getAttribute('checkable') === 'true',
                            checked: node.getAttribute('checked') === 'true',
                            focusable: focusable,
                            focused: node.getAttribute('focused') === 'true',
                            scrollable: node.getAttribute('scrollable') === 'true',
                            selected: node.getAttribute('selected') === 'true',
                            enabled: node.getAttribute('enabled') !== 'false',
                            xpath: `/hierarchy/node[${elements.length}]`
                        });
                        
                        if (element.isVisible) {
                            elements.push(element);
                            console.log(`添加元素[${elements.length - 1}]: ${element.toAiText()}`);
                        }
                    }
                }
            }
            
            // 递归处理子节点
            for (const child of node.children || []) {
                traverse(child);
            }
        };
        
        traverse(hierarchyNode);
    }
    
    // 递归提取元素
    _extractElementsFromNode(node, elements, indexCounter, xpathPrefix) {
        if (!node || node.nodeType !== Node.ELEMENT_NODE) return;
        
        const nodeClass = node.getAttribute('class') || 'node';
        const nodeIndex = indexCounter[nodeClass] || 0;
        indexCounter[nodeClass] = nodeIndex + 1;
        
        const xpath = xpathPrefix ? 
            `${xpathPrefix}/${nodeClass}[${nodeIndex}]` : 
            `//${nodeClass}[${nodeIndex}]`;
        
        // 检查是否应该添加到元素列表（匹配Arbigent的shouldAddElement逻辑）
        if (this._shouldAddElement(node)) {
            const bounds = this._parseBounds(node.getAttribute('bounds') || '');
            const element = new UIElement({
                index: elements.length,
                className: nodeClass,
                bounds: bounds,
                text: node.getAttribute('text') || '',
                contentDesc: node.getAttribute('content-desc') || '',
                resourceId: node.getAttribute('resource-id') || '',
                hint: node.getAttribute('hint') || node.getAttribute('hintText') || '',
                clickable: node.getAttribute('clickable') === 'true',
                checkable: node.getAttribute('checkable') === 'true',
                checked: node.getAttribute('checked') === 'true',
                focusable: node.getAttribute('focusable') === 'true',
                focused: node.getAttribute('focused') === 'true',
                scrollable: node.getAttribute('scrollable') === 'true',
                selected: node.getAttribute('selected') === 'true',
                enabled: node.getAttribute('enabled') !== 'false',
                xpath: xpath
            });
            
            if (element.isVisible) {
                elements.push(element);
            }
        }
        
        // 递归处理子节点
        const childIndexCounter = {};
        for (const child of node.children) {
            this._extractElementsFromNode(child, elements, childIndexCounter, xpath);
        }
    }
    
    // 判断是否应该添加到元素列表
    _shouldAddElement(node) {
        // Android平台逻辑：检查可交互属性
        if (node.getAttribute('clickable') === 'true' ||
            node.getAttribute('focused') === 'true' ||
            node.getAttribute('focusable') === 'true') {
            return true;
        }
        
        // 其他情况：检查是否有意义
        return this._isMeaningfulNode(node);
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
    
    // 生成树形结构的字符串表示
    getTreeString(node, depth = 0) {
        if (!node) return '';
        
        const indent = '  '.repeat(depth);
        const className = (node.getAttribute('class') || 'node').split('.').pop();
        
        // 构建节点描述
        const attrs = [];
        
        // 添加文本属性
        const text = node.getAttribute('text');
        if (text) attrs.push(`text=${text}`);
        
        const hint = node.getAttribute('hint') || node.getAttribute('hintText');
        if (hint) attrs.push(`hintText=${hint}`);
        
        const contentDesc = node.getAttribute('content-desc');
        if (contentDesc) attrs.push(`content-desc=${contentDesc}`);
        
        // 添加resource-id（简化）
        const resourceId = node.getAttribute('resource-id');
        if (resourceId) {
            const idPart = resourceId.includes(':') ? resourceId.split(':').pop() : resourceId;
            if (idPart) attrs.push(`id=${idPart}`);
        }
        
        // 添加状态属性
        if (node.getAttribute('checked') === 'true') attrs.push('checked=true');
        if (node.getAttribute('focused') === 'true') attrs.push('focused=true');
        if (node.getAttribute('selected') === 'true') attrs.push('selected=true');
        if (node.getAttribute('enabled') === 'false') attrs.push('enabled=false');
        
        const nodeStr = attrs.length > 0 ? 
            `${className}(${attrs.join(', ')})` : 
            `${className}()`;
        
        const lines = [indent + nodeStr];
        
        // 递归处理子节点
        for (const child of node.children) {
            const childStr = this.getTreeString(child, depth + 1);
            if (childStr) {
                lines.push(childStr);
            }
        }
        
        return lines.join('\n');
    }
}

// 导出到全局
window.UIElement = UIElement;
window.XMLParser = XMLParser;

// 导出模块
window.XMLParserModule = {
    UIElement,
    XMLParser,
    createParser: () => new XMLParser()
};