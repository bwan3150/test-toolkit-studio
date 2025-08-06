# Test Toolkit Studio

A cross-platform IDE for UI automation testing, designed for users without coding experience to create and run automated test scripts.

## Features

- 🎯 **User-Friendly Interface**: Modern, dark-themed IDE interface inspired by professional development tools
- 📱 **Android Testing Support**: Built-in Android SDK integration for mobile app testing
- 🔐 **Secure Authentication**: Login system with token-based authentication
- 📊 **Test Case Management**: Import test cases from CSV files and organize them in projects
- 📝 **Visual Script Editor**: Monaco-based code editor with syntax highlighting for YAML test scripts
- 🖥️ **Real-time Device Screen**: Live device screen capture and XML element inspection
- 💾 **Project Structure**: Organized file system for test cases, scripts, and device configurations

## Prerequisites

- Node.js 16+ and npm
- Android SDK (will be bundled in production build)

## Installation

1. Clone the repository:
```bash
cd /Users/eric_konec/Documents/GitHub/test-toolkit-studio
```

2. Install dependencies:
```bash
npm install
```

3. Set up Android SDK:
   - Download Android platform-tools
   - Place them in `android-sdk/platform-tools/` directory
   - Ensure `adb` executable is available

## Development

Run the application in development mode:
```bash
npm start
```

Run with live reload:
```bash
npm run dev
```

## Building

Build for macOS:
```bash
npm run build-mac
```

Build for Windows:
```bash
npm run build-win
```

Build for both platforms:
```bash
npm run build
```

## Project Structure

```
test-toolkit-studio/
├── main.js              # Main Electron process
├── renderer/            # Renderer process files
│   ├── index.html      # Main application UI
│   ├── login.html      # Login page
│   ├── styles/         # CSS stylesheets
│   │   ├── common.css  # Common styles
│   │   ├── login.css   # Login page styles
│   │   └── main.css    # Main app styles
│   └── js/             # JavaScript files
│       ├── login.js    # Login logic
│       └── app.js      # Main application logic
├── assets/             # Application assets
└── android-sdk/        # Bundled Android SDK tools

```

## Test Project Structure

When you create a new test project, it creates the following structure:

```
project_root/
├── cases/              # Individual test cases
│   └── case_001/       
│       ├── config.json # Test case configuration
│       ├── locator/    # Element locators
│       │   ├── element.json
│       │   └── img/    # Image recognition assets
│       └── script/     # Test scripts
│           └── script_001.yaml
├── devices/            # Device configurations
├── testcase_map.json   # Test case mapping
├── testcase_sheet.csv  # Test cases spreadsheet
└── workarea/           # Current device state
```

## Features Overview

### 1. Project Management
- Create new test projects with organized directory structure
- Import test cases from CSV files
- Manage multiple test projects

### 2. Test Case Editor
- Visual file explorer for test cases
- Monaco-based code editor with syntax highlighting
- Support for YAML test scripts
- Multi-tab editing

### 3. Device Management
- Add and configure test devices
- Real-time device connection status
- ADB integration for Android devices

### 4. Settings
- User profile management
- API server configuration
- Android SDK status check
- Application version information

## Security

- Token-based authentication with automatic refresh
- Secure storage of credentials
- Session management

## Keyboard Shortcuts

- `Cmd/Ctrl + N`: New Project
- `Cmd/Ctrl + O`: Open Project
- `F5`: Run Current Test
- `Shift + F5`: Stop Test
- `Cmd/Ctrl + D`: Refresh Device
- `F12`: Toggle Developer Tools

## Troubleshooting

### ADB Not Found
Ensure Android SDK platform-tools are properly installed in the `android-sdk` directory.

### Device Not Detected
1. Enable Developer Options on your Android device
2. Enable USB Debugging
3. Connect device via USB
4. Accept the debugging authorization prompt

