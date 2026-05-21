// 页面状态管理器
// 统一管理页面激活、数据加载、缓存和刷新逻辑
//
// 用法示例：
//   PageStateManager.registerPage('device', {
//     onActivate: async () => { await loadDeviceData(); },
//     onRefresh: async () => { await refreshDeviceData(); },
//     ttl: 60000  // 60秒缓存
//   });

(function() {
  'use strict';

  // 页面状态枚举
  const PageState = {
    INACTIVE: 'inactive',      // 未激活
    ACTIVATING: 'activating',  // 正在激活（首次加载）
    ACTIVE: 'active',          // 已激活
    REFRESHING: 'refreshing'   // 正在刷新
  };

  // 页面配置存储
  const pageConfigs = new Map();

  // 页面状态存储
  const pageStates = new Map();

  // 数据缓存存储
  const dataCache = new Map();

  // 最后更新时间戳
  const lastUpdated = new Map();

  /**
   * 注册页面配置
   * @param {string} pageName - 页面名称（如 'device', 'testcase'）
   * @param {Object} config - 页面配置
   * @param {Function} config.onActivate - 页面激活时的回调函数（async）
   * @param {Function} config.onRefresh - 页面刷新时的回调函数（async，可选）
   * @param {number} config.ttl - 数据缓存时间（毫秒），默认60秒
   * @param {Function} config.showLoading - 显示loading的函数（可选）
   * @param {Function} config.hideLoading - 隐藏loading的函数（可选）
   * @param {boolean} config.alwaysRefresh - 是否总是刷新（忽略缓存），默认false
   */
  function registerPage(pageName, config) {
    if (!pageName || typeof pageName !== 'string') {
      if (window.rError) {
        window.rError('❌ PageStateManager.registerPage: 无效的页面名称');
      }
      return;
    }

    if (!config || typeof config.onActivate !== 'function') {
      if (window.rError) {
        window.rError(`❌ PageStateManager.registerPage: ${pageName} 必须提供 onActivate 回调函数`);
      }
      return;
    }

    // 设置默认值
    const fullConfig = {
      onActivate: config.onActivate,
      onRefresh: config.onRefresh || config.onActivate, // 默认使用onActivate
      ttl: config.ttl !== undefined ? config.ttl : 60000, // 默认60秒
      showLoading: config.showLoading || null,
      hideLoading: config.hideLoading || null,
      alwaysRefresh: config.alwaysRefresh || false
    };

    pageConfigs.set(pageName, fullConfig);
    pageStates.set(pageName, PageState.INACTIVE);

    if (window.rLog) {
      window.rLog(`✅ PageStateManager: 已注册页面 ${pageName}`, {
        ttl: fullConfig.ttl,
        alwaysRefresh: fullConfig.alwaysRefresh
      });
    }
  }

  /**
   * 激活页面
   * @param {string} pageName - 页面名称
   * @param {boolean} forceRefresh - 是否强制刷新（忽略缓存）
   * @returns {Promise<boolean>} - 是否成功
   */
  async function activatePage(pageName, forceRefresh = false) {
    const config = pageConfigs.get(pageName);
    if (!config) {
      // 页面未注册，可能是不需要数据加载的页面（如project）
      if (window.rLog) {
        window.rLog(`ℹ️  PageStateManager: 页面 ${pageName} 未注册，跳过数据加载`);
      }
      return true;
    }

    const currentState = pageStates.get(pageName);
    const lastUpdate = lastUpdated.get(pageName);
    const now = Date.now();

    // 如果正在激活或刷新，跳过
    if (currentState === PageState.ACTIVATING || currentState === PageState.REFRESHING) {
      if (window.rLog) {
        window.rLog(`⏳ PageStateManager: 页面 ${pageName} 正在加载中，跳过`);
      }
      return false;
    }

    // 检查是否需要刷新数据
    const needsRefresh = forceRefresh ||
                         config.alwaysRefresh ||
                         !lastUpdate ||
                         (now - lastUpdate) > config.ttl;

    if (!needsRefresh) {
      if (window.rLog) {
        const age = Math.floor((now - lastUpdate) / 1000);
        window.rLog(`✅ PageStateManager: 页面 ${pageName} 使用缓存数据（${age}秒前）`);
      }
      return true;
    }

    // 开始加载数据
    try {
      const isFirstActivation = currentState === PageState.INACTIVE;
      pageStates.set(pageName, isFirstActivation ? PageState.ACTIVATING : PageState.REFRESHING);

      if (window.rLog) {
        window.rLog(`🔄 PageStateManager: 开始${isFirstActivation ? '激活' : '刷新'}页面 ${pageName}`);
      }

      // 显示loading
      if (config.showLoading && isFirstActivation) {
        try {
          config.showLoading();
        } catch (error) {
          if (window.rError) {
            window.rError(`❌ showLoading 执行失败 (${pageName}):`, error);
          }
        }
      }

      // 执行回调
      const callback = isFirstActivation ? config.onActivate : config.onRefresh;
      await callback();

      // 更新状态和时间戳
      pageStates.set(pageName, PageState.ACTIVE);
      lastUpdated.set(pageName, Date.now());

      // 隐藏loading
      if (config.hideLoading && isFirstActivation) {
        try {
          config.hideLoading();
        } catch (error) {
          if (window.rError) {
            window.rError(`❌ hideLoading 执行失败 (${pageName}):`, error);
          }
        }
      }

      if (window.rLog) {
        window.rLog(`✅ PageStateManager: 页面 ${pageName} ${isFirstActivation ? '激活' : '刷新'}完成`);
      }

      return true;
    } catch (error) {
      pageStates.set(pageName, PageState.INACTIVE);

      if (window.rError) {
        window.rError(`❌ PageStateManager: 页面 ${pageName} 加载失败:`, error);
      }

      // 隐藏loading
      if (config.hideLoading) {
        try {
          config.hideLoading();
        } catch (e) {
          // 忽略hideLoading的错误
        }
      }

      // 显示错误提示
      if (window.AppNotifications) {
        window.AppNotifications.showError(`页面加载失败: ${error.message}`);
      }

      return false;
    }
  }

  /**
   * 刷新页面数据（强制刷新，忽略缓存）
   * @param {string} pageName - 页面名称
   * @returns {Promise<boolean>} - 是否成功
   */
  async function refreshPage(pageName) {
    if (window.rLog) {
      window.rLog(`🔄 PageStateManager: 手动刷新页面 ${pageName}`);
    }
    return await activatePage(pageName, true);
  }

  /**
   * 获取页面状态
   * @param {string} pageName - 页面名称
   * @returns {string} - 页面状态
   */
  function getPageState(pageName) {
    return pageStates.get(pageName) || PageState.INACTIVE;
  }

  /**
   * 获取页面最后更新时间
   * @param {string} pageName - 页面名称
   * @returns {number|null} - 时间戳，如果未更新过则返回null
   */
  function getLastUpdated(pageName) {
    return lastUpdated.get(pageName) || null;
  }

  /**
   * 检查页面数据是否过期
   * @param {string} pageName - 页面名称
   * @returns {boolean} - 是否过期
   */
  function isPageDataStale(pageName) {
    const config = pageConfigs.get(pageName);
    if (!config) return true;

    const lastUpdate = lastUpdated.get(pageName);
    if (!lastUpdate) return true;

    const now = Date.now();
    return (now - lastUpdate) > config.ttl;
  }

  /**
   * 清除页面缓存
   * @param {string} pageName - 页面名称（可选，不传则清除所有）
   */
  function clearPageCache(pageName) {
    if (pageName) {
      dataCache.delete(pageName);
      lastUpdated.delete(pageName);
      pageStates.set(pageName, PageState.INACTIVE);

      if (window.rLog) {
        window.rLog(`🗑️  PageStateManager: 已清除页面 ${pageName} 的缓存`);
      }
    } else {
      dataCache.clear();
      lastUpdated.clear();

      // 重置所有页面状态为INACTIVE
      for (const [name] of pageConfigs) {
        pageStates.set(name, PageState.INACTIVE);
      }

      if (window.rLog) {
        window.rLog('🗑️  PageStateManager: 已清除所有页面缓存');
      }
    }
  }

  /**
   * 取消注册页面
   * @param {string} pageName - 页面名称
   */
  function unregisterPage(pageName) {
    pageConfigs.delete(pageName);
    pageStates.delete(pageName);
    dataCache.delete(pageName);
    lastUpdated.delete(pageName);

    if (window.rLog) {
      window.rLog(`🗑️  PageStateManager: 已取消注册页面 ${pageName}`);
    }
  }

  /**
   * 获取所有已注册的页面
   * @returns {Array<string>} - 页面名称数组
   */
  function getRegisteredPages() {
    return Array.from(pageConfigs.keys());
  }

  /**
   * 获取页面缓存信息（用于调试）
   * @returns {Object} - 缓存信息对象
   */
  function getCacheInfo() {
    const info = {};
    for (const [pageName, config] of pageConfigs) {
      const state = pageStates.get(pageName);
      const lastUpdate = lastUpdated.get(pageName);
      const isStale = isPageDataStale(pageName);

      info[pageName] = {
        state,
        ttl: config.ttl,
        lastUpdate: lastUpdate ? new Date(lastUpdate).toLocaleString() : null,
        ageSeconds: lastUpdate ? Math.floor((Date.now() - lastUpdate) / 1000) : null,
        isStale
      };
    }
    return info;
  }

  // 导出 PageStateManager 模块
  window.PageStateManager = {
    // 页面状态枚举
    PageState,

    // 核心方法
    registerPage,
    activatePage,
    refreshPage,

    // 查询方法
    getPageState,
    getLastUpdated,
    isPageDataStale,

    // 管理方法
    clearPageCache,
    unregisterPage,
    getRegisteredPages,

    // 调试方法
    getCacheInfo
  };

  if (window.rLog) {
    window.rLog('✅ PageStateManager 模块已加载');
  }

})();
