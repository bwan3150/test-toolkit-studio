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
                icon: '▶',
                commands: [
                    { 
                        type: 'launch', 
                        label: '启动', 
                        tksCommand: '启动',
                        params: [
                            { name: 'app', type: 'text', placeholder: '应用包名', default: '' }
                        ]
                    },
                    { 
                        type: 'close', 
                        label: '关闭',
                        tksCommand: '关闭',
                        params: []
                    }
                ]
            },
            action: {
                color: '#569cd6',
                icon: '👆',
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
                icon: '✓',
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
        this.currentMode = 'text';
        this.render();
    }
    
    switchToBlockMode() {
        console.log('切换到块编程模式');
        this.currentMode = 'block';
        this.render();
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
                    <p>点击下方按钮开始添加命令块</p>
                </div>
                ${finalInsertButton}
            `;
        } else {
            this.blocksContainer.innerHTML = blocksHtml + finalInsertButton;
        }
    }
    
    setupTextModeListeners() {
        if (!this.textContentEl) return;
        
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
    }
    
    setupBlockModeListeners() {
        // 点击事件
        this.container.addEventListener('click', (e) => {
            if (e.target.classList.contains('block-delete')) {
                const index = parseInt(e.target.dataset.index);
                this.removeCommand(index);
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
            const block = e.target.closest('.workspace-block.command-block');
            if (block) {
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
            if (!e.target.closest('.command-menu') && !e.target.closest('.block-insert-btn')) {
                this.hideCommandMenu();
            }
            if (!e.target.closest('.context-menu')) {
                this.hideContextMenu();
            }
        });
    }
    
    // 显示命令选择菜单
    showCommandMenu(insertArea, insertIndex) {
        if (this.isTestRunning) return;
        
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
        
        const menuHtml = `
            <div class="command-menu" id="commandMenu">
                ${menuItems.join('')}
            </div>
        `;
        
        // 插入菜单到插入区域
        insertArea.insertAdjacentHTML('beforeend', menuHtml);
        this.currentMenu = insertArea.querySelector('.command-menu');
        
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
        
        // 绑定菜单项点击事件
        this.currentContextMenu.addEventListener('click', (e) => {
            const menuItem = e.target.closest('.context-menu-item');
            if (menuItem) {
                const action = menuItem.dataset.action;
                const index = parseInt(menuItem.dataset.index);
                
                if (action === 'insert-below') {
                    // 显示命令选择菜单在指定块下方
                    this.showInsertMenuAtBlock(index + 1);
                }
                
                this.hideContextMenu();
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
        const blocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        let targetBlock = null;
        
        if (insertIndex > 0 && insertIndex - 1 < blocks.length) {
            targetBlock = blocks[insertIndex - 1];
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
        
        // 插入临时区域
        if (targetBlock) {
            targetBlock.insertAdjacentElement('afterend', tempInsertArea);
        } else {
            this.blocksContainer.insertBefore(tempInsertArea, this.blocksContainer.firstChild);
        }
        
        // 立即显示菜单
        this.showCommandMenu(tempInsertArea, insertIndex);
        
        // 菜单关闭时移除临时区域
        const originalHideMenu = this.hideCommandMenu.bind(this);
        this.hideCommandMenu = () => {
            originalHideMenu();
            if (tempInsertArea.parentNode) {
                tempInsertArea.remove();
            }
            // 恢复原始方法
            this.hideCommandMenu = originalHideMenu;
        };
    }
    
    removeCommand(index) {
        this.script.removeCommand(index);
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
            control: '流程控制',
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
        
        // 只有在切换到不同行时才清除之前的高亮
        if (this.currentHighlightedLine !== tksOriginalLineNumber) {
            console.log('✓ 切换到新行，清除之前的高亮');
            console.log('从行', this.currentHighlightedLine, '切换到行', tksOriginalLineNumber);
            this.clearExecutionHighlight();
            this.currentHighlightedLine = tksOriginalLineNumber;
        } else {
            console.log('✓ 同一行重复高亮请求，检查高亮是否仍然存在');
            
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
                console.warn('✗ 无效的显示行号:', displayLineNumber);
                console.log('=== 高亮请求结束（文本模式失败）===');
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
                console.log('块模式 - TKS行号', tksOriginalLineNumber, '映射到命令索引:', commandIndex);
                
                if (commandIndex !== null) {
                    // 命令索引转换为1基索引进行高亮
                    const blockIndex = commandIndex + 1;
                    this.highlightExecutingBlock(blockIndex, 'executing');
                    console.log('✓ 块模式高亮已创建 - 块索引:', blockIndex, '(命令索引:', commandIndex, ')');
                    console.log('=== 高亮请求结束（块模式完成）===');
                } else {
                    console.warn('✗ TKS行号不是命令行:', tksOriginalLineNumber);
                    console.log('=== 高亮请求结束（块模式失败）===');
                }
            } else {
                console.warn('TKS行号超出范围:', tksOriginalLineNumber);
            }
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
                    command.params.app = paramValues[0] || '';
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
                if (command.params.app) params.push(command.params.app);
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
        this.commands.splice(index, 1);
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