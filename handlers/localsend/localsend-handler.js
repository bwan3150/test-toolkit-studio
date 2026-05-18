// LocalSend 协议处理器
// 实现局域网文件/文字传输，兼容 LocalSend 协议 v2.1
// SEND 模式: 仅监听 UDP 多播发现设备，不通告自己，不启动 HTTP 服务器
// RECEIVE 模式: UDP 通告自己 + HTTP 接收服务器

const { ipcMain, dialog } = require('electron');
const dgram = require('dgram');
const http = require('http');
const os = require('os');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const Store = require('electron-store');

const store = new Store();

// 协议常量
const PORT = 53317;
const MULTICAST_GROUP = '224.0.0.167';
const API_VERSION = '2.1';
const API_BASE = '/api/localsend/v2';
const ANNOUNCE_INTERVAL_MS = 5000;
const STALE_AFTER_MS = 15000;
const STORE_KEY_SAVE_DIR = 'localsend.saveDir';
const STORE_KEY_PREFERRED_IFACE = 'localsend.preferredIface';

// 虚拟网卡前缀黑名单：VPN 隧道、Docker、回环、苹果私有接口等
const VIRTUAL_IFACE_PATTERN = /^(lo|utun|ppp|docker|br-|veth|vmnet|virbr|tun\d|tap\d|awdl|llw|gif|stf|pdp_ip|ap\d|bridge|XHC)/i;

// 运行时状态
let mainWindowRef = null;
let udpSocket = null;
let httpServer = null;
let announceTimer = null;
let pruneTimer = null;

const discoveredDevices = new Map(); // ip → { device, lastSeen }
const receivedItems = [];            // 已接收的文件/文字（内存缓存）
const pendingTokens = new Map();     // sessionId → { fileId: token }
const pendingSessions = new Map();   // sessionId → { fromDevice, files }
const receivedBuffers = new Map();   // sessionId → Map<fileId, Buffer>

// ── 工具函数 ──────────────────────────────────────────────────────────────

// 获取所有物理网卡（过滤掉 VPN/Docker/lo 等虚拟接口）
function getPhysicalInterfaces() {
  const ifaces = os.networkInterfaces();
  const result = [];
  for (const [name, addrs] of Object.entries(ifaces)) {
    if (VIRTUAL_IFACE_PATTERN.test(name)) continue;
    for (const addr of addrs) {
      if (addr.family === 'IPv4' && !addr.internal) {
        result.push({ name, ip: addr.address });
      }
    }
  }
  return result;
}

function getLocalIp() {
  const ifaces = getPhysicalInterfaces();
  if (ifaces.length === 0) {
    // 回退：找任意一个非 internal IPv4
    const all = os.networkInterfaces();
    for (const addrs of Object.values(all)) {
      for (const addr of addrs) {
        if (addr.family === 'IPv4' && !addr.internal) return addr.address;
      }
    }
    return '127.0.0.1';
  }
  // 优先使用用户手动选择的接口
  const preferred = store.get(STORE_KEY_PREFERRED_IFACE);
  if (preferred) {
    const found = ifaces.find(i => i.name === preferred);
    if (found) return found.ip;
  }
  return ifaces[0].ip;
}

function getLocalIfaceName() {
  const ifaces = getPhysicalInterfaces();
  if (ifaces.length === 0) return null;
  const preferred = store.get(STORE_KEY_PREFERRED_IFACE);
  if (preferred) {
    const found = ifaces.find(i => i.name === preferred);
    if (found) return found.name;
  }
  return ifaces[0].name;
}

// 获取所有本机 IPv4 地址（用于过滤自身发出的 UDP 包）
function getAllLocalIps() {
  const set = new Set();
  for (const addrs of Object.values(os.networkInterfaces())) {
    for (const addr of addrs) {
      if (addr.family === 'IPv4') set.add(addr.address);
    }
  }
  return set;
}

function getSaveDir() {
  return store.get(STORE_KEY_SAVE_DIR) || path.join(os.homedir(), 'Downloads', 'LocalSend');
}

