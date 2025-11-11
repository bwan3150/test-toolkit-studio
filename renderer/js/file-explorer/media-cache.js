// Media Cache Manager - 媒体文件缓存管理器
// 负责下载和缓存设备上的图片/视频，以及生成视频缩略图

(function() {
  const crypto = require('crypto');
  const path = require('path');
  const fs = require('fs');
  const os = require('os');

  const getIpcRenderer = () => window.AppGlobals.ipcRenderer;

  // 缓存目录配置
  const CACHE_BASE_DIR = path.join(os.homedir(), '.test-toolkit-studio', 'cache', 'media');

  class MediaCache {
    constructor() {
      this.cacheDir = CACHE_BASE_DIR;
      this.initCacheDir();
      window.rLog(`MediaCache 初始化，缓存目录: ${this.cacheDir}`);
    }

    /**
     * 初始化缓存目录
     */
    initCacheDir() {
      try {
        if (!fs.existsSync(this.cacheDir)) {
          fs.mkdirSync(this.cacheDir, { recursive: true });
          window.rLog(`创建缓存目录: ${this.cacheDir}`);
        }
      } catch (error) {
        window.rError('创建缓存目录失败:', error);
      }
    }

    /**
     * 生成缓存文件路径
     * @param {string} remotePath - 远程文件路径
     * @param {string} deviceId - 设备ID
     * @returns {string} 本地缓存文件路径
     */
    getCachePath(remotePath, deviceId) {
      // 使用 MD5(remotePath) 作为文件名，避免路径过长和特殊字符问题
      const hash = crypto.createHash('md5').update(remotePath).digest('hex');
      const ext = path.extname(remotePath);

      // 缓存结构: {cacheDir}/{deviceId}/{hash}.{ext}
      const deviceDir = path.join(this.cacheDir, deviceId);

      // 确保设备目录存在
      if (!fs.existsSync(deviceDir)) {
        fs.mkdirSync(deviceDir, { recursive: true });
      }

      return path.join(deviceDir, `${hash}${ext}`);
    }

    /**
     * 生成缩略图缓存路径
     * @param {string} videoPath - 视频文件路径（本地或远程）
     * @param {string} deviceId - 设备ID
     * @returns {string} 缩略图文件路径
     */
    getThumbnailPath(videoPath, deviceId) {
      const hash = crypto.createHash('md5').update(videoPath).digest('hex');
      const deviceDir = path.join(this.cacheDir, deviceId);

      if (!fs.existsSync(deviceDir)) {
        fs.mkdirSync(deviceDir, { recursive: true });
      }

      return path.join(deviceDir, `${hash}_thumb.jpg`);
    }

    /**
     * 检查文件是否已缓存
     * @param {string} remotePath - 远程文件路径
     * @param {string} deviceId - 设备ID
     * @returns {boolean} 是否已缓存
     */
    isCached(remotePath, deviceId) {
      const cachePath = this.getCachePath(remotePath, deviceId);
      return fs.existsSync(cachePath);
    }

    /**
     * 检查缩略图是否已缓存
     * @param {string} videoPath - 视频文件路径
     * @param {string} deviceId - 设备ID
     * @returns {boolean} 是否已缓存
     */
    isThumbnailCached(videoPath, deviceId) {
      const thumbPath = this.getThumbnailPath(videoPath, deviceId);
      return fs.existsSync(thumbPath);
    }

    /**
     * 下载媒体文件到本地缓存
     * @param {string} remotePath - 远程文件路径
     * @param {string} deviceId - 设备ID
     * @returns {Promise<string>} 本地缓存文件路径
     */
    async pullMedia(remotePath, deviceId) {
      const cachePath = this.getCachePath(remotePath, deviceId);

      // 如果已缓存，直接返回
      if (fs.existsSync(cachePath)) {
        window.rLog(`媒体文件已缓存: ${remotePath}`);
        return cachePath;
      }

      try {
        window.rLog(`开始下载媒体文件: ${remotePath}`);

        // 使用 tke file pull 下载文件
        await window.api.tkeFilePull({
          remote: remotePath,
          local: cachePath,
          deviceId: deviceId
        });

        window.rLog(`媒体文件下载完成: ${cachePath}`);
        return cachePath;
      } catch (error) {
        window.rError(`下载媒体文件失败: ${remotePath}`, error);
        throw error;
      }
    }

    /**
     * 生成视频缩略图（提取首帧）
     * @param {string} localVideoPath - 本地视频文件路径
     * @param {string} deviceId - 设备ID
     * @param {string} remotePath - 原始远程路径（用于生成缩略图文件名）
     * @returns {Promise<string>} 缩略图文件路径
     */
    async getVideoThumbnail(localVideoPath, deviceId, remotePath) {
      const thumbPath = this.getThumbnailPath(remotePath, deviceId);

      // 如果缩略图已存在，直接返回
      if (fs.existsSync(thumbPath)) {
        window.rLog(`视频缩略图已缓存: ${remotePath}`);
        return thumbPath;
      }

      try {
        window.rLog(`开始生成视频缩略图: ${localVideoPath}`);

        const ipcRenderer = getIpcRenderer();

        // 使用 tke ffmpeg 提取视频首帧
        // ffmpeg -i input.mp4 -vframes 1 -vf scale=200:-1 output.jpg
        const result = await ipcRenderer.invoke('tke-ffmpeg', {
          args: [
            '-i', localVideoPath,
            '-vframes', '1',
            '-vf', 'scale=200:-1',
            '-y', // 覆盖已存在的文件
            thumbPath
          ]
        });

        if (result.success && fs.existsSync(thumbPath)) {
          window.rLog(`视频缩略图生成成功: ${thumbPath}`);
          return thumbPath;
        } else {
          throw new Error('FFmpeg 执行失败或缩略图文件未生成');
        }
      } catch (error) {
        window.rError(`生成视频缩略图失败: ${localVideoPath}`, error);
        throw error;
      }
    }

    /**
     * 生成图片缩略图
     * @param {string} localImagePath - 本地图片文件路径
     * @param {string} deviceId - 设备ID
     * @param {string} remotePath - 原始远程路径（用于生成缩略图文件名）
     * @returns {Promise<string>} 缩略图文件路径
     */
    async getImageThumbnail(localImagePath, deviceId, remotePath) {
      const thumbPath = this.getThumbnailPath(remotePath, deviceId);

      // 如果缩略图已存在，直接返回
      if (fs.existsSync(thumbPath)) {
        window.rLog(`图片缩略图已缓存: ${remotePath}`);
        return thumbPath;
      }

      try {
        window.rLog(`开始生成图片缩略图: ${localImagePath}`);

        const ipcRenderer = getIpcRenderer();

        // 使用 tke ffmpeg 生成图片缩略图
        // ffmpeg -i input.jpg -vf scale=200:-1 output.jpg
        const result = await ipcRenderer.invoke('tke-ffmpeg', {
          args: [
            '-i', localImagePath,
            '-vf', 'scale=200:-1',
            '-y', // 覆盖已存在的文件
            thumbPath
          ]
        });

        if (result.success && fs.existsSync(thumbPath)) {
          window.rLog(`图片缩略图生成成功: ${thumbPath}`);
          return thumbPath;
        } else {
          throw new Error('FFmpeg 执行失败或缩略图文件未生成');
        }
      } catch (error) {
        window.rError(`生成图片缩略图失败: ${localImagePath}`, error);
        throw error;
      }
    }

    /**
     * 获取媒体缩略图URL（仅缓存缩略图，不缓存原始文件）
     * @param {string} remotePath - 远程文件路径
     * @param {string} deviceId - 设备ID
     * @param {string} fileType - 文件类型 ('image' | 'video')
     * @returns {Promise<string>} 本地缩略图文件路径（用于 img src）
     */
    async getThumbnailUrl(remotePath, deviceId, fileType) {
      try {
        // 检查缩略图是否已缓存
        if (this.isThumbnailCached(remotePath, deviceId)) {
          const thumbPath = this.getThumbnailPath(remotePath, deviceId);
          window.rLog(`使用已缓存的缩略图: ${remotePath}`);
          return thumbPath;
        }

        // 下载原始文件到临时目录（不放入缓存）
        const { ipcRenderer } = window.AppGlobals;
        const tempDir = await ipcRenderer.invoke('get-temp-dir');
        const path = window.nodeRequire ? window.nodeRequire('path') : require('path');
        const fileName = remotePath.split('/').filter(p => p).pop();
        const tempFilePath = path.join(tempDir, fileName);

        window.rLog(`下载媒体文件到临时目录: ${remotePath} -> ${tempFilePath}`);

        await window.api.tkeFilePull({
          remote: remotePath,
          local: tempDir,
          deviceId: deviceId
        });

        // 生成缩略图
        let thumbPath;
        if (fileType === 'video') {
          thumbPath = await this.getVideoThumbnail(tempFilePath, deviceId, remotePath);
        } else {
          thumbPath = await this.getImageThumbnail(tempFilePath, deviceId, remotePath);
        }

        // 删除临时文件（节省空间）
        try {
          if (fs.existsSync(tempFilePath)) {
            fs.unlinkSync(tempFilePath);
            window.rLog(`已删除临时文件: ${tempFilePath}`);
          }
        } catch (error) {
          window.rError(`删除临时文件失败: ${tempFilePath}`, error);
        }

        return thumbPath;
      } catch (error) {
        window.rError(`获取缩略图失败: ${remotePath}`, error);
        throw error;
      }
    }

    /**
     * 获取完整媒体文件（用于预览，不缓存）
     * @param {string} remotePath - 远程文件路径
     * @param {string} deviceId - 设备ID
     * @returns {Promise<string>} 本地临时文件路径
     */
    async getFullMediaUrl(remotePath, deviceId) {
      try {
        const { ipcRenderer } = window.AppGlobals;
        const tempDir = await ipcRenderer.invoke('get-temp-dir');
        const path = window.nodeRequire ? window.nodeRequire('path') : require('path');
        const fileName = remotePath.split('/').filter(p => p).pop();
        const tempFilePath = path.join(tempDir, fileName);

        // 检查临时文件是否已存在
        if (fs.existsSync(tempFilePath)) {
          window.rLog(`使用已存在的临时文件: ${tempFilePath}`);
          return tempFilePath;
        }

        window.rLog(`下载完整媒体文件: ${remotePath}`);

        await window.api.tkeFilePull({
          remote: remotePath,
          local: tempDir,
          deviceId: deviceId
        });

        return tempFilePath;
      } catch (error) {
        window.rError(`获取完整媒体文件失败: ${remotePath}`, error);
        throw error;
      }
    }

    /**
     * 清理特定设备的缓存
     * @param {string} deviceId - 设备ID
     */
    clearDeviceCache(deviceId) {
      try {
        const deviceDir = path.join(this.cacheDir, deviceId);

        if (fs.existsSync(deviceDir)) {
          const files = fs.readdirSync(deviceDir);

          files.forEach(file => {
            const filePath = path.join(deviceDir, file);
            fs.unlinkSync(filePath);
          });

          fs.rmdirSync(deviceDir);
          window.rLog(`已清理设备缓存: ${deviceId}`);
        }
      } catch (error) {
        window.rError(`清理设备缓存失败: ${deviceId}`, error);
      }
    }

    /**
     * 清理所有缓存
     */
    clearAllCache() {
      try {
        if (fs.existsSync(this.cacheDir)) {
          const devices = fs.readdirSync(this.cacheDir);

          devices.forEach(deviceId => {
            this.clearDeviceCache(deviceId);
          });

          window.rLog('已清理所有缓存');
        }
      } catch (error) {
        window.rError('清理缓存失败:', error);
      }
    }

    /**
     * 获取缓存统计信息
     * @returns {Object} 缓存统计 { totalSize: number, fileCount: number, devices: string[] }
     */
    getCacheStats() {
      try {
        let totalSize = 0;
        let fileCount = 0;
        const devices = [];

        if (fs.existsSync(this.cacheDir)) {
          const deviceDirs = fs.readdirSync(this.cacheDir);

          deviceDirs.forEach(deviceId => {
            devices.push(deviceId);
            const deviceDir = path.join(this.cacheDir, deviceId);
            const files = fs.readdirSync(deviceDir);

            files.forEach(file => {
              const filePath = path.join(deviceDir, file);
              const stats = fs.statSync(filePath);
              totalSize += stats.size;
              fileCount++;
            });
          });
        }

        return {
          totalSize,
          fileCount,
          devices,
          totalSizeMB: (totalSize / 1024 / 1024).toFixed(2)
        };
      } catch (error) {
        window.rError('获取缓存统计失败:', error);
        return { totalSize: 0, fileCount: 0, devices: [], totalSizeMB: '0.00' };
      }
    }
  }

  // 导出全局单例
  window.MediaCache = new MediaCache();

  window.rLog('Media Cache Manager 已加载');

})();
