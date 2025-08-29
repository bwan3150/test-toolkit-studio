// Bug Analyzer API 客户端
// 专门用于与Toolkit Analyzer API通信
(function() {
    'use strict';

    class BugAnalyzerClient {
        constructor() {
            // API基础URL - 暂定地址，后续会改
            this.baseURL = 'http://192.168.108.189:9000';
            this.cache = new Map(); // 简单的缓存机制
            this.cacheTimeout = 5 * 60 * 1000; // 5分钟缓存
        }

        // 通用请求方法
        async request(endpoint, options = {}) {
            try {
                const url = `${this.baseURL}${endpoint}`;
                
                // 使用渲染进程日志
                if (window.rDebug) {
                    window.rDebug('BugAnalyzer API请求:', url);
                }
                
                // 检查缓存
                const cacheKey = `${options.method || 'GET'}_${url}_${JSON.stringify(options.body)}`;
                if (this.cache.has(cacheKey)) {
                    const cached = this.cache.get(cacheKey);
                    if (Date.now() - cached.timestamp < this.cacheTimeout) {
                        if (window.rDebug) {
                            window.rDebug('使用缓存数据:', endpoint);
                        }
                        return cached.data;
                    }
                }

                const response = await fetch(url, {
                    method: options.method || 'GET',
                    headers: {
                        'Content-Type': 'application/json',
                        ...options.headers
                    },
                    body: options.body ? JSON.stringify(options.body) : undefined
                });

                if (!response.ok) {
                    const errorMsg = `HTTP ${response.status}: ${response.statusText}`;
                    if (window.rError) {
                        window.rError('API请求失败:', errorMsg, 'URL:', url);
                    }
                    throw new Error(errorMsg);
                }

                const data = await response.json();
                
                // 记录响应
                if (window.rDebug) {
                    window.rDebug('API响应:', endpoint, '数据大小:', JSON.stringify(data).length, '字节');
                }
                
                // 缓存结果
                this.cache.set(cacheKey, {
                    data: data,
                    timestamp: Date.now()
                });

                return data;
            } catch (error) {
                if (window.rError) {
                    window.rError('Bug Analyzer API请求失败:', error.message, 'Endpoint:', endpoint);
                }
                throw error;
            }
        }

        // 获取数据库字段信息
        async getSchemaFields(schema = 'normal_buglist') {
            try {
                return await this.request(`/api/schema/fields?schema=${schema}`);
            } catch (error) {
                console.error('获取字段信息失败:', error);
                return null;
            }
        }

        // 获取字段选项
        async getFieldOptions(field, schema = 'normal_buglist') {
            try {
                if (window.rInfo) {
                    window.rInfo(`正在获取字段选项: ${field} (schema: ${schema})`);
                }
                const result = await this.request(`/api/schema/field/options?schema=${schema}&field=${encodeURIComponent(field)}`);
                if (window.rInfo) {
                    window.rInfo(`${field}字段选项结果:`, result);
                }
                return result;
            } catch (error) {
                if (window.rError) {
                    window.rError(`获取${field}字段选项失败:`, error.message);
                }
                return null;
            }
        }

        // 执行数据分析
        async performAnalysis(params) {
            try {
                if (window.rInfo && params.filters) {
                    window.rInfo('执行分析，包含筛选条件:', params.filters);
                }
                return await this.request('/api/analysis', {
                    method: 'POST',
                    body: params
                });
            } catch (error) {
                if (window.rError) {
                    window.rError('执行分析失败:', error.message);
                }
                return null;
            }
        }

        // 生成图表URL
        async generateChartUrl(params) {
            try {
                const result = await this.request('/api/chart_url', {
                    method: 'POST',
                    body: params
                });
                return result?.chart_url || null;
            } catch (error) {
                console.error('生成图表失败:', error);
                return null;
            }
        }

        // 搜索Bug记录
        async searchBugs(params) {
            try {
                return await this.request('/api/search', {
                    method: 'POST',
                    body: params
                });
            } catch (error) {
                console.error('搜索Bug失败:', error);
                return null;
            }
        }

        // 获取今日Bug统计（Priority分布）
        async getTodayPriorityStats(filters = {}) {
            const today = new Date().toISOString().split('T')[0];
            
            const params = {
                dates: today,
                target_field: 'Priority',
                analysis_type: 'proportion'
            };

            // 添加筛选条件
            if (Object.keys(filters).length > 0) {
                params.filters = filters;
            }

            return await this.performAnalysis(params);
        }

        // 获取Bug趋势数据
        async getBugTrends(days = 30, targetField = 'Priority', filters = {}) {
            const endDate = new Date();
            const startDate = new Date();
            startDate.setDate(startDate.getDate() - days);

            const params = {
                dates: {
                    start: startDate.toISOString().split('T')[0],
                    end: endDate.toISOString().split('T')[0]
                },
                target_field: targetField,
                analysis_type: 'trend'
            };

            // 添加筛选条件
            if (Object.keys(filters).length > 0) {
                params.filters = filters;
            }

            return await this.performAnalysis(params);
        }

        // 获取总Bug数趋势
        async getTotalBugTrend(days = 30, filters = {}) {
            return await this.getBugTrends(days, 'total_count', filters);
        }

        // 获取问题模块统计
        async getModuleStats(date = null) {
            const targetDate = date || new Date().toISOString().split('T')[0];
            
            const params = {
                dates: targetDate,
                target_field: '问题模块',
                analysis_type: 'count'
            };

            return await this.performAnalysis(params);
        }

        // 清除缓存
        clearCache() {
            this.cache.clear();
        }

        // 更新基础URL（如果后续需要更改）
        updateBaseURL(newURL) {
            this.baseURL = newURL;
            this.clearCache(); // 更改URL后清除缓存
        }
    }

    // 创建全局实例
    const bugAnalyzerClient = new BugAnalyzerClient();

    // 导出到全局
    window.BugAnalyzerClient = {
        client: bugAnalyzerClient,
        
        // 便捷方法
        getSchemaFields: (schema) => bugAnalyzerClient.getSchemaFields(schema),
        getFieldOptions: (field, schema) => bugAnalyzerClient.getFieldOptions(field, schema),
        performAnalysis: (params) => bugAnalyzerClient.performAnalysis(params),
        generateChartUrl: (params) => bugAnalyzerClient.generateChartUrl(params),
        searchBugs: (params) => bugAnalyzerClient.searchBugs(params),
        getTodayPriorityStats: (filters) => bugAnalyzerClient.getTodayPriorityStats(filters),
        getBugTrends: (days, targetField, filters) => bugAnalyzerClient.getBugTrends(days, targetField, filters),
        getTotalBugTrend: (days, filters) => bugAnalyzerClient.getTotalBugTrend(days, filters),
        getModuleStats: (date) => bugAnalyzerClient.getModuleStats(date),
        clearCache: () => bugAnalyzerClient.clearCache(),
        updateBaseURL: (newURL) => bugAnalyzerClient.updateBaseURL(newURL)
    };

    console.log('Bug Analyzer API客户端已加载');
})();