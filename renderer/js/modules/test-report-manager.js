// Test Report Manager Module - JetBrains Style
(function() {
    'use strict';

    // Module state
    let currentTimeRange = 30;
    let currentProject = 'all';
    let currentSeverity = 'all';
    
    // Chart colors matching JetBrains theme (from screenshot)
    const SEVERITY_COLORS = {
        critical: '#e55765',  // Soft red/pink
        high: '#f09f5b',      // Warm orange
        moderate: '#f4c752',  // Golden yellow
        low: '#7ca82b',
        info: '#659ad2'
    };
    
    // Timeline chart gradient colors
    const TIMELINE_COLORS = {
        critical: 'rgba(229, 87, 101, 0.8)',   // Pink/red
        high: 'rgba(240, 159, 91, 0.7)',       // Orange
        moderate: 'rgba(244, 199, 82, 0.6)',   // Yellow
        low: 'rgba(124, 168, 43, 0.5)',        // Green
        info: 'rgba(101, 154, 210, 0.4)'       // Blue
    };
    
    // Initialize report page
    function initializeReportPage() {
        bindEventListeners();
        initializeCharts();
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
        
        const data = [
            { value: 1897, color: SEVERITY_COLORS.critical, label: 'Critical' },
            { value: 1613, color: SEVERITY_COLORS.high, label: 'High' },
            { value: 439, color: SEVERITY_COLORS.moderate, label: 'Moderate' }
        ];
        
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
        canvas.height = 200;
        
        drawTimelineChart(canvas);
    }
    
    // Draw timeline chart with smooth gradients and hover effects
    function drawTimelineChart(canvas) {
        const ctx = canvas.getContext('2d');
        const padding = { top: 20, right: 20, bottom: 20, left: 40 };
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
        const dataPoints = data.critical.length;
        
        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        
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
        
        // Draw stacked area chart with gradients
        const keys = ['info', 'low', 'moderate', 'high', 'critical'];
        const stackedData = [];
        
        // Calculate stacked values
        for (let i = 0; i < dataPoints; i++) {
            let stack = 0;
            stackedData[i] = {};
            keys.forEach(key => {
                stack += data[key][i];
                stackedData[i][key] = stack;
            });
        }
        
        // Draw each layer with gradient
        keys.reverse().forEach((key, keyIndex) => {
            const gradient = ctx.createLinearGradient(0, padding.top, 0, padding.top + chartHeight);
            gradient.addColorStop(0, TIMELINE_COLORS[key]);
            gradient.addColorStop(1, TIMELINE_COLORS[key].replace(/[\d.]+\)/, '0.1)'));
            
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
            
            // Draw the upper line with smooth curves
            for (let i = 0; i < dataPoints; i++) {
                const x = padding.left + (chartWidth / (dataPoints - 1)) * i;
                const y = padding.top + chartHeight - (stackedData[i][key] / 4000) * chartHeight;
                
                if (i === 0) {
                    ctx.lineTo(x, y);
                } else {
                    // Use quadratic curves for smoother lines
                    const prevX = padding.left + (chartWidth / (dataPoints - 1)) * (i - 1);
                    const prevY = padding.top + chartHeight - (stackedData[i - 1][key] / 4000) * chartHeight;
                    const cpX = (prevX + x) / 2;
                    const cpY = (prevY + y) / 2;
                    ctx.quadraticCurveTo(prevX, prevY, cpX, cpY);
                }
            }
            
            // Draw the last point
            const lastX = padding.left + chartWidth;
            const lastY = padding.top + chartHeight - (stackedData[dataPoints - 1][key] / 4000) * chartHeight;
            ctx.lineTo(lastX, lastY);
            
            // Complete the shape
            ctx.lineTo(canvas.width - padding.right, padding.top + chartHeight);
            ctx.closePath();
            ctx.fill();
            
            // Reset shadow
            ctx.shadowBlur = 0;
        });
        
        ctx.globalAlpha = 1;
        
        // Y-axis labels
        ctx.fillStyle = '#6c6c6c';
        ctx.font = '10px var(--font-family)';
        ctx.textAlign = 'right';
        
        const yLabels = ['0', '500', '1K', '1.5K', '2K'];
        for (let i = 0; i < yLabels.length; i++) {
            const y = padding.top + chartHeight - (chartHeight / (yLabels.length - 1)) * i;
            ctx.fillText(yLabels[i], padding.left - 5, y + 3);
        }
        
        // Draw hover indicator in fixed position
        if (canvas.hoveredLine) {
            drawFixedIndicator(ctx, canvas.hoveredLine, canvas.width - 100, 30);
        }
    }
    
    // Generate timeline data
    function generateTimelineData() {
        const dataPoints = 30;
        const data = {
            critical: [],
            high: [],
            moderate: [],
            low: [],
            info: []
        };
        
        // Generate more realistic data with trends
        for (let i = 0; i < dataPoints; i++) {
            const trend = Math.sin(i / 5) * 0.3 + 0.7;
            data.critical.push(Math.random() * 800 * trend + 200);
            data.high.push(Math.random() * 600 * trend + 150);
            data.moderate.push(Math.random() * 300 * trend + 100);
            data.low.push(Math.random() * 150 * trend + 50);
            data.info.push(Math.random() * 100 * trend + 30);
        }
        
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
        const dataIndex = Math.floor((x - padding.left) / ((canvas.width - padding.left - padding.right) / 29));
        
        if (dataIndex >= 0 && dataIndex < 30 && x >= padding.left && x <= canvas.width - padding.right) {
            let stack = 0;
            const keys = ['info', 'low', 'moderate', 'high', 'critical'];
            
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
            critical: 'Critical',
            high: 'High', 
            moderate: 'Moderate',
            low: 'Low',
            info: 'Info'
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
    function refreshReportData() {
        // Update stats
        updateStats();
        
        // Redraw charts
        initializeCharts();
        
        // Show subtle loading animation
        const reportContent = document.querySelector('.report-content');
        if (reportContent) {
            reportContent.style.opacity = '0.7';
            setTimeout(() => {
                reportContent.style.opacity = '1';
            }, 200);
        }
    }
    
    // Update statistics
    function updateStats() {
        // Update top stats with random values for demo
        const projectsValue = document.querySelector('.stat-box:nth-child(1) .stat-value');
        const scansValue = document.querySelector('.stat-box:nth-child(2) .stat-value');
        const coverageValue = document.querySelector('.stat-box:nth-child(3) .stat-value');
        
        if (projectsValue) {
            projectsValue.textContent = Math.floor(Math.random() * 30 + 10);
        }
        
        if (scansValue) {
            scansValue.textContent = Math.floor(Math.random() * 20 + 5);
        }
        
        if (coverageValue) {
            const coverage = (Math.random() * 30 + 70).toFixed(2);
            coverageValue.textContent = `${coverage}%`;
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
