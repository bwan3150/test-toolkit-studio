// Context Menu Module - 右键菜单模块
// 负责右键菜单的显示、隐藏和操作处理

window.ContextMenuManager = {
  menuElement: null,
  currentSelection: null,
  actionHandlers: {},

  /**
   * 初始化右键菜单
   * @param {HTMLElement} menuElement - 菜单元素
   */
  init(menuElement) {
    this.menuElement = menuElement;

    // 点击菜单项时触发对应操作
    this.menuElement.addEventListener('click', (e) => {
      if (e.target.classList.contains('context-menu-item')) {
        const action = e.target.dataset.action;
        this.handleAction(action);
        this.hide();
      }
    });

    // 点击页面其他地方关闭菜单
    document.addEventListener('click', (e) => {
      if (!this.menuElement.contains(e.target)) {
        this.hide();
      }
    });
  },

  /**
   * 显示右键菜单
   * @param {number} x - X坐标
   * @param {number} y - Y坐标
   * @param {Object} selection - 当前选择的项目信息
   */
  show(x, y, selection) {
    this.currentSelection = selection;
    this.menuElement.style.display = 'block';
    this.menuElement.style.left = x + 'px';
    this.menuElement.style.top = y + 'px';
  },

  /**
   * 隐藏右键菜单
   */
  hide() {
    this.menuElement.style.display = 'none';
    this.currentSelection = null;
  },

  /**
   * 注册操作处理器
   * @param {string} action - 操作名称
   * @param {Function} handler - 处理函数
   */
  registerHandler(action, handler) {
    this.actionHandlers[action] = handler;
  },

  /**
   * 处理菜单操作
   * @param {string} action - 操作名称
   */
  handleAction(action) {
    const handler = this.actionHandlers[action];
    if (handler && typeof handler === 'function') {
      handler(this.currentSelection);
    } else {
      window.rWarn(`未找到操作处理器: ${action}`);
    }
  },

  /**
   * 获取当前选择
   * @returns {Object} 当前选择的项目信息
   */
  getCurrentSelection() {
    return this.currentSelection;
  }
};