let _fingerprint = null;
function getMachineFingerprint() {
  if (_fingerprint) return _fingerprint;
  const ifaces = os.networkInterfaces();
  for (const name of Object.keys(ifaces)) {
    for (const iface of ifaces[name]) {
      if (!iface.internal && iface.mac && iface.mac !== '00:00:00:00:00:00') {
        _fingerprint = crypto.createHash('sha1').update(iface.mac).digest('hex').slice(0, 16);
        return _fingerprint;
      }
    }
  }
  _fingerprint = crypto.createHash('sha1').update(os.hostname()).digest('hex').slice(0, 16);
  return _fingerprint;
}

function buildDeviceDto(announce = false) {
  return {
    alias: os.hostname(),
    version: API_VERSION,
    deviceModel: 'Mac',
    deviceType: 'desktop',
    fingerprint: getMachineFingerprint(),
    port: PORT,
    protocol: 'http',
    download: true,
    announce,
  };
}

function emit(channel, data) {
  if (mainWindowRef && !mainWindowRef.isDestroyed()) {
    mainWindowRef.webContents.send(channel, data);
  }
}

// ── 停止所有服务 ──────────────────────────────────────────────────────────

function stopAllServices() {
  clearInterval(announceTimer);
  clearInterval(pruneTimer);
  announceTimer = null;
  pruneTimer = null;

  if (udpSocket) {
    try { udpSocket.close(); } catch (_) {}
    udpSocket = null;
  }
  if (httpServer) {
    httpServer.close();
    httpServer = null;
  }

  discoveredDevices.clear();
}

// ── SEND 模式：UDP 监听发现设备 ───────────────────────────────────────────

function startSendMode() {
  stopAllServices();

  pruneTimer = setInterval(() => {
    const now = Date.now();
    let changed = false;
    for (const [ip, entry] of discoveredDevices) {
      if (now - entry.lastSeen > STALE_AFTER_MS) {
        discoveredDevices.delete(ip);
        changed = true;
      }
    }
    if (changed) emitDevicesUpdated();
  }, 5000);

  const sock = dgram.createSocket({ type: 'udp4', reuseAddr: true });
  udpSocket = sock;

  sock.on('error', (err) => {
    emit('localsend:error', { message: `UDP 错误: ${err.message}` });
  });

  sock.on('message', (msg, rinfo) => {
    const fromIp = rinfo.address;
    // 过滤掉本机所有网卡地址（防止自身回环）
    if (getAllLocalIps().has(fromIp)) return;
    try {
      const dto = JSON.parse(msg.toString());
      if (!dto.fingerprint || !dto.alias) return;
      handleDeviceAnnouncement(dto, fromIp);
    } catch (_) {}
  });

  sock.bind(PORT, () => {
    try {
      // 指定物理网卡 IP 加入多播组，避免 VPN/Docker 接口干扰
      const localIp = getLocalIp();
      sock.addMembership(MULTICAST_GROUP, localIp);
      sock.setMulticastLoopback(false);
    } catch (e) {
      emit('localsend:error', { message: `加入多播组失败: ${e.message}` });
    }
  });
}

function handleDeviceAnnouncement(dto, fromIp) {
  const device = {
    id: dto.fingerprint,
    alias: dto.alias,
    ip: fromIp,
    port: dto.port || PORT,
    deviceType: dto.deviceType || 'mobile',
    fingerprint: dto.fingerprint,
  };
  const existing = discoveredDevices.get(fromIp);
  discoveredDevices.set(fromIp, { device, lastSeen: Date.now() });
  if (!existing) emitDevicesUpdated();
}

function emitDevicesUpdated() {
  const list = Array.from(discoveredDevices.values()).map(e => e.device);
  emit('localsend:devices-updated', list);
}

// ── RECEIVE 模式：UDP 通告 + HTTP 服务器 ──────────────────────────────────

function startReceiveMode() {
  stopAllServices();
  startHttpServer();
  startUdpAnnouncer();
}

function startUdpAnnouncer() {
  announcePresence();
  announceTimer = setInterval(announcePresence, ANNOUNCE_INTERVAL_MS);
}

function announcePresence() {
  const dto = buildDeviceDto(true);
  const msg = Buffer.from(JSON.stringify(dto));
  const sock = dgram.createSocket('udp4');
  const localIp = getLocalIp();
  // 绑定到物理网卡 IP，确保多播包从正确接口发出（而非 VPN 接口）
  sock.bind(0, localIp, () => {
    sock.send(msg, 0, msg.length, PORT, MULTICAST_GROUP, () => sock.close());
  });
}

