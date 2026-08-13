/**
 * 块号控制器
 * 负责在块模式左侧显示块号、hover播放按钮、单块执行功能
 * @version 1.0.0
 */
(window.rLog || console.log)('block-number-controller.js 开始加载');

class BlockNumberController {
    constructor(blocksContainer, blockEditor) {
        this.blocksContainer = blocksContainer; // 块容器
        this.blockEditor = blockEditor; // BlockModeEditor 实例
        this.blockNumbersContainer = null;
        this.wrapper = null;

        window.rLog('BlockNumberController 创建');
        this.init();
    }

    /**
     * 初始化块号显示
     */
    init() {
        // 获取块容器的父元素
        const parentContainer = this.blocksContainer.parentElement;

        // 创建包裹器
        const wrapper = document.createElement('div');
        wrapper.className = 'block-editor-wrapper';

        // 创建块号容器
        this.blockNumbersContainer = document.createElement('div');
        this.blockNumbersContainer.className = 'block-numbers';

        // 将块容器从父元素移除
        parentContainer.removeChild(this.blocksContainer);

        // 按顺序添加: 块号 -> 块容器
        wrapper.appendChild(this.blockNumbersContainer);
        wrapper.appendChild(this.blocksContainer);

        // 将包裹器添加到父容器
        parentContainer.appendChild(wrapper);

        // 保存引用
        this.wrapper = wrapper;

        // 初始渲染
        this.updateBlockNumbers();

        // 设置滚动同步
        this.setupScrollSync();

        // 监听块容器变化
        this.setupBlocksObserver();

        // 监听窗口resize
        this.resizeObserver = new ResizeObserver(() => {
            this.syncBlockHeights();
        });
        this.resizeObserver.observe(this.blocksContainer);

        window.rLog('块号控制器初始化完成');
    }

    /**
     * 设置块容器变化监听
     */
    setupBlocksObserver() {
        const observer = new MutationObserver(() => {
            this.updateBlockNumbers();
        });

        observer.observe(this.blocksContainer, {
            childList: true,
            subtree: true,
            attributes: true,
            attributeFilter: ['data-index']
        });

        this.observer = observer;
    }

    /**
     * 设置滚动同步
     */
    setupScrollSync() {
        // 块容器滚动时,同步块号容器的滚动
        this.blocksContainer.addEventListener('scroll', () => {
            this.blockNumbersContainer.scrollTop = this.blocksContainer.scrollTop;
        });

        // 块号容器滚动时,同步块容器的滚动
        this.blockNumbersContainer.addEventListener('scroll', () => {
            this.blocksContainer.scrollTop = this.blockNumbersContainer.scrollTop;
        });
    }

    /**
     * 更新块号显示
     */
    updateBlockNumbers() {
        // 获取所有命令块
        const commandBlocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');

        // 清空块号容器
        this.blockNumbersContainer.innerHTML = '';

        // 为每个块创建块号元素
        commandBlocks.forEach((block, index) => {
            const blockNumberEl = this.createBlockNumberElement(index + 1, block);
            this.blockNumbersContainer.appendChild(blockNumberEl);
        });

        // 在DOM渲染后同步高度
        requestAnimationFrame(() => {
            this.syncBlockHeights();
        });

        window.rLog(`更新块号显示: ${commandBlocks.length} 个块`);
    }

    /**
     * 同步块号和块的高度
     */
    syncBlockHeights() {
        const commandBlocks = this.blocksContainer.querySelectorAll('.workspace-block.command-block');
        const blockNumbers = this.blockNumbersContainer.querySelectorAll('.block-number');

        commandBlocks.forEach((block, index) => {
            if (blockNumbers[index]) {
                // 获取块的实际高度(包括margin)
                const blockHeight = block.offsetHeight;
                const blockStyle = window.getComputedStyle(block);
                const marginBottom = parseFloat(blockStyle.marginBottom) || 0;

                // 设置块号的高度和margin
                blockNumbers[index].style.height = `${blockHeight}px`;
                blockNumbers[index].style.marginBottom = `${marginBottom}px`;
            }
        });
    }

