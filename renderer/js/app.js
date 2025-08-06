// Store electron modules before Monaco loader interferes
const electron = window.nodeRequire ? window.nodeRequire('electron') : require('electron');
const { ipcRenderer } = electron;
const path = window.nodeRequire ? window.nodeRequire('path') : require('path');
const fs = (window.nodeRequire ? window.nodeRequire('fs') : require('fs')).promises;
const yaml = window.nodeRequire ? window.nodeRequire('js-yaml') : require('js-yaml');
const { parse } = window.nodeRequire ? window.nodeRequire('csv-parse/sync') : require('csv-parse/sync');

// Global variables
let currentProject = null;
let currentCase = null;
let monacoEditor = null;
let openTabs = [];
let deviceScreenInterval = null;

// Initialize the application
document.addEventListener('DOMContentLoaded', async () => {
    initializeNavigation();
    initializeProjectPage();
    initializeTestcasePage();
    initializeDevicePage();
    initializeSettingsPage();
    initializeMonacoEditor();
    
    // Load user info
    loadUserInfo();
    
    // Check for existing project
    const lastProject = await ipcRenderer.invoke('store-get', 'last_project');
    if (lastProject) {
        await loadProject(lastProject);
    }
});

// Navigation
function initializeNavigation() {
    const navItems = document.querySelectorAll('.nav-item');
    const pages = document.querySelectorAll('.page');
    
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const targetPage = item.dataset.page;
            
            // Update active nav item
            navItems.forEach(nav => nav.classList.remove('active'));
            item.classList.add('active');
            
            // Update active page
            pages.forEach(page => page.classList.remove('active'));
            document.getElementById(`${targetPage}Page`).classList.add('active');
            
            // Page-specific actions
            if (targetPage === 'device') {
                refreshConnectedDevices();
            } else if (targetPage === 'settings') {
                checkSDKStatus();
            }
        });
    });
}

// Project Page
function initializeProjectPage() {
    const createProjectBtn = document.getElementById('createProjectBtn');
    const openProjectBtn = document.getElementById('openProjectBtn');
    const importCsvBtn = document.getElementById('importCsvBtn');
    
    console.log('Initializing project page...');
    
    if (createProjectBtn) {
        createProjectBtn.addEventListener('click', async () => {
            console.log('Create project button clicked');
            try {
                const projectPath = await ipcRenderer.invoke('select-directory');
                console.log('Selected path:', projectPath);
                if (projectPath) {
                    const result = await ipcRenderer.invoke('create-project-structure', projectPath);
                    if (result.success) {
                        await loadProject(projectPath);
                        showNotification('Project created successfully', 'success');
                    } else {
                        showNotification(`Failed to create project: ${result.error}`, 'error');
                    }
                }
            } catch (error) {
                console.error('Error in create project:', error);
                showNotification(`Error: ${error.message}`, 'error');
            }
        });
    }
    
    if (openProjectBtn) {
        openProjectBtn.addEventListener('click', async () => {
            console.log('Open project button clicked');
            try {
                const projectPath = await ipcRenderer.invoke('select-directory');
                console.log('Selected path:', projectPath);
                if (projectPath) {
                    await loadProject(projectPath);
                }
            } catch (error) {
                console.error('Error in open project:', error);
                showNotification(`Error: ${error.message}`, 'error');
            }
        });
    }
    
    if (importCsvBtn) {
        importCsvBtn.addEventListener('click', async () => {
            if (!currentProject) {
                showNotification('Please open a project first', 'warning');
                return;
            }
            
            const csvPath = await ipcRenderer.invoke('select-file', [
                { name: 'CSV Files', extensions: ['csv'] }
            ]);
            
            if (csvPath) {
                const result = await ipcRenderer.invoke('read-file', csvPath);
                if (result.success) {
                    try {
                        const records = parse(result.content, {
                            columns: true,
                            skip_empty_lines: true
                        });
                        
                        // Save to project
                        const sheetPath = path.join(currentProject, 'testcase_sheet.csv');
                        await ipcRenderer.invoke('write-file', sheetPath, result.content);
                        
                        // Display test cases in table
                        displayTestCasesTable(records);
                        showNotification(`Imported ${records.length} test cases`, 'success');
                    } catch (error) {
                        showNotification(`Failed to parse CSV: ${error.message}`, 'error');
                    }
                }
            }
        });
    }
}

