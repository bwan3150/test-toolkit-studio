# tke 环境搭建与注意事项

部署/换机/排查问题时先看这里。坑都是实际踩过的。

## 二进制布局

```
bin/<platform>/                          # 与 tke 同目录 = 直通可用
├── tke                                  # 主程序 (bin-proj/toolkit-engine 构建输出)
├── adb, aapt                            # Android 工具链
├── chromedriver                         # Web 驱动 (单文件, 可以放这里)
├── k6, ffmpeg, tester-ai, tke-opencv, tke-scrcpy
└── config.toml                          # 默认配置 (可选, --config 可覆盖)

~/Library/Application Support/tke/
└── chrome-mac-arm64/
    └── Google Chrome for Testing.app    # 浏览器本体 (不能放 bin 目录, 见下)
```

## ⚠️ Chrome for Testing 的三个坑

### 1. 不可放在 ~/Documents / ~/Desktop / ~/Downloads 等 TCC 保护目录

macOS 对这些目录有隐私保护（TCC）。Chrome.app 有自己的 bundle 身份，从保护目录
启动时系统要弹授权，而自动化场景下弹窗无处显示 → **进程卡死在 dyld 阶段，
永久挂起无任何报错**。普通单文件二进制（chromedriver/adb）继承 shell 权限，
不受影响。

tke 查找 Chrome 的顺序：
1. `<tke 同目录>/chrome-mac-arm64/Google Chrome for Testing.app`（生产打包用，
   前提是 bin 不在保护目录下）
2. `~/Library/Application Support/tke/chrome-mac-arm64/...`（开发机推荐，当前使用）
3. 都找不到 → chromedriver 自己找系统 Chrome（版本可能不配对，慎用）

### 2. 版本必须与 chromedriver 配对

两者都从 Chrome for Testing 发布页下载同一个版本号
（https://googlechromelabs.github.io/chrome-for-testing/）：

```bash
V=149.0.7827.115   # chromedriver --version 查当前版本
cd ~/Library/Application\ Support/tke
curl -sLO https://storage.googleapis.com/chrome-for-testing-public/$V/mac-arm64/chrome-mac-arm64.zip
unzip -q chrome-mac-arm64.zip && rm chrome-mac-arm64.zip
xattr -cr "chrome-mac-arm64/Google Chrome for Testing.app"   # 清隔离属性
```

### 3. 用 curl/终端下载, 不要用浏览器下载

浏览器（Safari 等）下载会打上 com.apple.quarantine 隔离属性，且解压可能损坏
.app 包结构（codesign 报 "code has no resources"）。终端 curl + unzip/ditto 安全；
若已被隔离，`xattr -cr <app>` 清除。

## ⚠️ 其他实测踩过的坑

- **首次启动很慢**：macOS 对 600MB 的 Chrome 包首启要做完整 Gatekeeper 扫描
  （30-60 秒），之后秒开。tke 会话创建超时已设 90s 兜底。
- **某些终端模拟器下 Chrome 启动崩溃**（如 Ghostty：`Mach rendezvous failed` /
  `BUS_ADRALN`）：终端注入的进程上下文被 Chrome 继承导致。tke 拉起 chromedriver
  时已做环境清洗（env_clear 只保留 PATH/HOME/USER/TMPDIR/LANG）+ 脱离终端进程组。
- **直通 adb 时全局 -d 必须放在 adb 之前**：`tke -d <serial> adb shell ...` ✓；
  `tke adb -d <serial> ...` ✗（-d 会被透传给 adb 本身，语义完全不同）。
- **渲染确定性**：web 会话固定 `--window-size=1280,900` +
  `--force-device-scale-factor=1`，不同机器/显示器（视网膜或外接屏）截图尺寸与
  坐标系完全一致，脚本里的像素坐标可移植。不要移除这两个参数。
- **排查 web 启动失败**：chromedriver 日志在 `$TMPDIR/tke/web/chromedriver-<端口>.log`
  （报错信息里会带路径）；会话信息在 `$TMPDIR/tke/web/<设备>.json`。

## 运行时目录（systemTemp, 自动管理）

```
$TMPDIR/tke/
├── workarea/<设备ID>/      # 原子命令的页面采集缓存 (跨进程共享, 不删除)
├── run-<时间戳>-<pid>/     # run 工作流的临时工作区 (运行完自动删除)
└── web/                    # web 会话信息 + chromedriver 日志 + 浏览器 profile
```

## 构建

```bash
./bin-proj/toolkit-engine/build-mac.sh   # 构建并输出到 bin/darwin-<arch>/tke
# 不要直接 cargo build 出包; 迭代验证可用 cargo check
```
