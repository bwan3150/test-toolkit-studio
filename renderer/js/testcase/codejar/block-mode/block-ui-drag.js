// 块UI拖拽功能 - 负责块的拖拽重排

const BlockUIDrag = {
    /**
     * 显示拖拽插入提示横杠
     * @param {HTMLElement} block - 目标块元素
     * @param {string} position - 位置 ('before', 'after', 'container-start')
     */
    showDragInsertIndicator(block, position) {
        // 清除之前的提示
        this.clearDragInsertIndicator();

        const indicator = document.createElement('div');
        indicator.className = 'drag-insert-indicator';
        indicator.id = 'drag-insert-indicator';

        const containerRect = this.blocksContainer.getBoundingClientRect();
        let top;

        if (position === 'before' && block) {
            // 在块上方显示 - 计算与前一个块的中间位置
            const blockRect = block.getBoundingClientRect();
            const allBlocks = Array.from(this.blocksContainer.querySelectorAll('.workspace-block.command-block:not(.dragging)'));
            const blockIndex = allBlocks.indexOf(block);

            if (blockIndex > 0) {
                // 有前一个块，显示在两块中间
                const prevBlock = allBlocks[blockIndex - 1];
                const prevRect = prevBlock.getBoundingClientRect();
                top = (prevRect.bottom + blockRect.top) / 2 - containerRect.top;
            } else {
                // 第一个块，显示在块上方
                top = blockRect.top - containerRect.top - 8;
            }
        } else if (position === 'after' && block) {
            // 在块下方显示 - 计算与下一个块的中间位置
            const blockRect = block.getBoundingClientRect();
            const allBlocks = Array.from(this.blocksContainer.querySelectorAll('.workspace-block.command-block:not(.dragging)'));
            const blockIndex = allBlocks.indexOf(block);

            if (blockIndex < allBlocks.length - 1) {
                // 有下一个块，显示在两块中间
                const nextBlock = allBlocks[blockIndex + 1];
                const nextRect = nextBlock.getBoundingClientRect();
                top = (blockRect.bottom + nextRect.top) / 2 - containerRect.top;
            } else {
                // 最后一个块，显示在块下方
                top = blockRect.bottom - containerRect.top + 8;
            }
        } else if (position === 'container-start') {
            // 在容器顶部显示
            top = 8;
        }

        indicator.style.top = `${top}px`;
        this.blocksContainer.appendChild(indicator);

        window.rLog(`显示拖拽插入提示 - 位置: ${position}, top: ${top}px`);
    },

    /**
     * 清除拖拽插入提示横杠
     */
    clearDragInsertIndicator() {
        const indicator = this.blocksContainer.querySelector('#drag-insert-indicator');
        if (indicator) {
            indicator.remove();
        }
    },

    /**
     * 计算鼠标位置最近的插入位置（供dragover和drop使用）
     * @param {number} mouseY - 鼠标Y坐标
     * @param {number} draggingFromIndex - 正在拖拽的块的原始索引（-1表示不是重排）
     * @returns {Object} 包含insertIndex, block, position的对象
     */
    calculateNearestInsertPosition(mouseY, draggingFromIndex = -1) {
        // 获取所有非拖拽中的块
        const allBlocks = Array.from(this.blocksContainer.querySelectorAll('.workspace-block.command-block:not(.dragging)'));

        if (allBlocks.length === 0) {
            return { insertIndex: 0, block: null, position: 'container-start' };
        }

        // 找到鼠标位置最近的插入位置
        let closestDistance = Infinity;
        let closestBlock = null;
        let closestPosition = 'after';
        let closestInsertIndex = 0;

        // 检查每个块的上方和下方
        allBlocks.forEach((block, index) => {
            const rect = block.getBoundingClientRect();
            const blockIndex = parseInt(block.dataset.index);

            // 检查块上方的插入位置（除了第一个块）
            if (index > 0) {
                const prevBlock = allBlocks[index - 1];
                const prevRect = prevBlock.getBoundingClientRect();
                const insertY = (prevRect.bottom + rect.top) / 2; // 两块之间的中点
                const distance = Math.abs(mouseY - insertY);

                if (distance < closestDistance) {
                    closestDistance = distance;
                    closestBlock = block;
                    closestPosition = 'before';
                    closestInsertIndex = blockIndex;
                }
            }

            // 检查块下方的插入位置
            const insertY = index === allBlocks.length - 1 ?
                rect.bottom + 8 : // 最后一个块下方
                (rect.bottom + allBlocks[index + 1].getBoundingClientRect().top) / 2; // 与下一块的中点

            const distance = Math.abs(mouseY - insertY);
            if (distance < closestDistance) {
                closestDistance = distance;
                closestBlock = block;
                closestPosition = 'after';
                closestInsertIndex = blockIndex + 1;
            }
        });

        // 检查第一个块上方的插入位置
        if (allBlocks.length > 0) {
            const firstBlock = allBlocks[0];
            const firstRect = firstBlock.getBoundingClientRect();
            const insertY = firstRect.top - 8;
            const distance = Math.abs(mouseY - insertY);

            if (distance < closestDistance) {
                closestDistance = distance;
                closestBlock = firstBlock;
                closestPosition = 'before';
                closestInsertIndex = parseInt(firstBlock.dataset.index);
            }
        }

        // 如果拖拽的元素原本在插入位置之前，需要调整插入索引
        if (draggingFromIndex !== -1 && draggingFromIndex < closestInsertIndex) {
            closestInsertIndex--;
        }

        return {
            insertIndex: closestInsertIndex,
            block: closestBlock,
            position: closestPosition
        };
    }
};

// 导出到全局
window.BlockUIDrag = BlockUIDrag;

if (window.rLog) {
    window.rLog('✅ BlockUIDrag 模块已加载');
}
