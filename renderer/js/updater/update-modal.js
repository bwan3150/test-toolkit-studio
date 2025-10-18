// 更新弹窗管理模块
// 处理应用更新提示和安装

(function() {
  'use strict';

  const { ipcRenderer } = window.AppGlobals;

  // DOM 元素
  let updateModal = null;
  let updateVersion = null;
  let updateNotesGroup = null;
  let updateNotesContent = null;
  let updateNowBtn = null;
  let updateLaterBtn = null;
  let closeUpdateModal = null;

  // 当前更新信息
  let currentUpdateInfo = null;

  /**
   * 初始化更新弹窗
   */
  function initUpdateModal() {
    // 等待 DOM 加载完成
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', setupUpdateModal);
    } else {
      setupUpdateModal();
    }
  }

  /**
   * 设置更新弹窗
   */
  function setupUpdateModal() {
    // 获取 DOM 元素
    updateModal = document.getElementById('updateModal');
    updateVersion = document.getElementById('updateVersion');
    updateNotesGroup = document.getElementById('updateNotesGroup');
    updateNotesContent = document.getElementById('updateNotesContent');
    updateNowBtn = document.getElementById('updateNowBtn');
    updateLaterBtn = document.getElementById('updateLaterBtn');
    closeUpdateModal = document.getElementById('closeUpdateModal');

    if (!updateModal) {
      console.error('更新弹窗元素未找到');
      return;
    }

    // 绑定事件
    bindEvents();

    // 监听 main 进程的更新通知
    ipcRenderer.on('update-ready', handleUpdateReady);

    // 监听更新状态
    ipcRenderer.on('update-status', handleUpdateStatus);

    if (window.rLog) {
      window.rLog('✅ 更新弹窗已初始化');
    }
  }

  /**
   * 绑定事件
   */
  function bindEvents() {
    // 立即更新按钮
    if (updateNowBtn) {
      updateNowBtn.addEventListener('click', handleInstallNow);
    }

    // 稍后更新按钮
    if (updateLaterBtn) {
      updateLaterBtn.addEventListener('click', hideUpdateModal);
    }

    // 关闭按钮
    if (closeUpdateModal) {
      closeUpdateModal.addEventListener('click', hideUpdateModal);
    }

    // 不允许点击遮罩层关闭（重要更新提示）
  }

  /**
   * 处理更新就绪事件
   */
  function handleUpdateReady(event, updateInfo) {
    if (window.rLog) {
      window.rLog('📦 收到更新通知:', updateInfo);
    }

    currentUpdateInfo = updateInfo;

    // 更新版本号
    if (updateVersion) {
      updateVersion.textContent = updateInfo.version || 'Unknown';
    }

    // 更新发布说明
    if (updateNotesContent) {
      if (updateInfo.releaseNotes) {
        updateNotesContent.innerHTML = formatReleaseNotes(updateInfo.releaseNotes);
      } else {
        // 没有更新说明时显示默认文本
        updateNotesContent.innerHTML = '<p style="color: var(--text-secondary); font-style: italic;">暂无版本更新描述</p>';
      }
      if (updateNotesGroup) {
        updateNotesGroup.style.display = 'block';
      }
    }

    // 显示弹窗
    showUpdateModal();
  }

  /**
   * 处理更新状态事件
   */
  function handleUpdateStatus(event, statusData) {
    const { event: updateEvent, data } = statusData;

    if (window.rLog) {
      window.rLog(`📡 更新状态: ${updateEvent}`, data);
    }

    // 可以在这里添加下载进度显示等功能
    switch (updateEvent) {
      case 'checking-for-update':
        // 检查更新中
        break;
      case 'update-available':
        // 发现新版本
        break;
      case 'download-progress':
        // 下载进度（可以显示进度条）
        break;
      case 'update-downloaded':
        // 下载完成（由 handleUpdateReady 处理）
        break;
      case 'update-not-available':
        // 已是最新版本
        break;
      case 'update-error':
        // 更新错误
        if (window.rError) {
          window.rError('更新失败:', data);
        }
        break;
    }
  }

  /**
   * 格式化发布说明
   */
  function formatReleaseNotes(notes) {
    if (typeof notes === 'string') {
      // 简单的 Markdown 格式化
      return notes
        .replace(/\n/g, '<br>')
        .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.*?)\*/g, '<em>$1</em>');
    }
    return notes;
  }

  /**
   * 显示更新弹窗
   */
  function showUpdateModal() {
    if (updateModal) {
      updateModal.style.display = 'flex';
      updateModal.classList.add('active');
    }
  }

  /**
   * 隐藏更新弹窗
   */
  function hideUpdateModal() {
    if (updateModal) {
      updateModal.style.display = 'none';
      updateModal.classList.remove('active');
    }
  }

  /**
   * 处理立即安装
   */
  async function handleInstallNow() {
    if (window.rLog) {
      window.rLog('⚡ 用户确认安装更新，即将重启应用...');
    }

    // 禁用所有按钮和关闭操作
    if (updateNowBtn) {
      updateNowBtn.disabled = true;
      updateNowBtn.innerHTML = '<span class="spinner"></span> Installing...';
    }
    if (updateLaterBtn) {
      updateLaterBtn.disabled = true;
    }
    if (closeUpdateModal) {
      closeUpdateModal.disabled = true;
      closeUpdateModal.style.display = 'none';
    }

    try {
      // 调用 main 进程安装更新
      const result = await ipcRenderer.invoke('install-update-now');

      if (!result.success) {
        if (window.rError) {
          window.rError('安装更新失败:', result.error);
        }

        // 恢复按钮
        if (updateNowBtn) {
          updateNowBtn.disabled = false;
          updateNowBtn.textContent = 'Install and Restart';
        }
        if (updateLaterBtn) {
          updateLaterBtn.disabled = false;
        }
        if (closeUpdateModal) {
          closeUpdateModal.disabled = false;
          closeUpdateModal.style.display = 'block';
        }
      }
      // 如果成功，应用会自动重启，不需要处理
    } catch (error) {
      if (window.rError) {
        window.rError('安装更新时发生错误:', error);
      }

      // 恢复按钮
      if (updateNowBtn) {
        updateNowBtn.disabled = false;
        updateNowBtn.textContent = 'Install and Restart';
      }
      if (updateLaterBtn) {
        updateLaterBtn.disabled = false;
      }
      if (closeUpdateModal) {
        closeUpdateModal.disabled = false;
        closeUpdateModal.style.display = 'block';
      }
    }
  }

  // 导出模块
  window.UpdateModalModule = {
    init: initUpdateModal,
    show: showUpdateModal,
    hide: hideUpdateModal
  };

  // 自动初始化
  initUpdateModal();
})();
