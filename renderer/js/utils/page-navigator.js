// 页面导航工具模块
// 提供统一的页面切换和导航管理功能
//
// 用法示例：
//   PageNavigator.navigateTo('device');
//   PageNavigator.getCurrentPage();
//   PageNavigator.registerPageHook('testcase', () => { ... });

(function() {
  'use strict';

  // 页面配置映射
  const PAGE_CONFIG = {
    'project': {
      id: 'projectPage',
      label: 'Project',
      navSelector: '[data-page="project"]'
    },
    'testcase': {
      id: 'testcasePage',
      label: 'Testcase',
      navSelector: '[data-page="testcase"]'
    },
    'device': {
      id: 'devicePage',
      label: 'Device',
      navSelector: '[data-page="device"]'
    },
    'log': {
      id: 'logPage',
      label: 'Log Viewer',
      navSelector: '[data-page="log"]'
    },
    'report': {
      id: 'insightsPage',
      label: 'Insights',
      navSelector: '[data-page="report"]'
    },
    'settings': {
      id: 'settingsPage',
      label: 'Settings',
      navSelector: '[data-page="settings"]'
    }
  };

  // 当前激活的页面
  let currentPage = 'project';

  // 页面钩子存储 - 页面激活时执行的回调函数
  const pageHooks = {};

  /**
   * 初始化页面导航器
   * 设置侧边栏导航项的点击事件
   */
  function initialize() {
    const navItems = document.querySelectorAll('.nav-item');

    navItems.forEach(item => {
      item.addEventListener('click', () => {
        const targetPage = item.dataset.page;
        navigateTo(targetPage);
      });
    });

    // 确定初始页面（检查哪个页面有 active 类）
    const activeNav = document.querySelector('.nav-item.active');
    if (activeNav) {
      currentPage = activeNav.dataset.page;
    }

    if (window.rLog) {
      window.rLog('✅ PageNavigator 已初始化，当前页面:', currentPage);
    }
  }

  /**
   * 导航到指定页面
   * @param {string} pageName - 页面名称（如 'device', 'testcase' 等）
   * @param {boolean} silent - 是否静默模式（不触发钩子和日志）
   * @returns {boolean} - 导航是否成功
   */
  function navigateTo(pageName, silent = false) {
    // 验证页面是否存在
    const pageConfig = PAGE_CONFIG[pageName];
    if (!pageConfig) {
      if (window.rError && !silent) {
        window.rError(`❌ 无效的页面名称: ${pageName}`);
      }
      return false;
    }

    // 如果已经在目标页面，直接返回
    if (currentPage === pageName && !silent) {
      if (window.rLog) {
        window.rLog(`ℹ️  已在页面 ${pageName}，无需切换`);
      }
      return true;
    }

    const previousPage = currentPage;

    if (window.rLog && !silent) {
      window.rLog(`🔄 页面切换: ${previousPage} → ${pageName}`);
    }

    // 更新激活的导航项
    const navItems = document.querySelectorAll('.nav-item');
    navItems.forEach(nav => nav.classList.remove('active'));

    const targetNav = document.querySelector(pageConfig.navSelector);
    if (targetNav) {
      targetNav.classList.add('active');
    }

    // 更新激活的页面
    const pages = document.querySelectorAll('.page');
    pages.forEach(page => page.classList.remove('active'));

    const targetPageElement = document.getElementById(pageConfig.id);
    if (targetPageElement) {
      targetPageElement.classList.add('active');
      currentPage = pageName;
    } else {
      if (window.rError && !silent) {
        window.rError(`❌ 页面元素未找到: ${pageConfig.id}`);
      }
      return false;
    }

    // 触发页面激活钩子
    if (!silent) {
      triggerPageHooks(pageName);
      triggerBuiltInActions(pageName);
    }

    return true;
  }

  /**
   * 触发内置的页面激活动作
   * @param {string} pageName - 页面名称
   */
  function triggerBuiltInActions(pageName) {
    switch (pageName) {
      case 'device':
        if (window.DeviceManagerModule && window.DeviceManagerModule.refreshConnectedDevices) {
          window.DeviceManagerModule.refreshConnectedDevices();
        }
        break;

      case 'log':
        if (window.LogManagerModule && window.LogManagerModule.refreshDeviceList) {
          window.LogManagerModule.refreshDeviceList();
        }
        break;

      case 'settings':
        if (window.SettingsModule && window.SettingsModule.checkSDKStatus) {
          window.SettingsModule.checkSDKStatus();
        }
        break;

      case 'report':
        if (window.TestReportModule && window.TestReportModule.onPageActivated) {
          window.TestReportModule.onPageActivated();
        }
        break;

      case 'testcase':
        // 确保底部面板正确初始化和显示
        if (window.TestcaseController && window.TestcaseController.initializeBottomPanelDisplay) {
          setTimeout(() => {
            window.TestcaseController.initializeBottomPanelDisplay();
          }, 100);
        }
        break;
    }
  }

  /**
   * 触发注册的页面钩子
   * @param {string} pageName - 页面名称
   */
  function triggerPageHooks(pageName) {
    if (pageHooks[pageName] && Array.isArray(pageHooks[pageName])) {
      pageHooks[pageName].forEach(callback => {
        try {
          callback(pageName);
        } catch (error) {
          if (window.rError) {
            window.rError(`❌ 页面钩子执行失败 (${pageName}):`, error);
          }
        }
      });
    }
  }

  /**
   * 注册页面激活钩子
   * @param {string} pageName - 页面名称
   * @param {Function} callback - 回调函数，接收 pageName 作为参数
   */
  function registerPageHook(pageName, callback) {
    if (typeof callback !== 'function') {
      if (window.rError) {
        window.rError('❌ registerPageHook: callback 必须是函数');
      }
      return;
    }

    if (!PAGE_CONFIG[pageName]) {
      if (window.rError) {
        window.rError(`❌ registerPageHook: 无效的页面名称: ${pageName}`);
      }
      return;
    }

    if (!pageHooks[pageName]) {
      pageHooks[pageName] = [];
    }

    pageHooks[pageName].push(callback);

    if (window.rLog) {
      window.rLog(`✅ 已注册页面钩子: ${pageName}`);
    }
  }

  /**
   * 取消注册页面钩子
   * @param {string} pageName - 页面名称
   * @param {Function} callback - 要移除的回调函数
   */
  function unregisterPageHook(pageName, callback) {
    if (!pageHooks[pageName]) {
      return;
    }

    const index = pageHooks[pageName].indexOf(callback);
    if (index > -1) {
      pageHooks[pageName].splice(index, 1);
      if (window.rLog) {
        window.rLog(`✅ 已取消页面钩子: ${pageName}`);
      }
    }
  }

  /**
   * 获取当前激活的页面名称
   * @returns {string} - 当前页面名称
   */
  function getCurrentPage() {
    return currentPage;
  }

  /**
   * 获取当前激活的页面元素
   * @returns {HTMLElement|null} - 页面DOM元素
   */
  function getCurrentPageElement() {
    const pageConfig = PAGE_CONFIG[currentPage];
    if (!pageConfig) {
      return null;
    }
    return document.getElementById(pageConfig.id);
  }

  /**
   * 检查指定页面是否为当前激活页面
   * @param {string} pageName - 页面名称
   * @returns {boolean}
   */
  function isCurrentPage(pageName) {
    return currentPage === pageName;
  }

  /**
   * 获取所有可用的页面列表
   * @returns {Array} - 页面信息数组
   */
  function getAllPages() {
    return Object.keys(PAGE_CONFIG).map(pageName => ({
      name: pageName,
      label: PAGE_CONFIG[pageName].label,
      id: PAGE_CONFIG[pageName].id
    }));
  }

  /**
   * 检查页面是否存在
   * @param {string} pageName - 页面名称
   * @returns {boolean}
   */
  function pageExists(pageName) {
    return !!PAGE_CONFIG[pageName];
  }

  // 导出 PageNavigator 模块
  window.PageNavigator = {
    initialize,
    navigateTo,
    getCurrentPage,
    getCurrentPageElement,
    isCurrentPage,
    registerPageHook,
    unregisterPageHook,
    getAllPages,
    pageExists
  };

  // 不自动初始化，等待 app.js 所有模块加载完成后手动调用

})();
