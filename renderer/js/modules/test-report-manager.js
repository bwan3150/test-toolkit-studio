// Test Report Manager Module - JetBrains Style
(function() {
    'use strict';

    // Module state
    let currentTimeRange = 30;
    let currentProject = 'all';
    let currentSeverity = 'all';
    let apiData = {}; // 缓存API数据
    let timelineDates = []; // 存储趋势图的日期标签
    
    // Chart colors matching JetBrains theme - 使用API返回的实际字段名
    const SEVERITY_COLORS = {
        'Block': '#e55765',     // 红色 (原critical)
        'High': '#f09f5b',      // 橙色
        'Normal': '#f4c752',    // 黄色 (原moderate)
        'Low': '#7ca82b',       // 绿色
        'info': '#659ad2'       // 蓝色 (保留以防需要)
    };
    
    // Timeline chart gradient colors - 使用API返回的实际字段名
    const TIMELINE_COLORS = {
        'Block': 'rgba(229, 87, 101, 0.8)',    // 红色 (原critical)
        'High': 'rgba(240, 159, 91, 0.7)',     // 橙色
        'Normal': 'rgba(244, 199, 82, 0.6)',   // 黄色 (原moderate)
        'Low': 'rgba(124, 168, 43, 0.5)',      // 绿色
        'info': 'rgba(101, 154, 210, 0.4)'     // 蓝色 (保留)
    };
    
    // Initialize report page
    async function initializeReportPage() {
        bindEventListeners();
        
        // 尝试加载真实数据，但不阻塞初始化
        try {
            await loadRealData(); // 加载真实数据
        } catch (error) {
            console.error('加载API数据失败，但继续初始化:', error);
            // 即使API失败，也继续初始化图表（会显示无数据状态）
        }
        
        initializeCharts();
    }
    
    // 加载真实API数据
    async function loadRealData() {
        if (!window.BugAnalyzerClient) {
            console.warn('Bug Analyzer API客户端未加载，跳过数据加载');
            return;
        }

        try {
            console.log('开始加载真实Bug数据...');
            
            // 获取当前日期
            const today = new Date().toISOString().split('T')[0];
            
            // 1. 获取今日Priority统计（用于饼图）
            const priorityStats = await window.BugAnalyzerClient.getTodayPriorityStats();
            if (priorityStats && priorityStats.data && priorityStats.data[today]) {
                apiData.priorityStats = priorityStats.data[today];
                console.log('Priority统计数据:', apiData.priorityStats);
            } else {
                console.warn('Priority统计数据为空');
            }
            
            // 2. 获取趋势数据（用于趋势图）
            const trendData = await window.BugAnalyzerClient.getBugTrends(currentTimeRange, 'Priority');
            if (trendData && trendData.data) {
                apiData.trendData = trendData.data;
                console.log('趋势数据:', apiData.trendData);
            } else {
                console.warn('趋势数据为空');
            }
            
            // 3. 获取问题模块统计（用于表格）
            const moduleStats = await window.BugAnalyzerClient.getModuleStats();
            if (moduleStats && moduleStats.data && moduleStats.data[today]) {
                apiData.moduleStats = moduleStats.data[today];
                console.log('模块统计数据:', apiData.moduleStats);
            }
            
            // 4. 获取总Bug数（用于顶部统计）
            const totalBugTrend = await window.BugAnalyzerClient.getTotalBugTrend(1);
            if (totalBugTrend && totalBugTrend.data) {
                const dates = Object.keys(totalBugTrend.data);
                if (dates.length > 0) {
                    const latestDate = dates[dates.length - 1];
                    apiData.totalBugs = totalBugTrend.data[latestDate].total || 0;
                }
            }
            
            console.log('API数据加载完成:', apiData);
        } catch (error) {
            console.error('加载真实数据失败:', error);
            // 不显示错误界面，让图表显示"暂无数据"状态
            // 这样至少UI是正常的
        }
    }
    
    // Bind event listeners
    function bindEventListeners() {
        const projectSelect = document.getElementById('reportProjectSelect');
        const severityFilter = document.getElementById('reportSeverityFilter');
        const timeRange = document.getElementById('reportTimeRange');
        const moreFiltersBtn = document.getElementById('moreFiltersBtn');
        
        if (projectSelect) {
            projectSelect.addEventListener('change', (e) => {
                currentProject = e.target.value;
                refreshReportData();
            });
        }
        
        if (severityFilter) {
            severityFilter.addEventListener('change', (e) => {
                currentSeverity = e.target.value;
                refreshReportData();
            });
        }
        
        if (timeRange) {
            timeRange.addEventListener('change', (e) => {
                currentTimeRange = e.target.value === 'all' ? 365 : parseInt(e.target.value);
                refreshReportData();
            });
        }
        
        if (moreFiltersBtn) {
            moreFiltersBtn.addEventListener('click', () => {
                // TODO: Show more filters dialog
                console.log('More filters clicked');
            });
        }
    }
    
    // Initialize charts
    function initializeCharts() {
        initializeSeverityChart();
        initializeTimelineChart();
    }
    
    // Initialize severity donut chart
    function initializeSeverityChart() {
        const canvas = document.getElementById('severityChart');
        if (!canvas) return;
        
        canvas.width = 180;
        canvas.height = 180;
        
        drawSeverityDonut(canvas);
    }
    
    // Draw severity donut chart with hover effects
    function drawSeverityDonut(canvas) {
        const ctx = canvas.getContext('2d');
        const centerX = canvas.width / 2;
        const centerY = canvas.height / 2;
        const radius = 70;
        const innerRadius = 50;
        
        // Initialize hover state only once
        if (canvas.hoveredSegment === undefined) {
            canvas.hoveredSegment = null;
            canvas.segments = [];
            
            // Add mouse move listener only once
            canvas.addEventListener('mousemove', (e) => handleDonutHover(e, canvas));
            canvas.addEventListener('mouseleave', () => {
                canvas.hoveredSegment = null;
                drawSeverityDonut(canvas);
            });
        }
        
        // 使用真实API数据
        let data = [];
        if (!apiData.priorityStats || !apiData.priorityStats.breakdown) {
            console.error('没有Priority统计数据');
            return; // 没有数据时直接返回
        }
        
        // 使用真实数据
        const breakdown = apiData.priorityStats.breakdown;
        for (const [priority, stats] of Object.entries(breakdown)) {
            if (SEVERITY_COLORS[priority]) {
                data.push({
                    value: stats.count || 0,
                    color: SEVERITY_COLORS[priority],
                    label: priority
                });
            }
        }
        
        // 如果没有数据，显示空状态
        if (data.length === 0) {
            ctx.fillStyle = '#666';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('暂无数据', centerX, centerY);
            return;
        }
        
        const total = data.reduce((sum, item) => sum + item.value, 0);
        let currentAngle = -Math.PI / 2;
        
        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        
        // Clear and rebuild segment data
        canvas.segments = [];
        
        // Draw segments
        data.forEach((item, index) => {
            const sliceAngle = (item.value / total) * 2 * Math.PI;
            const endAngle = currentAngle + sliceAngle;
            
            // Store segment info for hover detection
            canvas.segments.push({
                startAngle: currentAngle,
                endAngle: endAngle,
                ...item
            });
            
            // Check if this segment is hovered
            const isHovered = canvas.hoveredSegment === index;
            
            ctx.save();
            
            if (isHovered) {
                // Scale up slightly on hover
                ctx.translate(centerX, centerY);
                ctx.scale(1.05, 1.05);
                ctx.translate(-centerX, -centerY);
                
                // Add shadow for depth
                ctx.shadowBlur = 15;
                ctx.shadowColor = item.color;
            }
            
            ctx.beginPath();
            ctx.arc(centerX, centerY, radius, currentAngle, endAngle);
            ctx.arc(centerX, centerY, innerRadius, endAngle, currentAngle, true);
            ctx.closePath();
            ctx.fillStyle = item.color;
            ctx.fill();
            
            ctx.restore();
            
            currentAngle = endAngle;
        });
        
        // 更新图例显示
        updateSeverityLegend(data, total);
    }
    
    // 更新图例显示
    function updateSeverityLegend(data, total) {
        const legendContainer = document.querySelector('.severity-legend');
        if (!legendContainer) return;
        
        // 清空现有图例
        legendContainer.innerHTML = '';
        
        // 为每个数据项创建图例
        data.forEach(item => {
            if (item.value > 0) { // 只显示有值的项
                const legendItem = document.createElement('div');
                legendItem.className = 'legend-item';
                legendItem.innerHTML = `
                    <span class="legend-dot" style="background: ${item.color}"></span>
                    <span class="legend-label">${item.label}:</span>
                    <span class="legend-value">${item.value}</span>
                `;
                legendContainer.appendChild(legendItem);
            }
        });
        
        // 更新中心文字（总数）
        const centerValue = document.querySelector('.chart-center-value');
        const centerLabel = document.querySelector('.chart-center-label');
        if (centerValue && total > 0) {
            // 格式化显示
            if (total >= 1000) {
                centerValue.textContent = (total / 1000).toFixed(1) + ' K';
            } else {
                centerValue.textContent = total.toString();
            }
        }
        if (centerLabel) {
            centerLabel.textContent = total === 1 ? 'problem' : 'problems';
        }
    }
    
    // Handle donut hover with throttling
    let donutHoverTimeout = null;
    function handleDonutHover(e, canvas) {
        // Throttle hover events to improve performance
        if (donutHoverTimeout) return;
        
        donutHoverTimeout = setTimeout(() => {
            donutHoverTimeout = null;
        }, 16); // ~60fps
        
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        const centerX = canvas.width / 2;
        const centerY = canvas.height / 2;
        
        // Calculate distance from center
        const dx = x - centerX;
        const dy = y - centerY;
        const distance = Math.sqrt(dx * dx + dy * dy);
        
        // Check if within donut bounds
        if (distance < 50 || distance > 70) {
            if (canvas.hoveredSegment !== null) {
                canvas.hoveredSegment = null;
                drawSeverityDonut(canvas);
            }
            return;
        }
        
        // Calculate angle
        let angle = Math.atan2(dy, dx);
        if (angle < -Math.PI / 2) angle += 2 * Math.PI;
        
        // Find which segment is hovered
        let hoveredSegment = null;
        if (canvas.segments) {
            canvas.segments.forEach((segment, index) => {
                if (angle >= segment.startAngle && angle <= segment.endAngle) {
                    hoveredSegment = index;
                }
            });
        }
        
        if (hoveredSegment !== canvas.hoveredSegment) {
            canvas.hoveredSegment = hoveredSegment;
            drawSeverityDonut(canvas);
        }
    }
    
    // Initialize timeline chart
    function initializeTimelineChart() {
        const canvas = document.getElementById('timelineChart');
        if (!canvas) return;
        
        const container = canvas.parentElement;
        canvas.width = container.offsetWidth;
        canvas.height = 220; // 增加高度以容纳日期标签
        
        // 刷新时需要重新生成数据
        canvas.chartData = generateTimelineData();
        canvas.isInitialized = true;
        
        drawTimelineChart(canvas);
    }
    
    // Draw timeline chart with smooth gradients and hover effects
    function drawTimelineChart(canvas) {
        const ctx = canvas.getContext('2d');
        const padding = { top: 20, right: 20, bottom: 40, left: 50 };
        
        // 根据数据点数量动态调整画布宽度（如果数据点多，增加宽度）
        const minWidth = canvas.parentElement.offsetWidth;
        const initialDataPoints = canvas.chartData ? (canvas.chartData['Block'] ? canvas.chartData['Block'].length : 0) : 0;
        const pointWidth = 40; // 每个数据点的宽度
        const requiredWidth = Math.max(minWidth, initialDataPoints * pointWidth + padding.left + padding.right);
        
        // 设置画布实际宽度
        if (canvas.width !== requiredWidth) {
            canvas.width = requiredWidth;
        }
        
        const chartWidth = canvas.width - padding.left - padding.right;
        const chartHeight = canvas.height - padding.top - padding.bottom;
        
        // Initialize data and listeners only once
        if (!canvas.isInitialized) {
            canvas.chartData = generateTimelineData();
            canvas.hoveredLine = null;
            canvas.isInitialized = true;
            
            // Add mouse move listener only once
            canvas.addEventListener('mousemove', (e) => handleTimelineHover(e, canvas));
            canvas.addEventListener('mouseleave', () => {
                canvas.hoveredLine = null;
                drawTimelineChart(canvas);
            });
        }
        
        const data = canvas.chartData;
        if (!data) {
            // 没有数据时显示提示
            ctx.fillStyle = '#666';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('暂无趋势数据', canvas.width / 2, canvas.height / 2);
            return;
        }
        
        // 获取数据点数量
        const dataPoints = data['Block'] ? data['Block'].length : 0;
        if (dataPoints === 0) {
            ctx.fillStyle = '#666';
            ctx.font = '14px sans-serif';
            ctx.textAlign = 'center';
            ctx.fillText('暂无数据点', canvas.width / 2, canvas.height / 2);
            return;
        }
        
        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        
        // 计算数据的最大值以确定Y轴比例
        let maxValue = 0;
        const keys = ['Low', 'Normal', 'High', 'Block']; // 移除info，因为API数据中没有
        const stackedData = [];
        
        // Calculate stacked values and find max
        for (let i = 0; i < dataPoints; i++) {
            let stack = 0;
            stackedData[i] = {};
            keys.forEach(key => {
                const value = data[key] && data[key][i] ? data[key][i] : 0;
                stack += value;
                stackedData[i][key] = stack;
            });
            maxValue = Math.max(maxValue, stack);
        }
        
        // 设置Y轴的合理范围（向上取整到最近的10的倍数）
        const yAxisMax = Math.ceil(maxValue / 10) * 10 || 100;
        
        // Draw subtle grid lines
        ctx.strokeStyle = 'rgba(60, 60, 60, 0.15)';
        ctx.lineWidth = 0.5;
        
        // Horizontal grid lines
        const ySteps = 4;
        for (let i = 0; i <= ySteps; i++) {
            const y = padding.top + (chartHeight / ySteps) * i;
            ctx.beginPath();
            ctx.moveTo(padding.left, y);
            ctx.lineTo(canvas.width - padding.right, y);
            ctx.stroke();
        }
        
        // Draw each layer with gradient
        keys.reverse().forEach((key, keyIndex) => {
            if (!TIMELINE_COLORS[key]) {
                console.warn(`没有找到${key}的颜色配置`);
                return;
            }
            
            const gradient = ctx.createLinearGradient(0, padding.top, 0, padding.top + chartHeight);
            const baseColor = TIMELINE_COLORS[key];
            gradient.addColorStop(0, baseColor);
            gradient.addColorStop(1, baseColor.replace(/[\d.]+\)/, '0.1)'));
            
            ctx.beginPath();
            ctx.fillStyle = gradient;
            
            // Check if this line is hovered
            const isHovered = canvas.hoveredLine === key;
            if (isHovered) {
                ctx.globalAlpha = 1;
                ctx.shadowBlur = 10;
                ctx.shadowColor = SEVERITY_COLORS[key];
            } else if (canvas.hoveredLine && canvas.hoveredLine !== key) {
                ctx.globalAlpha = 0.3;
            } else {
                ctx.globalAlpha = 0.8;
            }
            
            // Start from bottom left
            ctx.moveTo(padding.left, padding.top + chartHeight);
            
            // Draw the upper line with straight lines between points
            for (let i = 0; i < dataPoints; i++) {
                const x = padding.left + (chartWidth / Math.max(1, dataPoints - 1)) * i;
                const y = padding.top + chartHeight - (stackedData[i][key] / yAxisMax) * chartHeight;
                
                if (i === 0) {
                    ctx.lineTo(x, y);
                } else {
                    // 直接用直线连接数据点，避免曲线造成的波动
                    ctx.lineTo(x, y);
                }
            }
            
            // Draw the last point
            if (dataPoints > 0) {
                const lastX = padding.left + chartWidth;
                const lastY = padding.top + chartHeight - (stackedData[dataPoints - 1][key] / yAxisMax) * chartHeight;
                ctx.lineTo(lastX, lastY);
            }
            
            // Complete the shape
            ctx.lineTo(canvas.width - padding.right, padding.top + chartHeight);
            ctx.closePath();
            ctx.fill();
            
            // Reset shadow
            ctx.shadowBlur = 0;
        });
        
        ctx.globalAlpha = 1;
        
        // Y-axis labels - 根据实际数据范围动态生成
        ctx.fillStyle = '#6c6c6c';
        ctx.font = '10px var(--font-family)';
        ctx.textAlign = 'right';
        
        const yStepValue = yAxisMax / 4;
        for (let i = 0; i <= 4; i++) {
            const value = Math.round(yStepValue * i);
            const y = padding.top + chartHeight - (chartHeight / 4) * i;
            ctx.fillText(value.toString(), padding.left - 5, y + 3);
        }
        
        // 在canvas上直接绘制日期标签（这样标签会跟随滚动）
        drawTimelineLabels(ctx, padding, chartWidth, chartHeight);
        
        // Draw hover indicator in fixed position
        if (canvas.hoveredLine) {
            drawFixedIndicator(ctx, canvas.hoveredLine, canvas.width - 100, 30);
        }
    }
    
    // 在canvas上绘制时间轴标签
    function drawTimelineLabels(ctx, padding, chartWidth, chartHeight) {
        if (!timelineDates || timelineDates.length === 0) {
            return;
        }
        
        ctx.fillStyle = '#6c6c6c';
        ctx.font = '10px var(--font-family)';
        ctx.textAlign = 'center';
        
        const dataPoints = timelineDates.length;
        
        // 每隔几个数据点显示一个日期标签（避免重叠）
        const labelInterval = Math.max(1, Math.floor(dataPoints / 10)); // 最多显示10个标签
        
        for (let i = 0; i < dataPoints; i += labelInterval) {
            const x = padding.left + (chartWidth / Math.max(1, dataPoints - 1)) * i;
            const y = padding.top + chartHeight + 20; // 标签位置在图表下方
            
            const date = timelineDates[i];
            if (date) {
                // 格式化日期显示（例如：08-27）
                const dateObj = new Date(date);
                const month = String(dateObj.getMonth() + 1).padStart(2, '0');
                const day = String(dateObj.getDate()).padStart(2, '0');
                
                ctx.fillText(`${month}-${day}`, x, y);
            }
        }
        
        // 始终显示最后一个日期
        if (dataPoints > 0 && dataPoints % labelInterval !== 0) {
            const lastX = padding.left + chartWidth;
            const lastY = padding.top + chartHeight + 20;
            const lastDate = timelineDates[dataPoints - 1];
            if (lastDate) {
                const dateObj = new Date(lastDate);
                const month = String(dateObj.getMonth() + 1).padStart(2, '0');
                const day = String(dateObj.getDate()).padStart(2, '0');
                ctx.fillText(`${month}-${day}`, lastX, lastY);
            }
        }
    }
    
    // 更新HTML中的时间轴标签（现在已不需要，因为标签直接绘制在canvas上）
    function updateTimelineLabels() {
        // 隐藏HTML中的时间标签容器
        const labelsContainer = document.querySelector('.timeline-labels');
        if (labelsContainer) {
            labelsContainer.style.display = 'none';
        }
    }
    
    // Generate timeline data
    function generateTimelineData() {
        // 必须有API数据
        if (!apiData.trendData) {
            console.error('没有趋势数据');
            return null;
        }
        
        // 转换API数据格式
        const data = {
            'Block': [],
            'High': [],
            'Normal': [],
            'Low': []
        };
        
        // 获取所有日期并排序
        const dates = Object.keys(apiData.trendData).sort();
        timelineDates = dates; // 保存日期用于显示
        
        console.log('趋势图日期:', dates);
        
        // 遍历每个日期的数据
        dates.forEach(date => {
            const dayData = apiData.trendData[date];
            if (dayData && dayData.breakdown) {
                // 使用实际的字段名
                data['Block'].push(dayData.breakdown['Block'] || 0);
                data['High'].push(dayData.breakdown['High'] || 0);
                data['Normal'].push(dayData.breakdown['Normal'] || 0);  
                data['Low'].push(dayData.breakdown['Low'] || 0);
            } else {
                // 如果某天没有breakdown数据，填充0
                data['Block'].push(0);
                data['High'].push(0);
                data['Normal'].push(0);
                data['Low'].push(0);
            }
        });
        
        console.log('处理后的趋势数据:', data);
        return data;
    }
    
    // Handle timeline hover with throttling
    let timelineHoverTimeout = null;
    function handleTimelineHover(e, canvas) {
        // Throttle hover events to improve performance
        if (timelineHoverTimeout) return;
        
        timelineHoverTimeout = setTimeout(() => {
            timelineHoverTimeout = null;
        }, 16); // ~60fps
        
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        canvas.mousePos = { x, y };
        
        const padding = { top: 20, right: 20, bottom: 20, left: 40 };
        const chartHeight = canvas.height - padding.top - padding.bottom;
        
        // Determine which layer is hovered
        const relativeY = y - padding.top;
        const value = (1 - relativeY / chartHeight) * 4000;
        
        let hoveredLine = null;
        const data = canvas.chartData;
        if (!data || !data['Block']) return;
        
        const dataPoints = data['Block'].length;
        const dataIndex = Math.floor((x - padding.left) / ((canvas.width - padding.left - padding.right) / (dataPoints - 1)));
        
        if (dataIndex >= 0 && dataIndex < dataPoints && x >= padding.left && x <= canvas.width - padding.right) {
            let stack = 0;
            const keys = ['Low', 'Normal', 'High', 'Block'];
            
            for (const key of keys) {
                stack += data[key][dataIndex];
                if (value <= stack) {
                    hoveredLine = key;
                    break;
                }
            }
        }
        
        if (hoveredLine !== canvas.hoveredLine) {
            canvas.hoveredLine = hoveredLine;
            drawTimelineChart(canvas);
        }
    }
    
    // Draw fixed indicator with simpler style
    function drawFixedIndicator(ctx, severity, x, y) {
        const labels = {
            'Block': 'Block',
            'High': 'High', 
            'Normal': 'Normal',
            'Low': 'Low',
            'info': 'Info'
        };
        
        const text = labels[severity] || severity;
        
        ctx.save();
        
        // Modern badge style (like GitHub labels)
        ctx.font = '11px var(--font-family)';
        const metrics = ctx.measureText(text);
        const textWidth = metrics.width;
        
        // Badge dimensions
        const badgeHeight = 22;
        const badgePadding = 8;
        const badgeWidth = textWidth + badgePadding * 2;
        const borderRadius = 4;
        
        // Background with gradient effect
        const gradient = ctx.createLinearGradient(x, y, x, y + badgeHeight);
        const baseColor = SEVERITY_COLORS[severity];
        gradient.addColorStop(0, baseColor);
        gradient.addColorStop(1, baseColor + 'dd'); // Slightly darker at bottom
        
        // Draw badge background
        ctx.fillStyle = gradient;
        ctx.beginPath();
        ctx.roundRect(x, y, badgeWidth, badgeHeight, borderRadius);
        ctx.fill();
        
        // Add subtle inner shadow effect
        ctx.strokeStyle = 'rgba(0, 0, 0, 0.2)';
        ctx.lineWidth = 0.5;
        ctx.stroke();
        
        // Draw text (white with shadow for contrast)
        ctx.fillStyle = '#ffffff';
        ctx.shadowColor = 'rgba(0, 0, 0, 0.3)';
        ctx.shadowBlur = 1;
        ctx.shadowOffsetY = 1;
        
        // Center text both horizontally and vertically
        const textX = x + badgeWidth / 2;
        const textY = y + badgeHeight / 2;
        
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(text, textX, textY);
        
        ctx.restore();
    }
    
    // Refresh report data
    async function refreshReportData() {
        // Show loading animation
        const reportContent = document.querySelector('.report-content');
        if (reportContent) {
            reportContent.style.opacity = '0.7';
        }
        
        // 重新加载真实数据
        await loadRealData();
        
        // Update stats
        updateStats();
        
        // Redraw charts
        initializeCharts();
        
        // Hide loading animation
        if (reportContent) {
            setTimeout(() => {
                reportContent.style.opacity = '1';
            }, 200);
        }
    }
    
    // Update statistics
    function updateStats() {
        const projectsValue = document.querySelector('.stat-box:nth-child(1) .stat-value');
        const scansValue = document.querySelector('.stat-box:nth-child(2) .stat-value');
        const coverageValue = document.querySelector('.stat-box:nth-child(3) .stat-value');
        
        // 使用真实数据
        if (projectsValue && apiData.moduleStats) {
            // 计算问题模块的数量作为项目数
            const moduleCount = apiData.moduleStats.breakdown ? 
                Object.keys(apiData.moduleStats.breakdown).length : 0;
            projectsValue.textContent = moduleCount;
        }
        
        if (scansValue && apiData.totalBugs !== undefined) {
            // 使用总Bug数
            scansValue.textContent = apiData.totalBugs;
        }
        
        if (coverageValue) {
            // API中没有覆盖率数据，显示N/A
            coverageValue.textContent = 'N/A';
        }
    }
    
    // Handle page activation
    function onPageActivated() {
        // Ensure charts are properly sized
        initializeCharts();
        refreshReportData();
    }
    
    // Export module
    window.TestReportModule = {
        initializeReportPage,
        refreshReportData,
        onPageActivated
    };
})();
