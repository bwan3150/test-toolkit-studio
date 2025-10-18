// Bug Analysis API 代理处理器
// 通过主进程代理外部Bug分析API请求，避免渲染进程的CORS限制

const { ipcMain } = require('electron');
const http = require('http');
const https = require('https');

/**
 * 通用HTTP/HTTPS请求函数
 * @param {string} url - 完整的URL
 * @param {string} method - HTTP方法 (GET, POST等)
 * @param {object} body - 请求体 (可选)
 * @param {object} headers - 额外的请求头 (可选)
 * @returns {Promise<object>} 响应数据
 */
function makeRequest(url, method = 'GET', body = null, headers = {}) {
    return new Promise((resolve, reject) => {
        try {
            const urlObj = new URL(url);
            const isHttps = urlObj.protocol === 'https:';
            const httpModule = isHttps ? https : http;

            const requestOptions = {
                hostname: urlObj.hostname,
                port: urlObj.port || (isHttps ? 443 : 80),
                path: urlObj.pathname + urlObj.search,
                method: method,
                headers: {
                    'Content-Type': 'application/json',
                    ...headers
                }
            };

            console.log(`[Bug Analysis API] ${method} ${url}`);
            if (body) {
                console.log('[Bug Analysis API] 请求体:', JSON.stringify(body).substring(0, 200));
            }

            const req = httpModule.request(requestOptions, (res) => {
                let data = '';

                res.on('data', (chunk) => {
                    data += chunk;
                });

                res.on('end', () => {
                    console.log(`[Bug Analysis API] 响应状态: ${res.statusCode}`);

                    if (res.statusCode >= 200 && res.statusCode < 300) {
                        try {
                            const jsonData = JSON.parse(data);
                            console.log('[Bug Analysis API] 响应成功');
                            resolve(jsonData);
                        } catch (parseError) {
                            console.error('[Bug Analysis API] 解析JSON失败:', parseError);
                            reject(new Error(`解析响应失败: ${parseError.message}`));
                        }
                    } else {
                        console.error(`[Bug Analysis API] HTTP错误 ${res.statusCode}:`, data.substring(0, 200));
                        reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
                    }
                });
            });

            req.on('error', (error) => {
                console.error('[Bug Analysis API] 请求失败:', error.message);
                reject(error);
            });

            // 发送请求体
            if (body) {
                const bodyStr = JSON.stringify(body);
                req.write(bodyStr);
            }

            req.end();
        } catch (error) {
            console.error('[Bug Analysis API] 创建请求失败:', error);
            reject(error);
        }
    });
}

/**
 * 注册Bug Analysis API代理处理器
 */
function registerBugAnalysisProxyHandlers() {
    const BASE_URL = 'http://analysis.test-toolkit.app';

    // 通用分析接口
    ipcMain.handle('bug-analysis-request', async (event, endpoint, method = 'GET', body = null) => {
        try {
            const url = `${BASE_URL}${endpoint}`;
            const data = await makeRequest(url, method, body);
            return { success: true, data };
        } catch (error) {
            console.error('[Bug Analysis API] 请求失败:', error.message);
            return { success: false, error: error.message };
        }
    });

    // 执行数据分析 (POST /api/analysis)
    ipcMain.handle('bug-analysis-perform-analysis', async (event, params) => {
        try {
            const url = `${BASE_URL}/api/analysis`;
            const data = await makeRequest(url, 'POST', params);
            return data; // 直接返回数据，保持与原API客户端的兼容性
        } catch (error) {
            console.error('[Bug Analysis API] 分析请求失败:', error.message);
            return null;
        }
    });

    // 获取字段选项 (GET /api/schema/field/options)
    ipcMain.handle('bug-analysis-get-field-options', async (event, field, schema = 'normal_buglist') => {
        try {
            const url = `${BASE_URL}/api/schema/field/options?schema=${schema}&field=${encodeURIComponent(field)}`;
            const data = await makeRequest(url, 'GET');
            return data;
        } catch (error) {
            console.error('[Bug Analysis API] 获取字段选项失败:', error.message);
            return null;
        }
    });

    // 获取字段信息 (GET /api/schema/fields)
    ipcMain.handle('bug-analysis-get-schema-fields', async (event, schema = 'normal_buglist') => {
        try {
            const url = `${BASE_URL}/api/schema/fields?schema=${schema}`;
            const data = await makeRequest(url, 'GET');
            return data;
        } catch (error) {
            console.error('[Bug Analysis API] 获取字段信息失败:', error.message);
            return null;
        }
    });

    // Bug搜索 (POST /api/search)
    ipcMain.handle('bug-analysis-search', async (event, params) => {
        try {
            const url = `${BASE_URL}/api/search`;
            const data = await makeRequest(url, 'POST', params);
            return data;
        } catch (error) {
            console.error('[Bug Analysis API] Bug搜索失败:', error.message);
            return null;
        }
    });

    // 生成图表URL (POST /api/chart_url)
    ipcMain.handle('bug-analysis-generate-chart-url', async (event, params) => {
        try {
            const url = `${BASE_URL}/api/chart_url`;
            const data = await makeRequest(url, 'POST', params);
            return data;
        } catch (error) {
            console.error('[Bug Analysis API] 生成图表URL失败:', error.message);
            return null;
        }
    });

    console.log('Bug Analysis API代理处理器已注册');
}

module.exports = {
    registerBugAnalysisProxyHandlers
};
