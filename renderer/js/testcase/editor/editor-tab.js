class EditorTab {
    constructor(container, editorManager) {
        this.container = container;
        this.editorManager = editorManager; // 保存管理器引用
        this.currentMode = editorManager ? editorManager.getGlobalEditMode() : 'block'; // 从管理器读取模式
        this.buffer = null; // 基于TKE的编辑器缓冲区
        this.script = new ScriptModel(); // 使用独立的脚本模型
        this.listeners = [];
        this.saveTimeout = null;
        this.isTestRunning = false;
        this.currentHighlightedLine = null; // 跟踪当前高亮的行号
        this.uniqueId = `editor-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`; // 生成唯一ID
        
        // 块定义
        this.blockDefinitions = {
            application: {
                color: '#c586c0',
                icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M16.56,5.44L15.11,6.89C16.84,7.94 18,9.83 18,12A6,6 0 0,1 12,18A6,6 0 0,1 6,12C6,9.83 7.16,7.94 8.88,6.88L7.44,5.44C5.36,6.88 4,9.28 4,12A8,8 0 0,0 12,20A8,8 0 0,0 20,12C20,9.28 18.64,6.88 16.56,5.44M13,3H11V13H13"/></svg>',
                commands: [
                    { 
                        type: 'launch', 
                        label: '启动', 
                        tksCommand: '启动',
                        params: [
                            { name: 'package', type: 'text', placeholder: '包名', default: 'com.example.test_toolkit' },
                            { name: 'activity', type: 'text', placeholder: 'Activity名字', default: '.MainActivity' }
                        ]
                    },
                    { 
                        type: 'close', 
                        label: '关闭',
                        tksCommand: '关闭',
                        params: [
                            { name: 'package', type: 'text', placeholder: '包名', default: 'com.example.app' },
                            { name: 'activity', type: 'text', placeholder: 'Activity名字', default: '.MainActivity' }
                        ]
                    }
                ]
            },
            action: {
                color: '#569cd6',
                icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 14a8 8 0 0 1-8 8"/><path d="M18 11v-1a2 2 0 0 0-2-2a2 2 0 0 0-2 2"/><path d="M14 10V9a2 2 0 0 0-2-2a2 2 0 0 0-2 2v1"/><path d="M10 9.5V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v10"/><path d="M18 11a2 2 0 1 1 4 0v3a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15"/></svg>',
                commands: [
                    { 
                        type: 'click', 
                        label: '点击',
                        tksCommand: '点击',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML/图片元素', default: '' }
                        ]
                    },
                    { 
                        type: 'press', 
                        label: '按压',
                        tksCommand: '按压',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML/图片元素', default: '' },
                            { name: 'duration', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                        ]
                    },
                    { 
                        type: 'swipe', 
                        label: '滑动',
                        tksCommand: '滑动',
                        params: [
                            { name: 'startPoint', type: 'coordinate', placeholder: '起点坐标', default: '{200,400}' },
                            { name: 'endPoint', type: 'coordinate', placeholder: '终点坐标', default: '{300,600}' },
                            { name: 'duration', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                        ]
                    },
                    { 
                        type: 'drag', 
                        label: '拖动',
                        tksCommand: '拖动',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML/图片元素', default: '' },
                            { name: 'endPoint', type: 'coordinate', placeholder: '终点坐标', default: '{500,800}' },
                            { name: 'duration', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                        ]
                    },
                    { 
                        type: 'directional_drag', 
                        label: '定向拖动',
                        tksCommand: '定向拖动',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML/图片元素', default: '' },
                            { name: 'direction', type: 'select', placeholder: '方向', default: 'up', options: ['up', 'down', 'left', 'right'] },
                            { name: 'distance', type: 'number', placeholder: '拖动距离', default: '300' },
                            { name: 'duration', type: 'number', placeholder: '持续时长/ms', default: '1000' }
                        ]
                    }
                ]
            },
            input: {
                color: '#4ec9b0',
                icon: '⌨',
                commands: [
                    { 
                        type: 'input', 
                        label: '输入',
                        tksCommand: '输入',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML元素', default: '' },
                            { name: 'text', type: 'text', placeholder: '输入的文本内容', default: '' }
                        ]
                    },
                    { 
                        type: 'clear', 
                        label: '清理',
                        tksCommand: '清理',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML元素', default: '' }
                        ]
                    },
                    { 
                        type: 'hide_keyboard', 
                        label: '隐藏键盘',
                        tksCommand: '隐藏键盘',
                        params: []
                    }
                ]
            },
            control: {
                color: '#ce9178',
                icon: '⏱',
                commands: [
                    { 
                        type: 'wait', 
                        label: '等待',
                        tksCommand: '等待',
                        params: [
                            { name: 'duration', type: 'number', placeholder: '等待时长/ms', default: '1000' }
                        ]
                    },
                    { 
                        type: 'back', 
                        label: '返回',
                        tksCommand: '返回',
                        params: []
                    }
                ]
            },
            assertion: {
                color: '#f48771',
                icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z"/></svg>',
                commands: [
                    { 
                        type: 'assert', 
                        label: '断言',
                        tksCommand: '断言',
                        params: [
                            { name: 'target', type: 'element', placeholder: 'XML/图片元素', default: '' },
                            { name: 'condition', type: 'select', options: ['存在', '不存在', '可见', '不可见'], default: '存在' }
                        ]
                    }
                ]
            },
            text: {
                color: '#9cdcfe',
                icon: '📖',
                commands: [
                    { 
                        type: 'read', 
                        label: '读取',
                        tksCommand: '读取',
                        params: [
                            { name: 'target', type: 'element', placeholder: '坐标/XML元素', default: '' },
                            { name: 'leftRight', type: 'number', placeholder: '左右扩展', default: '' },
                            { name: 'upDown', type: 'number', placeholder: '上下扩展', default: '' }
                        ]
                    }
                ]
            }
        };
        
        // 确保模块方法已混合
        this.ensureModulesMixed();
        
        this.init();
    }
    
    // 确保所有模块方法都已混合到原型中
    ensureModulesMixed() {
        if (!this.setupLocatorInputDragDrop && typeof window.mixinEditorModules === 'function') {
            if (window.rLog) window.rLog('🔧 EditorTab实例化时检测到方法缺失，尝试混合模块...');
            window.mixinEditorModules();
        }
    }
    
    init() {
        window.rLog('EditorTab 初始化中...');
        this.createEditor();
        this.setupEventListeners();
        // 显示初始占位界面
        this.renderPlaceholder();
        window.rLog('EditorTab 初始化完成，当前模式:', this.currentMode);
    }
    
    renderPlaceholder() {
        if (this.editorContainer) {
            this.editorContainer.innerHTML = `
                <div class="editor-loading-placeholder" style="
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 100%;
                    color: #666;
                    font-size: 14px;
                ">
                    正在加载文件...
                </div>
            `;
        }
    }
    
    createEditor() {
        const containerId = `${this.uniqueId}-container`;
        this.container.innerHTML = `
            <div class="unified-editor">
                <!-- 统一的内容容器 -->
                <div class="editor-content-container" id="${containerId}">
                    <!-- 内容将根据模式动态渲染 -->
                </div>
            </div>
        `;
        
        this.editorContainer = this.container.querySelector(`#${containerId}`);
    }
    
    setupEventListeners() {
        // 不再在这里监听全局快捷键，由 EditorManager 统一处理
        // 只处理编辑器内部的事件
    }
    
    toggleMode() {
        // 这个方法保留用于向后兼容，但优先使用全局切换
        if (this.currentMode === 'text') {
            this.switchToBlockMode();
        } else {
            this.switchToTextMode();
        }
    }
    
    switchToTextMode() {
        window.rLog('切换到文本模式');
        
        // 保存当前高亮状态
        const savedHighlightLine = this.currentHighlightedLine;
        const wasTestRunning = this.isTestRunning;
        
        this.currentMode = 'text';
        this.render();
        
        // 恢复高亮状态
        if (savedHighlightLine !== null && wasTestRunning) {
            window.rLog('恢复文本模式高亮:', savedHighlightLine);
            // 延迟一点确保DOM已完全渲染
            setTimeout(() => {
                this.highlightExecutingLine(savedHighlightLine);
            }, 50);
        }
    }
    
    switchToBlockMode() {
        window.rLog('切换到块编程模式');
        
        // 保存当前高亮状态
        const savedHighlightLine = this.currentHighlightedLine;
        const wasTestRunning = this.isTestRunning;
        
        this.currentMode = 'block';
        this.render();
        
        // 恢复高亮状态
        if (savedHighlightLine !== null && wasTestRunning) {
            window.rLog('恢复块模式高亮:', savedHighlightLine);
            // 延迟一点确保DOM已完全渲染
            setTimeout(() => {
                this.highlightExecutingLine(savedHighlightLine);
            }, 50);
        }
    }
    
    render() {
        if (this.currentMode === 'text') {
            this.renderTextMode();
        } else {
            this.renderBlockMode();
        }
    }
    
    renderTextMode() {
        window.rLog('渲染文本模式...');
        const tksCode = this.script.toTKSCode();
        
        const lineNumbersId = `${this.uniqueId}-lines`;
        const textContentId = `${this.uniqueId}-text`;
        
        this.editorContainer.innerHTML = `
            <div class="text-editor-view">
                <div class="text-editor-wrapper">
                    <div class="line-numbers" id="${lineNumbersId}"></div>
                    <div class="text-content" id="${textContentId}" contenteditable="true">${this.highlightTKSSyntax(tksCode)}</div>
                </div>
            </div>
        `;
        
        // 在编辑器范围内创建状态指示器
        const editorContainer = this.container;
        if (editorContainer) {
            // 移除旧的内联状态指示器（如果存在）
            const oldIndicators = this.editorContainer.querySelectorAll('.editor-status-indicator');
            oldIndicators.forEach(indicator => indicator.remove());
            
            // 更新状态指示器
            this.updateStatusIndicator();
        } else {
            window.rError('找不到编辑器容器');
        }
        
        this.textContentEl = this.editorContainer.querySelector('.text-content');
        this.lineNumbersEl = this.editorContainer.querySelector('.line-numbers');
        
        window.rLog('文本模式DOM元素:', {
            textContentEl: this.textContentEl,
            lineNumbersEl: this.lineNumbersEl,
            statusIndicatorEl: this.statusIndicatorEl
        });
        
        this.setupTextModeListeners();
        this.updateLineNumbers();
        
        // 更新状态指示器
        this.updateStatusIndicator();
    }
    
    renderBlockMode() {
        const blocksContainerId = `${this.uniqueId}-blocks`;
        
        this.editorContainer.innerHTML = `
            <div class="block-editor-view">
                <div class="blocks-workspace">
                    <div class="blocks-container" id="${blocksContainerId}"></div>
                </div>
            </div>
        `;
        
        // 在块模式下，更新状态指示器
        this.updateStatusIndicator();
        
        this.blocksContainer = this.editorContainer.querySelector('.blocks-container');
        
        window.rLog('块编辑器DOM元素:', {
            editorContainer: this.editorContainer,
            blocksContainer: this.blocksContainer
        });
        
        if (this.blocksContainer) {
            this.renderBlocks();
            this.setupBlockModeListeners();
        } else {
            window.rError('无法找到块编辑器DOM元素');
        }
    }
    
    
    renderBlocks() {
        // 获取命令
        const commands = this.script.getCommands();
        let blocksHtml = '';
        
        // 为每个命令块生成HTML，包括块间的插入按钮
        commands.forEach((command, index) => {
            const definition = this.findCommandDefinition(command.type);
            const category = this.findCommandCategory(command.type);
            
            if (!definition || !category) return;
            
            // 创建带参数孔的指令块 - 混合文本和输入框
            let commandContent = `<span class="block-icon">${category.icon}</span><span class="command-label">${definition.label}</span>`;
            
            // 为每个参数创建输入框，并整合到命令中
            if (definition.params.length > 0) {
                definition.params.forEach((param, paramIndex) => {
                    const value = command.params[param.name] || param.default || '';
                    const paramId = `param-${index}-${param.name}`;
                    
                    if (param.type === 'select') {
                        const optionsHtml = param.options.map(opt => 
                            `<option value="${opt}" ${value === opt ? 'selected' : ''}>${opt}</option>`
                        ).join('');
                        commandContent += `
                            <select class="param-hole" 
                                    id="${paramId}"
                                    data-param="${param.name}"
                                    data-command-index="${index}"
                                    title="${param.placeholder}">
                                ${optionsHtml}
                            </select>
                        `;
                    } else {
                        // 检查参数类型是否为element，以支持可视化渲染
                        if (param.type === 'element') {
                            if (value && (value.match(/^@\{(.+)\}$/) || value.match(/^\{(.+)\}$/))) {
                                // 检查值是否为图片引用格式 @{name} 或 XML元素引用格式 {name}
                                const imageMatch = value.match(/^@\{(.+)\}$/);
                                const xmlMatch = value.match(/^\{(.+)\}$/);
                                
                                // 创建一个容器用于显示可视化元素
                                commandContent += `
                                    <div class="param-hole-container" 
                                         data-param="${param.name}"
                                         data-command-index="${index}"
                                         data-type="element">
                                        <input class="param-hole hidden-input" 
                                               id="${paramId}"
                                               type="hidden"
                                               data-param="${param.name}"
                                               data-command-index="${index}"
                                               value="${value}">
                                        <div class="param-visual-element" 
                                             id="visual-${paramId}"
                                             data-param="${param.name}"
                                             data-command-index="${index}">
                                            <!-- 可视化内容将在渲染后动态添加 -->
                                        </div>
                                    </div>
                                `;
                            } else {
                                // element 类型的普通文本输入框（无论是否有值）
                                commandContent += `
                                    <input class="param-hole" 
                                           id="${paramId}"
                                           type="text"
                                           data-param="${param.name}"
                                           data-command-index="${index}"
                                           data-param-type="element"
                                           placeholder="${param.placeholder}"
                                           title="${param.placeholder}"
                                           value="${value}">
                                `;
                            }
                        } else {
                            // 非locator类型的普通输入框
                            commandContent += `
                                <input class="param-hole" 
                                       id="${paramId}"
                                       type="${param.type === 'number' ? 'number' : 'text'}"
                                       data-param="${param.name}"
                                       data-command-index="${index}"
                                       placeholder="${param.placeholder}"
                                       title="${param.placeholder}"
                                       value="${value}">
                            `;
                        }
                    }
                });
            }
            
            // 单行命令块 - 保持原有颜色背景
            const blockHtml = `
                <div class="workspace-block command-block" 
                     data-index="${index}"
                     data-type="${command.type}"
                     draggable="true"
                     style="background: linear-gradient(135deg, ${category.color}ee, ${category.color}cc);">
                    <div class="command-content">
                        ${commandContent}
                    </div>
                    <button class="block-delete" data-index="${index}" title="删除">×</button>
                </div>
            `;
            
            blocksHtml += blockHtml;
        });
        
        // 最后添加一个插入按钮
        const finalInsertButton = `
            <div class="block-insert-area final" data-insert-index="${commands.length}">
                <button class="block-insert-btn" title="添加命令块">
                    <svg width="16" height="16" viewBox="0 0 16 16">
                        <path fill="currentColor" d="M8 2v12m-6-6h12"/>
                    </svg>
                </button>
            </div>
        `;
        
        if (commands.length === 0) {
            // 空状态
            this.blocksContainer.innerHTML = `
                <div class="empty-state">
                    <svg width="48" height="48" viewBox="0 0 48 48" opacity="0.3">
                        <path fill="currentColor" d="M38 8H10c-2.21 0-4 1.79-4 4v24c0 2.21 1.79 4 4 4h28c2.21 0 4-1.79 4-4V12c0-2.21-1.79-4-4-4z"/>
                    </svg>
                    <p>在左侧Explorer中点击脚本后开始编辑</p>
                </div>
                ${finalInsertButton}
            `;
        } else {
            this.blocksContainer.innerHTML = blocksHtml + finalInsertButton;
        }
        
        // 渲染完成后，处理可视化元素
        this.renderVisualElements();
        
        // 为locator类型的输入框添加拖放支持
        this.setupLocatorInputDragDrop();
    }
    
    // 渲染可视化元素
    renderVisualElements() {
        const visualElements = this.blocksContainer.querySelectorAll('.param-visual-element');
        
        visualElements.forEach(element => {
            const commandIndex = parseInt(element.dataset.commandIndex);
            const paramName = element.dataset.param;
            const command = this.script.getCommands()[commandIndex];
            
            if (command && command.params[paramName]) {
                const value = command.params[paramName];
                
                const imageMatch = value.match(/^@\{(.+)\}$/);
                const xmlMatch = value.match(/^\{(.+)\}$/);
                
                if (imageMatch) {
                    // 渲染图片元素
                    const imageName = imageMatch[1];
                    // 获取项目路径
                    const { path: PathModule } = window.AppGlobals;
                    const projectPath = window.AppGlobals.currentProject;
                    const imagePath = projectPath ? PathModule.join(projectPath, 'locator/img', `${imageName}.png`) : '';
                    
                    element.innerHTML = `
                        <div class="visual-image-card">
                            <img src="${imagePath}" alt="${imageName}" onerror="this.style.display='none'; this.nextElementSibling.style.display='block';">
                            <div class="image-fallback" style="display:none;">
                                <svg width="24" height="24" viewBox="0 0 24 24">
                                    <path fill="currentColor" d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
                                </svg>
                            </div>
                            <span class="visual-name">${imageName}</span>
                            <button class="visual-remove" data-command-index="${commandIndex}" data-param="${paramName}">×</button>
                        </div>
                    `;
                } else if (xmlMatch) {
                    // 渲染XML元素卡片
                    const elementName = xmlMatch[1];
                    element.innerHTML = `
                        <div class="visual-xml-card">
                            <svg width="20" height="20" viewBox="0 0 24 24">
                                <path fill="#4a90e2" d="M8 3a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2H3v2h1a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2v-2H8v-4a2 2 0 0 0-2-2 2 2 0 0 0 2-2V5h2V3m6 0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h1v2h-1a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2v-2h2v-4a2 2 0 0 1 2-2 2 2 0 0 1-2-2V5h-2V3"/>
                            </svg>
                            <span class="visual-name">${elementName}</span>
                            <button class="visual-remove" data-command-index="${commandIndex}" data-param="${paramName}">×</button>
                        </div>
                    `;
                }
            }
        });
        
        // 为移除按钮添加事件监听
        this.blocksContainer.querySelectorAll('.visual-remove').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const commandIndex = parseInt(btn.dataset.commandIndex);
                const paramName = btn.dataset.param;
                
                // 清空参数值
                const command = this.script.getCommands()[commandIndex];
                if (command) {
                    command.params[paramName] = '';
                    this.renderBlocks();
                    this.setupBlockModeListeners();
                    this.triggerChange();
                }
            });
        });
    }
    


    
    setupTextModeListeners() {
        // 获取文本内容元素
        this.textContentEl = this.editorContainer.querySelector('.text-content');
        if (!this.textContentEl) {
            window.rLog('textContent元素未找到');
            return;
        }
        
        this.textContentEl.addEventListener('input', () => {
            if (this.isTestRunning) return;
            
            // 从文本更新脚本模型
            const tksCode = this.textContentEl.textContent || '';
            this.script.fromTKSCode(tksCode);
            this.updateLineNumbers();
            this.triggerChange();
        });
        
        this.textContentEl.addEventListener('keydown', (e) => {
            if (this.isTestRunning) {
                e.preventDefault();
                return;
            }
            
            // 只处理编辑器特定的快捷键
            if (e.key === 'Tab') {
                e.preventDefault();
                document.execCommand('insertText', false, '  ');
            } else if (e.key === 'Enter') {
                e.preventDefault();
                document.execCommand('insertText', false, '\n');
            }
            // 其他按键正常处理，不阻止
        });
        
        // 添加拖放支持
        this.textContentEl.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.stopPropagation();
            e.dataTransfer.dropEffect = 'copy';
        });
        
        this.textContentEl.addEventListener('drop', (e) => {
            e.preventDefault();
            e.stopPropagation();
            
            // 获取拖拽的文本数据
            const text = e.dataTransfer.getData('text/plain');
            
            if (text) {
                // 根据鼠标位置创建插入点
                const range = document.caretRangeFromPoint(e.clientX, e.clientY);
                if (range) {
                    // 确保插入点在文本编辑器内
                    if (this.textContentEl.contains(range.startContainer)) {
                        // 清除当前选区并设置新位置
                        const selection = window.getSelection();
                        selection.removeAllRanges();
                        selection.addRange(range);
                        
                        // 插入文本
                        range.deleteContents();
                        const textNode = document.createTextNode(text);
                        range.insertNode(textNode);
                        
                        // 移动光标到插入文本的末尾
                        range.setStartAfter(textNode);
                        range.setEndAfter(textNode);
                        range.collapse(true);
                        selection.removeAllRanges();
                        selection.addRange(range);
                        
                        // 更新脚本模型
                        const tksCode = this.textContentEl.textContent || '';
                        this.script.fromTKSCode(tksCode);
                        this.updateLineNumbers();
                        this.triggerChange();
                    }
                } else {
                    // 降级方案：使用当前光标位置
                    const selection = window.getSelection();
                    if (selection.rangeCount > 0) {
                        const range = selection.getRangeAt(0);
                        range.deleteContents();
                        const textNode = document.createTextNode(text);
                        range.insertNode(textNode);
                        
                        // 移动光标到插入文本的末尾
                        range.setStartAfter(textNode);
                        range.setEndAfter(textNode);
                        selection.removeAllRanges();
                        selection.addRange(range);
                        
                        // 更新脚本模型
                        const tksCode = this.textContentEl.textContent || '';
                        this.script.fromTKSCode(tksCode);
                        this.updateLineNumbers();
                        this.triggerChange();
                    }
                }
            }
        });
    }
    
    setupBlockModeListeners() {
        // 先移除之前的事件监听器，避免重复绑定
        if (this.blockClickHandler) {
            this.container.removeEventListener('click', this.blockClickHandler);
        }
        
        // 点击事件处理器
        this.blockClickHandler = (e) => {
            if (e.target.classList.contains('block-delete')) {
                e.preventDefault();
                e.stopPropagation();
                const index = parseInt(e.target.dataset.index);
                window.rLog(`删除命令块，索引: ${index}, 当前命令数量: ${this.script.getCommands().length}`);
                
                // 验证索引有效性
                if (index >= 0 && index < this.script.getCommands().length) {
                    this.removeCommand(index);
                } else {
                    window.rLog(`无效的删除索引: ${index}`);
                }
            } else if (e.target.classList.contains('block-insert-btn') || e.target.closest('.block-insert-btn')) {
                const insertArea = e.target.closest('.block-insert-area');
                const insertIndex = parseInt(insertArea.dataset.insertIndex);
                this.showCommandMenu(insertArea, insertIndex);
            }
        };
        this.container.addEventListener('click', this.blockClickHandler);
        
        // 拖拽事件
        this.container.addEventListener('dragstart', (e) => {
            if (this.isTestRunning) {
                e.preventDefault();
                return;
            }
            
            const block = e.target.closest('.workspace-block.command-block');
            if (block) {
                block.classList.add('dragging');
                e.dataTransfer.setData('text/plain', JSON.stringify({
                    type: 'reorder',
                    fromIndex: parseInt(block.dataset.index)
                }));
                e.dataTransfer.effectAllowed = 'move';
            }
        });
        
        this.container.addEventListener('dragend', (e) => {
            const block = e.target.closest('.workspace-block.command-block');
            if (block) {
                block.classList.remove('dragging');
            }
            // 清除所有拖拽高亮
            this.blocksContainer.querySelectorAll('.drag-over').forEach(el => {
                el.classList.remove('drag-over');
            });
        });
        
        this.container.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = 'move';
            
            // 找到最近的块并高亮
            const block = e.target.closest('.workspace-block.command-block');
            if (block && !block.classList.contains('dragging')) {
                // 清除之前的高亮
                this.blocksContainer.querySelectorAll('.drag-over').forEach(el => {
                    el.classList.remove('drag-over');
                });
                
                // 添加新的高亮
                const rect = block.getBoundingClientRect();
                const midY = rect.top + rect.height / 2;
                if (e.clientY < midY) {
                    block.classList.add('drag-over');
                } else {
                    // 高亮下一个块的上边缘
                    const nextBlock = block.nextElementSibling;
                    if (nextBlock && nextBlock.classList.contains('command-block')) {
                        nextBlock.classList.add('drag-over');
                    }
                }
            }
        });
        
        this.container.addEventListener('drop', (e) => {
            e.preventDefault();
            if (this.isTestRunning) return;
            
            const data = JSON.parse(e.dataTransfer.getData('text/plain'));
            if (data.type === 'reorder') {
                const targetBlock = e.target.closest('.workspace-block.command-block');
                if (targetBlock && !targetBlock.classList.contains('dragging')) {
                    const toIndex = parseInt(targetBlock.dataset.index);
                    const rect = targetBlock.getBoundingClientRect();
                    const midY = rect.top + rect.height / 2;
                    
                    // 确定插入位置
                    let insertIndex = toIndex;
                    if (e.clientY >= midY) {
                        insertIndex = toIndex + 1;
                    }
                    
                    this.reorderCommand(data.fromIndex, insertIndex);
                }
            }
        });
        
        // 右键菜单
        this.container.addEventListener('contextmenu', (e) => {
            window.rLog('右键菜单事件触发');
            const block = e.target.closest('.workspace-block.command-block');
            window.rLog('找到的块元素:', !!block);
            if (block) {
                window.rLog('块索引:', block.dataset.index);
                window.rLog('测试运行状态:', this.isTestRunning);
                e.preventDefault();
                this.showContextMenu(e.clientX, e.clientY, parseInt(block.dataset.index));
            }
        });
        
        // 参数输入
        this.container.addEventListener('input', (e) => {
            if (this.isTestRunning) return;
            
            if (e.target.classList.contains('param-hole')) {
                const index = parseInt(e.target.dataset.commandIndex);
                const param = e.target.dataset.param;
                this.updateCommandParam(index, param, e.target.value);
            }
        });
        
        this.container.addEventListener('change', (e) => {
            if (e.target.classList.contains('param-hole') && e.target.tagName === 'SELECT') {
                const index = parseInt(e.target.dataset.commandIndex);
                const param = e.target.dataset.param;
                this.updateCommandParam(index, param, e.target.value);
            }
        });
        
        // 点击其他地方关闭菜单
        document.addEventListener('click', (e) => {
            if (!e.target.closest('.command-menu') && !e.target.closest('.block-insert-btn') && !e.target.closest('.temp-insert')) {
                this.hideCommandMenu();
            }
            if (!e.target.closest('.context-menu')) {
                this.hideContextMenu();
            }
        });
        
        // 重新设置拖拽监听器（确保删除元素后拖拽功能仍然可用）
        this.setupLocatorInputDragDrop();
    }
    
    // 显示命令选择菜单
    showCommandMenu(insertArea, insertIndex) {
        window.rLog(`showCommandMenu 被调用，插入位置: ${insertIndex}, 插入区域存在: ${!!insertArea}`);
        
        if (this.isTestRunning) {
            window.rLog('测试运行中，无法显示命令菜单');
            return;
        }
        
        if (!insertArea) {
            window.rError('插入区域不存在，无法显示命令菜单');
            return;
        }
        
        // 隐藏现有菜单
        this.hideCommandMenu();
        
        // 创建菜单HTML
        const menuItems = [];
        Object.entries(this.blockDefinitions).forEach(([categoryKey, category]) => {
            category.commands.forEach(cmd => {
                menuItems.push(`
                    <div class="command-menu-item" data-type="${cmd.type}" data-insert-index="${insertIndex}">
                        <span class="menu-item-icon">${category.icon}</span>
                        <span class="menu-item-label">${cmd.label}</span>
                    </div>
                `);
            });
        });
        
        window.rLog(`创建了 ${menuItems.length} 个菜单项`);
        
        const menuHtml = `
            <div class="command-menu" id="commandMenu">
                ${menuItems.join('')}
            </div>
        `;
        
        // 插入菜单到插入区域
        window.rLog('将菜单HTML插入到插入区域');
        insertArea.insertAdjacentHTML('beforeend', menuHtml);
        this.currentMenu = insertArea.querySelector('.command-menu');
        
        if (this.currentMenu) {
            window.rLog('菜单元素创建成功，菜单项数量:', this.currentMenu.querySelectorAll('.command-menu-item').length);
            // 确保菜单可见
            this.currentMenu.style.display = 'block';
            this.currentMenu.style.visibility = 'visible';
            window.rLog('菜单样式:', window.getComputedStyle(this.currentMenu).display, window.getComputedStyle(this.currentMenu).visibility);
        } else {
            window.rError('菜单元素创建失败');
        }
        
        // 绑定菜单项点击事件
        this.currentMenu.addEventListener('click', (e) => {
            const menuItem = e.target.closest('.command-menu-item');
            if (menuItem) {
                const commandType = menuItem.dataset.type;
                const index = parseInt(menuItem.dataset.insertIndex);
                this.insertCommand(commandType, index);
                this.hideCommandMenu();
            }
        });
    }
    
    // 隐藏命令选择菜单
    hideCommandMenu() {
        if (this.currentMenu) {
            this.currentMenu.remove();
            this.currentMenu = null;
        }
    }

    // 脚本操作方法
    addCommand(type) {
        const definition = this.findCommandDefinition(type);
        if (!definition) return;
        
        const command = {
            type: type,
            params: {}
        };
        
        // 初始化参数
        definition.params.forEach(param => {
            command.params[param.name] = param.default || '';
        });
        
        this.script.addCommand(command);
        this.renderBlocks();
        this.triggerChange();
    }
    
    // 插入命令（在指定位置）
    insertCommand(type, index) {
        const definition = this.findCommandDefinition(type);
        if (!definition) return;
        
        const command = {
            type: type,
            params: {}
        };
        
        // 初始化参数
        definition.params.forEach(param => {
            command.params[param.name] = param.default || '';
        });
        
        this.script.insertCommand(command, index);
        this.renderBlocks();
        this.triggerChange();
    }
    
    // 重新排序命令
    reorderCommand(fromIndex, toIndex) {
        if (fromIndex === toIndex) return;
        
        // 调整索引以适应移动后的位置
        let adjustedToIndex = toIndex;
        if (fromIndex < toIndex) {
            adjustedToIndex = toIndex - 1;
        }
        
        this.script.reorderCommand(fromIndex, adjustedToIndex);
        this.renderBlocks();
        this.triggerChange();
        
        window.rLog(`已移动命令：从位置 ${fromIndex} 到位置 ${adjustedToIndex}`);
    }
    
    // 显示右键菜单
    showContextMenu(x, y, blockIndex) {
        window.rLog(`显示右键菜单，位置: (${x}, ${y}), 块索引: ${blockIndex}`);
        
        // 移除现有菜单
        this.hideContextMenu();
        
        const menuHtml = `
            <div class="context-menu" id="blockContextMenu" style="left: ${x}px; top: ${y}px;">
                <div class="context-menu-item" data-action="insert-below" data-index="${blockIndex}">
                    <span class="context-menu-item-icon">+</span>
                    在下方插入命令
                </div>
            </div>
        `;
        
        // 移除旧的菜单
        if (this.currentContextMenu) {
            this.currentContextMenu.remove();
        }
        
        const menuId = `${this.uniqueId}-context-menu`;
        const updatedMenuHtml = menuHtml.replace('id="blockContextMenu"', `id="${menuId}"`);
        document.body.insertAdjacentHTML('beforeend', updatedMenuHtml);
        this.currentContextMenu = document.querySelector(`#${menuId}`);
        
        window.rLog('右键菜单DOM元素已创建:', !!this.currentContextMenu);
        if (this.currentContextMenu) {
            window.rLog('菜单位置:', this.currentContextMenu.style.left, this.currentContextMenu.style.top);
            window.rLog('菜单尺寸:', this.currentContextMenu.offsetWidth, 'x', this.currentContextMenu.offsetHeight);
        }
        
        // 绑定菜单项点击事件
        this.currentContextMenu.addEventListener('click', (e) => {
            window.rLog('右键菜单项被点击');
            e.preventDefault();
            e.stopPropagation(); // 防止事件冒泡到document，导致菜单立即隐藏
            
            const menuItem = e.target.closest('.context-menu-item');
            if (menuItem) {
                const action = menuItem.dataset.action;
                const index = parseInt(menuItem.dataset.index);
                window.rLog(`菜单项动作: ${action}, 索引: ${index}`);
                
                if (action === 'insert-below') {
                    // 显示命令选择菜单在指定块下方
                    window.rLog(`尝试在块 ${index} 下方插入命令（插入位置: ${index + 1}）`);
                    this.hideContextMenu(); // 先隐藏右键菜单
                    
                    // 使用 setTimeout 延迟执行，避免立即被全局点击事件隐藏
                    setTimeout(() => {
                        this.showInsertMenuAtBlock(index + 1);
                    }, 50);
                }
            }
        });
        
        // 点击其他地方关闭菜单
        setTimeout(() => {
            document.addEventListener('click', this.hideContextMenu.bind(this), { once: true });
        }, 0);
    }
    
    // 隐藏右键菜单
    hideContextMenu() {
        if (this.currentContextMenu) {
            this.currentContextMenu.remove();
            this.currentContextMenu = null;
        }
    }
    
    // 在指定块下方显示插入菜单
    showInsertMenuAtBlock(insertIndex) {
        window.rLog(`showInsertMenuAtBlock 被调用，插入位置: ${insertIndex}`);
        
        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        window.rLog(`找到 ${blocks.length} 个命令块`);
        
        let targetBlock = null;
        
        if (insertIndex > 0 && insertIndex - 1 < blocks.length) {
            targetBlock = blocks[insertIndex - 1];
            window.rLog(`目标块索引: ${insertIndex - 1}, 找到目标块:`, !!targetBlock);
        }
        
        // 创建临时插入区域
        const tempInsertArea = document.createElement('div');
        tempInsertArea.className = 'block-insert-area temp-insert';
        tempInsertArea.dataset.insertIndex = insertIndex;
        tempInsertArea.innerHTML = `
            <button class="block-insert-btn temp" title="选择要插入的命令">
                <svg width="16" height="16" viewBox="0 0 16 16">
                    <path fill="currentColor" d="M8 2v12m-6-6h12"/>
                </svg>
            </button>
        `;
        
        window.rLog('临时插入区域已创建');
        
        // 插入临时区域
        if (targetBlock) {
            targetBlock.insertAdjacentElement('afterend', tempInsertArea);
            window.rLog('临时区域已插入到目标块后面');
        } else {
            this.blocksContainer.insertBefore(tempInsertArea, this.blocksContainer.firstChild);
            window.rLog('临时区域已插入到容器开头');
        }
        
        // 验证临时区域是否成功插入到DOM
        window.rLog('临时区域是否在DOM中:', document.contains(tempInsertArea));
        window.rLog('临时区域的父元素:', tempInsertArea.parentElement);
        window.rLog('临时区域位置:', tempInsertArea.getBoundingClientRect());
        
        // 立即显示菜单
        window.rLog('准备显示命令菜单');
        this.showCommandMenu(tempInsertArea, insertIndex);
        
        // 菜单关闭时移除临时区域
        const originalHideMenu = this.hideCommandMenu.bind(this);
        this.hideCommandMenu = () => {
            originalHideMenu();
            
            // 移除临时插入区域
            if (tempInsertArea && tempInsertArea.parentNode) {
                window.rLog('移除临时插入区域');
                tempInsertArea.remove();
            }
            
            // 恢复原来的 hideCommandMenu 方法
            this.hideCommandMenu = originalHideMenu;
        };
    }
    
    removeCommand(index) {
        window.rLog(`开始删除命令，索引: ${index}`);
        const commandsBefore = this.script.getCommands().length;
        window.rLog(`删除前命令数量: ${commandsBefore}`);
        window.rLog('删除前的命令列表:', this.script.getCommands().map((cmd, i) => `${i}: ${cmd.type}`));
        
        this.script.removeCommand(index);
        
        const commandsAfter = this.script.getCommands().length;
        window.rLog(`删除后命令数量: ${commandsAfter}`);
        window.rLog('删除后的命令列表:', this.script.getCommands().map((cmd, i) => `${i}: ${cmd.type}`));
        
        this.renderBlocks();
        this.triggerChange();
    }
    
    
    updateCommandParam(index, param, value) {
        this.script.updateCommandParam(index, param, value);
        this.triggerChange();
    }
    
    // 工具方法
    findCommandDefinition(type) {
        for (const category of Object.values(this.blockDefinitions)) {
            const cmd = category.commands.find(c => c.type === type);
            if (cmd) return cmd;
        }
        return null;
    }
    
    findCommandCategory(type) {
        for (const [key, category] of Object.entries(this.blockDefinitions)) {
            if (category.commands.find(c => c.type === type)) {
                return category;
            }
        }
        return null;
    }
    
    getCategoryName(key) {
        const names = {
            application: '应用控制',
            action: '动作操作', 
            input: '输入操作',
            control: '时间控制',
            navigation: '导航操作',
            assertion: '断言验证'
        };
        return names[key] || key;
    }
    
    highlightTKSSyntax(text) {
        // 新TKS语法高亮
        return text
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            // 高亮命令名
            .replace(/(启动|关闭|点击|按压|滑动|拖动|定向拖动|输入|清理|隐藏键盘|返回|等待|断言|读取)/g, '<span class="syntax-action">$1</span>')
            // 高亮图片元素 @{图片名称}
            .replace(/@\{([^}]+)\}/g, '@{<span class="syntax-string">$1</span>}')
            // 高亮坐标 {x,y}
            .replace(/\{(\d+\s*,\s*\d+)\}/g, '{<span class="syntax-coordinate">$1</span>}')
            // 高亮XML元素 {元素名称}
            .replace(/\{([^}]+)\}/g, '{<span class="syntax-xml-element">$1</span>}')
            // 高亮参数列表
            .replace(/\[([^\]]+)\]/g, '[<span class="syntax-param">$1</span>]');
    }
    
    updateLineNumbers() {
        if (!this.lineNumbersEl || !this.textContentEl) return;
        
        const text = this.textContentEl.textContent || '';
        const lines = text.split('\n');
        const lineNumbersHtml = lines.map((_, index) => 
            `<div class="line-number">${index + 1}</div>`
        ).join('');
        this.lineNumbersEl.innerHTML = lineNumbersHtml;
    }
    
    triggerChange() {
        window.rLog(`📤 triggerChange 被调用，模式: ${this.currentMode}`);
        
        // 获取当前内容
        const content = this.script.toTKSCode();
        
        // 通知监听器
        this.listeners.forEach(listener => {
            if (listener.type === 'change') {
                listener.callback(content);
            }
        });
        
        // 异步保存到文件
        if (this.buffer) {
            this.buffer.updateFromText(content).catch(error => {
                window.rError(`❌ 保存文件失败: ${error.message}`);
            });
        }
        
        // 自动保存
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            if (window.EditorManager && window.EditorManager.saveCurrentFile) {
                window.EditorManager.saveCurrentFile();
            }
        }, 1000);
    }
    
    // 公共API - 设置文件路径并加载内容
    async setFile(filePath) {
        try {
            // 创建TKE缓冲区用于文件操作
            this.buffer = new window.TKEEditorBuffer(filePath);
            await this.buffer.initialize();
            
            // 加载文件内容
            await this.buffer.loadFromFile();
            
            // 将内容加载到ScriptModel
            const content = this.buffer.getRawContent();
            this.script.fromTKSCode(content);
            
            // 渲染编辑器
            this.render();
            
            window.rLog(`📁 EditorTab文件设置完成: ${filePath}`);
        } catch (error) {
            window.rError(`❌ EditorTab设置文件失败: ${error.message}`, error);
            throw error; // 重新抛出错误供上层处理
        }
    }
    
    getValue() {
        // 从ScriptModel获取TKS代码
        const content = this.script.toTKSCode();
        window.rLog(`📖 从ScriptModel获取内容长度: ${content.length}`);
        return content;
    }
    
    setPlaceholder(text) {
        // 实现占位符逻辑
    }
    
    insertText(text) {
        if (this.currentMode === 'text' && this.textContentEl) {
            // 在光标位置插入文本
            const selection = window.getSelection();
            if (selection.rangeCount > 0) {
                const range = selection.getRangeAt(0);
                const textNode = document.createTextNode(text);
                range.insertNode(textNode);
                
                // 移动光标到插入文本的末尾
                range.setStartAfter(textNode);
                range.setEndAfter(textNode);
                selection.removeAllRanges();
                selection.addRange(range);
                
                // 更新脚本模型
                const tksCode = this.textContentEl.textContent || '';
                this.script.fromTKSCode(tksCode);
                this.updateLineNumbers();
                this.triggerChange();
            } else {
                // 如果没有选区，追加到末尾
                this.textContentEl.textContent += text;
                this.script.fromTKSCode(this.textContentEl.textContent);
                this.updateLineNumbers();
                this.triggerChange();
            }
        }
    }
    
    focus() {
        if (this.currentMode === 'text' && this.textContentEl) {
            this.textContentEl.focus();
        }
    }
    
    on(event, callback) {
        this.listeners.push({ type: event, callback });
    }
    

 
    
    updateStatusIndicator() {
        window.rLog('更新状态指示器 - 运行状态:', this.isTestRunning, '当前模式:', this.currentMode);
        
        const statusBar = document.querySelector('.status-bar');
        const modeText = document.getElementById('editorModeText');
        
        if (!statusBar || !modeText) {
            window.rError('找不到状态栏或模式文本元素');
            return;
        }
        
        // 清除所有模式类
        statusBar.classList.remove('status-bar-text-mode', 'status-bar-running');
        
        if (this.isTestRunning) {
            // 运行中状态
            modeText.textContent = '运行中';
            statusBar.classList.add('status-bar-running');
        } else if (this.currentMode === 'text') {
            // 文本编辑模式
            modeText.textContent = '文本编辑';
            statusBar.classList.add('status-bar-text-mode');
        } else {
            // 普通模式（块模式）
            modeText.textContent = '';
            // 不添加任何类，保持默认样式
        }
        
        window.rLog('状态栏已更新:', modeText.textContent || '普通模式');
    }
    
    createRunningIndicator() {
        // 在块模式下运行时，更新状态栏
        this.updateStatusIndicator();
        window.rLog('块模式运行指示器已更新到状态栏');
    }
    
 
 
    
    removeStatusIndicator() {
        // 清理状态栏状态
        const statusBar = document.querySelector('.status-bar');
        const modeText = document.getElementById('editorModeText');
        
        if (statusBar) {
            statusBar.classList.remove('status-bar-text-mode', 'status-bar-running');
        }
        
        if (modeText) {
            modeText.textContent = '';
        }
        
        // 清理可能存在的所有旧的内联状态指示器（用于兼容）
        const indicators = document.querySelectorAll('.editor-status-indicator');
        indicators.forEach(indicator => indicator.remove());
        
        // 从编辑器视图中移除内联指示器
        if (this.editorContainer) {
            const textEditorView = this.editorContainer.querySelector('.text-editor-view');
            if (textEditorView) {
                const indicator = textEditorView.querySelector('.editor-status-indicator');
                if (indicator) {
                    indicator.remove();
                }
            }
        }
    }

    destroy() {
        clearTimeout(this.saveTimeout);
        // 不再需要移除全局快捷键监听器，因为它在 EditorManager 中
        this.removeStatusIndicator();
        this.hideCommandMenu();
        this.hideContextMenu();
        
        // 清理拖拽监听器
        if (this.dragOverHandler) {
            this.blocksContainer.removeEventListener('dragover', this.dragOverHandler);
        }
        if (this.dragLeaveHandler) {
            this.blocksContainer.removeEventListener('dragleave', this.dragLeaveHandler);
        }
        if (this.dropHandler) {
            this.blocksContainer.removeEventListener('drop', this.dropHandler);
        }
        
        this.listeners = [];
    }
}

