// Release Notes API 代理处理器
// 负责从 S3 获取版本索引和 Release Notes
const { ipcMain } = require('electron');
const axios = require('axios');

const S3_BASE_URL = 'https://toolkit-studio-updates.s3.ap-southeast-2.amazonaws.com';
const INDEX_URL = `${S3_BASE_URL}/release-notes/release-note-index.json`;

/**
 * 注册 Release Notes 相关的 IPC 处理器
 */
function registerReleaseNotesHandlers() {
  // 获取版本索引
  ipcMain.handle('get-release-notes-index', async () => {
    try {
      const response = await axios.get(INDEX_URL, {
        timeout: 10000
      });

      if (response.status === 200 && response.data) {
        return {
          success: true,
          data: response.data
        };
      }

      throw new Error('Failed to fetch index');
    } catch (error) {
      console.error('获取 Release Notes 索引失败:', error);
      return {
        success: false,
        error: error.message
      };
    }
  });

  // 获取特定版本的 Release Notes
  ipcMain.handle('get-release-note', async (event, version) => {
    try {
      if (!version) {
        throw new Error('Version is required');
      }

      const noteUrl = `${S3_BASE_URL}/release-notes/${version}.md`;
      const response = await axios.get(noteUrl, {
        timeout: 10000,
        responseType: 'text'
      });

      if (response.status === 200) {
        return {
          success: true,
          data: response.data
        };
      }

      throw new Error('Failed to fetch release note');
    } catch (error) {
      console.error(`获取版本 ${version} 的 Release Note 失败:`, error);
      return {
        success: false,
        error: error.message
      };
    }
  });
}

module.exports = { registerReleaseNotesHandlers };
