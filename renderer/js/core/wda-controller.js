// WebDriverAgent 控制器类
// 支持 iOS 设备的自动化测试操作

class WDAController {
    constructor(deviceConfig) {
        this.deviceConfig = deviceConfig;
        this.sessionId = null;
        this.proxyProcess = null;
        this.isConnected = false;
        
        // 根据连接类型确定 baseURL
        if (deviceConfig.connectionType === 'wifi') {
            this.baseURL = `http://${deviceConfig.ipAddress}:${deviceConfig.port || 8100}`;
        } else if (deviceConfig.connectionType === 'usb') {
            this.baseURL = 'http://localhost:8100';
        } else {
            throw new Error('不支持的连接类型');
        }
        
        console.log('WDA Controller 初始化:', {
            deviceId: deviceConfig.deviceId,
            connectionType: deviceConfig.connectionType,
            baseURL: this.baseURL
        });
    }
    
    // 连接到设备
    async connect() {
        try {
            if (this.deviceConfig.connectionType === 'usb') {
                // USB 连接需要启动端口转发
                await this.startUSBForwarding();
            }
            
            // 检查 WDA 服务状态
            const statusResponse = await this.makeRequest('GET', '/status');
            if (statusResponse.ok) {
                this.isConnected = true;
                console.log('WDA 服务连接成功');
                return { success: true };
            } else {
                throw new Error('WDA 服务不可用');
            }
        } catch (error) {
            console.error('WDA 连接失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 启动 USB 端口转发
    async startUSBForwarding() {
        const { ipcRenderer } = window.AppGlobals;
        
        if (!this.deviceConfig.udid) {
            throw new Error('USB 连接需要设备 UDID');
        }
        
        try {
            const result = await ipcRenderer.invoke('start-ios-usb-forwarding', {
                deviceUDID: this.deviceConfig.udid,
                localPort: 8100,
                devicePort: 8100
            });
            
            if (!result.success) {
                throw new Error(`USB 转发失败: ${result.error}`);
            }
            
            console.log('iOS USB 端口转发启动成功');
            // 等待端口转发建立
            await this.sleep(2000);
            
        } catch (error) {
            throw new Error(`启动 USB 转发失败: ${error.message}`);
        }
    }
    
    // 创建 WDA 会话
    async createSession() {
        try {
            const response = await this.makeRequest('POST', '/session', {
                capabilities: {
                    platformName: 'iOS',
                    automationName: 'XCUITest',
                    deviceName: this.deviceConfig.name || 'iOS Device',
                    udid: this.deviceConfig.udid
                }
            });
            
            if (response.ok) {
                const data = await response.json();
                this.sessionId = data.sessionId || data.value.sessionId;
                console.log('WDA 会话创建成功:', this.sessionId);
                return { success: true, sessionId: this.sessionId };
            } else {
                const errorData = await response.json();
                throw new Error(`会话创建失败: ${errorData.value?.message || response.statusText}`);
            }
        } catch (error) {
            console.error('创建 WDA 会话失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 截图
    async takeScreenshot() {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('GET', `/session/${this.sessionId}/screenshot`);
            if (response.ok) {
                const data = await response.json();
                return {
                    success: true,
                    screenshot: data.value // base64 编码的图片
                };
            } else {
                throw new Error('截图失败');
            }
        } catch (error) {
            console.error('截图失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 获取页面源码 (XML)
    async getPageSource() {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('GET', `/session/${this.sessionId}/source`);
            if (response.ok) {
                const data = await response.json();
                return {
                    success: true,
                    source: data.value
                };
            } else {
                throw new Error('获取页面源码失败');
            }
        } catch (error) {
            console.error('获取页面源码失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 点击坐标
    async tapCoordinate(x, y) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/wda/tap/0`, {
                x: x,
                y: y
            });
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('点击失败');
            }
        } catch (error) {
            console.error('点击坐标失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 查找元素
    async findElement(strategy, selector) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/element`, {
                using: strategy, // 'accessibility id', 'xpath', 'class name', etc.
                value: selector
            });
            
            if (response.ok) {
                const data = await response.json();
                return {
                    success: true,
                    elementId: data.value.ELEMENT || data.value['element-6066-11e4-a52e-4f735466cecf']
                };
            } else {
                throw new Error('元素未找到');
            }
        } catch (error) {
            console.error('查找元素失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 点击元素
    async tapElement(elementId) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/element/${elementId}/click`);
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('点击元素失败');
            }
        } catch (error) {
            console.error('点击元素失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 长按元素
    async pressElement(elementId, duration = 1000) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            // 先获取元素位置
            const locationResponse = await this.makeRequest('GET', `/session/${this.sessionId}/element/${elementId}/location`);
            if (!locationResponse.ok) {
                throw new Error('获取元素位置失败');
            }
            
            const locationData = await locationResponse.json();
            const { x, y } = locationData.value;
            
            // 执行长按
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/wda/touchAndHold`, {
                x: x,
                y: y,
                duration: duration / 1000 // WDA 需要秒为单位
            });
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('长按失败');
            }
        } catch (error) {
            console.error('长按元素失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 滑动
    async swipe(startX, startY, endX, endY, duration = 1000) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/wda/dragfromtoforduration`, {
                fromX: startX,
                fromY: startY,
                toX: endX,
                toY: endY,
                duration: duration / 1000 // WDA 需要秒为单位
            });
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('滑动失败');
            }
        } catch (error) {
            console.error('滑动失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 输入文本
    async sendKeys(elementId, text) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/element/${elementId}/value`, {
                value: text.split('')
            });
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('输入文本失败');
            }
        } catch (error) {
            console.error('输入文本失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 清除文本
    async clearText(elementId) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/element/${elementId}/clear`);
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('清除文本失败');
            }
        } catch (error) {
            console.error('清除文本失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 隐藏键盘
    async hideKeyboard() {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/wda/keyboard/dismiss`);
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('隐藏键盘失败');
            }
        } catch (error) {
            console.error('隐藏键盘失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 等待
    async wait(duration) {
        await this.sleep(duration);
        return { success: true };
    }
    
    // 启动应用
    async launchApp(bundleId) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/wda/apps/launch`, {
                bundleId: bundleId
            });
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('启动应用失败');
            }
        } catch (error) {
            console.error('启动应用失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 关闭应用
    async terminateApp(bundleId) {
        if (!this.sessionId) {
            throw new Error('需要先创建会话');
        }
        
        try {
            const response = await this.makeRequest('POST', `/session/${this.sessionId}/wda/apps/terminate`, {
                bundleId: bundleId
            });
            
            if (response.ok) {
                return { success: true };
            } else {
                throw new Error('关闭应用失败');
            }
        } catch (error) {
            console.error('关闭应用失败:', error);
            return { success: false, error: error.message };
        }
    }
    
    // 发送 HTTP 请求的辅助方法
    async makeRequest(method, endpoint, data = null) {
        const url = this.baseURL + endpoint;
        const options = {
            method: method,
            headers: {
                'Content-Type': 'application/json'
            }
        };
        
        if (data) {
            options.body = JSON.stringify(data);
        }
        
        return fetch(url, options);
    }
    
    // 辅助方法：等待
    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
    
    // 销毁会话和清理资源
    async cleanup() {
        try {
            // 关闭 WDA 会话
            if (this.sessionId) {
                await this.makeRequest('DELETE', `/session/${this.sessionId}`);
                this.sessionId = null;
            }
            
            // 停止 USB 转发
            if (this.deviceConfig.connectionType === 'usb' && this.proxyProcess) {
                const { ipcRenderer } = window.AppGlobals;
                await ipcRenderer.invoke('stop-ios-usb-forwarding', this.deviceConfig.udid);
                this.proxyProcess = null;
            }
            
            this.isConnected = false;
            console.log('WDA 资源清理完成');
        } catch (error) {
            console.error('WDA 资源清理失败:', error);
        }
    }
}

// 导出类
window.WDAController = WDAController;