function startHttpServer() {
  const localIp = getLocalIp();
  const localDto = buildDeviceDto();

  const server = http.createServer((req, res) => {
    const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
    const pathname = url.pathname;

    const readJson = () => new Promise((resolve, reject) => {
      let body = '';
      req.on('data', c => { body += c; });
      req.on('end', () => {
        try { resolve(JSON.parse(body)); } catch (e) { reject(e); }
      });
      req.on('error', reject);
    });

    const readBytes = () => new Promise((resolve) => {
      const chunks = [];
      req.on('data', c => chunks.push(c));
      req.on('end', () => resolve(Buffer.concat(chunks)));
      req.on('error', () => resolve(Buffer.alloc(0)));
    });

    const sendJson = (statusCode, obj) => {
      const body = JSON.stringify(obj);
      res.writeHead(statusCode, { 'Content-Type': 'application/json' });
      res.end(body);
    };

    if (req.method === 'GET' && pathname === `${API_BASE}/info`) {
      sendJson(200, localDto);
      return;
    }

    if (req.method === 'POST' && pathname === `${API_BASE}/register`) {
      readJson().then(() => sendJson(200, localDto)).catch(() => res.writeHead(400).end());
      return;
    }

    if (req.method === 'POST' && pathname === `${API_BASE}/prepare-upload`) {
      readJson().then(reqBody => {
        const sessionId = crypto.randomUUID();
        const tokens = {};
        for (const fileId of Object.keys(reqBody.files || {})) {
          tokens[fileId] = crypto.randomUUID();
        }
        pendingTokens.set(sessionId, tokens);
        pendingSessions.set(sessionId, {
          fromDevice: reqBody.info,
          files: Object.values(reqBody.files || {}),
        });
        receivedBuffers.set(sessionId, new Map());

        emit('localsend:receive-incoming', {
          sessionId,
          fromAlias: reqBody.info?.alias || 'Unknown device',
          files: Object.values(reqBody.files || {}),
        });

        sendJson(200, { sessionId, files: tokens });
      }).catch(() => res.writeHead(400).end());
      return;
    }

    if (req.method === 'POST' && pathname === `${API_BASE}/upload`) {
      const sessionId = url.searchParams.get('sessionId');
      const fileId = url.searchParams.get('fileId');
      const token = url.searchParams.get('token');

      if (!sessionId || !fileId || !token) { res.writeHead(400).end(); return; }
      if (pendingTokens.get(sessionId)?.[fileId] !== token) { res.writeHead(403).end(); return; }

      readBytes().then(bytes => {
        const sessionBuffers = receivedBuffers.get(sessionId);
        if (sessionBuffers) sessionBuffers.set(fileId, bytes);

        const expected = Object.keys(pendingTokens.get(sessionId) || {});
        const got = Array.from((receivedBuffers.get(sessionId) || new Map()).keys());
        const progress = expected.length ? got.length / expected.length : 0;

        emit('localsend:receive-progress', { sessionId, progress });

        if (expected.length > 0 && got.length >= expected.length) {
          finalizeSession(sessionId);
        }
        res.writeHead(200).end();
      });
      return;
    }

    if (req.method === 'POST' && pathname === `${API_BASE}/cancel`) {
      const sessionId = url.searchParams.get('sessionId');
      if (sessionId) {
        pendingTokens.delete(sessionId);
        pendingSessions.delete(sessionId);
        receivedBuffers.delete(sessionId);
        emit('localsend:receive-cancelled', { sessionId });
      }
      res.writeHead(200).end();
      return;
    }

    res.writeHead(404).end();
  });

  // 绑定到 0.0.0.0（接受所有接口的入站连接），对外通告物理网卡 IP
  server.listen(PORT, '0.0.0.0', () => {
    emit('localsend:receive-ready', { ip: localIp, port: PORT, ifaceName: getLocalIfaceName() });
  });

  server.on('error', (err) => {
    emit('localsend:error', { message: `HTTP 服务器错误: ${err.message}` });
  });

  httpServer = server;
}

