/**
 * 行号控制器
 * 负责显示行号、hover播放按钮、单行执行功能
 * @version 1.0.0
 */
(window.rLog || console.log)('line-number-controller.js 开始加载');

class LineNumberController {
    constructor(editorElement, onLineExecute) {
        this.editorElement = editorElement; // CodeJar 的 contenteditable div
        this.onLineExecute = onLineExecute; // 单行执行回调函数
        this.lineNumbersContainer = null;
        this.currentLineCount = 0;

        window.rLog('LineNumberController 创建');
        this.init();
    }

    /**
     * 初始化行号显示
     */
    init() {
        // 获取编辑器的父容器
        const parentContainer = this.editorElement.parentElement;

        // 创建一个包裹器,用于容纳行号和编辑器
        const wrapper = document.createElement('div');
        wrapper.className = 'text-editor-wrapper';

        // 创建行号容器
        this.lineNumbersContainer = document.createElement('div');
        this.lineNumbersContainer.className = 'line-numbers';

        // 将编辑器从父容器中移除
        parentContainer.removeChild(this.editorElement);

        // 按顺序添加: 行号 -> 编辑器
        wrapper.appendChild(this.lineNumbersContainer);
        wrapper.appendChild(this.editorElement);

        // 将包裹器添加到父容器
        parentContainer.appendChild(wrapper);

        // 保存包裹器引用
        this.wrapper = wrapper;

        // 初始渲染
        this.updateLineNumbers();

        // 监听编辑器内容变化
        this.setupContentObserver();

        // 设置滚动同步
        this.setupScrollSync();

        window.rLog('行号控制器初始化完成');
    }

    /**
     * 设置内容变化监听
     */
    setupContentObserver() {
        // 使用 MutationObserver 监听编辑器内容变化
        const observer = new MutationObserver(() => {
            this.updateLineNumbers();
        });

        observer.observe(this.editorElement, {
            childList: true,
            subtree: true,
            characterData: true
        });

        this.observer = observer;
    }

    /**
     * 设置滚动同步
     */
    setupScrollSync() {
        // 编辑器滚动时,同步行号容器的滚动
        this.editorElement.addEventListener('scroll', () => {
            this.lineNumbersContainer.scrollTop = this.editorElement.scrollTop;
        });

        // 行号容器滚动时,同步编辑器的滚动
        this.lineNumbersContainer.addEventListener('scroll', () => {
            this.editorElement.scrollTop = this.lineNumbersContainer.scrollTop;
        });
    }

    /**
     * 更新行号显示
     */
    updateLineNumbers() {
        const content = this.editorElement.textContent || '';
        const lines = content.split('\n');
        const lineCount = lines.length;

        // 如果行数没有变化，不重新渲染
        if (lineCount === this.currentLineCount) {
            return;
        }

        this.currentLineCount = lineCount;

        // 清空行号容器
        this.lineNumbersContainer.innerHTML = '';

        // 创建每一行的行号元素
        for (let i = 1; i <= lineCount; i++) {
            const lineNumberEl = this.createLineNumberElement(i, lines[i - 1]);
            this.lineNumbersContainer.appendChild(lineNumberEl);
        }

        window.rLog(`更新行号显示: ${lineCount} 行`);
    }

    /**
     * 创建单个行号元素
     * @param {number} lineNumber - 行号（1-based）
     * @param {string} lineContent - 该行内容
     */
    createLineNumberElement(lineNumber, lineContent) {
        const lineNumberEl = document.createElement('div');
        lineNumberEl.className = 'line-number';
        lineNumberEl.dataset.lineNumber = lineNumber;

        // 判断该行是否是可执行的命令行
        const isExecutable = this.isExecutableLine(lineNumber, lineContent);

        if (isExecutable) {
            lineNumberEl.classList.add('executable');
        }

        // 创建数字显示
        const numberSpan = document.createElement('span');
        numberSpan.className = 'line-number-text';
        numberSpan.textContent = lineNumber;
        lineNumberEl.appendChild(numberSpan);

        // 如果是可执行行，添加播放按钮
        if (isExecutable) {
            const playButton = document.createElement('span');
            playButton.className = 'line-play-button';
            playButton.innerHTML = '▶';
            playButton.title = `执行第 ${lineNumber} 行`;
            lineNumberEl.appendChild(playButton);

            // 点击事件
            lineNumberEl.addEventListener('click', (e) => {
                e.preventDefault();
                e.stopPropagation();
                this.executeLineAt(lineNumber, lineContent);
            });
        }

        return lineNumberEl;
    }

    /**
     * 判断某行是否是可执行的命令行
     * @param {number} lineNumber - 行号
     * @param {string} lineContent - 行内容
     */
    isExecutableLine(lineNumber, lineContent) {
        const trimmed = lineContent.trim();

        // 空行、注释行不可执行
        if (!trimmed || trimmed.startsWith('#')) {
            return false;
        }

        // 元数据行不可执行
        if (trimmed.startsWith('用例:') ||
            trimmed.startsWith('脚本名:') ||
            trimmed.startsWith('详情:') ||
            trimmed === '步骤:') {
            return false;
        }

        // 其他行都认为是可执行的命令行
        return true;
    }

    /**
     * 执行指定行
     * @param {number} lineNumber - 行号
     * @param {string} lineContent - 行内容
     */
    executeLineAt(lineNumber, lineContent) {
        window.rLog(`🎯 执行第 ${lineNumber} 行: ${lineContent}`);

        // 调用回调函数
        if (this.onLineExecute) {
            this.onLineExecute(lineNumber, lineContent);
        }
    }

    /**
     * 高亮指定行号
     * @param {number} lineNumber - 行号
     * @param {string} type - 类型 ('executing' 或 'error')
     */
    highlightLine(lineNumber, type) {
        // 清除之前的高亮
        this.clearHighlight();

        // 高亮指定行
        const lineNumberEl = this.lineNumbersContainer.querySelector(
            `.line-number[data-line-number="${lineNumber}"]`
        );

        if (lineNumberEl) {
            lineNumberEl.classList.add('highlighted', type);
        }
    }

    /**
     * 清除所有高亮
     */
    clearHighlight() {
        const highlighted = this.lineNumbersContainer.querySelectorAll('.line-number.highlighted');
        highlighted.forEach(el => {
            el.classList.remove('highlighted', 'executing', 'error');
        });
    }

    /**
     * 销毁控制器
     */
    destroy() {
        if (this.observer) {
            this.observer.disconnect();
        }
        if (this.lineNumbersContainer) {
            this.lineNumbersContainer.remove();
        }
        window.rLog('行号控制器已销毁');
    }
}

// 导出到全局
window.LineNumberController = LineNumberController;
(window.rLog || console.log)('✅ LineNumberController 模块已加载');
