// LocalSend 页面控制器
// 负责 SEND/RECEIVE 两个模式的 UI 逻辑，通过 IPC 与主进程通信

(function () {
  'use strict';

  const { ipcRenderer } = require('electron');

  // ── 状态 ─────────────────────────────────────────────────────────────────

  let currentMode = 'send';
  let selectedDevice = null;
  let selectedFilePath = null;
  let isSending = false;

  const $ = (id) => document.getElementById(id);

  // ── SVG 图标库 ────────────────────────────────────────────────────────────

  const ICONS = {
    desktop: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,
    laptop:  `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="4" width="20" height="13" rx="2"/><path d="M1 21h22"/></svg>`,
    tablet:  `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="2" width="16" height="20" rx="2"/><circle cx="12" cy="18" r="1" fill="currentColor"/></svg>`,
    mobile:  `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="2" width="14" height="20" rx="2"/><circle cx="12" cy="18" r="1" fill="currentColor"/></svg>`,

    // 文件类型图标（带内容区分的文件图标）
    fileGeneric: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`,
    fileText:    `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>`,
    fileImage:   `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>`,
    fileVideo:   `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="4" width="20" height="16" rx="2"/><polygon points="10 9 16 12 10 15 10 9" fill="currentColor" stroke="none"/></svg>`,
    fileAudio:   `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`,
    fileArchive: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>`,
    filePdf:     `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M9 13h1a1 1 0 0 1 0 2H9v-2zm0 2v2m5-4h1.5a1.5 1.5 0 0 1 0 3H14v-3zm0 3v1m3-4v4"/></svg>`,
    fileCode:    `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><polyline points="10 14 8 16 10 18"/><polyline points="14 14 16 16 14 18"/></svg>`,

    // 操作图标
    revealFile:  `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>`,
    copyText:    `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>`,
  };

  function deviceTypeIcon(type) {
    switch ((type || '').toLowerCase()) {
      case 'desktop': return ICONS.desktop;
      case 'laptop':  return ICONS.laptop;
      case 'tablet':  return ICONS.tablet;
      default:        return ICONS.mobile;
    }
  }

  function fileTypeIcon(name, mimeType) {
    const ext = (name || '').split('.').pop()?.toLowerCase();
    const mime = (mimeType || '').toLowerCase();

    if (mime.startsWith('image/') || ['jpg','jpeg','png','gif','webp','svg','bmp','ico'].includes(ext))
      return ICONS.fileImage;
    if (mime.startsWith('video/') || ['mp4','mov','avi','mkv','wmv','flv'].includes(ext))
      return ICONS.fileVideo;
    if (mime.startsWith('audio/') || ['mp3','wav','ogg','flac','aac','m4a'].includes(ext))
      return ICONS.fileAudio;
    if (mime === 'application/pdf' || ext === 'pdf')
      return ICONS.filePdf;
    if (['zip','rar','tar','gz','7z','bz2'].includes(ext))
      return ICONS.fileArchive;
    if (['js','ts','json','html','css','py','java','kt','swift','go','rs','c','cpp','h'].includes(ext))
      return ICONS.fileCode;
    if (mime.startsWith('text/') || ['txt','md','log','csv','xml','yaml','yml'].includes(ext))
      return ICONS.fileText;
    return ICONS.fileGeneric;
  }

  // ── 初始化 ────────────────────────────────────────────────────────────────

  async function init() {
    window.rLog('LocalSend: 初始化控制器');

    try {
      const info = await ipcRenderer.invoke('localsend:get-device-info');
      const badge = $('lsDeviceBadge');
      if (badge) badge.textContent = `${info.alias} (${info.ip})`;
    } catch (e) {
      window.rError('LocalSend: 获取设备信息失败', e.message);
    }

    $('lsSendModeBtn').addEventListener('click', () => switchMode('send'));
    $('lsReceiveModeBtn').addEventListener('click', () => switchMode('receive'));

    $('lsTextTabBtn').addEventListener('click', () => switchType('text'));
    $('lsFileTabBtn').addEventListener('click', () => switchType('file'));

    $('lsPasteBtn').addEventListener('click', async () => {
      try {
        const text = await ipcRenderer.invoke('localsend:read-clipboard');
        if (text) $('lsTextInput').value = text;
      } catch (e) {
        window.rError('LocalSend: 读取剪切板失败', e.message);
      }
    });

    $('lsSendTextBtn').addEventListener('click', sendText);

    $('lsFileDropZone').addEventListener('click', () => $('lsFileInput').click());
    $('lsFileInput').addEventListener('change', onFileSelected);

    const dropZone = $('lsFileDropZone');
    dropZone.addEventListener('dragover', (e) => { e.preventDefault(); dropZone.classList.add('drag-over'); });
    dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
    dropZone.addEventListener('drop', (e) => {
      e.preventDefault();
      dropZone.classList.remove('drag-over');
      const files = e.dataTransfer.files;
      if (files.length > 0) selectFile(files[0].path, files[0].name, files[0].size);
    });

    $('lsClearFileBtn').addEventListener('click', clearFile);
    $('lsSendFileBtn').addEventListener('click', sendFile);

    $('lsClearReceivedBtn').addEventListener('click', async () => {
      await ipcRenderer.invoke('localsend:clear-received');
      renderReceivedList([]);
    });

    $('lsChangeSaveDirBtn').addEventListener('click', chooseSaveDir);

    // 主进程推送事件
    ipcRenderer.on('localsend:devices-updated', (_, devices) => renderDevices(devices));
    ipcRenderer.on('localsend:send-progress', (_, data) => onSendProgress(data));
    ipcRenderer.on('localsend:send-complete', (_, data) => onSendComplete(data));
    ipcRenderer.on('localsend:send-error', (_, data) => onSendError(data));
    ipcRenderer.on('localsend:receive-ready', (_, data) => onReceiveReady(data));
    ipcRenderer.on('localsend:receive-incoming', (_, data) => onReceiveIncoming(data));
    ipcRenderer.on('localsend:receive-progress', (_, data) => onReceiveProgress(data));
    ipcRenderer.on('localsend:receive-complete', (_, data) => onReceiveComplete(data));
    ipcRenderer.on('localsend:receive-cancelled', () => hideIncomingArea());
    ipcRenderer.on('localsend:error', (_, data) => window.rError('LocalSend 错误:', data.message));

    if (window.PageNavigator) {
      window.PageNavigator.registerPageHook('localsend', onPageActivate);
    }
  }

  // ── 页面激活 ──────────────────────────────────────────────────────────────

  async function onPageActivate() {
    window.rLog('LocalSend: 页面激活，当前模式:', currentMode);
    if (currentMode === 'send') {
      await activateSendMode();
    } else {
      await activateReceiveMode();
      const items = await ipcRenderer.invoke('localsend:get-received-items');
      renderReceivedList(items);
      await refreshSaveDirDisplay();
    }
  }

  // ── 保存目录 ──────────────────────────────────────────────────────────────

  async function refreshSaveDirDisplay() {
    try {
      const dir = await ipcRenderer.invoke('localsend:get-save-dir');
      const el = $('lsSaveDirPath');
      if (el) el.textContent = dir;
    } catch (_) {}
  }

  async function chooseSaveDir() {
    const result = await ipcRenderer.invoke('localsend:choose-save-dir');
    if (result.success) {
      const el = $('lsSaveDirPath');
      if (el) el.textContent = result.dirPath;
    }
  }

  // ── 模式切换 ──────────────────────────────────────────────────────────────

  async function switchMode(mode) {
    if (mode === currentMode) return;
    currentMode = mode;

    $('lsSendModeBtn').classList.toggle('active', mode === 'send');
    $('lsReceiveModeBtn').classList.toggle('active', mode === 'receive');
    $('lsSendMode').classList.toggle('active', mode === 'send');
    $('lsReceiveMode').classList.toggle('active', mode === 'receive');

    await ipcRenderer.invoke('localsend:stop');
    selectedDevice = null;
    clearFile();
    hideSendPanel();

    if (mode === 'send') {
      await activateSendMode();
    } else {
      await activateReceiveMode();
      const items = await ipcRenderer.invoke('localsend:get-received-items');
      renderReceivedList(items);
      await refreshSaveDirDisplay();
    }
  }

  async function activateSendMode() {
    const result = await ipcRenderer.invoke('localsend:start-send-mode');
    if (!result.success) window.rError('LocalSend: 启动 SEND 模式失败', result.error);
    const devices = await ipcRenderer.invoke('localsend:get-devices');
    renderDevices(devices);
  }

  async function activateReceiveMode() {
    const localIp = await ipcRenderer.invoke('localsend:get-local-ip');
    const badge = $('lsReceiveIpBadge');
    if (badge) badge.textContent = `${localIp}:53317`;

    const result = await ipcRenderer.invoke('localsend:start-receive-mode');
    if (!result.success) window.rError('LocalSend: 启动 RECEIVE 模式失败', result.error);
  }

  // ── 设备列表渲染 ──────────────────────────────────────────────────────────

  function renderDevices(devices) {
    const list = $('lsDevicesList');
    const empty = $('lsDevicesEmpty');

    list.querySelectorAll('.device-item').forEach(el => el.remove());

    if (!devices || devices.length === 0) {
      if (empty) empty.style.display = '';
      return;
    }
    if (empty) empty.style.display = 'none';

    for (const device of devices) {
      const item = document.createElement('div');
      item.className = 'device-item';
      if (selectedDevice && selectedDevice.ip === device.ip) item.classList.add('selected');
      item.dataset.ip = device.ip;

      item.innerHTML = `
        <div class="device-icon">${deviceTypeIcon(device.deviceType)}</div>
        <div class="device-info">
          <div class="device-alias">${escapeHtml(device.alias)}</div>
          <div class="device-ip">${device.ip}</div>
        </div>
      `;
      item.addEventListener('click', () => selectDevice(device));
      list.appendChild(item);
    }
  }

  function selectDevice(device) {
    selectedDevice = device;
    document.querySelectorAll('.device-item').forEach(el => {
      el.classList.toggle('selected', el.dataset.ip === device.ip);
    });
    $('lsSendPlaceholder').style.display = 'none';
    $('lsSendPanel').style.display = '';
    $('lsTargetName').textContent = `${device.alias} (${device.ip})`;
    resetSendState();
  }

  function hideSendPanel() {
    $('lsSendPlaceholder').style.display = '';
    $('lsSendPanel').style.display = 'none';
  }

  // ── 文字/文件 Tab 切换 ────────────────────────────────────────────────────

  function switchType(type) {
    $('lsTextTabBtn').classList.toggle('active', type === 'text');
    $('lsFileTabBtn').classList.toggle('active', type === 'file');
    $('lsTextSendArea').style.display = type === 'text' ? '' : 'none';
    $('lsFileSendArea').style.display  = type === 'file' ? '' : 'none';
    resetSendState();
  }

  // ── 发送文字 ──────────────────────────────────────────────────────────────

  async function sendText() {
    if (!selectedDevice || isSending) return;
    const text = $('lsTextInput').value.trim();
    if (!text) { window.rWarn('LocalSend: 文字内容为空'); return; }
    isSending = true;
    setSendButtonsDisabled(true);
    showProgress();
    try {
      await ipcRenderer.invoke('localsend:send-text', { device: selectedDevice, text });
    } catch (e) {
      window.rError('LocalSend: 发送文字异常', e.message);
      isSending = false;
      setSendButtonsDisabled(false);
    }
  }

  // ── 文件选择 ──────────────────────────────────────────────────────────────

  function onFileSelected(e) {
    const file = e.target.files[0];
    if (!file) return;
    selectFile(file.path, file.name, file.size);
  }

  function selectFile(filePath, fileName, fileSize) {
    selectedFilePath = filePath;
    $('lsFileDropZone').style.display = 'none';
    $('lsSelectedFileRow').style.display = '';
    $('lsSelectedFileName').textContent = fileName;
    $('lsSelectedFileSize').textContent = formatBytes(fileSize);
    // 用对应类型的 SVG 图标替换文件图标
    $('lsSelectedFileIcon').innerHTML = fileTypeIcon(fileName, '');
    $('lsSendFileBtn').disabled = false;
  }

  function clearFile() {
    selectedFilePath = null;
    $('lsFileDropZone').style.display = '';
    $('lsSelectedFileRow').style.display = 'none';
    $('lsSendFileBtn').disabled = true;
    const input = $('lsFileInput');
    if (input) input.value = '';
  }

  // ── 发送文件 ──────────────────────────────────────────────────────────────

  async function sendFile() {
    if (!selectedDevice || !selectedFilePath || isSending) return;
    isSending = true;
    setSendButtonsDisabled(true);
    showProgress();
    try {
      await ipcRenderer.invoke('localsend:send-file', { device: selectedDevice, filePath: selectedFilePath });
    } catch (e) {
      window.rError('LocalSend: 发送文件异常', e.message);
      isSending = false;
      setSendButtonsDisabled(false);
    }
  }

  // ── 发送进度回调 ──────────────────────────────────────────────────────────

  function onSendProgress(data) {
    const pct = Math.round((data.progress || 0) * 100);
    updateProgress(pct, data.state === 'connecting' ? '连接中...' : `上传 ${pct}%`);
  }

  function onSendComplete() {
    updateProgress(100, '发送完成');
    isSending = false;
    setSendButtonsDisabled(false);
    setTimeout(hideProgress, 2000);
  }

  function onSendError(data) {
    updateProgress(0, `发送失败: ${data.message}`);
    isSending = false;
    setSendButtonsDisabled(false);
    setTimeout(hideProgress, 3000);
  }

  // ── 接收模式回调 ──────────────────────────────────────────────────────────

  function onReceiveReady(data) {
    const badge = $('lsReceiveIpBadge');
    if (badge) badge.textContent = `${data.ip}:${data.port}`;
  }

  function onReceiveIncoming(data) {
    $('lsIncomingArea').style.display = '';
    $('lsIncomingFrom').textContent = `来自 ${data.fromAlias}`;
    const fileNames = (data.files || []).map(f => f.fileName).join(', ');
    $('lsIncomingFiles').textContent = fileNames || '正在接收...';
    $('lsReceiveProgressFill').style.width = '0%';
    $('lsReceiveProgressLabel').textContent = '0%';
  }

  function onReceiveProgress(data) {
    const pct = Math.round((data.progress || 0) * 100);
    $('lsReceiveProgressFill').style.width = `${pct}%`;
    $('lsReceiveProgressLabel').textContent = `${pct}%`;
  }

  async function onReceiveComplete(data) {
    $('lsReceiveProgressFill').style.width = '100%';
    $('lsReceiveProgressLabel').textContent = '完成';
    setTimeout(hideIncomingArea, 2000);

    // 重新从主进程拉完整历史（新条目已合并进去）
    const items = await ipcRenderer.invoke('localsend:get-received-items');
    renderReceivedList(items);
  }

  function hideIncomingArea() {
    $('lsIncomingArea').style.display = 'none';
  }

  // ── 接收历史渲染 ──────────────────────────────────────────────────────────

  function renderReceivedList(items) {
    const list = $('lsReceivedList');
    const empty = $('lsReceivedEmpty');
    list.querySelectorAll('.received-item').forEach(el => el.remove());

    if (!items || items.length === 0) {
      if (empty) empty.style.display = '';
      return;
    }
    if (empty) empty.style.display = 'none';

    for (const item of items) {
      list.appendChild(buildReceivedItemEl(item));
    }
  }

  function buildReceivedItemEl(item) {
    const isText = item.mimeType?.startsWith('text/') || item.previewText != null;
    const iconSvg = isText ? ICONS.fileText : fileTypeIcon(item.fileName, item.mimeType);
    const timeStr = item.receivedAt ? new Date(item.receivedAt).toLocaleTimeString() : '';

    const el = document.createElement('div');
    el.className = 'received-item';

    const previewHtml = item.previewText
      ? `<div class="received-item-preview">${escapeHtml(item.previewText)}</div>` : '';

    // 操作按钮（用 SVG 图标）
    const revealBtn = item.localPath
      ? `<button class="icon-btn" data-action="reveal" title="在 Finder 中显示">${ICONS.revealFile}</button>` : '';
    const copyBtn = item.previewText
      ? `<button class="icon-btn" data-action="copy" title="复制到剪切板">${ICONS.copyText}</button>` : '';

    el.innerHTML = `
      <div class="received-item-icon">${iconSvg}</div>
      <div class="received-item-body">
        <div class="received-item-name">${escapeHtml(item.fileName)}</div>
        <div class="received-item-meta">
          <span>来自 ${escapeHtml(item.fromAlias || '未知')}</span>
          <span>${formatBytes(item.size || 0)}</span>
          <span>${timeStr}</span>
        </div>
        ${previewHtml}
      </div>
      <div class="received-item-actions">
        ${revealBtn}
        ${copyBtn}
      </div>
    `;

    el.querySelector('[data-action="reveal"]')?.addEventListener('click', () => {
      ipcRenderer.invoke('localsend:reveal-file', { filePath: item.localPath });
    });

    el.querySelector('[data-action="copy"]')?.addEventListener('click', () => {
      if (item.previewText) {
        const { clipboard } = require('electron');
        clipboard.writeText(item.previewText);
        window.rLog('LocalSend: 文字已复制到剪切板');
      }
    });

    return el;
  }

  // ── UI 辅助 ───────────────────────────────────────────────────────────────

  function showProgress() {
    $('lsSendProgressArea').style.display = '';
    updateProgress(0, '准备中...');
  }

  function hideProgress() {
    $('lsSendProgressArea').style.display = 'none';
  }

  function updateProgress(pct, label) {
    $('lsProgressFill').style.width = `${pct}%`;
    $('lsProgressLabel').textContent = label || `${pct}%`;
  }

  function setSendButtonsDisabled(disabled) {
    $('lsSendTextBtn').disabled = disabled;
    $('lsSendFileBtn').disabled = disabled || !selectedFilePath;
    $('lsPasteBtn').disabled = disabled;
  }

  function resetSendState() {
    isSending = false;
    setSendButtonsDisabled(false);
    hideProgress();
  }

  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  function escapeHtml(str) {
    if (typeof str !== 'string') return '';
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // ── 导出 ──────────────────────────────────────────────────────────────────

  window.LocalSendModule = { init, onPageActivate };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    setTimeout(init, 0);
  }

})();