// 会话完成：保存文件到用户指定目录
function finalizeSession(sessionId) {
  const session = pendingSessions.get(sessionId);
  const buffers = receivedBuffers.get(sessionId);
  if (!session || !buffers) return;

  const saveDir = getSaveDir();
  if (!fs.existsSync(saveDir)) fs.mkdirSync(saveDir, { recursive: true });

  const items = [];
  for (const [fileId, bytes] of buffers) {
    const meta = session.files.find(f => f.id === fileId);
    if (!meta) continue;

    const isText = meta.fileType?.startsWith('text/') || meta.fileName?.endsWith('.txt');
    let savedPath = null;
    let previewText = null;

    if (isText) {
      previewText = bytes.toString('utf8');
    }

    const filePath = path.join(saveDir, meta.fileName);
    try {
      fs.writeFileSync(filePath, bytes);
      savedPath = filePath;
    } catch (_) {}

    items.push({
      id: meta.id,
      fileName: meta.fileName,
      mimeType: meta.fileType,
      size: meta.size,
      localPath: savedPath,
      previewText,
      fromAlias: session.fromDevice?.alias || 'Unknown',
      receivedAt: Date.now(),
    });
  }

  receivedItems.unshift(...items);
  if (receivedItems.length > 50) receivedItems.splice(50);

  pendingTokens.delete(sessionId);
  pendingSessions.delete(sessionId);
  receivedBuffers.delete(sessionId);

  emit('localsend:receive-complete', { items });
}

// ── 发送逻辑 ──────────────────────────────────────────────────────────────

function httpPost(targetIp, targetPort, urlPath, body) {
  return new Promise((resolve, reject) => {
    const buf = Buffer.from(body);
    const req = http.request({
      hostname: targetIp, port: targetPort, path: urlPath, method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Content-Length': buf.length },
    }, (res) => {
      let data = '';
      res.on('data', c => { data += c; });
      res.on('end', () => {
        if (res.statusCode >= 200 && res.statusCode < 300) {
          try { resolve(JSON.parse(data)); } catch { resolve({}); }
        } else {
          reject(new Error(`HTTP ${res.statusCode}: ${data}`));
        }
      });
    });
    req.on('error', reject);
    req.end(buf);
  });
}

function httpPostBytes(targetIp, targetPort, urlPath, contentType, bytes) {
  return new Promise((resolve, reject) => {
    const req = http.request({
      hostname: targetIp, port: targetPort, path: urlPath, method: 'POST',
      headers: { 'Content-Type': contentType, 'Content-Length': bytes.length },
    }, (res) => {
      res.resume();
      res.on('end', () => {
        if (res.statusCode >= 200 && res.statusCode < 300) resolve();
        else reject(new Error(`HTTP ${res.statusCode}`));
      });
    });
    req.on('error', reject);
    req.end(bytes);
  });
}

async function sendPayload(device, files) {
  const localDto = buildDeviceDto();
  const filesMap = {};
  for (const f of files) {
    filesMap[f.id] = { id: f.id, fileName: f.fileName, size: f.bytes.length, fileType: f.fileType };
  }

  emit('localsend:send-progress', { progress: 0, state: 'connecting' });

  const prepBody = JSON.stringify({ info: localDto, files: filesMap });
  const prepResp = await httpPost(device.ip, device.port, `${API_BASE}/prepare-upload`, prepBody);

  const sessionId = prepResp.sessionId;
  const tokenMap = prepResp.files || {};

  emit('localsend:send-progress', { progress: 0, state: 'uploading', sessionId });

  for (let i = 0; i < files.length; i++) {
    const f = files[i];
    const token = tokenMap[f.id];
    if (!token) continue;

    const params = new URLSearchParams({ sessionId, fileId: f.id, token });
    await httpPostBytes(device.ip, device.port, `${API_BASE}/upload?${params}`,
      f.fileType || 'application/octet-stream', f.bytes);

    emit('localsend:send-progress', { progress: (i + 1) / files.length, state: 'uploading', sessionId });
  }

  emit('localsend:send-complete', { sessionId, fileCount: files.length });
}

// ── IPC 注册 ──────────────────────────────────────────────────────────────

