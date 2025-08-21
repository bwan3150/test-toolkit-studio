# Windows iOS Autotest Scheme

## 核心逻辑

### Mac不在同局域网

```
Mac服务器(远程) → 部署WDA → iOS设备(本地WiFi)
Windows(本地) ─────────直连────→ iOS设备(本地WiFi)
```

**关键**: Mac只负责部署，部署完成后网络无关

-   Mac可以在云端、公司机房、任意位置
-   只要Mac能SSH/远程访问iOS设备进行部署即可
-   Windows和iOS只需在同一局域网即可直连

## 核心实施方案

### 1. 远程Mac部署WDA

```bash
# Mac远程部署(通过SSH或远程桌面)
xcodebuild -project WebDriverAgent.xcodeproj \
           -scheme WebDriverAgentRunner \
           -destination 'platform=iOS,id=设备UDID' \
           test

# 成功后iOS设备启动WDA服务: http://iOS_IP:8100
```

### 2. 获取iOS设备局域网IP

```bash
# 方法1: Mac终端查看
arp -a | grep "你的iPhone名称"

# 方法2: iOS设备查看
# 设置 -> Wi-Fi -> 点击已连接网络 -> 查看IP地址
# 例如: 192.168.1.100
```

### 3. Windows端JS直连控制

#### WiFi直连版本

```javascript
// 使用webdriverio或axios直接HTTP调用WDA
const axios = require('axios');

class WDAController {
    constructor(iosIP) {
        this.baseURL = `http://${iosIP}:8100`;
    }
    
    // 创建会话
    async createSession() {
        const response = await axios.post(`${this.baseURL}/session`, {
            capabilities: {
                platformName: 'iOS',
                automationName: 'XCUITest'
            }
        });
        this.sessionId = response.data.sessionId;
        return this.sessionId;
    }
    
    // 截图
    async takeScreenshot() {
        const response = await axios.get(`${this.baseURL}/session/${this.sessionId}/screenshot`);
        return response.data.value; // base64图片
    }
    
    // 点击元素
    async tapElement(elementId) {
        await axios.post(`${this.baseURL}/session/${this.sessionId}/element/${elementId}/click`);
    }
    
    // 查找元素
    async findElement(strategy, selector) {
        const response = await axios.post(`${this.baseURL}/session/${this.sessionId}/element`, {
            using: strategy, // 'accessibility id', 'xpath', etc.
            value: selector
        });
        return response.data.value.ELEMENT;
    }
}

// 使用示例 - WiFi直连
const wda = new WDAController('192.168.1.100');
await wda.createSession();
const elementId = await wda.findElement('accessibility id', 'Settings');
await wda.tapElement(elementId);
const screenshot = await wda.takeScreenshot();
```

#### USB转发版本

```javascript
// 先启动USB端口转发
const { spawn } = require('child_process');

class USBWDAController {
    constructor(deviceUDID) {
        this.deviceUDID = deviceUDID;
        this.baseURL = 'http://localhost:8100';
        this.proxyProcess = null;
    }
    
    // 启动USB转发
    async startUSBForwarding() {
        return new Promise((resolve, reject) => {
            // 启动iproxy进程
            this.proxyProcess = spawn('iproxy', ['8100', '8100', this.deviceUDID]);
            
            this.proxyProcess.stdout.on('data', (data) => {
                console.log(`iproxy: ${data}`);
                // 等待转发建立
                setTimeout(() => resolve(), 2000);
            });
            
            this.proxyProcess.on('error', (err) => {
                reject(`USB转发失败: ${err}`);
            });
        });
    }
    
    // 创建会话 (同WiFi版本，只是URL不同)
    async createSession() {
        const response = await axios.post(`${this.baseURL}/session`, {
            capabilities: {
                platformName: 'iOS',
                automationName: 'XCUITest'
            }
        });
        this.sessionId = response.data.sessionId;
        return this.sessionId;
    }
    
    // 其他方法与WiFi版本相同...
    
    // 清理USB转发
    cleanup() {
        if (this.proxyProcess) {
            this.proxyProcess.kill();
        }
    }
}

// 使用示例 - USB转发
const usbWDA = new USBWDAController('你的设备UDID');
await usbWDA.startUSBForwarding();
await usbWDA.createSession();
// ... 后续操作相同
```

## 完整工作流程

### 部署阶段

```bash
# 1. 远程Mac部署(一次性操作)
ssh mac-server "cd /path/to/WebDriverAgent && xcodebuild -project WebDriverAgent.xcodeproj -scheme WebDriverAgentRunner -destination 'platform=iOS,id=设备UDID' test"

# 2. 确认iOS设备WDA启动
curl http://iOS设备IP:8100/status
```

### Windows自动化阶段

```javascript
// 3. Windows端自动化测试
const automation = new SmartWDAController('192.168.1.100', '设备UDID');

async function runTest() {
    // 智能连接
    await automation.connect();
    
    // 创建会话
    await automation.createSession();
    
    // 执行自动化测试
    const settingsElement = await automation.findElement('accessibility id', 'Settings');
    await automation.tapElement(settingsElement);
    
    // 截图验证
    const screenshot = await automation.takeScreenshot();
    console.log('测试完成，截图已保存');
    
    // 清理资源
    automation.cleanup?.();
}

runTest().catch(console.error);
```

## 工作流程

```
1. Mac部署WDA → iOS设备启动服务(8100端口)
2. 记录iOS设备IP → 192.168.1.100:8100  
3. Windows直连测试 → 无延迟自动化控制
4. 一次部署 → 持续可用(直到iOS重启)
```

## 优势

-   **超简单**: 3步完成，无需额外开发
-   **零延迟**: Windows直连iOS，最佳性能
-   **即时验证**: 快速测试可行性
-   **成本最低**: 利用现有工具和网络

**核心价值**: 最快验证直连方案的可行性，为后续自动化部署打基础。



## 融入当前逻辑

1.   在Device页面, 可以将iOS(通过MAC部署好WDA的设备) 以Wi-Fi或USB方式, 保存在Config, 观察其可用状态, 拖入安装包进行安装的功能禁用
2.   在Log Viewer页面, 可以选择iOS设备, 进行log抓取和log导出
3.   在Testcase页面, 可以通过截图和xml结构获取, 同样的在右侧显示DEVICE SCREEN进行元素的抓取和存储用于后续测试
4.   ToolkitScript脚本转译执行的时候, 当连接到iOS设备, 需要转译成WDA的操作方法, 也就是现在一句脚本可以根据设备的Android/iOS执行时自动按照adb/WDA两种方案执行其中一种; 脚本语法参考 The_ToolkitScript_Reference.md