    /**
     * 创建单个块号元素
     * @param {number} blockNumber - 块号（1-based）
     * @param {HTMLElement} blockElement - 对应的块元素
     */
    createBlockNumberElement(blockNumber, blockElement) {
        const blockIndex = parseInt(blockElement.dataset.index);

        const blockNumberEl = document.createElement('div');
        blockNumberEl.className = 'block-number';
        blockNumberEl.dataset.blockIndex = blockIndex;

        // 创建数字显示
        const numberSpan = document.createElement('span');
        numberSpan.className = 'block-number-text';
        numberSpan.textContent = blockNumber;
        blockNumberEl.appendChild(numberSpan);

        // 创建播放按钮
        const playButton = document.createElement('span');
        playButton.className = 'block-play-button';
        playButton.innerHTML = '▶';
        playButton.title = `执行第 ${blockNumber} 个块`;
        blockNumberEl.appendChild(playButton);

        // 点击事件
        blockNumberEl.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            this.executeBlockAt(blockIndex);
        });

        return blockNumberEl;
    }

    /**
     * 执行指定块
     * @param {number} blockIndex - 块索引（0-based）
     */
    executeBlockAt(blockIndex) {
        window.rLog(`执行块索引: ${blockIndex}`);

        // 计算行号 (用于高亮)
        const lineNumber = this.blockIndexToLineNumber(blockIndex);
        if (lineNumber === -1) {
            window.rError('无法计算行号');
            return;
        }

        // 从textEditor读取对应行的内容
        const textEditor = this.blockEditor.textEditor;
        if (!textEditor) {
            window.rError('textEditor 不存在');
            return;
        }

        const content = textEditor.getContent();
        const lines = content.split('\n');

        // lineNumber是1-based，转换为0-based索引
        const lineIndex = lineNumber - 1;
        if (lineIndex < 0 || lineIndex >= lines.length) {
            window.rError('行号超出范围:', lineNumber);
            return;
        }

        const commandLine = lines[lineIndex].trim();
        if (!commandLine) {
            window.rError('该行为空');
            return;
        }

        window.rLog(`执行块 ${blockIndex}: ${commandLine}, 对应行号: ${lineNumber}`);

        // 调用单行执行器
        if (window.SingleLineRunner) {
            window.SingleLineRunner.executeLine(lineNumber, commandLine);
        } else {
            window.rError('SingleLineRunner 未加载');
            window.AppNotifications?.error('单块执行功能不可用');
        }
    }

    /**
     * 将块索引转换为行号
     * @param {number} blockIndex - 块索引（0-based）
     * @returns {number} 行号（1-based）
     */
    blockIndexToLineNumber(blockIndex) {
        // 找到"步骤:"行
        const headerLines = this.blockEditor.headerLines || [];
        let stepsLineNumber = -1;

        for (let i = 0; i < headerLines.length; i++) {
            if (headerLines[i].trim() === '步骤:') {
                stepsLineNumber = i + 1; // 1-based
                break;
            }
        }

        if (stepsLineNumber === -1) {
            window.rLog('未找到"步骤:"行');
            return -1;
        }

        // 命令行从"步骤:"的下一行开始
        const lineNumber = stepsLineNumber + 1 + blockIndex;

        return lineNumber;
    }

    /**
     * 销毁控制器
     */
    destroy() {
        if (this.observer) {
            this.observer.disconnect();
        }
        if (this.resizeObserver) {
            this.resizeObserver.disconnect();
        }
        if (this.wrapper) {
            // 恢复原始结构
            const parent = this.wrapper.parentElement;
            if (parent) {
                this.wrapper.removeChild(this.blocksContainer);
                parent.removeChild(this.wrapper);
                parent.appendChild(this.blocksContainer);
            }
        }
        window.rLog('块号控制器已销毁');
    }
}

// 导出到全局
window.BlockNumberController = BlockNumberController;
(window.rLog || console.log)('✅ BlockNumberController 模块已加载');