// Load project
async function loadProject(projectPath) {
    currentProject = projectPath;
    await ipcRenderer.invoke('store-set', 'last_project', projectPath);
    
    // Update UI
    document.getElementById('projectPath').textContent = projectPath;
    document.getElementById('currentProjectPath').textContent = projectPath;
    document.getElementById('projectInfo').style.display = 'block';
    document.getElementById('welcomeScreen').style.display = 'none';
    
    // Load test cases if CSV exists
    const sheetPath = path.join(projectPath, 'testcase_sheet.csv');
    const result = await ipcRenderer.invoke('read-file', sheetPath);
    if (result.success && result.content) {
        try {
            const records = parse(result.content, {
                columns: true,
                skip_empty_lines: true
            });
            displayTestCasesTable(records);
        } catch (error) {
            console.error('Failed to parse existing CSV:', error);
        }
    }
    
    // Load file tree
    await loadFileTree();
}

// Display test cases in table format
function displayTestCasesTable(records) {
    const testcaseList = document.getElementById('testcaseList');
    
    if (records.length === 0) {
        testcaseList.innerHTML = '<div class="text-muted">No test cases imported yet</div>';
        return;
    }
    
    // Get all column headers
    const headers = Object.keys(records[0]);
    
    // Create table
    const table = document.createElement('table');
    table.className = 'testcase-table';
    
    // Create header
    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    headers.forEach(header => {
        const th = document.createElement('th');
        th.textContent = header;
        headerRow.appendChild(th);
    });
    thead.appendChild(headerRow);
    table.appendChild(thead);
    
    // Create body
    const tbody = document.createElement('tbody');
    records.forEach((record, index) => {
        const row = document.createElement('tr');
        row.className = 'testcase-row';
        row.dataset.index = index;
        
        headers.forEach(header => {
            const td = document.createElement('td');
            td.textContent = record[header] || '';
            td.title = record[header] || ''; // Tooltip for long text
            row.appendChild(td);
        });
        
        // Create floating action button
        const actionBtn = document.createElement('button');
        actionBtn.className = 'floating-action-btn';
        actionBtn.innerHTML = `
            <svg viewBox="0 0 24 24" width="16" height="16">
                <path fill="currentColor" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
            </svg>
            <span>Create Case</span>
        `;
        actionBtn.onclick = (e) => {
            e.stopPropagation();
            createTestCase(record, index);
        };
        
        row.appendChild(actionBtn);
        
        tbody.appendChild(row);
    });
    
    table.appendChild(tbody);
    
    // Clear existing content and add table
    testcaseList.innerHTML = '';
    testcaseList.appendChild(table);
}

// Create test case
async function createTestCase(record, index) {
    if (!currentProject) {
        showNotification('No project loaded', 'error');
        return;
    }
    
    const caseName = `case_${String(index + 1).padStart(3, '0')}`;
    const casePath = path.join(currentProject, 'cases', caseName);
    
    try {
        // Create case directory structure
        await fs.mkdir(casePath, { recursive: true });
        await fs.mkdir(path.join(casePath, 'locator'), { recursive: true });
        await fs.mkdir(path.join(casePath, 'locator', 'img'), { recursive: true });
        await fs.mkdir(path.join(casePath, 'script'), { recursive: true });
        
        // Create config.json
        const config = {
            name: caseName,
            description: record[Object.keys(record)[0]] || '',
            requirements: record[Object.keys(record)[1]] || '',
            createdAt: new Date().toISOString(),
            record: record
        };
        
        await fs.writeFile(
            path.join(casePath, 'config.json'),
            JSON.stringify(config, null, 2)
        );
        
        // Create element.json
        await fs.writeFile(
            path.join(casePath, 'locator', 'element.json'),
            JSON.stringify({}, null, 2)
        );
        
        // Create sample script
        const sampleScript = {
            name: 'script_001',
            steps: [
                {
                    action: 'launch_app',
                    params: {
                        appPackage: 'com.example.app',
                        appActivity: '.MainActivity'
                    }
                }
            ]
        };
        
        await fs.writeFile(
            path.join(casePath, 'script', 'script_001.yaml'),
            yaml.dump(sampleScript)
        );
        
        // Update testcase_map.json
        const mapPath = path.join(currentProject, 'testcase_map.json');
        let testcaseMap = {};
        
        try {
            const mapContent = await fs.readFile(mapPath, 'utf-8');
            testcaseMap = JSON.parse(mapContent);
        } catch (error) {
            // File doesn't exist or is invalid
        }
        
        testcaseMap[caseName] = {
            index: index,
            createdAt: new Date().toISOString()
        };
        
        await fs.writeFile(mapPath, JSON.stringify(testcaseMap, null, 2));
        
        // Mark row as created
        const row = document.querySelector(`.testcase-row[data-index="${index}"]`);
        if (row) {
            row.classList.add('case-created');
        }
        
        // Navigate to testcase page
        document.querySelector('[data-page="testcase"]').click();
        
        // Reload file tree
        await loadFileTree();
        
        showNotification(`Created test case: ${caseName}`, 'success');
    } catch (error) {
        showNotification(`Failed to create test case: ${error.message}`, 'error');
    }
}

