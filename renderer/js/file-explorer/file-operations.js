// File Operations Module - 文件操作模块
// 负责新建文件夹、删除、重命名、下载(取出)、上传(放入)等操作

window.FileOperations = {
  /**
   * 新建文件夹
   * @param {string} currentPath - 当前路径
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success, newPath}
   */
  async createFolder(currentPath, deviceId) {
    window.rLog('开始创建新文件夹');

    try {
      // 确保路径以斜杠结尾
      const basePath = currentPath.endsWith('/') ? currentPath : currentPath + '/';

      // 自动生成文件夹名称
      let folderName = 'new_folder';
      let folderPath = `${basePath}${folderName}`;

      // 检查文件夹是否存在,如果存在则添加序号
      let counter = 1;
      while (true) {
        const checkResult = await window.api.tkeFileTree({
          path: folderPath,
          level: 1,
          deviceId: deviceId
        });

        // 如果返回错误(路径不存在),则可以使用这个名称
        if (!checkResult.success) {
          break;
        }

        // 如果存在,则添加序号 (使用下划线而不是括号,避免shell语法错误)
        folderName = `new_folder_${counter}`;
        folderPath = `${basePath}${folderName}`;
        counter++;
      }

      window.rLog(`准备创建文件夹: ${folderPath}`);

      const result = await window.api.tkeFileMkdir({
        path: folderPath,
        deviceId: deviceId
      });

      if (result.success) {
        window.rLog(`✅ 新建文件夹成功: ${folderName}`);
        return { success: true, newPath: folderPath, folderName };
      } else {
        throw new Error(result.error || '创建失败');
      }
    } catch (error) {
      window.rError('新建文件夹失败:', error);
      window.AppNotifications?.error('创建文件夹失败: ' + error.message);
      return { success: false };
    }
  },

  /**
   * 删除文件或文件夹
   * @param {Array<string>} paths - 要删除的路径列表
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success, successCount, failCount}
   */
  async deleteFiles(paths, deviceId) {
    if (paths.length === 0) return { success: false, successCount: 0, failCount: 0 };

    const confirmMsg = `确定要删除选中的 ${paths.length} 个项目吗?`;
    if (!confirm(confirmMsg)) {
      return { success: false, successCount: 0, failCount: 0 };
    }

    let successCount = 0;
    let failCount = 0;

    for (const path of paths) {
      try {
        const result = await window.api.tkeFileRm({
          path: path,
          deviceId: deviceId
        });

        if (result.success) {
          window.rLog(`✅ 删除成功: ${path}`);
          successCount++;
        } else {
          throw new Error(result.error || '删除失败');
        }
      } catch (error) {
        window.rError(`❌ 删除失败 ${path}:`, error);
        failCount++;
      }
    }

    if (successCount > 0) {
      window.rLog(`✅ 成功删除 ${successCount} 个项目`);
    }
    if (failCount > 0) {
      window.rError(`❌ ${failCount} 个项目删除失败`);
    }

    if (failCount > 0) {
      alert(`Deleted ${successCount} item(s), ${failCount} failed`);
    }

    return { success: successCount > 0, successCount, failCount };
  },

  /**
   * 重命名文件或文件夹 (执行重命名操作)
   * @param {string} oldPath - 旧路径
   * @param {string} newName - 新名称
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success, newPath}
   */
  async renameFile(oldPath, newName, deviceId) {
    const oldName = oldPath.split('/').filter(p => p).pop();

    if (!newName || newName === oldName) {
      return { success: false };
    }

    try {
      // 构建新路径
      const pathParts = oldPath.split('/').filter(p => p);
      pathParts[pathParts.length - 1] = newName;
      const newPath = '/' + pathParts.join('/') + (oldPath.endsWith('/') ? '/' : '');

      window.rLog(`准备重命名: ${oldPath} -> ${newPath}`);

      const result = await window.api.tkeFileMv({
        source: oldPath,
        dest: newPath,
        deviceId: deviceId
      });

      if (result.success) {
        window.rLog(`✅ 重命名成功: ${oldName} -> ${newName}`);
        window.AppNotifications?.success(`已重命名: ${newName}`);
        return { success: true, newPath };
      } else {
        throw new Error(result.error);
      }
    } catch (error) {
      window.rError('重命名失败:', error);
      window.AppNotifications?.error('重命名失败: ' + error.message);
      return { success: false };
    }
  },

  /**
   * 取出单个文件
   * @param {string} remotePath - 远程路径
   * @param {string} fileName - 文件名
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success}
   */
  async pullSingleFile(remotePath, fileName, deviceId) {
    try {
      const { ipcRenderer } = window.AppGlobals;

      // 选择保存目录
      const localDir = await ipcRenderer.invoke('select-directory');
      if (!localDir) {
        window.rLog('用户取消了取出操作');
        return { success: false };
      }

      window.rLog(`开始取出文件: ${fileName} 到 ${localDir}`);

      const pullResult = await window.api.tkeFilePull({
        remote: remotePath,
        local: localDir,
        deviceId: deviceId
      });

      if (pullResult.success) {
        window.rLog(`✅ 取出成功: ${fileName}`);
        alert(`Successfully pulled ${fileName}`);
        return { success: true };
      } else {
        throw new Error(pullResult.error || '取出失败');
      }
    } catch (error) {
      window.rError(`❌ 取出失败 ${fileName}:`, error);
      alert(`Failed to pull ${fileName}: ${error.message}`);
      return { success: false };
    }
  },

  /**
   * 取出多个文件
   * @param {Array<string>} remotePaths - 远程路径列表
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success, successCount, failCount}
   */
  async pullMultipleFiles(remotePaths, deviceId) {
    if (remotePaths.length === 0) {
      return { success: false, successCount: 0, failCount: 0 };
    }

    try {
      const { ipcRenderer } = window.AppGlobals;

      // 选择保存目录
      const localDir = await ipcRenderer.invoke('select-directory');
      if (!localDir) {
        window.rLog('用户取消了取出操作');
        return { success: false, successCount: 0, failCount: 0 };
      }

      window.rLog(`开始取出 ${remotePaths.length} 个文件到 ${localDir}`);

      let successCount = 0;
      let failCount = 0;

      // 逐个下载文件
      for (const remotePath of remotePaths) {
        try {
          const pullResult = await window.api.tkeFilePull({
            remote: remotePath,
            local: localDir,
            deviceId: deviceId
          });

          if (pullResult.success) {
            window.rLog(`✅ 取出成功: ${remotePath}`);
            successCount++;
          } else {
            throw new Error(pullResult.error || '取出失败');
          }
        } catch (error) {
          window.rError(`❌ 取出失败 ${remotePath}:`, error);
          failCount++;
        }
      }

      window.rLog(`✅ 文件取出完成`);
      alert(`Successfully pulled ${successCount} file(s)${failCount > 0 ? `, ${failCount} failed` : ''}`);

      return { success: successCount > 0, successCount, failCount };
    } catch (error) {
      window.rError('取出文件失败:', error);
      alert('Failed to pull files: ' + error.message);
      return { success: false, successCount: 0, failCount: 0 };
    }
  },

  /**
   * 放入文件到设备(通过对话框选择)
   * @param {string} remotePath - 远程目录路径
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success, successCount, failCount}
   */
  async pushFiles(remotePath, deviceId) {
    if (!deviceId) {
      alert('Please select a device first');
      return { success: false, successCount: 0, failCount: 0 };
    }

    try {
      const { ipcRenderer } = window.AppGlobals;

      // 选择要上传的文件
      const localFiles = await ipcRenderer.invoke('select-files');

      if (!localFiles || localFiles.length === 0) {
        return { success: false, successCount: 0, failCount: 0 };
      }

      window.rLog(`开始放入 ${localFiles.length} 个文件到 ${remotePath}`);

      let successCount = 0;
      let failCount = 0;

      // 逐个上传文件
      for (const localPath of localFiles) {
        try {
          const pushResult = await window.api.tkeFilePush({
            local: localPath,
            remote: remotePath,
            deviceId: deviceId
          });

          if (pushResult.success) {
            window.rLog(`✅ 放入成功: ${localPath}`);
            successCount++;
          } else {
            throw new Error(pushResult.error || '放入失败');
          }
        } catch (error) {
          window.rError(`❌ 放入失败 ${localPath}:`, error);
          failCount++;
        }
      }

      window.rLog(`✅ 文件放入完成`);
      alert(`Successfully pushed ${successCount} file(s)${failCount > 0 ? `, ${failCount} failed` : ''}`);

      return { success: successCount > 0, successCount, failCount };
    } catch (error) {
      window.rError('放入文件失败:', error);
      alert('Failed to push files: ' + error.message);
      return { success: false, successCount: 0, failCount: 0 };
    }
  },

  /**
   * 从文件路径列表放入文件(用于拖拽上传)
   * @param {Array<string>} filePaths - 本地文件路径列表
   * @param {string} remotePath - 远程目录路径
   * @param {string} deviceId - 设备ID
   * @returns {Promise<Object>} {success, successCount, failCount}
   */
  async pushFilesFromPaths(filePaths, remotePath, deviceId) {
    window.rLog(`开始拖拽放入 ${filePaths.length} 个文件到 ${remotePath}`);

    let successCount = 0;
    let failCount = 0;

    for (const localPath of filePaths) {
      try {
        const pushResult = await window.api.tkeFilePush({
          local: localPath,
          remote: remotePath,
          deviceId: deviceId
        });

        if (pushResult.success) {
          window.rLog(`✅ 放入成功: ${localPath}`);
          successCount++;
        } else {
          throw new Error(pushResult.error || '放入失败');
        }
      } catch (error) {
        window.rError(`❌ 放入失败 ${localPath}:`, error);
        failCount++;
      }
    }

    // 显示结果
    if (successCount > 0) {
      window.rLog(`✅ 成功放入 ${successCount} 个文件`);
    }
    if (failCount > 0) {
      window.rError(`❌ ${failCount} 个文件放入失败`);
    }

    if (successCount > 0 || failCount > 0) {
      alert(`Pushed ${successCount} file(s)${failCount > 0 ? `, ${failCount} failed` : ''}`);
    }

    return { success: successCount > 0, successCount, failCount };
  }
};
