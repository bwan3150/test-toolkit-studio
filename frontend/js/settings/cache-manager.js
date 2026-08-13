// 缓存管理模块 - Cache Manager Module
// 负责媒体缓存的统计和清理功能

window.CacheManager = {
  cacheSizeEl: null,
  cacheFileCountEl: null,
  clearCacheBtn: null,

  /**
   * 初始化缓存管理模块
   */
  init() {
    this.cacheSizeEl = document.getElementById('cacheSize');
    this.cacheFileCountEl = document.getElementById('cacheFileCount');
    this.clearCacheBtn = document.getElementById('clearCacheBtn');

    // 绑定事件
    this.bindEvents();

    // 初始加载缓存统计
    this.updateCacheStats();

    window.rLog('CacheManager 模块已初始化');
  },

  /**
   * 绑定事件监听器
   */
  bindEvents() {
    // 清空缓存
    if (this.clearCacheBtn) {
      this.clearCacheBtn.addEventListener('click', async () => {
        await this.clearCache();
      });
    }
  },

  /**
   * 更新缓存统计信息
   */
  updateCacheStats() {
    try {
      if (!window.MediaCache) {
        window.rWarn('MediaCache 模块未加载');
        this.setCacheStats('-', '-');
        return;
      }

      const stats = window.MediaCache.getCacheStats();

      if (stats) {
        const sizeText = stats.totalSizeMB
          ? `${stats.totalSizeMB} MB`
          : '0 MB';

        this.setCacheStats(sizeText, stats.fileCount);

        window.rLog(`缓存统计: ${sizeText}, ${stats.fileCount} 个文件`);
      } else {
        this.setCacheStats('-', '-');
      }
    } catch (error) {
      window.rError('获取缓存统计失败:', error);
      this.setCacheStats('错误', '错误');
    }
  },

  /**
   * 设置缓存统计显示
   * @param {string} size - 缓存大小文本
   * @param {number|string} fileCount - 文件数量
   */
  setCacheStats(size, fileCount) {
    if (this.cacheSizeEl) {
      this.cacheSizeEl.textContent = size;
    }
    if (this.cacheFileCountEl) {
      this.cacheFileCountEl.textContent = fileCount;
    }
  },

  /**
   * 清空所有缓存
   */
  async clearCache() {
    try {
      // 禁用按钮,防止重复点击
      if (this.clearCacheBtn) {
        this.clearCacheBtn.disabled = true;
        this.clearCacheBtn.classList.add('btn-loading');
        this.clearCacheBtn.innerHTML = `
          <svg class="btn-icon btn-spinner" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
            <path fill="currentColor" d="M12,4V2A10,10 0 0,0 2,12H4A8,8 0 0,1 12,4Z"/>
          </svg>
          清空中
        `;
      }

      if (!window.MediaCache) {
        throw new Error('MediaCache 模块未加载');
      }

      // 执行清空操作
      window.MediaCache.clearAllCache();

      window.rLog('✅ 媒体缓存已清空');

      // 显示成功状态
      if (this.clearCacheBtn) {
        this.clearCacheBtn.classList.remove('btn-loading');
        this.clearCacheBtn.classList.add('btn-success');
        this.clearCacheBtn.innerHTML = `
          <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
            <path fill="currentColor" d="M9,20.42L2.79,14.21L5.62,11.38L9,14.77L18.88,4.88L21.71,7.71L9,20.42Z"/>
          </svg>
          已清空
        `;
      }

      // 更新统计
      this.updateCacheStats();

      // 2秒后恢复按钮
      setTimeout(() => {
        if (this.clearCacheBtn) {
          this.clearCacheBtn.classList.remove('btn-success');
          this.clearCacheBtn.disabled = false;
          this.clearCacheBtn.innerHTML = `
            <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
              <path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
            </svg>
            清空
          `;
        }
      }, 2000);

    } catch (error) {
      window.rError('清空缓存失败:', error);

      // 显示错误状态
      if (this.clearCacheBtn) {
        this.clearCacheBtn.classList.remove('btn-loading');
        this.clearCacheBtn.classList.add('btn-error');
        this.clearCacheBtn.innerHTML = `
          <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
            <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
          </svg>
          失败
        `;
      }

      // 2秒后恢复按钮
      setTimeout(() => {
        if (this.clearCacheBtn) {
          this.clearCacheBtn.classList.remove('btn-error');
          this.clearCacheBtn.disabled = false;
          this.clearCacheBtn.innerHTML = `
            <svg class="btn-icon" viewBox="0 0 24 24" style="width: 16px; height: 16px;">
              <path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
            </svg>
            清空
          `;
        }
      }, 2000);
    }
  }
};
