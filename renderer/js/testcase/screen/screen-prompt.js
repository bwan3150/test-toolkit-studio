// 屏幕提示模块
// 负责在屏幕中央显示提示按钮（连接设备、获取屏幕信息等）

const ScreenPrompt = {
  /**
   * 显示"请先连接设备"提示
   */
  showConnectDevicePrompt() {
    this._removeExistingPrompt();

    const screenContent = document.getElementById('screenContent');
    if (!screenContent) return;

    const promptDiv = document.createElement('div');
    promptDiv.className = 'screen-prompt';
    promptDiv.innerHTML = `
      <button class="screen-prompt-btn" id="connectDevicePromptBtn">
        <svg viewBox="0 0 24 24" width="16" height="16" style="margin-right: 6px;">
          <path fill="currentColor" d="M17 1.01L7 1c-1.1 0-2 .9-2 2v18c0 1.1.9 2 2 2h10c1.1 0 2-.9 2-2V3c0-1.1-.9-1.99-2-1.99zM17 19H7V5h10v14z"/>
        </svg>
        请先连接设备
      </button>
    `;

    screenContent.appendChild(promptDiv);

    // 绑定点击事件 - 跳转到设备页面
    const btn = document.getElementById('connectDevicePromptBtn');
    if (btn) {
      btn.addEventListener('click', () => {
        if (window.PageNavigator) {
          window.PageNavigator.navigateTo('device');
        }
      });
    }
  },

  /**
   * 显示"点击获取屏幕信息"提示
   */
  showCaptureScreenPrompt() {
    this._removeExistingPrompt();

    const screenContent = document.getElementById('screenContent');
    if (!screenContent) return;

    const promptDiv = document.createElement('div');
    promptDiv.className = 'screen-prompt';
    promptDiv.innerHTML = `
      <button class="screen-prompt-btn" id="captureScreenPromptBtn">
        <svg viewBox="0 0 24 24" width="16" height="16" style="margin-right: 6px;">
          <path fill="currentColor" d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
        </svg>
        点击获取屏幕信息
      </button>
    `;

    screenContent.appendChild(promptDiv);

    // 绑定点击事件 - 调用 tke-controller-capture
    const btn = document.getElementById('captureScreenPromptBtn');
    if (btn) {
      btn.addEventListener('click', async () => {
        await this._captureScreenAndUnlock();
      });
    }
  },

  /**
   * 移除提示
   */
  removePrompt() {
    this._removeExistingPrompt();
  },

  /**
   * 将提示按钮设置为 loading 状态
   */
  setButtonLoading() {
    const btn = document.getElementById('captureScreenPromptBtn');
    if (btn) {
      btn.disabled = true;
      btn.style.border = 'none';
      btn.style.background = 'transparent';
      btn.innerHTML = `
        <svg viewBox="0 0 24 24" width="16" height="16" style="margin-right: 6px; animation: spin 1s linear infinite;">
          <path fill="currentColor" d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74C4.46 8.97 4 10.43 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z"/>
        </svg>
        获取中...
      `;
    }
  },

  /**
   * 恢复提示按钮到原始状态
   */
  resetButton() {
    const btn = document.getElementById('captureScreenPromptBtn');
    if (btn) {
      btn.disabled = false;
      btn.style.border = '';
      btn.style.background = '';
      btn.innerHTML = `
        <svg viewBox="0 0 24 24" width="16" height="16" style="margin-right: 6px;">
          <path fill="currentColor" d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
        </svg>
        点击获取屏幕信息
      `;
    }
  },

  /**
   * 移除已存在的提示
   */
  _removeExistingPrompt() {
    const existingPrompt = document.querySelector('.screen-prompt');
    if (existingPrompt) {
      existingPrompt.remove();
    }
  },

  /**
   * 获取屏幕信息并解锁滑块
   */
  async _captureScreenAndUnlock() {
    const deviceSelect = document.getElementById('deviceSelect');
    const projectPath = window.AppGlobals?.currentProject;

    if (!deviceSelect?.value) {
      window.AppNotifications?.deviceRequired();
      return;
    }

    if (!projectPath) {
      window.AppNotifications?.projectRequired();
      return;
    }

    // 显示加载状态
    this.setButtonLoading();

    try {
      // 调用 tke-controller-capture 获取截图和 XML
      const { ipcRenderer } = window.AppGlobals;
      const result = await ipcRenderer.invoke('tke-controller-capture', deviceSelect.value, projectPath);

      if (!result.success) {
        throw new Error(result.error || '获取屏幕信息失败');
      }

      // 解析结果
      let captureData;
      try {
        captureData = JSON.parse(result.output);
      } catch (e) {
        throw new Error('解析返回数据失败');
      }

      if (!captureData.success || !captureData.screenshot) {
        throw new Error('未获取到截图');
      }

      // 显示截图
      const img = document.getElementById('deviceScreenshot');
      if (img) {
        await new Promise((resolve) => {
          img.onload = () => {
            img.style.display = 'block';
            resolve();
          };
          img.src = `file://${captureData.screenshot}?t=${Date.now()}`;
        });
      }

      // 更新设备信息和UI结构
      if (window.UIExtractor && window.UIExtractor.updateDeviceInfoAndGetUIStructure) {
        await window.UIExtractor.updateDeviceInfoAndGetUIStructure(captureData.xml);
      }

      // 移除提示
      this.removePrompt();

      // 解锁滑块
      if (window.ModeSlider) {
        window.ModeSlider.unlockSlider();
      }

      window.AppNotifications?.success('屏幕信息获取成功');

    } catch (error) {
      window.rError('获取屏幕信息失败:', error);
      window.AppNotifications?.error(`获取失败: ${error.message}`);

      // 恢复按钮
      this.resetButton();
    }
  }
};

// 添加样式
const style = document.createElement('style');
style.textContent = `
  .screen-prompt {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
  }

  .screen-prompt-btn {
    display: flex;
    align-items: center;
    padding: 10px 20px;
    background: rgba(30, 30, 30, 0.9);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
    backdrop-filter: blur(10px);
    white-space: nowrap;
  }

  .screen-prompt-btn:hover:not(:disabled) {
    background: rgba(40, 40, 40, 0.95);
    border-color: rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.9);
    transform: translateY(-1px);
  }

  .screen-prompt-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
`;
document.head.appendChild(style);

// 导出模块
window.ScreenPrompt = ScreenPrompt;
