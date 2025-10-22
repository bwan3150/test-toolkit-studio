/**
 * ContentEditable 光标位置管理
 * 用于在重新渲染 HTML 后恢复光标位置
 */

const EditorCursor = {
  /**
   * 保存光标位置（基于文本偏移量）
   * @param {HTMLElement} element - contenteditable 元素
   * @returns {number} 光标位置（字符偏移量）
   */
  saveCursorPosition(element) {
    const selection = window.getSelection();
    if (!selection.rangeCount) return 0;

    const range = selection.getRangeAt(0);
    const preCaretRange = range.cloneRange();
    preCaretRange.selectNodeContents(element);
    preCaretRange.setEnd(range.endContainer, range.endOffset);

    return preCaretRange.toString().length;
  },

  /**
   * 恢复光标位置
   * @param {HTMLElement} element - contenteditable 元素
   * @param {number} cursorPosition - 光标位置（字符偏移量）
   */
  restoreCursorPosition(element, cursorPosition) {
    const selection = window.getSelection();
    const range = document.createRange();

    let charCount = 0;
    let nodeStack = [element];
    let node;
    let foundStart = false;

    while ((node = nodeStack.pop())) {
      if (node.nodeType === Node.TEXT_NODE) {
        const nextCharCount = charCount + node.length;
        if (!foundStart && cursorPosition >= charCount && cursorPosition <= nextCharCount) {
          range.setStart(node, cursorPosition - charCount);
          range.setEnd(node, cursorPosition - charCount);
          foundStart = true;
          break;
        }
        charCount = nextCharCount;
      } else {
        // 从后往前压栈，保证从前往后遍历
        for (let i = node.childNodes.length - 1; i >= 0; i--) {
          nodeStack.push(node.childNodes[i]);
        }
      }
    }

    if (foundStart) {
      selection.removeAllRanges();
      selection.addRange(range);
    }
  },

  /**
   * 获取纯文本内容（保留换行）
   * @param {HTMLElement} element - contenteditable 元素
   * @returns {string} 纯文本
   */
  getPlainText(element) {
    // 使用 innerText 保留换行符
    return element.innerText || '';
  }
};

// 导出到全局
if (typeof window !== 'undefined') {
  window.EditorCursor = EditorCursor;
}

// 记录加载
if (window.rLog) {
  window.rLog('✅ EditorCursor 模块已加载');
}
