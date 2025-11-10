// File API - TKE File 模块的 API 封装
// 提供设备文件系统管理的统一接口

(function() {
  const getIpcRenderer = () => window.AppGlobals.ipcRenderer;

  // TKE File API 封装
  window.api = window.api || {};

  // 列出目录内容 (树形结构)
  window.api.tkeFileLs = async ({ path, level, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-ls', { path, level, deviceId });
  };

  // 搜索文件
  window.api.tkeFileFind = async ({ path, pattern, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-find', { path, pattern, deviceId });
  };

  // 读取文件内容
  window.api.tkeFileCat = async ({ path, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-cat', { path, deviceId });
  };

  // 获取目录大小
  window.api.tkeFileSize = async ({ path, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-size', { path, deviceId });
  };

  // 创建目录
  window.api.tkeFileMkdir = async ({ path, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-mkdir', { path, deviceId });
  };

  // 删除文件/目录
  window.api.tkeFileRm = async ({ path, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-rm', { path, deviceId });
  };

  // 复制文件/目录
  window.api.tkeFileCp = async ({ source, dest, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-cp', { source, dest, deviceId });
  };

  // 移动/重命名文件
  window.api.tkeFileMv = async ({ source, dest, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-mv', { source, dest, deviceId });
  };

  // 写入文件
  window.api.tkeFileWrite = async ({ path, content, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-write', { path, content, deviceId });
  };

  // 下载文件到本地
  window.api.tkeFilePull = async ({ remote, local, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-pull', { remote, local, deviceId });
  };

  // 上传文件到设备
  window.api.tkeFilePush = async ({ local, remote, deviceId }) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-file-push', { local, remote, deviceId });
  };

  // 获取设备列表 (复用 controller 的接口)
  window.api.tkeControllerDevices = async (projectPath) => {
    const ipcRenderer = getIpcRenderer();
    return await ipcRenderer.invoke('tke-controller-devices', projectPath);
  };

  window.rLog('File API 已加载');

})();