// Testcase Page
function initializeTestcasePage() {
    const runTestBtn = document.getElementById('runTestBtn');
    const clearConsoleBtn = document.getElementById('clearConsoleBtn');
    const toggleXmlBtn = document.getElementById('toggleXmlBtn');
    const refreshDeviceBtn = document.getElementById('refreshDeviceBtn');
    const deviceSelect = document.getElementById('deviceSelect');
    
    if (runTestBtn) runTestBtn.addEventListener('click', runCurrentTest);
    if (clearConsoleBtn) clearConsoleBtn.addEventListener('click', clearConsole);
    if (toggleXmlBtn) toggleXmlBtn.addEventListener('click', toggleXmlOverlay);
    if (refreshDeviceBtn) refreshDeviceBtn.addEventListener('click', () => {
        // Manual refresh when button is clicked
        refreshDeviceScreen();
    });
    
    if (deviceSelect) {
        deviceSelect.addEventListener('change', (e) => {
            // Store selected device
            if (e.target.value) {
                ipcRenderer.invoke('store-set', 'selected_device', e.target.value);
            }
            // Don't start auto-refresh anymore
        });
    }
    
    // Load devices
    refreshDeviceList();
}

// Load file tree
async function loadFileTree() {
    if (!currentProject) return;
    
    const fileTree = document.getElementById('fileTree');
    fileTree.innerHTML = '';
    
    const casesPath = path.join(currentProject, 'cases');
    
    try {
        const cases = await fs.readdir(casesPath);
        
        for (const caseName of cases) {
            const casePath = path.join(casesPath, caseName);
            const stat = await fs.stat(casePath);
            
            if (stat.isDirectory()) {
                const caseItem = createTreeItem(caseName, 'folder', casePath);
                fileTree.appendChild(caseItem);
                
                // Load scripts
                const scriptPath = path.join(casePath, 'script');
                try {
                    const scripts = await fs.readdir(scriptPath);
                    const scriptContainer = document.createElement('div');
                    scriptContainer.className = 'tree-children';
                    
                    for (const script of scripts) {
                        if (script.endsWith('.yaml')) {
                            const scriptItem = createTreeItem(
                                script,
                                'file',
                                path.join(scriptPath, script)
                            );
                            scriptContainer.appendChild(scriptItem);
                        }
                    }
                    
                    caseItem.appendChild(scriptContainer);
                } catch (error) {
                    console.error('Failed to load scripts:', error);
                }
            }
        }
    } catch (error) {
        console.error('Failed to load file tree:', error);
    }
}

// Create tree item
function createTreeItem(name, type, fullPath) {
    const item = document.createElement('div');
    item.className = `tree-item ${type === 'folder' ? 'tree-folder' : ''}`;
    
    const icon = document.createElement('svg');
    icon.className = 'tree-icon';
    icon.setAttribute('viewBox', '0 0 24 24');
    
    if (type === 'folder') {
        icon.innerHTML = '<path d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>';
    } else {
        icon.innerHTML = '<path d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm-1 7V3.5L18.5 9H13z"/>';
    }
    
    const label = document.createElement('span');
    label.textContent = name;
    
    item.appendChild(icon);
    item.appendChild(label);
    
    if (type === 'file') {
        item.addEventListener('click', () => openFile(fullPath));
    }
    
    return item;
}

// Open file in editor
async function openFile(filePath) {
    const result = await ipcRenderer.invoke('read-file', filePath);
    if (result.success) {
        const fileName = path.basename(filePath);
        
        // Check if already open
        const existingTab = openTabs.find(tab => tab.path === filePath);
        if (existingTab) {
            selectTab(existingTab.id);
            return;
        }
        
        // Create new tab
        const tabId = `tab-${Date.now()}`;
        const tab = {
            id: tabId,
            path: filePath,
            name: fileName,
            content: result.content
        };
        
        openTabs.push(tab);
        createTab(tab);
        selectTab(tabId);
        
        // Load content in editor
        if (monacoEditor) {
            monacoEditor.setValue(result.content);
            
            // Set language based on file extension
            const ext = path.extname(fileName).toLowerCase();
            let language = 'plaintext';
            
            if (ext === '.yaml' || ext === '.yml') {
                language = 'yaml';
            } else if (ext === '.json') {
                language = 'json';
            } else if (ext === '.xml') {
                language = 'xml';
            }
            
            monaco.editor.setModelLanguage(monacoEditor.getModel(), language);
        }
    } else {
        showNotification(`Failed to open file: ${result.error}`, 'error');
    }
}