// 导出到全局
window.EditorTab = EditorTab;

// 将拆分的模块方法混入到EditorTab原型中
// 必须在类导出后立即混入，以确保方法可用
function mixinEditorModules() {
    // 使用安全的日志记录方式
    if (window.rLog) {
        window.rLog('🔧 开始混入模块，可用模块:', {
            EditorHighlighting: !!window.EditorHighlighting,
            EditorLineMapping: !!window.EditorLineMapping,
            EditorFontSettings: !!window.EditorFontSettings,
            EditorDragDrop: !!window.EditorDragDrop
        });
    }

    if (window.EditorHighlighting) {
        Object.assign(EditorTab.prototype, window.EditorHighlighting);
        if (window.rLog) window.rLog('✅ EditorHighlighting 模块已混入');
    }

    if (window.EditorLineMapping) {
        Object.assign(EditorTab.prototype, window.EditorLineMapping);
        if (window.rLog) window.rLog('✅ EditorLineMapping 模块已混入');
    }

    if (window.EditorFontSettings) {
        Object.assign(EditorTab.prototype, window.EditorFontSettings);
        if (window.rLog) window.rLog('✅ EditorFontSettings 模块已混入');
    }

    if (window.EditorDragDrop) {
        Object.assign(EditorTab.prototype, window.EditorDragDrop);
        if (window.rLog) {
            window.rLog('✅ EditorDragDrop 模块已混入');
            window.rLog('EditorDragDrop 模块包含的方法:', Object.keys(window.EditorDragDrop));
        }
    }
}

// 将混合函数暴露到全局，以便实例化时调用
window.mixinEditorModules = mixinEditorModules;

// 立即尝试混合模块
mixinEditorModules();

// 如果第一次混合失败，使用 setTimeout 延迟混合
if (!EditorTab.prototype.setupLocatorInputDragDrop) {
    setTimeout(() => {
        if (window.rLog) window.rLog('🔧 延迟混合执行...');
        mixinEditorModules();
    }, 0);
}

// 旧的ScriptModel已被TKEEditorBuffer完全替代，所有.tks解析和处理都由TKE负责
