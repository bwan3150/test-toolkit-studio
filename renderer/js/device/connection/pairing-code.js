// 配对码连接功能模块
// 依赖：需要全局变量 window.AppGlobals, window.AppNotifications
// 依赖函数：refreshConnectedDevices (来自 device-scanner.js), resetQRDisplay (来自 qr-pairing.js)

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 使用配对码连接设备
async function connectWithPairingCode() {
    const { ipcRenderer } = getGlobals();

    try {
        // 获取用户输入的信息
        const deviceIp = document.getElementById('deviceIpInput').value.trim();
        const adbPort = document.getElementById('deviceAdbPortInput').value.trim();
        const pairingPort = document.getElementById('devicePairingPortInput').value.trim();
        const pairingCode = document.getElementById('devicePairingCodeInput').value.trim();

        // 验证输入
        if (!deviceIp || !adbPort || !pairingPort || !pairingCode) {
            window.AppNotifications?.warn('Please fill in all fields');
            return;
        }

        // 验证IP地址格式
        const ipRegex = /^(\d{1,3}\.){3}\d{1,3}$/;
        if (!ipRegex.test(deviceIp)) {
            window.AppNotifications?.error('Invalid IP address format');
            return;
        }

        // 验证端口号
        const adbPortNum = parseInt(adbPort);
        const pairingPortNum = parseInt(pairingPort);
        if (isNaN(adbPortNum) || isNaN(pairingPortNum) || adbPortNum < 1 || adbPortNum > 65535 || pairingPortNum < 1 || pairingPortNum > 65535) {
            window.AppNotifications?.error('Invalid port number (must be 1-65535)');
            return;
        }

        window.AppNotifications?.info('Connecting with pairing code...');

        // 使用配对码连接
        const pairResult = await ipcRenderer.invoke('adb-pair-wireless', deviceIp, pairingPortNum, pairingCode);

        if (!pairResult.success) {
            window.AppNotifications?.error(`Pairing failed: ${pairResult.error}`);
            return;
        }

        window.AppNotifications?.success('Device paired successfully!');

        // 连接到设备
        const connectResult = await ipcRenderer.invoke('adb-connect-wireless', deviceIp, adbPortNum);

        if (connectResult.success) {
            window.AppNotifications?.success('Device connected successfully!');

            // 刷新设备列表
            if (window.DeviceScanner?.refreshConnectedDevices) {
                await window.DeviceScanner.refreshConnectedDevices();
            }

            // 清空输入框
            document.getElementById('deviceIpInput').value = '';
            document.getElementById('deviceIpInput2').value = '';
            document.getElementById('deviceAdbPortInput').value = '';
            document.getElementById('devicePairingPortInput').value = '';
            document.getElementById('devicePairingCodeInput').value = '';

            // 关闭弹窗
            if (window.hideConnectionGuide) {
                window.hideConnectionGuide();
            }
        } else {
            window.AppNotifications?.error(`Connection failed: ${connectResult.error}`);
        }

    } catch (error) {
        window.rError('Pairing with code failed:', error);
        window.AppNotifications?.error(`Pairing failed: ${error.message}`);
    }
}

// 重置配对状态
function resetPairingStatus() {
    // 清空配对码输入框
    const pairingInputs = ['deviceIpInput', 'deviceIpInput2', 'deviceAdbPortInput', 'devicePairingPortInput', 'devicePairingCodeInput'];
    pairingInputs.forEach(id => {
        const element = document.getElementById(id);
        if (element) {
            element.value = '';
        }
    });

    // 重置QR码显示
    if (window.QRPairing?.resetQRDisplay) {
        window.QRPairing.resetQRDisplay();
    }

    // 隐藏配对区域
    const codeSection = document.getElementById('pairingCodeSection');
    const qrSection = document.getElementById('qrCodeSection');
    if (codeSection) codeSection.style.display = 'none';
    if (qrSection) qrSection.style.display = 'none';

    // 重置按钮状态
    const showCodeBtn = document.getElementById('showPairingCodeBtn');
    const showQrBtn = document.getElementById('showQrCodeBtn');
    if (showCodeBtn) showCodeBtn.classList.remove('active');
    if (showQrBtn) showQrBtn.classList.remove('active');
}

// 全局函数
window.connectWithPairingCode = connectWithPairingCode;

// 导出模块
window.PairingCode = {
    connectWithPairingCode,
    resetPairingStatus
};