// Create tab
function createTab(tab) {
    const tabsContainer = document.getElementById('editorTabs');
    
    const tabElement = document.createElement('div');
    tabElement.className = 'tab';
    tabElement.id = tab.id;
    tabElement.innerHTML = `
        <span>${tab.name}</span>
        <span class="tab-close">×</span>
    `;
    
    tabElement.addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab-close')) {
            selectTab(tab.id);
        }
    });
    
    tabElement.querySelector('.tab-close').addEventListener('click', (e) => {
        e.stopPropagation();
        closeTab(tab.id);
    });
    
    tabsContainer.appendChild(tabElement);
}

// Select tab
function selectTab(tabId) {
    const tabs = document.querySelectorAll('.tab');
    tabs.forEach(tab => tab.classList.remove('active'));
    
    const selectedTab = document.getElementById(tabId);
    if (selectedTab) {
        selectedTab.classList.add('active');
        
        const tabData = openTabs.find(tab => tab.id === tabId);
        if (tabData && monacoEditor) {
            monacoEditor.setValue(tabData.content);
        }
    }
}

// Close tab
function closeTab(tabId) {
    const index = openTabs.findIndex(tab => tab.id === tabId);
    if (index > -1) {
        openTabs.splice(index, 1);
        document.getElementById(tabId).remove();
        
        // Select another tab if available
        if (openTabs.length > 0) {
            selectTab(openTabs[openTabs.length - 1].id);
        } else {
            if (monacoEditor) {
                monacoEditor.setValue('');
            }
        }
    }
}

