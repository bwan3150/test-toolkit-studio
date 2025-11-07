# Scrcpy-Server 修复日志

## 问题描述
视频流无法启动,WebSocket 连接失败,错误提示 `ECONNREFUSED` 和 `执行 pidof 命令失败`。

## 根本原因

### 1. pidof 命令兼容性问题
- **问题**: 使用 `pidof` 命令检测 scrcpy-server 进程
- **影响**: 某些 Android 设备上 `pidof` 命令不存在或不可用
- **现象**: 启动 scrcpy-server 时报错 "执行 pidof 命令失败"

### 2. 进程检测不准确
- **问题**: `ps | grep scrcpy` 无法匹配进程
- **原因**: `ps` 命令输出可能被截断,命令行参数不完整
- **结果**: 检测不到已运行的 scrcpy-server,导致启动失败或重复启动

### 3. CleanUp 进程干扰
- **问题**: `com.genymobile.scrcpy.CleanUp` 进程也包含 "genymobile"
- **影响**: 误认为 scrcpy-server 已经运行,或错误地杀死 CleanUp 进程

## 解决方案

### 修改文件
`src/adb/scrcpy_server.rs`

### 具体修改

#### 1. 改进 `is_server_running()` 函数
**旧实现**:
```rust
// 使用 pidof 命令
let args = self.build_adb_args(&["-s", udid, "shell", "pidof", SERVER_PROCESS_NAME]);
```

**新实现**:
```rust
// 步骤 1: 获取所有 app_process 进程
let args = self.build_adb_args(&["-s", udid, "shell", "ps | grep app_process"]);

// 步骤 2: 遍历每个进程,检查 /proc/{pid}/cmdline
let cmd = format!("cat /proc/{}/cmdline", pid);

// 步骤 3: 精确匹配 scrcpy.Server (排除 CleanUp)
if cmdline.contains(SERVER_PACKAGE) &&
   cmdline.contains("Server") &&
   !cmdline.contains("CleanUp") {
    // 找到了真正的 scrcpy-server
}
```

**优点**:
- ✅ 不依赖 `pidof` 命令
- ✅ 通过 `/proc/*/cmdline` 获取完整命令行
- ✅ 精确区分 Server 和 CleanUp 进程
- ✅ 兼容所有 Android 版本

#### 2. 改进 `stop_server()` 函数
**改进逻辑**:
- 同样使用 `ps | grep app_process` + `/proc/*/cmdline`
- 只杀死 `scrcpy.Server` 进程
- 不影响 `CleanUp` 进程

## 测试结果

### 测试环境
- 设备: CPH2305 (UDID: f64b3b4d)
- 系统: macOS arm64
- TKE 路径: `/Applications/Toolkit Studio.app/Contents/Resources/darwin/toolkit-engine/tke`

### 测试步骤
1. 编译: `./build-mac.sh`
2. 启动: `tke-scrcpy` (环境变量 `ADB_PATH` 指向 tke)
3. 测试: WebSocket 连接 `ws://localhost:8000/?action=proxy-adb&remote=tcp%3A8886&udid=f64b3b4d`

### 测试日志
```
✅ 成功停止旧的 scrcpy-server (PID: 10854)
✅ 推送 scrcpy-server.jar 成功
✅ 启动 scrcpy-server 成功 (PID: 31363)
✅ 建立 ADB forward 成功 (本地端口: 49701)
✅ 连接到 scrcpy-server WebSocket 成功
✅ 收到设备初始化消息 (231 bytes)
```

### 测试结论
✅ **视频流已成功启动**

## 部署说明

### 构建命令
```bash
cd scrcpy-server
./build-mac.sh
```

### 输出文件
- 二进制: `resources/darwin/scrcpy-server/tke-scrcpy`
- JAR 文件: `resources/darwin/scrcpy-server/scrcpy-server.jar`

### 环境变量
```bash
export ADB_PATH="/path/to/tke"  # 必须指定 tke 路径
export PORT=8000                 # 可选,默认 8000
```

### 启动方式
```bash
# 方式 1: 直接运行
/path/to/tke-scrcpy

# 方式 2: 后台运行
nohup /path/to/tke-scrcpy > /tmp/ws-scrcpy.log 2>&1 &
```

## 注意事项

1. **必须使用 tke adb**: 不要使用系统 adb,用户可能没有安装
2. **环境变量**: 启动前必须设置 `ADB_PATH`
3. **端口冲突**: 确保 8000 端口未被占用
4. **进程管理**: 使用 `killall -9 tke-scrcpy` 停止旧进程

## 版本信息
- 修复版本: 0.6.0-beta
- 修复日期: 2025-11-07
- 修复内容: Android 进程检测兼容性问题
