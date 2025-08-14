// Test Report Manager Module - JetBrains Style
(function() {
    'use strict';

    // Module state
    let currentTimeRange = 30;
    let currentProject = 'all';
    let currentSeverity = 'all';
    
    // Chart colors matching JetBrains theme
    const SEVERITY_COLORS = {
        critical: '#db2838',
        high: '#f68e5f', 
        moderate: '#fbb04d',
        low: '#7ca82b',
        info: '#659ad2'
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
    
    // Draw severity donut chart
    function drawSeverityDonut(canvas) {
        const ctx = canvas.getContext('2d');
        const centerX = canvas.width / 2;
        const centerY = canvas.height / 2;
        const radius = 70;
        const innerRadius = 50;
        
        const data = [
            { value: 1897, color: SEVERITY_COLORS.critical },
            { value: 1613, color: SEVERITY_COLORS.high },
            { value: 439, color: SEVERITY_COLORS.moderate }
        ];
        
        const total = data.reduce((sum, item) => sum + item.value, 0);
        let currentAngle = -Math.PI / 2;
        
        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        
        // Draw segments
        data.forEach(item => {
            const sliceAngle = (item.value / total) * 2 * Math.PI;
            
            ctx.beginPath();
            ctx.arc(centerX, centerY, radius, currentAngle, currentAngle + sliceAngle);
            ctx.arc(centerX, centerY, innerRadius, currentAngle + sliceAngle, currentAngle, true);
            ctx.closePath();
            ctx.fillStyle = item.color;
            ctx.fill();
            
            currentAngle += sliceAngle;
        });
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
    
    // Draw timeline chart
    function drawTimelineChart(canvas) {
        const ctx = canvas.getContext('2d');
        const padding = { top: 20, right: 20, bottom: 20, left: 40 };
        const chartWidth = canvas.width - padding.left - padding.right;
        const chartHeight = canvas.height - padding.top - padding.bottom;
        
        // Clear canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        
        // Draw grid lines
        ctx.strokeStyle = 'rgba(60, 60, 60, 0.2)';
        ctx.lineWidth = 0.5;
        
        // Horizontal grid lines
        const ySteps = 5;
        for (let i = 0; i <= ySteps; i++) {
            const y = padding.top + (chartHeight / ySteps) * i;
            ctx.beginPath();
            ctx.moveTo(padding.left, y);
            ctx.lineTo(canvas.width - padding.right, y);
            ctx.stroke();
        }
        
        // Generate sample data
        const dataPoints = 30;
        const data = {
            critical: [],
            high: [],
            moderate: [],
            low: []
        };
        
        for (let i = 0; i < dataPoints; i++) {
            data.critical.push(Math.random() * 2000);
            data.high.push(Math.random() * 1500);
            data.moderate.push(Math.random() * 500);
            data.low.push(Math.random() * 200);
        }
        
        // Draw stacked area chart
        const keys = ['critical', 'high', 'moderate', 'low'];
        const colors = [SEVERITY_COLORS.critical, SEVERITY_COLORS.high, SEVERITY_COLORS.moderate, SEVERITY_COLORS.low];
        
        keys.forEach((key, keyIndex) => {
            ctx.beginPath();
            ctx.fillStyle = colors[keyIndex];
            ctx.globalAlpha = 0.6;
            
            // Start from bottom
            ctx.moveTo(padding.left, padding.top + chartHeight);
            
            // Draw the line
            for (let i = 0; i < dataPoints; i++) {
                const x = padding.left + (chartWidth / (dataPoints - 1)) * i;
                let y = 0;
                
                // Stack values
                for (let j = 0; j <= keyIndex; j++) {
                    y += data[keys[j]][i];
                }
                
                const normalizedY = padding.top + chartHeight - (y / 4000) * chartHeight;
                ctx.lineTo(x, normalizedY);
            }
            
            // Close the path
            ctx.lineTo(canvas.width - padding.right, padding.top + chartHeight);
            ctx.closePath();
            ctx.fill();
        });
        
        ctx.globalAlpha = 1;
        
        // Y-axis labels
        ctx.fillStyle = 'var(--text-secondary)';
        ctx.font = '10px var(--font-family)';
        ctx.textAlign = 'right';
        
        const yLabels = ['0', '500', '1K', '1.5K', '2K'];
        for (let i = 0; i < yLabels.length; i++) {
            const y = padding.top + chartHeight - (chartHeight / (yLabels.length - 1)) * i;
            ctx.fillText(yLabels[i], padding.left - 5, y + 3);
        }
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