// Initialize Monaco Editor
function initializeMonacoEditor() {
    // Check if require is already AMD
    if (typeof require !== 'undefined' && require.config) {
        require.config({ paths: { vs: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs' } });
        
        require(['vs/editor/editor.main'], function() {
            monacoEditor = monaco.editor.create(document.getElementById('monacoEditor'), {
                value: '# Welcome to Test Toolkit Studio\n# Open a test script to start editing',
                language: 'yaml',
                theme: 'vs-dark',
                automaticLayout: true,
                minimap: { enabled: false },
                fontSize: 13,
                fontFamily: "'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace",
                scrollBeyondLastLine: false,
                renderLineHighlight: 'all',
                lineNumbers: 'on',
                glyphMargin: false,
                folding: true,
                lineDecorationsWidth: 0,
                lineNumbersMinChars: 3
            });
            
            // Save on change
            monacoEditor.onDidChangeModelContent(() => {
                const activeTab = document.querySelector('.tab.active');
                if (activeTab) {
                    const tabId = activeTab.id;
                    const tab = openTabs.find(t => t.id === tabId);
                    if (tab) {
                        tab.content = monacoEditor.getValue();
                        // Auto-save
                        saveCurrentFile();
                    }
                }
            });
        });
    }
}

// Save current file
async function saveCurrentFile() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) return;
    
    const tabId = activeTab.id;
    const tab = openTabs.find(t => t.id === tabId);
    
    if (tab) {
        const result = await ipcRenderer.invoke('write-file', tab.path, tab.content);
        if (!result.success) {
            console.error('Failed to save file:', result.error);
        }
    }
}

// Run current test
async function runCurrentTest() {
    const activeTab = document.querySelector('.tab.active');
    if (!activeTab) {
        showNotification('No test script open', 'warning');
        return;
    }
    
    const deviceSelect = document.getElementById('deviceSelect');
    if (!deviceSelect.value) {
        showNotification('Please select a device', 'warning');
        return;
    }
    
    addConsoleLog('Running test...', 'info');
    
    // TODO: Implement actual test execution
    setTimeout(() => {
        addConsoleLog('Test execution not yet implemented', 'warning');
    }, 1000);
}

// Console management
function addConsoleLog(message, type = 'info') {
    const consoleOutput = document.getElementById('consoleOutput');
    const line = document.createElement('div');
    line.className = `console-line ${type}`;
    line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    consoleOutput.appendChild(line);
    consoleOutput.scrollTop = consoleOutput.scrollHeight;
}

function clearConsole() {
    document.getElementById('consoleOutput').innerHTML = '';
}

// Device screen management
async function refreshDeviceScreen() {
    const deviceSelect = document.getElementById('deviceSelect');
    if (!deviceSelect || !deviceSelect.value) {
        showNotification('Please select a device', 'warning');
        return;
    }
    
    const result = await ipcRenderer.invoke('adb-screenshot', deviceSelect.value);
    if (result.success) {
        const img = document.getElementById('deviceScreenshot');
        img.src = `data:image/png;base64,${result.data}`;
        img.style.display = 'block';
        document.querySelector('.screen-placeholder').style.display = 'none';
    } else {
        showNotification(`Failed to capture screen: ${result.error}`, 'error');
    }
}

function toggleXmlOverlay() {
    showNotification('XML overlay not yet implemented', 'info');
}

// Device Page
function initializeDevicePage() {
    const addDeviceBtn = document.getElementById('addDeviceBtn');
    const scanDevicesBtn = document.getElementById('scanDevicesBtn');
    const deviceForm = document.getElementById('deviceForm');
    const newDeviceForm = document.getElementById('newDeviceForm');
    const cancelDeviceBtn = document.getElementById('cancelDeviceBtn');
    
    if (addDeviceBtn) {
        addDeviceBtn.addEventListener('click', () => {
            deviceForm.style.display = deviceForm.style.display === 'none' ? 'block' : 'none';
        });
    }
    
    if (cancelDeviceBtn) {
        cancelDeviceBtn.addEventListener('click', () => {
            deviceForm.style.display = 'none';
            newDeviceForm.reset();
        });
    }
    
    if (scanDevicesBtn) {
        scanDevicesBtn.addEventListener('click', refreshConnectedDevices);
    }
    
    if (newDeviceForm) {
        newDeviceForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            
            if (!currentProject) {
                showNotification('Please open a project first', 'warning');
                return;
            }
            
            const formData = new FormData(newDeviceForm);
            const deviceConfig = {};
            
            for (const [key, value] of formData.entries()) {
                deviceConfig[key] = value === 'true' ? true : value === 'false' ? false : value;
            }
            
            // Check if we're in edit mode
            const deviceForm = document.getElementById('deviceForm');
            const mode = deviceForm.dataset.mode;
            let devicePath;
            
            if (mode === 'edit' && deviceForm.dataset.filename) {
                // Edit mode - use existing filename
                devicePath = path.join(currentProject, 'devices', deviceForm.dataset.filename);
            } else {
                // Create mode - generate new filename
                const timestamp = Date.now();
                const deviceFileName = `device_${timestamp}.yaml`;
                devicePath = path.join(currentProject, 'devices', deviceFileName);
            }
            
            try {
                await fs.writeFile(devicePath, yaml.dump(deviceConfig));
                showNotification(
                    mode === 'edit' ? 'Device updated successfully' : 'Device saved successfully',
                    'success'
                );
                
                deviceForm.style.display = 'none';
                newDeviceForm.reset();
                delete deviceForm.dataset.mode;
                delete deviceForm.dataset.filename;
                
                await loadSavedDevices();
                await refreshDeviceList();
            } catch (error) {
                showNotification(`Failed to save device: ${error.message}`, 'error');
            }
        });
    }
    
    // Load saved devices
    loadSavedDevices();
}

