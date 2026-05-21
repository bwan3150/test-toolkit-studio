// Modal 管理器组件
// 用于控制现有 modal 和动态创建新的 modal

const ModalManager = {
  /**
   * 显示已存在的 modal（通过 ID）
   * @param {string} modalId - modal 元素的 ID
   */
  show(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
      modal.style.display = 'flex';
    } else {
      window.rError(`Modal ${modalId} 未找到`);
    }
  },

  /**
   * 隐藏已存在的 modal（通过 ID）
   * @param {string} modalId - modal 元素的 ID
   */
  hide(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
      modal.style.display = 'none';
    }
  },

  /**
   * 显示简单的提示对话框（阻塞式）
   * @param {Object} options - 配置选项
   * @param {string} options.title - 标题（默认"提示"）
   * @param {string} options.message - 消息内容
   * @param {string} options.confirmText - 确认按钮文字（默认"确定"）
   * @param {Function} options.onConfirm - 确认后的回调函数
   * @param {boolean} options.blockPage - 是否阻塞整个页面（默认 true）
   */
  showAlert(options) {
    const {
      title = '提示',
      message = '',
      confirmText = '确定',
      onConfirm = null,
      blockPage = true
    } = options;

    // 移除已存在的动态 modal
    this._removeDynamicModal();

    // 创建 modal 结构（复用现有样式）
    const modal = document.createElement('div');
    modal.className = 'modal modal-dynamic';
    modal.style.display = 'flex';
    if (blockPage) {
      modal.style.zIndex = '10000';
    }

    modal.innerHTML = `
      <div class="modal-content">
        ${title ? `<div class="modal-header"><h3>${title}</h3></div>` : ''}
        <div class="modal-body" style="${!title ? 'padding-top: 24px;' : ''}">
          <p style="margin: 0; line-height: 1.6;">${message}</p>
        </div>
        <div class="modal-footer">
          <button class="btn btn-primary modal-confirm-btn">${confirmText}</button>
        </div>
      </div>
    `;

    document.body.appendChild(modal);

    // 绑定确认按钮
    const confirmBtn = modal.querySelector('.modal-confirm-btn');
    if (confirmBtn) {
      confirmBtn.addEventListener('click', () => {
        this._removeDynamicModal();
        if (onConfirm && typeof onConfirm === 'function') {
          onConfirm();
        }
      });

      // 自动聚焦到确认按钮
      setTimeout(() => confirmBtn.focus(), 100);
    }

    // 如果是阻塞模式，点击背景不关闭，添加抖动效果
    if (blockPage) {
      modal.addEventListener('click', (e) => {
        if (e.target === modal) {
          const content = modal.querySelector('.modal-content');
          content.classList.add('modal-shake');
          setTimeout(() => {
            content.classList.remove('modal-shake');
          }, 300);
        }
      });
    } else {
      // 非阻塞模式，点击背景可以关闭
      modal.addEventListener('click', (e) => {
        if (e.target === modal) {
          this._removeDynamicModal();
        }
      });
    }

    // 支持 ESC 键关闭（仅非阻塞模式）
    if (!blockPage) {
      const escHandler = (e) => {
        if (e.key === 'Escape') {
          this._removeDynamicModal();
          document.removeEventListener('keydown', escHandler);
        }
      };
      document.addEventListener('keydown', escHandler);
    }
  },

  /**
   * 显示确认对话框
   * @param {Object} options - 配置选项
   * @param {string} options.title - 标题（默认"确认"）
   * @param {string} options.message - 消息内容
   * @param {string} options.confirmText - 确认按钮文字（默认"确定"）
   * @param {string} options.cancelText - 取消按钮文字（默认"取消"）
   * @param {Function} options.onConfirm - 确认后的回调函数
   * @param {Function} options.onCancel - 取消后的回调函数
   * @param {boolean} options.blockPage - 是否阻塞整个页面（默认 false）
   */
  showConfirm(options) {
    const {
      title = '确认',
      message = '',
      confirmText = '确定',
      cancelText = '取消',
      onConfirm = null,
      onCancel = null,
      blockPage = false
    } = options;

    // 移除已存在的动态 modal
    this._removeDynamicModal();

    // 创建 modal 结构
    const modal = document.createElement('div');
    modal.className = 'modal modal-dynamic';
    modal.style.display = 'flex';
    if (blockPage) {
      modal.style.zIndex = '10000';
    }

    modal.innerHTML = `
      <div class="modal-content">
        ${title ? `<div class="modal-header"><h3>${title}</h3></div>` : ''}
        <div class="modal-body" style="${!title ? 'padding-top: 24px;' : ''}">
          <p style="margin: 0; line-height: 1.6;">${message}</p>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary modal-cancel-btn">${cancelText}</button>
          <button class="btn btn-primary modal-confirm-btn">${confirmText}</button>
        </div>
      </div>
    `;

    document.body.appendChild(modal);

    // 绑定确认按钮
    const confirmBtn = modal.querySelector('.modal-confirm-btn');
    if (confirmBtn) {
      confirmBtn.addEventListener('click', () => {
        this._removeDynamicModal();
        if (onConfirm && typeof onConfirm === 'function') {
          onConfirm();
        }
      });
    }

    // 绑定取消按钮
    const cancelBtn = modal.querySelector('.modal-cancel-btn');
    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => {
        this._removeDynamicModal();
        if (onCancel && typeof onCancel === 'function') {
          onCancel();
        }
      });

      // 自动聚焦到取消按钮
      setTimeout(() => cancelBtn.focus(), 100);
    }

    // 点击背景关闭
    modal.addEventListener('click', (e) => {
      if (e.target === modal) {
        this._removeDynamicModal();
        if (onCancel && typeof onCancel === 'function') {
          onCancel();
        }
      }
    });

    // 支持 ESC 键关闭
    const escHandler = (e) => {
      if (e.key === 'Escape') {
        this._removeDynamicModal();
        if (onCancel && typeof onCancel === 'function') {
          onCancel();
        }
        document.removeEventListener('keydown', escHandler);
      }
    };
    document.addEventListener('keydown', escHandler);
  },

  /**
   * 移除动态创建的 modal
   */
  _removeDynamicModal() {
    const modal = document.querySelector('.modal-dynamic');
    if (modal) {
      modal.remove();
    }
  },

  /**
   * 关闭所有动态 modal
   */
  closeAll() {
    this._removeDynamicModal();
  }
};

// 添加抖动动画样式（如果还没有的话）
if (!document.getElementById('modal-manager-styles')) {
  const style = document.createElement('style');
  style.id = 'modal-manager-styles';
  style.textContent = `
    @keyframes modal-shake {
      0%, 100% { transform: translateX(0); }
      25% { transform: translateX(-8px); }
      75% { transform: translateX(8px); }
    }

    .modal-shake {
      animation: modal-shake 0.3s ease-out;
    }

    /* 确保动态 modal 在最上层 */
    .modal-dynamic {
      z-index: 10000;
    }
  `;
  document.head.appendChild(style);
}

// 导出模块
window.ModalManager = ModalManager;
window.rLog('✅ ModalManager 组件已加载');
