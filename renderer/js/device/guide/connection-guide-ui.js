// 连接向导UI模块
// 负责显示和管理设备连接向导界面
// 依赖：无

// 获取全局变量的辅助函数
function getGlobals() {
    return window.AppGlobals;
}

// 显示连接向导
function showConnectionGuide() {
    const modal = document.getElementById('connectionGuideModal');
    const deviceForm = document.getElementById('deviceForm');

    // 隐藏添加设备表单
    if (deviceForm) {
        deviceForm.style.display = 'none';
    }

    // 显示连接向导模态框
    if (modal) {
        modal.style.display = 'flex';

        // 重置到USB连接指导（恢复原来的行为）
        showConnectionMethod('usb');

        // 重置配对状态
        resetPairingStatus();
    }
}

// 隐藏连接向导
function hideConnectionGuide() {
    const modal = document.getElementById('connectionGuideModal');
    if (modal) {
        modal.style.display = 'none';
    }

    // 重置配对状态
    resetPairingStatus();
}

// 初始化连接方式切换器
function initializeMethodSwitcher() {
    const methodBtns = document.querySelectorAll('.method-btn');
    const methodContents = document.querySelectorAll('.method-content');

    methodBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const method = btn.dataset.method;

            // 切换按钮状态
            methodBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            // 切换内容显示
            methodContents.forEach(content => {
                if (content.id === method + 'Method') {
                    content.classList.add('active');
                } else {
                    content.classList.remove('active');
                }
            });

            // 重置配对状态
            resetPairingStatus();
        });
    });
}

// 初始化连接向导标签页切换
function initializeConnectionTabs() {
    const tabButtons = document.querySelectorAll('.tab-button');
    const tabContents = document.querySelectorAll('.tab-content');

    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabName = btn.dataset.tab;

            // 切换按钮状态
            tabButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            // 切换内容显示
            tabContents.forEach(content => {
                content.classList.remove('active');
                if (content.id === tabName + 'Tab') {
                    content.classList.add('active');
                }
            });

            // 重置配对状态
            resetPairingStatus();
        });
    });
}

// 显示连接方法
function showConnectionMethod(method) {
    const tabButtons = document.querySelectorAll('.tab-button');
    const tabContents = document.querySelectorAll('.tab-content');

    // 切换按钮状态
    tabButtons.forEach(btn => {
        btn.classList.remove('active');
        if (btn.dataset.tab === method) {
            btn.classList.add('active');
        }
    });

    // 切换内容显示
    tabContents.forEach(content => {
        content.classList.remove('active');
        if (content.id === method + 'Tab') {
            content.classList.add('active');
        }
    });
}

// 初始化IP地址同步
function initializeIpSync() {
    const deviceIpInput = document.getElementById('deviceIpInput');
    const deviceIpInput2 = document.getElementById('deviceIpInput2');

    if (deviceIpInput && deviceIpInput2) {
        deviceIpInput.addEventListener('input', (e) => {
            deviceIpInput2.value = e.target.value;
        });

        // 初始化时也同步一次
        deviceIpInput2.value = deviceIpInput.value;
    }
}

// 显示配对方式
function showPairingMethod(method) {
    const codeSection = document.getElementById('pairingCodeSection');
    const qrSection = document.getElementById('qrCodeSection');
    const showCodeBtn = document.getElementById('showPairingCodeBtn');
    const showQrBtn = document.getElementById('showQrCodeBtn');

    if (!codeSection || !qrSection) {
        window.rError('配对区域元素未找到');
        return;
    }

    if (method === 'code') {
        codeSection.style.display = 'block';
        qrSection.style.display = 'none';
        if (showCodeBtn) showCodeBtn.classList.add('active');
        if (showQrBtn) showQrBtn.classList.remove('active');
    } else {
        codeSection.style.display = 'none';
        qrSection.style.display = 'block';
        if (showCodeBtn) showCodeBtn.classList.remove('active');
        if (showQrBtn) showQrBtn.classList.add('active');
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
    if (window.ConnectionQR && window.ConnectionQR.resetQRDisplay) {
        window.ConnectionQR.resetQRDisplay();
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

// 导出模块
window.ConnectionGuideUI = {
    showConnectionGuide,
    hideConnectionGuide,
    initializeMethodSwitcher,
    initializeConnectionTabs,
    showConnectionMethod,
    initializeIpSync,
    showPairingMethod
};

// 注册全局函数（为了向后兼容）
window.showConnectionGuide = showConnectionGuide;
window.hideConnectionGuide = hideConnectionGuide;
window.showConnectionMethod = showConnectionMethod;
window.initializeConnectionTabs = initializeConnectionTabs;
window.initializeIpSync = initializeIpSync;
window.showPairingMethod = showPairingMethod;