// Load saved devices
async function loadSavedDevices() {
    if (!currentProject) return;
    
    const deviceGrid = document.getElementById('deviceGrid');
    deviceGrid.innerHTML = '';
    
    const devicesPath = path.join(currentProject, 'devices');
    
    try {
        // Get connected devices first
        const connectedResult = await ipcRenderer.invoke('adb-devices');
        const connectedDevices = connectedResult.success ? connectedResult.devices : [];
        
        const files = await fs.readdir(devicesPath);
        
        for (const file of files) {
            if (file.endsWith('.yaml')) {
                const filePath = path.join(devicesPath, file);
                const content = await fs.readFile(filePath, 'utf-8');
                const config = yaml.load(content);
                
                // Check if this device is connected by matching deviceId
                const isConnected = config.deviceId && connectedDevices.some(d => 
                    d.id === config.deviceId && d.status === 'device'
                );
                
                const card = document.createElement('div');
                card.className = 'device-card';
                card.innerHTML = `
                    <div class="device-status ${isConnected ? 'connected' : ''}" title="${isConnected ? 'Connected' : 'Not Connected'}"></div>
                    <div class="device-name">${config.deviceName}</div>
                    <div class="device-info">
                        <span title="${config.platformName} ${config.platformVersion}">Platform: ${config.platformName} ${config.platformVersion}</span>
                        <span title="${config.appPackage}">Package: ${config.appPackage}</span>
                        <span title="${config.automationName}">Automation: ${config.automationName}</span>
                        ${config.deviceId ? `<span title="${config.deviceId}">ID: ${config.deviceId}</span>` : ''}
                    </div>
                    <div class="device-actions">
                        <button class="btn btn-secondary btn-small" onclick="editDevice('${file}')">
                            <svg viewBox="0 0 24 24" width="14" height="14" style="vertical-align: middle; margin-right: 4px;">
                                <path fill="currentColor" d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                            </svg>
                            Edit
                        </button>
                        <button class="btn btn-outline btn-small btn-icon-only" onclick="deleteDevice('${file}')" title="Delete">
                            <svg viewBox="0 0 24 24" width="14" height="14">
                                <path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
                            </svg>
                        </button>
                    </div>
                `;
                
                deviceGrid.appendChild(card);
            }
        }
    } catch (error) {
        console.error('Failed to load saved devices:', error);
    }
}

// Refresh connected devices
async function refreshConnectedDevices() {
    const result = await ipcRenderer.invoke('adb-devices');
    const connectedList = document.getElementById('connectedList');
    
    connectedList.innerHTML = '';
    
    if (result.success && result.devices.length > 0) {
        // Get saved devices to check which ones are already saved
        let savedDeviceIds = [];
        if (currentProject) {
            try {
                const devicesPath = path.join(currentProject, 'devices');
                const files = await fs.readdir(devicesPath);
                for (const file of files) {
                    if (file.endsWith('.yaml')) {
                        const content = await fs.readFile(path.join(devicesPath, file), 'utf-8');
                        const config = yaml.load(content);
                        if (config.deviceId) {
                            savedDeviceIds.push(config.deviceId);
                        }
                    }
                }
            } catch (error) {
                console.error('Error loading saved devices:', error);
            }
        }
        
        result.devices.forEach(device => {
            const isSaved = savedDeviceIds.includes(device.id);
            const item = document.createElement('div');
            item.className = 'connected-item';
            item.innerHTML = `
                <span class="device-id">${device.id}</span>
                <span class="device-status-text">${device.status}</span>
                ${!isSaved && device.status === 'device' ? `
                    <button class="btn btn-primary" onclick="createDeviceFromConnected('${device.id}')">
                        <svg viewBox="0 0 24 24" width="14" height="14" style="vertical-align: middle; margin-right: 4px;">
                            <path fill="currentColor" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
                        </svg>
                        Save
                    </button>
                ` : isSaved ? '<span class="text-muted" style="font-size: 12px;">Already saved</span>' : ''}
            `;
            connectedList.appendChild(item);
        });
    } else {
        connectedList.innerHTML = '<div class="text-muted">No devices connected</div>';
    }
    
    // Also reload saved devices to update connection status
    await loadSavedDevices();
    
    // Update device select dropdown
    refreshDeviceList();
}

// Refresh device list in dropdown
async function refreshDeviceList() {
    const result = await ipcRenderer.invoke('adb-devices');
    const deviceSelect = document.getElementById('deviceSelect');
    
    if (!deviceSelect) return;
    
    // Get saved device to restore selection
    const savedSelection = await ipcRenderer.invoke('store-get', 'selected_device');
    
    deviceSelect.innerHTML = '<option value="">Select Device</option>';
    
    if (result.success && result.devices.length > 0) {
        // Load saved device configurations to show names instead of IDs
        let deviceConfigs = {};
        if (currentProject) {
            try {
                const devicesPath = path.join(currentProject, 'devices');
                const files = await fs.readdir(devicesPath);
                for (const file of files) {
                    if (file.endsWith('.yaml')) {
                        const content = await fs.readFile(path.join(devicesPath, file), 'utf-8');
                        const config = yaml.load(content);
                        if (config.deviceId) {
                            deviceConfigs[config.deviceId] = config.deviceName;
                        }
                    }
                }
            } catch (error) {
                console.error('Error loading device configs:', error);
            }
        }
        
        result.devices.forEach(device => {
            if (device.status === 'device') {
                const option = document.createElement('option');
                option.value = device.id;
                // Use device name if saved, otherwise use ID
                option.textContent = deviceConfigs[device.id] || device.id;
                deviceSelect.appendChild(option);
            }
        });
        
        // Restore selection if device is still available
        if (savedSelection && Array.from(deviceSelect.options).some(opt => opt.value === savedSelection)) {
            deviceSelect.value = savedSelection;
        }
    }
}

