// QR码配对功能模块
// 依赖：需要全局变量 window.AppGlobals, window.AppNotifications
// 依赖函数：refreshConnectedDevices (来自 device-scanner.js)

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// QR码相关变量
let qrTimer = null;
let qrExpiryTime = null;

// 生成QR码
async function generateQRCode() {
    const { ipcRenderer } = getGlobals();

    try {
        // 显示加载状态
        window.AppNotifications?.info('正在生成QR码...');

        // 生成配对数据
        const pairingDataResult = await ipcRenderer.invoke('generate-qr-pairing-data');

        if (!pairingDataResult.success) {
            window.AppNotifications?.error(`生成配对数据失败: ${pairingDataResult.error}`);
            return;
        }

        const { serviceName, pairingCode, qrData, expiryTime, localIP, pairingPort, adbPort } = pairingDataResult;

        // 生成QR码图片
        const qrResult = await ipcRenderer.invoke('generate-qr-code', qrData, { width: 200 });

        if (!qrResult.success) {
            window.AppNotifications?.error(`生成QR码失败: ${qrResult.error}`);
            return;
        }

        // 显示QR码
        displayQRCode(qrResult.dataURL, serviceName, pairingCode, expiryTime, localIP, pairingPort);

        // 启动配对服务
        const serviceResult = await ipcRenderer.invoke('start-adb-pairing-service', serviceName, pairingCode, localIP, pairingPort);

        if (serviceResult.success) {
            window.AppNotifications?.success('QR码生成成功，请在设备上扫描');

            // 开始检查配对状态
            startPairingStatusCheck();
        } else {
            window.AppNotifications?.warn(`启动配对服务失败: ${serviceResult.error}`);
        }

    } catch (error) {
        window.rError('生成QR码失败:', error);
        window.AppNotifications?.error(`生成QR码失败: ${error.message}`);
    }
}

// 显示QR码
function displayQRCode(dataURL, serviceName, pairingCode, expiryTime, localIP, pairingPort) {
    const qrDisplay = document.getElementById('qrDisplay');
    const qrCanvas = document.getElementById('qrCanvas');
    const qrInfo = document.getElementById('qrInfo');
    const generateBtn = document.getElementById('generateQrBtn');
    const refreshBtn = document.getElementById('refreshQrBtn');

    if (!qrDisplay || !qrCanvas || !qrInfo) {
        window.rError('QR code display elements not found');
        return;
    }

    const qrPlaceholder = qrDisplay.querySelector('.qr-placeholder');

    // 隐藏占位符，显示QR码
    if (qrPlaceholder) qrPlaceholder.style.display = 'none';
    qrCanvas.style.display = 'block';

    // 设置QR码图片
    const img = new Image();
    img.onload = function() {
        const ctx = qrCanvas.getContext('2d');
        qrCanvas.width = img.width;
        qrCanvas.height = img.height;
        ctx.drawImage(img, 0, 0);
    };
    img.src = dataURL;

    // 更新配对信息显示
    const qrInfoHTML = `
        <div class="info-item">
            <label>Pairing Code:</label>
            <span class="highlight-code">${pairingCode}</span>
        </div>
        <div class="info-item">
            <label>Pairing Port:</label>
            <span class="highlight-code">${pairingPort}</span>
        </div>
        <div class="info-item">
            <label>Local IP:</label>
            <span>${localIP}</span>
        </div>
        <div class="info-item">
            <label>Service Name:</label>
            <span class="highlight-code">${serviceName}</span>
        </div>
    `;
    qrInfo.innerHTML = qrInfoHTML;
    qrInfo.style.display = 'block';

    // 切换按钮状态
    if (generateBtn) generateBtn.style.display = 'none';
    if (refreshBtn) refreshBtn.style.display = 'block';

    // 启动倒计时
    qrExpiryTime = expiryTime;
    startQRTimer();
}

// 启动QR码倒计时
function startQRTimer() {
    if (qrTimer) {
        clearInterval(qrTimer);
    }

    qrTimer = setInterval(() => {
        const now = Date.now();
        const timeLeft = qrExpiryTime - now;

        if (timeLeft <= 0) {
            // 过期
            clearInterval(qrTimer);
            resetQRDisplay();
            window.AppNotifications?.warn('QR码已过期，请重新生成');
            return;
        }

        // 更新倒计时显示
        const minutes = Math.floor(timeLeft / 60000);
        const seconds = Math.floor((timeLeft % 60000) / 1000);
        const timerValue = document.getElementById('timerValue');
        if (timerValue) {
            timerValue.textContent = `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
        }
    }, 1000);
}

// 重置QR码显示
function resetQRDisplay() {
    const qrDisplay = document.getElementById('qrDisplay');
    const qrCanvas = document.getElementById('qrCanvas');
    const qrInfo = document.getElementById('qrInfo');
    const generateBtn = document.getElementById('generateQrBtn');
    const refreshBtn = document.getElementById('refreshQrBtn');

    if (qrDisplay) {
        const qrPlaceholder = qrDisplay.querySelector('.qr-placeholder');
        if (qrPlaceholder) {
            qrPlaceholder.style.display = 'block';
        }
    }

    // 显示占位符，隐藏QR码
    if (qrCanvas) qrCanvas.style.display = 'none';
    if (qrInfo) qrInfo.style.display = 'none';

    // 重置按钮状态
    if (generateBtn) generateBtn.style.display = 'block';
    if (refreshBtn) refreshBtn.style.display = 'none';

    // 清理倒计时
    if (qrTimer) {
        clearInterval(qrTimer);
        qrTimer = null;
    }
}

// 检查配对状态
async function startPairingStatusCheck() {
    const { ipcRenderer } = getGlobals();
    let checkCount = 0;
    const maxChecks = 60; // 最多检查5分钟

    const checkInterval = setInterval(async () => {
        checkCount++;

        try {
            const statusResult = await ipcRenderer.invoke('check-pairing-status');

            if (statusResult.success && statusResult.hasNewDevices) {
                // 发现新设备，配对可能成功
                clearInterval(checkInterval);
                window.AppNotifications?.success('检测到新设备连接，配对可能成功！');

                // 刷新设备列表
                if (window.DeviceScanner?.refreshConnectedDevices) {
                    await window.DeviceScanner.refreshConnectedDevices();
                }

                // 重置QR码显示
                resetQRDisplay();
                return;
            }

            if (checkCount >= maxChecks) {
                // 检查超时
                clearInterval(checkInterval);
                window.rLog('配对状态检查超时');
            }
        } catch (error) {
            window.rError('检查配对状态失败:', error);
        }
    }, 5000); // 每5秒检查一次
}

// 全局函数
window.generateQRCode = generateQRCode;

// 导出模块
window.QRPairing = {
    generateQRCode,
    displayQRCode,
    startQRTimer,
    resetQRDisplay,
    startPairingStatusCheck
};
