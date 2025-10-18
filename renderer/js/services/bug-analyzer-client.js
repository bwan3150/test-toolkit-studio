// Bug Analyzer API 客户端
// 专门用于与Toolkit Analyzer API通信
(function() {
    'use strict';

    class BugAnalyzerClient {
        constructor() {
            // 通过主进程代理请求，避免CORS问题
            this.ipcRenderer = require('electron').ipcRenderer;
        }

        // 通用请求方法 - 通过主进程代理
        async request(endpoint, options = {}) {
            try {
                // 详细记录请求（用于调试）
                if (window.rInfo && options.body && options.body.filters) {
                    window.rInfo('请求endpoint:', endpoint);
                    window.rInfo('请求体:', JSON.stringify(options.body));
                }

                // 通过主进程IPC代理请求
                const result = await this.ipcRenderer.invoke(
                    'bug-analysis-request',
                    endpoint,
                    options.method || 'GET',
                    options.body || null
                );

                if (!result.success) {
                    throw new Error(result.error || '请求失败');
                }

                const data = result.data;

                // 记录响应数据
                if (window.rInfo && endpoint === '/api/analysis') {
                    if (data && data.data) {
                        const dates = Object.keys(data.data);
                        if (dates.length > 0) {
                            const firstDateData = data.data[dates[0]];
                            if (options.body && options.body.filters) {
                                window.rInfo('===筛选后的API响应===');
                                window.rInfo(`日期:${dates[0]}, total:${firstDateData.total}`);
                                window.rInfo('breakdown:', JSON.stringify(firstDateData.breakdown));
                                window.rInfo('===================');
                            } else {
                                window.rInfo(`API响应(无筛选) - date:${dates[0]}, total:${firstDateData.total}`);
                            }
                        }
                    }
                }

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
                const result = await this.request(`/api/schema/field/options?schema=${schema}&field=${encodeURIComponent(field)}`);
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
            // 使用今天的日期
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
        getModuleStats: (date) => bugAnalyzerClient.getModuleStats(date)
    };

    console.log('Bug Analyzer API客户端已加载');
})();
