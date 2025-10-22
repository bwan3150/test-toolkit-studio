// 块输入处理器 - 负责块模式下的用户交互事件

const BlockInputHandler = {
    /**
     * 设置块模式的事件监听器
     */
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
                window.rLog(`删除命令块，索引: ${index}, 当前命令数量: ${this.getCommands().length}`);

                // 验证索引有效性
                if (index >= 0 && index < this.getCommands().length) {
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
            // 清除所有拖拽高亮和插入提示
            this.blocksContainer.querySelectorAll('.drag-over').forEach(el => {
                el.classList.remove('drag-over');
            });
            this.clearDragInsertIndicator();
        });

        this.container.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = 'move';

            // 清除之前的插入提示
            this.clearDragInsertIndicator();

            // 使用统一的位置计算方法
            const insertInfo = this.calculateNearestInsertPosition(e.clientY);

            if (insertInfo && insertInfo.block) {
                this.showDragInsertIndicator(insertInfo.block, insertInfo.position);
            } else if (insertInfo && insertInfo.position === 'container-start') {
                this.showDragInsertIndicator(null, 'container-start');
            }
        });

        this.container.addEventListener('drop', (e) => {
            e.preventDefault();
            if (this.isTestRunning) return;

            const data = JSON.parse(e.dataTransfer.getData('text/plain'));
            if (data.type === 'reorder') {
                // 使用与dragover相同的逻辑计算最近的插入位置
                const insertInfo = this.calculateNearestInsertPosition(e.clientY, data.fromIndex);

                if (insertInfo) {
                    window.rLog(`执行重排: 从索引 ${data.fromIndex} 移动到索引 ${insertInfo.insertIndex}`);
                    this.reorderCommand(data.fromIndex, insertInfo.insertIndex);
                } else {
                    window.rLog('未找到有效的插入位置');
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
};

// 导出到全局
window.BlockInputHandler = BlockInputHandler;

if (window.rLog) {
    window.rLog('✅ BlockInputHandler 模块已加载');
}
