// 统一脚本编辑器 - 支持文本和块两种渲染模式
// 基于统一的数据模型，可以在两种模式间无缝切换

class UnifiedScriptEditor {
    constructor(container) {
        this.container = container;
        this.currentMode = 'block'; // 'text' 或 'block'
        this.script = new ScriptModel(); // 统一的脚本数据模型
        this.listeners = [];
        this.saveTimeout = null;
        this.isTestRunning = false;
        this.currentHighlightedLine = null; // 跟踪当前高亮的行号
        
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
                            { name: 'package', type: 'text', placeholder: '应用包名', default: 'com.example.app' },
                            { name: 'activity', type: 'text', placeholder: 'Activity名', default: '.MainActivity' }
                        ]
                    },
                    { 
                        type: 'close', 
                        label: '关闭',
                        tksCommand: '关闭',
                        params: [
                            { name: 'package', type: 'text', placeholder: '应用包名', default: 'com.example.app' },
                            { name: 'activity', type: 'text', placeholder: 'Activity名', default: '.MainActivity' }
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
                            { name: 'target', type: 'locator', placeholder: '@{元素名}或坐标', default: '' }
                        ]
                    },
                    { 
                        type: 'press', 
                        label: '按压',
                        tksCommand: '按压',
                        params: [
                            { name: 'target', type: 'locator', placeholder: '@{元素名}或坐标', default: '' },
                            { name: 'duration', type: 'number', placeholder: '持续时间(ms)', default: '1000' }
                        ]
                    },
                    { 
                        type: 'swipe', 
                        label: '滑动',
                        tksCommand: '滑动',
                        params: [
                            { name: 'from', type: 'coordinate', placeholder: '起始坐标 x,y', default: '' },
                            { name: 'to', type: 'coordinate', placeholder: '结束坐标 x,y', default: '' },
                            { name: 'duration', type: 'number', placeholder: '持续时间(ms)', default: '500' }
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
                            { name: 'target', type: 'locator', placeholder: '@{输入框}或坐标', default: '' },
                            { name: 'text', type: 'text', placeholder: '输入内容', default: '' }
                        ]
                    },
                    { 
                        type: 'clear', 
                        label: '清理',
                        tksCommand: '清理',
                        params: [
                            { name: 'target', type: 'locator', placeholder: '@{输入框}或坐标', default: '' }
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
                            { name: 'duration', type: 'text', placeholder: '等待时间(ms或s)', default: '1000ms' }
                        ]
                    }
                ]
            },
            navigation: {
                color: '#dcdcaa',
                icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7v6h6"/><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"/></svg>',
                commands: [
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
                            { name: 'target', type: 'locator', placeholder: '@{元素名}', default: '' },
                            { name: 'condition', type: 'select', options: ['存在', '不存在', '可见', '不可见'], default: '存在' }
                        ]
                    }
                ]
            }
        };
        
        this.init();
    }
    
    init() {
        console.log('UnifiedScriptEditor 初始化中...');
        this.createEditor();
        this.setupEventListeners();
        // 默认渲染为块模式
        this.render();
        console.log('UnifiedScriptEditor 初始化完成，当前模式:', this.currentMode);
    }
    
    createEditor() {
        this.container.innerHTML = `
            <div class="unified-editor">
                <!-- 统一的内容容器 -->
                <div class="editor-content-container" id="editorContainer">
                    <!-- 内容将根据模式动态渲染 -->
                </div>
            </div>
        `;
        
        this.editorContainer = document.getElementById('editorContainer');
    }
    
    setupEventListeners() {
        // 全局快捷键监听器 - 使用 Cmd/Ctrl + / 切换模式
        this.globalKeyHandler = (e) => {
            if (e.key === '/' && (e.metaKey || e.ctrlKey)) {
                // 检查是否在Testcase页面
                const testcasePage = document.getElementById('testcasePage');
                const isTestcasePageActive = testcasePage && testcasePage.classList.contains('active');
                
                if (isTestcasePageActive) {
                    e.preventDefault();
                    console.log('切换编辑器模式:', this.currentMode, '->', this.currentMode === 'text' ? 'block' : 'text');
                    this.toggleMode();
                    return;
                }
            }
        };
        
        document.addEventListener('keydown', this.globalKeyHandler);
    }
    
    toggleMode() {
        if (this.currentMode === 'text') {
            this.switchToBlockMode();
        } else {
            this.switchToTextMode();
        }
    }
    
    switchToTextMode() {
        console.log('切换到文本模式');
        
        // 保存当前高亮状态
        const savedHighlightLine = this.currentHighlightedLine;
        const wasTestRunning = this.isTestRunning;
        
        this.currentMode = 'text';
        this.render();
        
        // 恢复高亮状态
        if (savedHighlightLine !== null && wasTestRunning) {
            console.log('恢复文本模式高亮:', savedHighlightLine);
            // 延迟一点确保DOM已完全渲染
            setTimeout(() => {
                this.highlightExecutingLine(savedHighlightLine);
            }, 50);
        }
    }
    
    switchToBlockMode() {
        console.log('切换到块编程模式');
        
        // 保存当前高亮状态
        const savedHighlightLine = this.currentHighlightedLine;
        const wasTestRunning = this.isTestRunning;
        
        this.currentMode = 'block';
        this.render();
        
        // 恢复高亮状态
        if (savedHighlightLine !== null && wasTestRunning) {
            console.log('恢复块模式高亮:', savedHighlightLine);
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
        console.log('渲染文本模式...');
        const tksCode = this.script.toTKSCode();
        
        this.editorContainer.innerHTML = `
            <div class="text-editor-view">
                <div class="text-editor-wrapper">
                    <div class="line-numbers" id="lineNumbers"></div>
                    <div class="text-content" id="textContent" contenteditable="true">${this.highlightTKSSyntax(tksCode)}</div>
                </div>
            </div>
        `;
        
        // 在编辑器范围内创建状态指示器
        const simpleEditor = document.getElementById('simpleEditor');
        if (simpleEditor) {
            // 移除之前的状态指示器
            this.removeStatusIndicator();
            
            // 确保编辑器容器有相对定位
            simpleEditor.style.position = 'relative';
            
            // 创建新的状态指示器，直接添加到文本编辑器视图中
            const statusIndicator = document.createElement('div');
            statusIndicator.className = 'editor-status-indicator text-mode';
            statusIndicator.id = 'editorStatusIndicator';
            statusIndicator.textContent = '文本编辑';
            statusIndicator.style.cssText = `
                position: absolute !important;
                top: 8px !important;
                right: 12px !important;
                padding: 4px 8px !important;
                border-radius: 4px !important;
                font-size: 11px !important;
                font-weight: 600 !important;
                color: white !important;
                z-index: 100 !important;
                user-select: none !important;
                pointer-events: none !important;
                display: block !important;
                visibility: visible !important;
                opacity: 1 !important;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1) !important;
            `;
            
            // 添加到文本编辑器视图内部，而不是整个容器
            const textEditorView = this.editorContainer.querySelector('.text-editor-view');
            if (textEditorView) {
                textEditorView.appendChild(statusIndicator);
                this.statusIndicatorEl = statusIndicator;
                
                // 立即更新状态指示器以设置正确的背景色
                this.updateStatusIndicator();
                
                console.log('状态指示器已添加到文本编辑器视图内:', this.statusIndicatorEl);
            } else {
                console.error('找不到文本编辑器视图');
            }
        } else {
            console.error('找不到 simpleEditor 容器');
        }
        
        this.textContentEl = this.editorContainer.querySelector('#textContent');
        this.lineNumbersEl = this.editorContainer.querySelector('#lineNumbers');
        
        console.log('文本模式DOM元素:', {
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
        this.editorContainer.innerHTML = `
            <div class="block-editor-view">
                <div class="blocks-workspace">
                    <div class="blocks-container" id="blocksContainer"></div>
                </div>
            </div>
        `;
        
        // 在块模式下，移除状态指示器
        this.removeStatusIndicator();
        
        this.blocksContainer = this.editorContainer.querySelector('#blocksContainer');
        
        console.log('块编辑器DOM元素:', {
            editorContainer: this.editorContainer,
            blocksContainer: this.blocksContainer
        });
        
        if (this.blocksContainer) {
            this.renderBlocks();
            this.setupBlockModeListeners();
        } else {
            console.error('无法找到块编辑器DOM元素');
        }
    }
    
    
    renderBlocks() {
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
                        // 检查参数类型是否为locator，以支持可视化渲染
                        if (param.type === 'locator' && value) {
                            // 检查值是否为图片引用格式 @{name}
                            const imageMatch = value.match(/^@\{(.+)\}$/);
                            // 检查值是否为XML元素引用格式 [name]
                            const xmlMatch = value.match(/^\[(.+)\]$/);
                            
                            if (imageMatch || xmlMatch) {
                                // 创建一个容器用于显示可视化元素
                                commandContent += `
                                    <div class="param-hole-container" 
                                         data-param="${param.name}"
                                         data-command-index="${index}"
                                         data-type="locator">
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
                                // 普通文本输入框
                                commandContent += `
                                    <input class="param-hole" 
                                           id="${paramId}"
                                           type="text"
                                           data-param="${param.name}"
                                           data-command-index="${index}"
                                           data-param-type="locator"
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
                const xmlMatch = value.match(/^\[(.+)\]$/);
                
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
    
    // 为locator类型的输入框添加拖放支持
    setupLocatorInputDragDrop() {
        const locatorInputs = this.blocksContainer.querySelectorAll('input[data-param-type="locator"], .param-hole-container[data-type="locator"]');
        
        locatorInputs.forEach(element => {
            // 如果是容器，需要找到实际的输入元素
            const isContainer = element.classList.contains('param-hole-container');
            const dropTarget = isContainer ? element : element;
            
            dropTarget.addEventListener('dragover', (e) => {
                e.preventDefault();
                e.stopPropagation();
                e.dataTransfer.dropEffect = 'copy';
                dropTarget.classList.add('drag-over');
            });
            
            dropTarget.addEventListener('dragleave', (e) => {
                e.stopPropagation();
                dropTarget.classList.remove('drag-over');
            });
            
            dropTarget.addEventListener('drop', (e) => {
                e.preventDefault();
                e.stopPropagation();
                dropTarget.classList.remove('drag-over');
                
                const commandIndex = parseInt(element.dataset.commandIndex);
                const paramName = element.dataset.param;
                
                // 尝试获取自定义格式的数据
                const locatorDataStr = e.dataTransfer.getData('application/x-locator');
                const textData = e.dataTransfer.getData('text/plain');
                
                if (textData) {
                    // 更新参数值
                    const command = this.script.getCommands()[commandIndex];
                    if (command) {
                        command.params[paramName] = textData;
                        
                        // 重新渲染块以显示可视化元素
                        this.renderBlocks();
                        this.setupBlockModeListeners();
                        this.triggerChange();
                    }
                }
            });
        });
    }
    
    setupTextModeListeners() {
        // 获取文本内容元素
        this.textContentEl = document.getElementById('textContent');
        if (!this.textContentEl) {
            console.warn('textContent元素未找到');
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
            
            if (e.key === 'Tab') {
                e.preventDefault();
                document.execCommand('insertText', false, '  ');
            } else if (e.key === 'Enter') {
                e.preventDefault();
                document.execCommand('insertText', false, '\n');
            }
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
        // 不再使用全局标记，因为每次重新渲染后都需要重新绑定事件
        
        // 点击事件处理器
        this.container.addEventListener('click', (e) => {
            if (e.target.classList.contains('block-delete')) {
                e.preventDefault();
                e.stopPropagation();
                const index = parseInt(e.target.dataset.index);
                console.log(`删除命令块，索引: ${index}, 当前命令数量: ${this.script.getCommands().length}`);
                
                // 验证索引有效性
                if (index >= 0 && index < this.script.getCommands().length) {
                    this.removeCommand(index);
                } else {
                    console.warn(`无效的删除索引: ${index}`);
                }
            } else if (e.target.classList.contains('block-insert-btn') || e.target.closest('.block-insert-btn')) {
                const insertArea = e.target.closest('.block-insert-area');
                const insertIndex = parseInt(insertArea.dataset.insertIndex);
                this.showCommandMenu(insertArea, insertIndex);
            }
        });
        
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
            console.log('右键菜单事件触发');
            const block = e.target.closest('.workspace-block.command-block');
            console.log('找到的块元素:', !!block);
            if (block) {
                console.log('块索引:', block.dataset.index);
                console.log('测试运行状态:', this.isTestRunning);
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
    }
    
    // 显示命令选择菜单
    showCommandMenu(insertArea, insertIndex) {
        console.log(`showCommandMenu 被调用，插入位置: ${insertIndex}, 插入区域存在: ${!!insertArea}`);
        
        if (this.isTestRunning) {
            console.log('测试运行中，无法显示命令菜单');
            return;
        }
        
        if (!insertArea) {
            console.error('插入区域不存在，无法显示命令菜单');
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
        
        console.log(`创建了 ${menuItems.length} 个菜单项`);
        
        const menuHtml = `
            <div class="command-menu" id="commandMenu">
                ${menuItems.join('')}
            </div>
        `;
        
        // 插入菜单到插入区域
        console.log('将菜单HTML插入到插入区域');
        insertArea.insertAdjacentHTML('beforeend', menuHtml);
        this.currentMenu = insertArea.querySelector('.command-menu');
        
        if (this.currentMenu) {
            console.log('菜单元素创建成功，菜单项数量:', this.currentMenu.querySelectorAll('.command-menu-item').length);
            // 确保菜单可见
            this.currentMenu.style.display = 'block';
            this.currentMenu.style.visibility = 'visible';
            console.log('菜单样式:', window.getComputedStyle(this.currentMenu).display, window.getComputedStyle(this.currentMenu).visibility);
        } else {
            console.error('菜单元素创建失败');
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
        
        console.log(`已移动命令：从位置 ${fromIndex} 到位置 ${adjustedToIndex}`);
    }
    
    // 显示右键菜单
    showContextMenu(x, y, blockIndex) {
        console.log(`显示右键菜单，位置: (${x}, ${y}), 块索引: ${blockIndex}`);
        
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
        
        document.body.insertAdjacentHTML('beforeend', menuHtml);
        this.currentContextMenu = document.getElementById('blockContextMenu');
        
        console.log('右键菜单DOM元素已创建:', !!this.currentContextMenu);
        if (this.currentContextMenu) {
            console.log('菜单位置:', this.currentContextMenu.style.left, this.currentContextMenu.style.top);
            console.log('菜单尺寸:', this.currentContextMenu.offsetWidth, 'x', this.currentContextMenu.offsetHeight);
        }
        
        // 绑定菜单项点击事件
        this.currentContextMenu.addEventListener('click', (e) => {
            console.log('右键菜单项被点击');
            e.preventDefault();
            e.stopPropagation(); // 防止事件冒泡到document，导致菜单立即隐藏
            
            const menuItem = e.target.closest('.context-menu-item');
            if (menuItem) {
                const action = menuItem.dataset.action;
                const index = parseInt(menuItem.dataset.index);
                console.log(`菜单项动作: ${action}, 索引: ${index}`);
                
                if (action === 'insert-below') {
                    // 显示命令选择菜单在指定块下方
                    console.log(`尝试在块 ${index} 下方插入命令（插入位置: ${index + 1}）`);
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
        console.log(`showInsertMenuAtBlock 被调用，插入位置: ${insertIndex}`);
        
        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        console.log(`找到 ${blocks.length} 个命令块`);
        
        let targetBlock = null;
        
        if (insertIndex > 0 && insertIndex - 1 < blocks.length) {
            targetBlock = blocks[insertIndex - 1];
            console.log(`目标块索引: ${insertIndex - 1}, 找到目标块:`, !!targetBlock);
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
        
        console.log('临时插入区域已创建');
        
        // 插入临时区域
        if (targetBlock) {
            targetBlock.insertAdjacentElement('afterend', tempInsertArea);
            console.log('临时区域已插入到目标块后面');
        } else {
            this.blocksContainer.insertBefore(tempInsertArea, this.blocksContainer.firstChild);
            console.log('临时区域已插入到容器开头');
        }
        
        // 验证临时区域是否成功插入到DOM
        console.log('临时区域是否在DOM中:', document.contains(tempInsertArea));
        console.log('临时区域的父元素:', tempInsertArea.parentElement);
        console.log('临时区域位置:', tempInsertArea.getBoundingClientRect());
        
        // 立即显示菜单
        console.log('准备显示命令菜单');
        this.showCommandMenu(tempInsertArea, insertIndex);
        
        // 菜单关闭时移除临时区域
        const originalHideMenu = this.hideCommandMenu.bind(this);
        this.hideCommandMenu = () => {
            originalHideMenu();
            
            // 移除临时插入区域
            if (tempInsertArea && tempInsertArea.parentNode) {
                console.log('移除临时插入区域');
                tempInsertArea.remove();
            }
            
            // 恢复原来的 hideCommandMenu 方法
            this.hideCommandMenu = originalHideMenu;
        };
    }
    
    removeCommand(index) {
        console.log(`开始删除命令，索引: ${index}`);
        const commandsBefore = this.script.getCommands().length;
        console.log(`删除前命令数量: ${commandsBefore}`);
        console.log('删除前的命令列表:', this.script.getCommands().map((cmd, i) => `${i}: ${cmd.type}`));
        
        this.script.removeCommand(index);
        
        const commandsAfter = this.script.getCommands().length;
        console.log(`删除后命令数量: ${commandsAfter}`);
        console.log('删除后的命令列表:', this.script.getCommands().map((cmd, i) => `${i}: ${cmd.type}`));
        
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
        // 简化的语法高亮
        return text
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/(启动|关闭|点击|按压|滑动|输入|清理|隐藏键盘|返回|等待|断言)/g, '<span class="syntax-action">$1</span>')
            .replace(/@\{([^}]+)\}/g, '@{<span class="syntax-string">$1</span>}')
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
        this.listeners.forEach(listener => {
            if (listener.type === 'change') {
                listener.callback(this.script.toTKSCode());
            }
        });
        
        // 自动保存
        clearTimeout(this.saveTimeout);
        this.saveTimeout = setTimeout(() => {
            if (window.EditorModule && window.EditorModule.saveCurrentFile) {
                window.EditorModule.saveCurrentFile();
            }
        }, 1000);
    }
    
    // 公共API
    setValue(tksCode) {
        this.script.fromTKSCode(tksCode);
        this.render();
    }
    
    getValue() {
        return this.script.toTKSCode();
    }
    
    setPlaceholder(text) {
        // 实现占位符逻辑
    }
    
    focus() {
        if (this.currentMode === 'text' && this.textContentEl) {
            this.textContentEl.focus();
        }
    }
    
    on(event, callback) {
        this.listeners.push({ type: event, callback });
    }
    
    setTestRunning(isRunning, clearHighlight = false) {
        console.log('设置测试运行状态:', isRunning, '清除高亮:', clearHighlight, '当前模式:', this.currentMode);
        this.isTestRunning = isRunning;
        
        // 只有在明确要求清除高亮时才清除（成功完成时）
        if (!isRunning && clearHighlight) {
            console.log('测试成功结束，清除所有高亮');
            this.clearExecutionHighlight();
        } else if (!isRunning) {
            console.log('测试结束但保持错误高亮（如果有）');
        }
        
        // 更新状态指示器
        this.updateStatusIndicator();
        
        // 在文本模式下禁用/启用编辑
        if (this.currentMode === 'text' && this.textContentEl) {
            this.textContentEl.contentEditable = !isRunning;
            this.textContentEl.style.opacity = isRunning ? '0.7' : '1';
            this.textContentEl.style.cursor = isRunning ? 'not-allowed' : 'text';
        }
    }
    
    updateStatusIndicator() {
        console.log('更新状态指示器 - 运行状态:', this.isTestRunning, '当前模式:', this.currentMode);
        
        // 在文本模式和块模式下都可能需要显示运行状态
        if (this.statusIndicatorEl) {
            if (this.isTestRunning) {
                this.statusIndicatorEl.textContent = '运行中';
                // 移除所有类并重新设置
                this.statusIndicatorEl.className = 'editor-status-indicator running';
                this.statusIndicatorEl.style.cssText = `
                    position: absolute !important;
                    top: 8px !important;
                    right: 12px !important;
                    padding: 4px 8px !important;
                    border-radius: 4px !important;
                    font-size: 11px !important;
                    font-weight: 600 !important;
                    color: white !important;
                    background: #ff8c00 !important;
                    z-index: 100 !important;
                    user-select: none !important;
                    pointer-events: none !important;
                    display: block !important;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1) !important;
                `;
            } else if (this.currentMode === 'text') {
                this.statusIndicatorEl.textContent = '文本编辑';
                this.statusIndicatorEl.className = 'editor-status-indicator text-mode';
                this.statusIndicatorEl.style.cssText = `
                    position: absolute !important;
                    top: 8px !important;
                    right: 12px !important;
                    padding: 4px 8px !important;
                    border-radius: 4px !important;
                    font-size: 11px !important;
                    font-weight: 600 !important;
                    color: white !important;
                    background: #0078d4 !important;
                    z-index: 100 !important;
                    user-select: none !important;
                    pointer-events: none !important;
                    display: block !important;
                    box-shadow: 0 2px 4px rgba(0,0,0,0.1) !important;
                `;
            } else {
                // 块模式下非运行时隐藏
                this.statusIndicatorEl.style.display = 'none !important';
            }
            console.log('状态指示器已更新:', this.statusIndicatorEl.textContent, '背景色:', this.statusIndicatorEl.style.background);
        }
        
        // 在块模式运行时，也需要创建状态指示器
        if (this.currentMode === 'block' && this.isTestRunning && !this.statusIndicatorEl) {
            this.createRunningIndicator();
        }
    }
    
    createRunningIndicator() {
        const simpleEditor = document.getElementById('simpleEditor');
        if (simpleEditor) {
            const statusIndicator = document.createElement('div');
            statusIndicator.className = 'editor-status-indicator running';
            statusIndicator.id = 'editorStatusIndicator';
            statusIndicator.textContent = '运行中';
            statusIndicator.style.cssText = `
                position: absolute !important;
                top: 8px !important;
                right: 12px !important;
                padding: 4px 8px !important;
                border-radius: 4px !important;
                font-size: 11px !important;
                font-weight: 600 !important;
                color: white !important;
                background: #ff8c00 !important;
                z-index: 100 !important;
                user-select: none !important;
                pointer-events: none !important;
                display: block !important;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1) !important;
            `;
            
            const blockEditorView = this.editorContainer.querySelector('.block-editor-view');
            if (blockEditorView) {
                blockEditorView.appendChild(statusIndicator);
                this.statusIndicatorEl = statusIndicator;
                console.log('块模式运行指示器已创建');
            }
        }
    }
    
    // 行高亮功能
    highlightExecutingLine(tksOriginalLineNumber) {
        console.log('=== 高亮请求开始 ===');
        console.log('TKS原始行号:', tksOriginalLineNumber);
        console.log('当前模式:', this.currentMode);
        console.log('上一次高亮行号:', this.currentHighlightedLine);
        console.log('测试运行状态:', this.isTestRunning);
        
        // 先检查这是否是一个有效的命令行
        let isValidCommandLine = false;
        if (this.script.originalLines && this.script.lineToCommandMap) {
            const originalLineIndex = tksOriginalLineNumber - 1;
            if (originalLineIndex >= 0 && originalLineIndex < this.script.lineToCommandMap.length) {
                isValidCommandLine = this.script.lineToCommandMap[originalLineIndex] !== null;
            }
        }
        
        if (!isValidCommandLine) {
            console.log('✓ 非命令行请求，保持当前高亮连续性');
            const originalLineIndex = tksOriginalLineNumber - 1;
            console.log('行内容:', this.script.originalLines ? `"${this.script.originalLines[originalLineIndex]}"` : '无法获取');
            console.log('=== 高亮请求结束（非命令行，保持当前高亮）===');
            return;
        }
        
        // 只有在切换到不同的有效命令行时才清除之前的高亮
        if (this.currentHighlightedLine !== tksOriginalLineNumber) {
            console.log('✓ 切换到新的命令行，清除之前的高亮');
            console.log('从行', this.currentHighlightedLine, '切换到行', tksOriginalLineNumber);
            this.clearExecutionHighlight();
            this.currentHighlightedLine = tksOriginalLineNumber;
        } else {
            console.log('✓ 同一命令行重复高亮请求，检查高亮是否仍然存在');
            
            // 检查当前高亮是否仍然存在
            let highlightExists = false;
            if (this.currentMode === 'text') {
                const existingHighlights = this.container.querySelectorAll('.line-highlight.executing');
                highlightExists = existingHighlights.length > 0;
                console.log('文本模式 - 现有执行高亮数量:', existingHighlights.length);
            } else if (this.currentMode === 'block') {
                const existingBlockHighlights = this.container.querySelectorAll('.workspace-block.highlighted.executing');
                highlightExists = existingBlockHighlights.length > 0;
                console.log('块模式 - 现有执行高亮数量:', existingBlockHighlights.length);
            }
            
            if (highlightExists) {
                console.log('✓ 高亮仍然存在，跳过重新创建');
                console.log('=== 高亮请求结束（跳过）===');
                return;
            } else {
                console.log('✗ 高亮丢失，重新创建');
            }
        }
        
        if (this.currentMode === 'text' && this.textContentEl) {
            // 文本模式：将TKS原始行号转换为显示行号
            const displayLineNumber = this.calculateDisplayLineNumber(tksOriginalLineNumber);
            console.log('文本模式 - 计算显示行号:', displayLineNumber);
            if (displayLineNumber > 0) {
                this.addLineHighlight(displayLineNumber, 'executing');
                console.log('✓ 文本模式高亮已创建 - 显示行号:', displayLineNumber, '(TKS原始行号:', tksOriginalLineNumber, ')');
                console.log('=== 高亮请求结束（文本模式完成）===');
            } else {
                console.error('✗ 有效命令行但显示行号计算失败:', displayLineNumber);
                console.log('=== 高亮请求结束（文本模式失败）===');
            }
        } else if (this.currentMode === 'block') {
            // 块模式：将TKS原始行号转换为命令索引（已确认是有效命令行）
            const originalLineIndex = tksOriginalLineNumber - 1;
            const commandIndex = this.script.lineToCommandMap[originalLineIndex];
            console.log('块模式 - TKS行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);
            
            // 命令索引转换为1基索引进行高亮
            const blockIndex = commandIndex + 1;
            this.highlightExecutingBlock(blockIndex, 'executing');
            console.log('✓ 块模式高亮已创建 - 块索引:', blockIndex, '(命令索引:', commandIndex, ')');
            console.log('=== 高亮请求结束（块模式完成）===');
        } else {
            console.warn('高亮条件不满足:', {
                currentMode: this.currentMode,
                hasTextContentEl: !!this.textContentEl,
                hasBlocksContainer: !!this.blocksContainer
            });
        }
    }
    
    highlightErrorLine(tksOriginalLineNumber) {
        console.log('收到错误高亮请求 - TKS原始行号:', tksOriginalLineNumber, '当前模式:', this.currentMode);
        
        // 不清除之前的高亮，直接转换为错误高亮
        
        if (this.currentMode === 'text' && this.textContentEl) {
            // 文本模式：将TKS原始行号转换为显示行号
            const displayLineNumber = this.calculateDisplayLineNumber(tksOriginalLineNumber);
            console.log('文本模式 - 计算错误显示行号:', displayLineNumber);
            if (displayLineNumber > 0) {
                // 先清除执行高亮，然后添加错误高亮
                this.clearExecutionHighlight();
                this.addLineHighlight(displayLineNumber, 'error');
                console.log('文本模式高亮错误行:', displayLineNumber, '(TKS原始行号:', tksOriginalLineNumber, ')');
            } else {
                console.warn('无效的错误显示行号:', displayLineNumber);
            }
        } else if (this.currentMode === 'block') {
            // 块模式：将TKS原始行号转换为命令索引
            if (!this.script.originalLines || !this.script.lineToCommandMap) {
                console.warn('缺少行号映射数据');
                return;
            }
            
            const originalLineIndex = tksOriginalLineNumber - 1;
            if (originalLineIndex >= 0 && originalLineIndex < this.script.lineToCommandMap.length) {
                const commandIndex = this.script.lineToCommandMap[originalLineIndex];
                console.log('块模式 - TKS错误行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);
                
                if (commandIndex !== null) {
                    // 先清除执行高亮，然后添加错误高亮
                    this.clearExecutionHighlight();
                    const blockIndex = commandIndex + 1;
                    this.highlightExecutingBlock(blockIndex, 'error');
                    console.log('块模式高亮错误块:', blockIndex, '(命令索引:', commandIndex, ')');
                } else {
                    console.warn('TKS错误行号不是命令行:', tksOriginalLineNumber);
                }
            } else {
                console.warn('TKS错误行号超出范围:', tksOriginalLineNumber);
            }
        } else {
            console.warn('错误高亮条件不满足:', {
                currentMode: this.currentMode,
                hasTextContentEl: !!this.textContentEl,
                hasBlocksContainer: !!this.blocksContainer
            });
        }
    }
    
    // 计算显示行号：将TKS引擎的原始行号转换为编辑器中显示的行号
    calculateDisplayLineNumber(tksOriginalLineNumber) {
        console.log('计算显示行号 - TKS引擎原始行号:', tksOriginalLineNumber);
        console.log('脚本模型信息:', {
            originalLines: this.script.originalLines ? this.script.originalLines.length : '无',
            commands: this.script.commands.length,
            mapping: this.script.lineToCommandMap ? this.script.lineToCommandMap.length : '无'
        });
        
        if (!this.textContentEl || !this.script.originalLines) {
            console.warn('缺少必要的数据');
            return -1;
        }
        
        // TKS引擎报告的是基于原始文本的行号(1基索引)，需要转换为0基索引
        const originalLineIndex = tksOriginalLineNumber - 1;
        
        // 检查原始行号是否有效
        if (originalLineIndex < 0 || originalLineIndex >= this.script.originalLines.length) {
            console.warn('TKS原始行号超出范围:', tksOriginalLineNumber, '有效范围: 1-' + this.script.originalLines.length);
            return -1;
        }
        
        // 从映射中查找对应的命令索引
        const commandIndex = this.script.lineToCommandMap[originalLineIndex];
        console.log('原始行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);
        
        if (commandIndex === null) {
            console.warn('原始行号不是命令行:', tksOriginalLineNumber);
            return -1;
        }
        
        // 现在需要在显示的文本中找到这个命令对应的行
        const lines = this.textContentEl.textContent.split('\n');
        let stepsStartLine = -1;
        
        // 找到"步骤:"行
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].trim() === '步骤:') {
                stepsStartLine = i + 1;
                break;
            }
        }
        
        if (stepsStartLine === -1) {
            console.warn('未找到步骤行');
            return -1;
        }
        
        // 在步骤区域中找到第N个命令行
        let foundCommandCount = 0;
        for (let i = stepsStartLine; i < lines.length; i++) {
            const line = lines[i].trim();
            if (!line || line.startsWith('#') || 
                line.startsWith('用例:') || line.startsWith('脚本名:') ||
                line === '详情:' || line === '步骤:' ||
                line.includes('appPackage:') || line.includes('appActivity:')) {
                continue;
            }
            
            // 这是一个命令行
            if (foundCommandCount === commandIndex) {
                const displayLine = i + 1;
                console.log('找到显示行号:', displayLine, '(原始行号:', tksOriginalLineNumber, '命令索引:', commandIndex, ')');
                return displayLine;
            }
            foundCommandCount++;
        }
        
        console.warn('未找到对应的显示行号');
        return -1;
    }
    
    clearExecutionHighlight() {
        console.log('清除执行高亮');
        this.currentHighlightedLine = null; // 重置当前高亮行号
        
        // 文本模式：清除行高亮
        if (this.currentMode === 'text') {
            // 从文本编辑器容器和内容元素中移除所有行高亮
            const containers = [this.textContentEl, this.textContentEl?.parentElement, this.editorContainer];
            containers.forEach(container => {
                if (container) {
                    const existingHighlights = container.querySelectorAll('.line-highlight');
                    console.log(`从 ${container.className} 中移除 ${existingHighlights.length} 个高亮元素`);
                    existingHighlights.forEach(highlight => highlight.remove());
                }
            });
            
            // 移除行号高亮
            if (this.lineNumbersEl) {
                const highlightedLineNumbers = this.lineNumbersEl.querySelectorAll('.line-number.highlighted');
                highlightedLineNumbers.forEach(lineNum => {
                    lineNum.classList.remove('highlighted', 'executing', 'error');
                });
                console.log(`移除了 ${highlightedLineNumbers.length} 个行号高亮`);
            }
        }
        
        // 块模式：清除块高亮
        if (this.currentMode === 'block' && this.blocksContainer) {
            const highlightedBlocks = this.blocksContainer.querySelectorAll('.workspace-block.highlighted');
            highlightedBlocks.forEach(block => {
                block.classList.remove('highlighted', 'executing', 'error');
                block.style.boxShadow = '';
                block.style.transform = '';
                block.style.animation = '';
                
                // 移除高亮覆盖层
                const overlays = block.querySelectorAll('.block-highlight-overlay');
                overlays.forEach(overlay => overlay.remove());
            });
            console.log(`移除了 ${highlightedBlocks.length} 个块高亮`);
        }
        
        console.log('已清除所有高亮');
    }
    
    // 块模式高亮功能
    highlightExecutingBlock(commandIndex, type) {
        console.log('块模式高亮请求:', commandIndex, type, '容器存在:', !!this.blocksContainer);
        
        if (!this.blocksContainer) {
            console.warn('blocksContainer不存在');
            return;
        }
        
        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        console.log('找到块数量:', blocks.length, '目标索引:', commandIndex - 1);
        
        if (commandIndex >= 1 && commandIndex <= blocks.length) {
            const targetBlock = blocks[commandIndex - 1]; // 转换为0基索引
            if (targetBlock) {
                // 添加高亮类
                targetBlock.classList.add('highlighted', type);
                
                // 创建高亮覆盖层，而不是直接修改块的样式
                const highlightOverlay = document.createElement('div');
                highlightOverlay.className = `block-highlight-overlay ${type}`;
                highlightOverlay.style.cssText = `
                    position: absolute !important;
                    top: -2px !important;
                    left: -2px !important;
                    right: -2px !important;
                    bottom: -2px !important;
                    border-radius: 10px !important;
                    pointer-events: none !important;
                    z-index: 5 !important;
                    transition: all 0.2s ease !important;
                `;
                
                if (type === 'executing') {
                    highlightOverlay.style.cssText += `
                        border: 3px solid #ffc107 !important;
                        background: rgba(255, 193, 7, 0.1) !important;
                        box-shadow: 0 0 12px rgba(255, 193, 7, 0.4), inset 0 0 12px rgba(255, 193, 7, 0.1) !important;
                    `;
                } else if (type === 'error') {
                    highlightOverlay.style.cssText += `
                        border: 3px solid #dc3545 !important;
                        background: rgba(220, 53, 69, 0.1) !important;
                        box-shadow: 0 0 12px rgba(220, 53, 69, 0.4), inset 0 0 12px rgba(220, 53, 69, 0.1) !important;
                    `;
                }
                
                // 确保目标块有相对定位
                if (!targetBlock.style.position) {
                    targetBlock.style.position = 'relative';
                }
                
                // 添加覆盖层到块中
                targetBlock.appendChild(highlightOverlay);
                
                // 添加脉搏效果
                if (type === 'executing') {
                    targetBlock.style.animation = 'pulse-executing 2s infinite';
                }
                
                // 滚动到可见区域
                setTimeout(() => {
                    targetBlock.scrollIntoView({ behavior: 'smooth', block: 'center' });
                }, 100);
                
                console.log('块模式高亮已添加:', commandIndex, type, '覆盖层已添加到块中');
            } else {
                console.warn('找不到目标块:', commandIndex - 1);
            }
        } else {
            console.warn('无效的块索引:', commandIndex, '有效范围: 1-' + blocks.length);
        }
    }
    
    addLineHighlight(lineNumber, type) {
        console.log('添加行高亮:', lineNumber, type);
        
        // 高亮行号
        if (this.lineNumbersEl) {
            const lineNumbers = this.lineNumbersEl.querySelectorAll('.line-number');
            console.log('行号元素数量:', lineNumbers.length, '目标行号:', lineNumber - 1);
            if (lineNumbers[lineNumber - 1]) {
                lineNumbers[lineNumber - 1].classList.add('highlighted', type);
                console.log('行号高亮已添加');
            } else {
                console.warn('找不到行号元素:', lineNumber - 1);
            }
        } else {
            console.warn('lineNumbersEl不存在');
        }
        
        // 在文本内容上添加高亮背景
        if (!this.textContentEl) {
            console.warn('textContentEl不存在，无法添加高亮');
            return;
        }
        
        // 确保容器有正确的定位
        const textWrapper = this.textContentEl.parentElement;
        if (textWrapper && !textWrapper.style.position) {
            textWrapper.style.position = 'relative';
        }
        
        const lineHeight = 21; // 与CSS中的line-height保持一致
        const topOffset = 16; // 与padding-top保持一致
        
        const highlightDiv = document.createElement('div');
        highlightDiv.className = `line-highlight ${type}`;
        highlightDiv.style.cssText = `
            position: absolute !important;
            left: 0 !important;
            right: 0 !important;
            height: ${lineHeight}px !important;
            top: ${topOffset + (lineNumber - 1) * lineHeight}px !important;
            background: ${type === 'executing' ? 'rgba(255, 193, 7, 0.3)' : 'rgba(220, 53, 69, 0.3)'} !important;
            border-left: 3px solid ${type === 'executing' ? '#ffc107' : '#dc3545'} !important;
            pointer-events: none !important;
            z-index: 10 !important;
            margin: 0 !important;
            padding: 0 !important;
        `;
        
        // 添加到文本编辑器的包装器而不是内容元素
        const targetContainer = textWrapper || this.textContentEl;
        targetContainer.appendChild(highlightDiv);
        
        // 滚动到高亮行
        setTimeout(() => {
            const lineNumberEl = this.lineNumbersEl?.querySelectorAll('.line-number')[lineNumber - 1];
            if (lineNumberEl) {
                lineNumberEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
        }, 100);
        
        console.log('文本高亮已添加到:', targetContainer.className);
        console.log('高亮div样式:', highlightDiv.style.cssText);
        console.log('高亮div实际父元素:', highlightDiv.parentElement);
    }
    
    removeStatusIndicator() {
        // 移除状态指示器（从所有可能的位置）
        if (this.statusIndicatorEl) {
            this.statusIndicatorEl.remove();
        }
        
        // 清理可能存在的所有状态指示器
        const indicators = document.querySelectorAll('.editor-status-indicator');
        indicators.forEach(indicator => indicator.remove());
        
        // 从编辑器视图中移除
        if (this.editorContainer) {
            const textEditorView = this.editorContainer.querySelector('.text-editor-view');
            if (textEditorView) {
                const indicator = textEditorView.querySelector('.editor-status-indicator');
                if (indicator) {
                    indicator.remove();
                }
            }
        }
        
        this.statusIndicatorEl = null;
    }

    destroy() {
        clearTimeout(this.saveTimeout);
        if (this.globalKeyHandler) {
            document.removeEventListener('keydown', this.globalKeyHandler);
        }
        this.removeStatusIndicator();
        this.hideCommandMenu();
        this.hideContextMenu();
        this.listeners = [];
    }
}

// 统一的脚本数据模型
class ScriptModel {
    constructor() {
        this.header = {
            name: '',
            scriptName: '',
            details: {
                appPackage: '',
                appActivity: ''
            }
        };
        this.commands = [];
    }
    
    fromTKSCode(tksCode) {
        this.commands = [];
        this.originalLines = tksCode.split('\n'); // 保留原始行
        this.lineToCommandMap = []; // 行号到命令的映射
        
        let commandIndex = 0;
        
        this.originalLines.forEach((line, lineIndex) => {
            const trimmed = line.trim();
            
            // 跳过头部信息
            if (!trimmed || trimmed.startsWith('#') || 
                trimmed.startsWith('用例:') || trimmed.startsWith('脚本名:') ||
                trimmed === '详情:' || trimmed === '步骤:' ||
                trimmed.includes('appPackage:') || trimmed.includes('appActivity:')) {
                this.lineToCommandMap.push(null); // 非命令行
                return;
            }
            
            const command = this.parseTKSLine(trimmed);
            if (command) {
                this.commands.push(command);
                this.lineToCommandMap.push(commandIndex); // 映射到命令索引
                commandIndex++;
            } else {
                this.lineToCommandMap.push(null); // 无效命令行
            }
        });
        
        console.log('脚本解析完成:', {
            totalLines: this.originalLines.length,
            commands: this.commands.length,
            lineMapping: this.lineToCommandMap
        });
    }
    
    parseTKSLine(line) {
        const match = line.match(/^(\S+)(?:\s+\[(.*?)\])?$/);
        if (!match) return null;
        
        const commandName = match[1];
        const params = match[2] || '';
        
        // 根据命令名称确定类型
        const typeMap = {
            '启动': 'launch',
            '关闭': 'close',
            '点击': 'click',
            '按压': 'press',
            '滑动': 'swipe',
            '输入': 'input',
            '清理': 'clear',
            '隐藏键盘': 'hide_keyboard',
            '等待': 'wait',
            '返回': 'back',
            '断言': 'assert'
        };
        
        const type = typeMap[commandName];
        if (!type) return null;
        
        const command = { type, params: {} };
        
        // 解析参数
        if (params) {
            const paramValues = this.parseParams(params);
            
            // 根据命令类型分配参数
            switch (type) {
                case 'launch':
                    command.params.package = paramValues[0] || '';
                    command.params.activity = paramValues[1] || '';
                    break;
                case 'close':
                    command.params.package = paramValues[0] || '';
                    command.params.activity = paramValues[1] || '';
                    break;
                case 'click':
                case 'clear':
                    command.params.target = paramValues[0] || '';
                    break;
                case 'press':
                    command.params.target = paramValues[0] || '';
                    command.params.duration = paramValues[1] || '1000';
                    break;
                case 'input':
                    command.params.target = paramValues[0] || '';
                    command.params.text = paramValues[1] || '';
                    break;
                case 'wait':
                    command.params.duration = paramValues[0] || '1000ms';
                    break;
                case 'assert':
                    command.params.target = paramValues[0] || '';
                    command.params.condition = paramValues[1] || '存在';
                    break;
            }
        }
        
        return command;
    }
    
    parseParams(paramsStr) {
        if (!paramsStr) return [];
        
        const parts = [];
        let current = '';
        let depth = 0;
        
        for (let i = 0; i < paramsStr.length; i++) {
            const char = paramsStr[i];
            
            if (char === '@' && paramsStr[i + 1] === '{') {
                depth++;
            } else if (char === '}' && depth > 0) {
                depth--;
            }
            
            if (char === ',' && depth === 0) {
                parts.push(current.trim());
                current = '';
            } else {
                current += char;
            }
        }
        
        if (current.trim()) {
            parts.push(current.trim());
        }
        
        return parts;
    }
    
    toTKSCode() {
        if (this.commands.length === 0) {
            return `用例: 新测试用例
脚本名: new_test
详情:
    appPackage: com.example.app
    appActivity: .MainActivity
步骤:
    启动 [com.example.app]
    等待 [1000ms]`;
        }
        
        const commandLines = this.commands.map(command => {
            const commandName = this.getCommandName(command.type);
            const params = this.getCommandParams(command);
            
            if (params.length > 0) {
                return `    ${commandName} [${params.join(', ')}]`;
            } else {
                return `    ${commandName}`;
            }
        }).join('\n');
        
        return `用例: 测试用例
脚本名: test_script
详情:
    appPackage: com.example.app
    appActivity: .MainActivity
步骤:
${commandLines}`;
    }
    
    getCommandName(type) {
        const nameMap = {
            'launch': '启动',
            'close': '关闭',
            'click': '点击',
            'press': '按压',
            'swipe': '滑动',
            'input': '输入',
            'clear': '清理',
            'hide_keyboard': '隐藏键盘',
            'wait': '等待',
            'back': '返回',
            'assert': '断言'
        };
        return nameMap[type] || type;
    }
    
    getCommandParams(command) {
        const params = [];
        
        switch (command.type) {
            case 'launch':
                if (command.params.package) params.push(command.params.package);
                if (command.params.activity) params.push(command.params.activity);
                break;
            case 'close':
                if (command.params.package) params.push(command.params.package);
                if (command.params.activity) params.push(command.params.activity);
                break;
            case 'click':
            case 'clear':
                if (command.params.target) params.push(command.params.target);
                break;
            case 'press':
                if (command.params.target) params.push(command.params.target);
                if (command.params.duration) params.push(command.params.duration);
                break;
            case 'input':
                if (command.params.target) params.push(command.params.target);
                if (command.params.text) params.push(command.params.text);
                break;
            case 'wait':
                if (command.params.duration) params.push(command.params.duration);
                break;
            case 'assert':
                if (command.params.target) params.push(command.params.target);
                if (command.params.condition) params.push(command.params.condition);
                break;
        }
        
        return params;
    }
    
    getCommands() {
        return this.commands;
    }
    
    addCommand(command) {
        this.commands.push(command);
    }
    
    insertCommand(command, index) {
        this.commands.splice(index, 0, command);
    }
    
    reorderCommand(fromIndex, toIndex) {
        if (fromIndex < 0 || fromIndex >= this.commands.length) return;
        if (toIndex < 0 || toIndex >= this.commands.length) return;
        if (fromIndex === toIndex) return;
        
        // 移除原位置的命令
        const [command] = this.commands.splice(fromIndex, 1);
        // 插入到新位置
        this.commands.splice(toIndex, 0, command);
    }
    
    removeCommand(index) {
        console.log(`ScriptModel.removeCommand: 尝试删除索引 ${index}`);
        console.log(`当前commands数组长度: ${this.commands.length}`);
        
        if (index < 0 || index >= this.commands.length) {
            console.error(`索引无效: ${index}, 数组长度: ${this.commands.length}`);
            return;
        }
        
        console.log(`删除的命令: ${JSON.stringify(this.commands[index])}`);
        this.commands.splice(index, 1);
        console.log(`删除后数组长度: ${this.commands.length}`);
    }
    
    clearCommands() {
        this.commands = [];
    }
    
    updateCommandParam(index, param, value) {
        if (this.commands[index]) {
            this.commands[index].params[param] = value;
        }
    }
}

// 导出到全局
window.UnifiedScriptEditor = UnifiedScriptEditor;