// Settings Page
function initializeSettingsPage() {
    const logoutBtn = document.getElementById('logoutBtn');
    const checkSdkBtn = document.getElementById('checkSdkBtn');
    const updateBaseUrlBtn = document.getElementById('updateBaseUrlBtn');
    const settingsBaseUrl = document.getElementById('settingsBaseUrl');
    
    if (logoutBtn) {
        logoutBtn.addEventListener('click', async () => {
            await ipcRenderer.invoke('logout');
            await ipcRenderer.invoke('navigate-to-login');
        });
    }
    
    if (checkSdkBtn) {
        checkSdkBtn.addEventListener('click', checkSDKStatus);
    }
    
    if (updateBaseUrlBtn) {
        updateBaseUrlBtn.addEventListener('click', async () => {
            await ipcRenderer.invoke('store-set', 'base_url', settingsBaseUrl.value);
            showNotification('Base URL updated', 'success');
        });
    }
    
    // Load base URL
    ipcRenderer.invoke('store-get', 'base_url').then(url => {
        if (url && settingsBaseUrl) {
            settingsBaseUrl.value = url;
        }
    });
}

// Load user info
async function loadUserInfo() {
    const result = await ipcRenderer.invoke('get-user-info');
    
    if (result.success && result.data.user) {
        const user = result.data.user;
        
        // Update sidebar
        const userAvatar = document.querySelector('.user-avatar');
        const userName = document.querySelector('.user-name');
        
        if (userAvatar) userAvatar.textContent = user.username.charAt(0).toUpperCase();
        if (userName) userName.textContent = user.username;
        
        // Update settings page
        const settingsUsername = document.getElementById('settingsUsername');
        const settingsEmail = document.getElementById('settingsEmail');
        const settingsMemberGroup = document.getElementById('settingsMemberGroup');
        const settingsMemberSince = document.getElementById('settingsMemberSince');
        
        if (settingsUsername) settingsUsername.textContent = user.username;
        if (settingsEmail) settingsEmail.textContent = user.email;
        if (settingsMemberGroup) settingsMemberGroup.textContent = user.memberGroup;
        if (settingsMemberSince) settingsMemberSince.textContent = new Date(user.createdAt).toLocaleDateString();
    }
}

// Check SDK status
async function checkSDKStatus() {
    const sdkStatus = document.getElementById('sdkStatus');
    const adbStatus = document.getElementById('adbStatus');
    
    if (sdkStatus) sdkStatus.textContent = 'Checking...';
    if (adbStatus) adbStatus.textContent = 'Checking...';
    
    // Check ADB
    const result = await ipcRenderer.invoke('adb-devices');
    
    if (result.success) {
        if (adbStatus) {
            adbStatus.textContent = `Available (v${result.adbVersion || 'Unknown'})`;
            adbStatus.className = 'status-indicator success';
        }
        if (sdkStatus) {
            const platform = process.platform === 'darwin' ? 'macOS' : process.platform === 'win32' ? 'Windows' : 'Linux';
            sdkStatus.textContent = `Built-in SDK (${platform})`;
            sdkStatus.className = 'status-indicator success';
        }
    } else {
        if (adbStatus) {
            adbStatus.textContent = 'Not Available';
            adbStatus.className = 'status-indicator error';
        }
        if (sdkStatus) {
            sdkStatus.textContent = 'Not Found';
            sdkStatus.className = 'status-indicator error';
        }
    }
}

