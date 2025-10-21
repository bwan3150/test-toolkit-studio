// API客户端工具，提供统一的HTTP请求处理和认证管理
// 
// 此模块负责：
// 1. 统一处理所有API请求
// 2. 自动处理token过期和刷新
// 3. 统一错误处理
// 4. 请求拦截和响应拦截

// 使用立即执行函数封装，避免全局作用域污染
(function() {
    // 从全局获取ipcRenderer，避免重复声明
    const getGlobals = () => window.AppGlobals;
    const getIpcRenderer = () => getGlobals().ipcRenderer;

// API客户端类
class ApiClient {
    constructor() {
        this.baseURL = '';
        this.isRefreshing = false;
        this.refreshPromise = null;
    }

    // 初始化基础URL
    async initialize() {
        try {
            this.baseURL = await getIpcRenderer().invoke('store-get', 'base_url');
            return this.baseURL;
        } catch (error) {
            console.error('初始化API客户端失败:', error);
            throw error;
        }
    }

    // 获取访问token
    async getAccessToken() {
        try {
            return await getIpcRenderer().invoke('store-get', 'access_token');
        } catch (error) {
            console.error('获取访问token失败:', error);
            return null;
        }
    }

    // 检查token是否过期
    async isTokenExpired() {
        try {
            const tokenExpiry = await getIpcRenderer().invoke('store-get', 'token_expiry');
            if (!tokenExpiry) return true;
            
            const now = new Date();
            const expiry = new Date(tokenExpiry);
            
            // 提前5分钟认为token过期，给刷新留出时间
            const bufferTime = 5 * 60 * 1000; // 5分钟
            return now.getTime() > (expiry.getTime() - bufferTime);
        } catch (error) {
            console.error('检查token过期状态失败:', error);
            return true;
        }
    }

    // 刷新访问token
    async refreshAccessToken() {
        try {
            // 防止同时多个请求触发刷新
            if (this.isRefreshing) {
                return this.refreshPromise;
            }

            this.isRefreshing = true;
            this.refreshPromise = getIpcRenderer().invoke('refresh-token');
            
            const result = await this.refreshPromise;
            
            if (result.success) {
                console.log('Token刷新成功');
                return result;
            } else {
                console.error('Token刷新失败:', result.error);
                // Token刷新失败，需要重新登录
                await this.handleAuthFailure();
                throw new Error('Token刷新失败，需要重新登录');
            }
        } catch (error) {
            console.error('刷新token异常:', error);
            await this.handleAuthFailure();
            throw error;
        } finally {
            this.isRefreshing = false;
            this.refreshPromise = null;
        }
    }

    // 处理认证失败
    async handleAuthFailure() {
        try {
            // 清除本地token
            await getIpcRenderer().invoke('logout');

            // 显示通知
            if (window.AppNotifications) {
                window.AppNotifications.warn('登录已过期，请重新登录');
            }

            // 跳转到登录页面
            await getIpcRenderer().invoke('navigate-to-login');
        } catch (error) {
            console.error('处理认证失败时出错:', error);
        }
    }

    // 获取有效的访问token
    async getValidAccessToken() {
        try {
            const isExpired = await this.isTokenExpired();
            
            if (isExpired) {
                console.log('Token已过期，正在刷新...');
                await this.refreshAccessToken();
            }
            
            return await this.getAccessToken();
        } catch (error) {
            console.error('获取有效token失败:', error);
            throw error;
        }
    }

    // 发送HTTP请求
    async request(method, endpoint, data = null, options = {}) {
        try {
            // 确保API客户端已初始化
            if (!this.baseURL) {
                await this.initialize();
            }

            // 获取有效的访问token
            const token = await this.getValidAccessToken();
            if (!token) {
                throw new Error('无法获取有效的访问token');
            }

            // 构建请求配置
            const config = {
                method: method.toUpperCase(),
                headers: {
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'application/json',
                    ...options.headers
                },
                ...options
            };

            // 添加请求体
            if (data && ['POST', 'PUT', 'PATCH'].includes(config.method)) {
                if (config.headers['Content-Type'] === 'application/json') {
                    config.body = JSON.stringify(data);
                } else if (config.headers['Content-Type'] === 'application/x-www-form-urlencoded') {
                    config.body = new URLSearchParams(data).toString();
                } else {
                    config.body = data;
                }
            }

            // 发送请求
            const url = `${this.baseURL}${endpoint}`;
            console.log(`发送${method.toUpperCase()}请求:`, url);
            
            const response = await fetch(url, config);

            // 处理401错误（token无效）
            if (response.status === 401) {
                console.log('收到401错误，尝试刷新token...');
                
                try {
                    await this.refreshAccessToken();
                    
                    // 使用新token重试请求
                    const newToken = await this.getAccessToken();
                    config.headers['Authorization'] = `Bearer ${newToken}`;
                    
                    const retryResponse = await fetch(url, config);
                    return await this.handleResponse(retryResponse);
                } catch (refreshError) {
                    console.error('Token刷新后重试请求失败:', refreshError);
                    throw refreshError;
                }
            }

            return await this.handleResponse(response);
        } catch (error) {
            console.error(`API请求失败 [${method.toUpperCase()} ${endpoint}]:`, error);
            throw error;
        }
    }

    // 处理响应
    async handleResponse(response) {
        try {
            const contentType = response.headers.get('content-type');
            let data;

            if (contentType && contentType.includes('application/json')) {
                data = await response.json();
            } else {
                data = await response.text();
            }

            if (!response.ok) {
                throw new Error(data.message || data.error || `HTTP ${response.status}: ${response.statusText}`);
            }

            return {
                success: true,
                data,
                status: response.status,
                headers: response.headers
            };
        } catch (error) {
            throw {
                success: false,
                error: error.message,
                status: response.status,
                statusText: response.statusText
            };
        }
    }

    // 便捷方法
    async get(endpoint, options = {}) {
        return this.request('GET', endpoint, null, options);
    }

    async post(endpoint, data, options = {}) {
        return this.request('POST', endpoint, data, options);
    }

    async put(endpoint, data, options = {}) {
        return this.request('PUT', endpoint, data, options);
    }

    async patch(endpoint, data, options = {}) {
        return this.request('PATCH', endpoint, data, options);
    }

    async delete(endpoint, options = {}) {
        return this.request('DELETE', endpoint, null, options);
    }
}

// 创建全局API客户端实例
const apiClient = new ApiClient();

// 监听主进程的token刷新事件（延迟注册，确保其他模块已加载）
setTimeout(() => {
    ipcRenderer.on('token-refreshed', () => {
        console.log('收到token刷新通知');
        window.AppNotifications?.success('登录状态已自动续期');
    });
}, 100);

// 导出API客户端模块
window.ApiClient = {
    // 主要的API客户端实例
    client: apiClient,

    // 初始化API客户端
    async initialize() {
        try {
            await apiClient.initialize();
            console.log('API客户端已初始化');
            return true;
        } catch (error) {
            console.error('API客户端初始化失败:', error);
            throw error;
        }
    },

    // 便捷的API调用方法
    async get(endpoint, options) {
        return apiClient.get(endpoint, options);
    },

    async post(endpoint, data, options) {
        return apiClient.post(endpoint, data, options);
    },

    async put(endpoint, data, options) {
        return apiClient.put(endpoint, data, options);
    },

    async patch(endpoint, data, options) {
        return apiClient.patch(endpoint, data, options);
    },

    async delete(endpoint, options) {
        return apiClient.delete(endpoint, options);
    },

    // 检查认证状态
    async checkAuthStatus() {
        try {
            const result = await apiClient.get('/api/users/me');
            return result.success;
        } catch (error) {
            console.error('检查认证状态失败:', error);
            return false;
        }
    },

    // 手动刷新token
    async refreshToken() {
        return apiClient.refreshAccessToken();
    },

    // 登出
    async logout() {
        return apiClient.handleAuthFailure();
    }
};

console.log('API客户端模块已加载');

})(); // 立即执行函数结束