function registerLocalSendHandlers(mainWindow) {
  mainWindowRef = mainWindow;

  ipcMain.handle('localsend:get-local-ip', () => getLocalIp());

  ipcMain.handle('localsend:get-device-info', () => ({
    ...buildDeviceDto(),
    ip: getLocalIp(),
    ifaceName: getLocalIfaceName(),
  }));

  // 获取所有可用物理网卡列表
  ipcMain.handle('localsend:get-network-interfaces', () => getPhysicalInterfaces());

  // 设置用户偏好网卡（持久化），重启后生效
  ipcMain.handle('localsend:set-preferred-interface', (_, ifaceName) => {
    store.set(STORE_KEY_PREFERRED_IFACE, ifaceName || null);
    return { success: true };
  });

  ipcMain.handle('localsend:start-send-mode', () => {
    try { startSendMode(); return { success: true }; }
    catch (e) { return { success: false, error: e.message }; }
  });

  ipcMain.handle('localsend:start-receive-mode', () => {
    try { startReceiveMode(); return { success: true }; }
    catch (e) { return { success: false, error: e.message }; }
  });

  ipcMain.handle('localsend:stop', () => {
    stopAllServices();
    return { success: true };
  });

  ipcMain.handle('localsend:get-devices', () =>
    Array.from(discoveredDevices.values()).map(e => e.device)
  );

  ipcMain.handle('localsend:send-text', async (_, { device, text }) => {
    try {
      const bytes = Buffer.from(text, 'utf8');
      await sendPayload(device, [{
        id: crypto.randomUUID(),
        fileName: `message_${Date.now()}.txt`,
        fileType: 'text/plain',
        bytes,
      }]);
      return { success: true };
    } catch (e) {
      emit('localsend:send-error', { message: e.message });
      return { success: false, error: e.message };
    }
  });

  ipcMain.handle('localsend:send-file', async (_, { device, filePath }) => {
    try {
      const bytes = fs.readFileSync(filePath);
      const fileName = path.basename(filePath);
      const ext = path.extname(fileName).toLowerCase().slice(1);
      const mimeMap = {
        jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png',
        gif: 'image/gif', webp: 'image/webp', svg: 'image/svg+xml',
        mp4: 'video/mp4', mov: 'video/quicktime', avi: 'video/x-msvideo',
        mp3: 'audio/mpeg', wav: 'audio/wav', ogg: 'audio/ogg',
        pdf: 'application/pdf', zip: 'application/zip',
        txt: 'text/plain', json: 'application/json',
        html: 'text/html', css: 'text/css', js: 'application/javascript',
      };
      await sendPayload(device, [{
        id: crypto.randomUUID(),
        fileName,
        fileType: mimeMap[ext] || 'application/octet-stream',
        bytes,
      }]);
      return { success: true };
    } catch (e) {
      emit('localsend:send-error', { message: e.message });
      return { success: false, error: e.message };
    }
  });

  ipcMain.handle('localsend:get-received-items', () => [...receivedItems]);

  ipcMain.handle('localsend:clear-received', () => {
    receivedItems.splice(0);
    return { success: true };
  });

  // 获取保存目录
  ipcMain.handle('localsend:get-save-dir', () => getSaveDir());

  // 设置保存目录（直接传路径）
  ipcMain.handle('localsend:set-save-dir', (_, dirPath) => {
    store.set(STORE_KEY_SAVE_DIR, dirPath);
    return { success: true };
  });

  // 打开文件夹选择对话框，选中后自动保存
  ipcMain.handle('localsend:choose-save-dir', async () => {
    const result = await dialog.showOpenDialog(mainWindowRef, {
      title: '选择文件保存位置',
      defaultPath: getSaveDir(),
      properties: ['openDirectory', 'createDirectory'],
    });
    if (result.canceled || !result.filePaths.length) {
      return { success: false, canceled: true };
    }
    const chosen = result.filePaths[0];
    store.set(STORE_KEY_SAVE_DIR, chosen);
    return { success: true, dirPath: chosen };
  });

  ipcMain.handle('localsend:reveal-file', (_, { filePath }) => {
    const { shell } = require('electron');
    if (fs.existsSync(filePath)) {
      shell.showItemInFolder(filePath);
      return { success: true };
    }
    return { success: false, error: '文件不存在' };
  });

  ipcMain.handle('localsend:read-clipboard', () => {
    const { clipboard } = require('electron');
    return clipboard.readText();
  });
}

module.exports = { registerLocalSendHandlers };
