// Release Notes 弹窗管理模块
// 处理版本更新日志的查看和导航

(function() {
  'use strict';

  const { ipcRenderer } = window.AppGlobals;

  // DOM 元素
  let releaseNotesModal = null;
  let releaseNotesLoading = null;
  let releaseNotesError = null;
  let releaseNotesErrorMsg = null;
  let releaseNotesContent = null;
  let currentReleaseVersion = null;
  let currentReleaseDate = null;
  let currentVersionBadge = null;
  let releaseMarkdownContent = null;
  let versionIndicator = null;
  let prevVersionBtn = null;
  let nextVersionBtn = null;
  let closeReleaseNotesBtn = null;
  let closeReleaseNotesModal = null;

  // 状态
  let versionsList = [];
  let currentVersionIndex = 0;
  let appVersion = '';

  /**
   * 初始化 Release Notes 弹窗
   */
  function initReleaseNotesModal() {
    // 等待 DOM 加载完成
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', setupReleaseNotesModal);
    } else {
      setupReleaseNotesModal();
    }
  }

  /**
   * 设置 Release Notes 弹窗
   */
  function setupReleaseNotesModal() {
    // 获取 DOM 元素
    releaseNotesModal = document.getElementById('releaseNotesModal');
    releaseNotesLoading = document.getElementById('releaseNotesLoading');
    releaseNotesError = document.getElementById('releaseNotesError');
    releaseNotesErrorMsg = document.getElementById('releaseNotesErrorMsg');
    releaseNotesContent = document.getElementById('releaseNotesContent');
    currentReleaseVersion = document.getElementById('currentReleaseVersion');
    currentReleaseDate = document.getElementById('currentReleaseDate');
    currentVersionBadge = document.getElementById('currentVersionBadge');
    releaseMarkdownContent = document.getElementById('releaseMarkdownContent');
    versionIndicator = document.getElementById('versionIndicator');
    prevVersionBtn = document.getElementById('prevVersionBtn');
    nextVersionBtn = document.getElementById('nextVersionBtn');
    closeReleaseNotesBtn = document.getElementById('closeReleaseNotesBtn');
    closeReleaseNotesModal = document.getElementById('closeReleaseNotesModal');

    if (!releaseNotesModal) {
      console.error('Release Notes modal 元素未找到');
      return;
    }

    // 绑定事件
    bindEvents();

    // 获取当前应用版本
    getAppVersion();

    if (window.rLog) {
      window.rLog('Release Notes modal 已初始化');
    }
  }

  /**
   * 绑定事件
   */
  function bindEvents() {
    // 关闭按钮
    if (closeReleaseNotesBtn) {
      closeReleaseNotesBtn.addEventListener('click', hideModal);
    }

    if (closeReleaseNotesModal) {
      closeReleaseNotesModal.addEventListener('click', hideModal);
    }

    // 版本导航按钮
    if (prevVersionBtn) {
      prevVersionBtn.addEventListener('click', showPreviousVersion);
    }

    if (nextVersionBtn) {
      nextVersionBtn.addEventListener('click', showNextVersion);
    }

    // 点击 modal 外部关闭
    if (releaseNotesModal) {
      releaseNotesModal.addEventListener('click', (e) => {
        if (e.target === releaseNotesModal) {
          hideModal();
        }
      });
    }

    // "View Release Notes" 按钮
    const viewReleaseNotesBtn = document.getElementById('viewReleaseNotesBtn');
    if (viewReleaseNotesBtn) {
      viewReleaseNotesBtn.addEventListener('click', showModal);
    }
  }

  /**
   * 获取当前应用版本
   */
  async function getAppVersion() {
    try {
      const version = await ipcRenderer.invoke('get-app-version');
      appVersion = version;
      if (window.rLog) {
        window.rLog(`当前应用版本: ${appVersion}`);
      }
    } catch (error) {
      if (window.rError) {
        window.rError('获取应用版本失败:', error);
      }
    }
  }

  /**
   * 显示 modal
   */
  async function showModal() {
    if (!releaseNotesModal) return;

    // 显示 modal
    releaseNotesModal.style.display = 'flex';

    // 显示加载状态
    showLoading();

    try {
      // 获取版本索引
      const indexResult = await ipcRenderer.invoke('get-release-notes-index');

      if (!indexResult.success || !indexResult.data || !indexResult.data.versions) {
        throw new Error(indexResult.error || 'Failed to load version index');
      }

      versionsList = indexResult.data.versions;

      if (versionsList.length === 0) {
        throw new Error('No versions available');
      }

      // 找到当前应用版本的索引
      currentVersionIndex = versionsList.findIndex(v => v.version === appVersion);

      // 如果找不到当前版本，默认显示最新版本
      if (currentVersionIndex === -1) {
        currentVersionIndex = 0;
      }

      // 加载当前版本的 release notes
      await loadReleaseNote(currentVersionIndex);

    } catch (error) {
      showError(error.message);
      if (window.rError) {
        window.rError('加载 Release Notes 失败:', error);
      }
    }
  }

  /**
   * 隐藏 modal
   */
  function hideModal() {
    if (releaseNotesModal) {
      releaseNotesModal.style.display = 'none';
    }
  }

  /**
   * 显示加载状态
   */
  function showLoading() {
    if (releaseNotesLoading) releaseNotesLoading.style.display = 'block';
    if (releaseNotesError) releaseNotesError.style.display = 'none';
    if (releaseNotesContent) releaseNotesContent.style.display = 'none';
  }

  /**
   * 显示错误信息
   */
  function showError(message) {
    if (releaseNotesLoading) releaseNotesLoading.style.display = 'none';
    if (releaseNotesError) releaseNotesError.style.display = 'block';
    if (releaseNotesContent) releaseNotesContent.style.display = 'none';
    if (releaseNotesErrorMsg) releaseNotesErrorMsg.textContent = message;
  }

  /**
   * 显示内容
   */
  function showContent() {
    if (releaseNotesLoading) releaseNotesLoading.style.display = 'none';
    if (releaseNotesError) releaseNotesError.style.display = 'none';
    if (releaseNotesContent) releaseNotesContent.style.display = 'block';
  }

  /**
   * 加载指定版本的 Release Note
   */
  async function loadReleaseNote(index) {
    try {
      const versionInfo = versionsList[index];
      const result = await ipcRenderer.invoke('get-release-note', versionInfo.version);

      if (!result.success) {
        throw new Error(result.error || 'Failed to load release note');
      }

      // 更新版本信息
      if (currentReleaseVersion) {
        currentReleaseVersion.textContent = versionInfo.version;
      }

      if (currentReleaseDate) {
        const date = new Date(versionInfo.date);
        currentReleaseDate.textContent = date.toLocaleDateString('en-US', {
          year: 'numeric',
          month: 'long',
          day: 'numeric'
        });
      }

      // 显示/隐藏 "Current" 标签
      if (currentVersionBadge) {
        if (versionInfo.version === appVersion) {
          currentVersionBadge.style.display = 'block';
        } else {
          currentVersionBadge.style.display = 'none';
        }
      }

      // 渲染 Markdown 内容
      if (releaseMarkdownContent) {
        // 使用 marked 库渲染 Markdown
        if (typeof marked !== 'undefined') {
          releaseMarkdownContent.innerHTML = marked.parse(result.data);
        } else {
          // 如果没有 marked 库，直接显示原始文本
          releaseMarkdownContent.innerHTML = `<pre>${escapeHtml(result.data)}</pre>`;
        }
      }

      // 更新版本指示器
      updateVersionIndicator();

      // 更新按钮状态
      updateNavigationButtons();

      // 显示内容
      showContent();

    } catch (error) {
      showError(error.message);
      if (window.rError) {
        window.rError('加载 Release Note 失败:', error);
      }
    }
  }

  /**
   * 更新版本指示器
   * 最新版本显示为 n/n，最旧版本显示为 1/n
   */
  function updateVersionIndicator() {
    if (versionIndicator) {
      const displayIndex = versionsList.length - currentVersionIndex;
      versionIndicator.textContent = `${displayIndex} / ${versionsList.length}`;
    }
  }

  /**
   * 更新导航按钮状态
   */
  function updateNavigationButtons() {
    if (prevVersionBtn) {
      prevVersionBtn.disabled = currentVersionIndex >= versionsList.length - 1;
    }

    if (nextVersionBtn) {
      nextVersionBtn.disabled = currentVersionIndex <= 0;
    }
  }

  /**
   * 显示上一个版本（更旧的版本）
   */
  async function showPreviousVersion() {
    if (currentVersionIndex < versionsList.length - 1) {
      currentVersionIndex++;
      showLoading();
      await loadReleaseNote(currentVersionIndex);
    }
  }

  /**
   * 显示下一个版本（更新的版本）
   */
  async function showNextVersion() {
    if (currentVersionIndex > 0) {
      currentVersionIndex--;
      showLoading();
      await loadReleaseNote(currentVersionIndex);
    }
  }

  /**
   * 转义 HTML
   */
  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // 初始化
  initReleaseNotesModal();

})();