// Show notification
function showNotification(message, type = 'info') {
    console.log(`[${type.toUpperCase()}] ${message}`);
    
    // Create a simple notification toast
    const toast = document.createElement('div');
    toast.className = `notification notification-${type}`;
    toast.textContent = message;
    toast.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        padding: 12px 20px;
        background: ${type === 'success' ? '#4ec9b0' : type === 'error' ? '#f48771' : type === 'warning' ? '#ce9178' : '#569cd6'};
        color: white;
        border-radius: 6px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        z-index: 10000;
        animation: slideInUp 0.3s ease;
        font-size: 13px;
    `;
    
    document.body.appendChild(toast);
    
    setTimeout(() => {
        toast.style.animation = 'slideOutDown 0.3s ease';
        setTimeout(() => {
            if (toast.parentNode) {
                document.body.removeChild(toast);
            }
        }, 300);
    }, 3000);
    
    // Also add to console if on testcase page
    if (document.getElementById('testcasePage') && document.getElementById('testcasePage').classList.contains('active')) {
        addConsoleLog(message, type);
    }
}

// Handle IPC messages from main process
ipcRenderer.on('menu-new-project', () => {
    const btn = document.getElementById('createProjectBtn');
    if (btn) btn.click();
});

ipcRenderer.on('menu-open-project', () => {
    const btn = document.getElementById('openProjectBtn');
    if (btn) btn.click();
});

ipcRenderer.on('menu-run-test', () => {
    if (document.getElementById('testcasePage').classList.contains('active')) {
        runCurrentTest();
    }
});

ipcRenderer.on('menu-stop-test', () => {
    showNotification('Stop test not yet implemented', 'info');
});

ipcRenderer.on('menu-refresh-device', () => {
    if (document.getElementById('testcasePage').classList.contains('active')) {
        refreshDeviceScreen();
    }
});

// Global functions for device management
window.createDeviceFromConnected = async function(deviceId) {
    if (!currentProject) {
        showNotification('Please open a project first', 'warning');
        return;
    }
    
    // Show device form with pre-filled device ID
    const deviceForm = document.getElementById('deviceForm');
    const newDeviceForm = document.getElementById('newDeviceForm');
    
    // Reset form and set mode
    newDeviceForm.reset();
    deviceForm.dataset.mode = 'create';
    
    // Pre-fill the form
    if (newDeviceForm) {
        newDeviceForm.querySelector('input[name="deviceId"]').value = deviceId;
        newDeviceForm.querySelector('input[name="deviceName"]').value = `Device ${deviceId.substring(0, 8)}`;
        newDeviceForm.querySelector('input[name="platformName"]').value = 'Android';
        newDeviceForm.querySelector('input[name="automationName"]').value = 'UiAutomator2';
        newDeviceForm.querySelector('select[name="noReset"]').value = 'true';
        newDeviceForm.querySelector('input[name="newCommandTimeout"]').value = '6000';
    }
    
    // Update form title
    deviceForm.querySelector('h3').textContent = 'Add New Device';
    deviceForm.style.display = 'block';
    showNotification('Please complete the device configuration', 'info');
};

// Edit device configuration
window.editDevice = async function(filename) {
    if (!currentProject) return;
    
    try {
        const devicePath = path.join(currentProject, 'devices', filename);
        const content = await fs.readFile(devicePath, 'utf-8');
        const config = yaml.load(content);
        
        // Show device form with existing data
        const deviceForm = document.getElementById('deviceForm');
        const newDeviceForm = document.getElementById('newDeviceForm');
        
        // Set mode and store filename
        deviceForm.dataset.mode = 'edit';
        deviceForm.dataset.filename = filename;
        
        // Fill the form with existing data
        if (newDeviceForm) {
            newDeviceForm.querySelector('input[name="deviceName"]').value = config.deviceName || '';
            newDeviceForm.querySelector('input[name="deviceId"]').value = config.deviceId || '';
            newDeviceForm.querySelector('input[name="platformName"]').value = config.platformName || 'Android';
            newDeviceForm.querySelector('input[name="platformVersion"]').value = config.platformVersion || '';
            newDeviceForm.querySelector('input[name="appPackage"]').value = config.appPackage || '';
            newDeviceForm.querySelector('input[name="appActivity"]').value = config.appActivity || '';
            newDeviceForm.querySelector('input[name="automationName"]').value = config.automationName || 'UiAutomator2';
            newDeviceForm.querySelector('input[name="newCommandTimeout"]').value = config.newCommandTimeout || '6000';
            newDeviceForm.querySelector('select[name="noReset"]').value = config.noReset ? 'true' : 'false';
        }
        
        // Update form title
        deviceForm.querySelector('h3').textContent = 'Edit Device';
        deviceForm.style.display = 'block';
    } catch (error) {
        showNotification(`Failed to load device: ${error.message}`, 'error');
    }
};

// Delete device configuration
window.deleteDevice = async function(filename) {
    if (!currentProject) return;
    
    if (confirm('Are you sure you want to delete this device configuration?')) {
        try {
            const devicePath = path.join(currentProject, 'devices', filename);
            await fs.unlink(devicePath);
            showNotification('Device configuration deleted', 'success');
            await loadSavedDevices();
            await refreshDeviceList();
        } catch (error) {
            showNotification(`Failed to delete device: ${error.message}`, 'error');
        }
    }
};
