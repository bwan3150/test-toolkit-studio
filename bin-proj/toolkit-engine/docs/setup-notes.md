# tke 环境搭建与注意事项

部署/换机/排查问题时先看这里。坑都是实际踩过的。

## 二进制布局

```
bin/<platform>/                          # 与 tke 同目录 = 直通可用
├── tke                                  # 主程序 (bin-proj/toolkit-engine 构建输出)
├── adb, aapt                            # Android 工具链
├── chromedriver                         # Web 驱动 (单文件, 可以放这里)
├── go-ios                               # iOS 驱动基础设施 (单文件, 隧道/启动WDA/端口转发)
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

tke 查找 Chrome 的顺序（2026-08-12 起**跨平台**，此前只认 mac-arm64）：

**搜索根**（依次）：
1. `<tke 同目录>/`（生产打包用，前提是 bin 不在 TCC 保护目录下）
2. 用户数据目录 `<data_dir>/tke/`（开发机/CI 推荐）——
   mac `~/Library/Application Support`、Linux `~/.local/share`、Windows `%APPDATA%`
3. 都找不到 → chromedriver 自己找系统 Chrome（版本可能不配对，慎用）

**相对路径按 Chrome for Testing 官方 zip 解压后的原样结构**，把官方包（或自建 S3 镜像里的
同名包）整个解压到搜索根下即可，**不必改名**：

| 平台 | 相对路径 |
|---|---|
| macOS | `chrome-mac-arm64/` 或 `chrome-mac-x64/` → `Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing` |
| Linux | `chrome-linux64/chrome` |
| Windows | `chrome-win64/chrome.exe` |

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

## ⚠️ iOS（WebDriverAgent）注意事项

iOS 全部基础设施由 tke 经 **go-ios**（与 tke 同目录的单文件二进制）自动管理：
隧道（iOS 17+）、拉起设备上的 WDA（runwda，经 testmanagerd，**无需 Xcode**）、
USB 端口转发。冷启动全链路约 10 秒，之后热复用 <1 秒。

**唯一的一次性前置（每台新设备）**：用 Xcode 把 WebDriverAgent 装到设备上——

```bash
cd ~/Documents/GitHub/WebDriverAgent   # WDA 源码
xcodebuild -project WebDriverAgent.xcodeproj \
  -scheme WebDriverAgentRunner \
  -destination "id=<设备UDID>" \
  -allowProvisioningUpdates DEVELOPMENT_TEAM=<TeamID> CODE_SIGN_STYLE=Automatic \
  test
# 装上后 Ctrl-C 退出即可（运行交给 tke 的 go-ios）；
# 设备上需信任开发者: 设置→通用→VPN与设备管理
```

实测踩过的坑（iPhone 12, iOS 18.6.2）：

- **iOS 17+ 上旧工具全部失效**：tidevice/老 instruments 协议连不上；
  `xcrun devicectl ... launch xctrunner` 能拉起 runner 但进程**立即退出**
  （WDA 必须经 XCTest 会话启动，直接点 App 图标同理）。可靠方式只有
  xcodebuild test（装+跑）和 go-ios runwda（只跑，tke 用的就是它）。
- **签名**：xcodebuild 报 "requires a development team" 时，命令行直接传
  `DEVELOPMENT_TEAM=<TeamID>`（`security find-identity -v -p codesigning` 查证书，
  团队 ID 在证书 OU 字段）。
- **设备寻址**：`-d <UDID>`（25 位、第 9 位是连字符的自动识别为 iOS，
  如 `00008101-000C75842192001E`），或显式 `-d wda:<UDID>`。
  `go-ios list` / `xcrun devicectl list devices` 查 UDID。
- **坐标系**：脚本里写截图像素坐标（与 Android/Web 一致）；
  WDA 协议用逻辑点，tke 按 scale（视网膜倍率，iPhone 12=3）自动换算。
- **会话语义**：只有 `启动 [BundleID]` 会创建 WDA 会话；其余操作要求已有会话。
  `关闭 [BundleID]` 只关 App 不销毁会话；`control close x` 销毁会话；
  flow 结束自动关 App + 销毁会话。隧道/转发/WDA 进程跨命令常驻复用。
- **排查**：go-ios 各进程日志在 `$TMPDIR/tke/ios/`（tunnel.log /
  runwda-<udid>.log / forward-<udid>.log）；会话状态在同目录 `<udid>.json`。
- **selfIdentity.plist**：go-ios 的隧道配对身份文件（密钥对），它默认写到
  进程 cwd——tke 已把 go-ios 的 cwd 固定为 `$TMPDIR/tke/ios/`。如果在项目目录
  里看到这个文件，是旧版本留下的，删掉即可（会自动重新配对生成）。

## 无头 / CI / docker（2026-08-12，**坐标可移植性已实测验证**）

```bash
tke ...                 # auto（默认）：有桌面→有头；无桌面→无头
tke --headless ...      # 等价 --headless=on，强制无头（有桌面的机器上跑 CI）
tke --headless=off ...  # 强制有头
```

**必须用等号形式**（`--headless=on`）——裸 `--headless` 也可以，但 `--headless on` 不行：
`on` 会被当成子命令位置（见 PITFALLS P-16）。配置文件里是 `headless = "auto"|"on"|"off"`。

- **auto 的判据**：mac/win 恒有桌面；Linux 看 `DISPLAY` / `WAYLAND_DISPLAY`，两个都没有 → 无头。
- **无头用 `--headless=new`**（不是老实现）：新版走完整浏览器渲染路径，与有头一致；
  老 `--headless` 是另一套精简渲染，截图会和有头对不上。
- **容器 / root 自动加** `--no-sandbox --disable-dev-shm-usage`（探测 `/.dockerenv`、
  `/run/.containerenv`、uid==0）。普通桌面环境保留沙箱。
- **窗口尺寸/缩放因子照旧固定**（`--window-size=1280,900 --force-device-scale-factor=1`），
  无头下同样生效——这是脚本像素坐标可移植的前提。

### ✅ 坐标可移植性：已验证（2026-08-12）

**mac 有头 = mac 无头 = Linux 无头 = 1280x813**，元素 bounds `diff` 零差异。
（1280x813 = window-size 1280x900 减去 87px 浏览器 UI 高度，`headless=new` 会模拟真实窗口装饰。）
**结论：像素坐标跨模式、跨平台可移植，"本地录、CI 回放"成立。**

> ⚠️ 做这类对照时**必须先销毁会话**：`rm -f $TMPDIR/tke/web/*.json` + `pkill -f "Google Chrome for Testing"`。
> 否则第二条命令会复用第一个会话，得到"两种模式一致"的**假阳性**（P-18，已修：模式不符会销毁重建）。

### ⚠️ 还没验证的
1. **docker 镜像里的系统依赖**：Chrome 需要 libnss3/libatk/libgbm/libasound2 等一堆 `.so`，
   精简镜像没有；**中文页面还需要中文字体**（缺了整页豆腐块，元素位置全变）。
   这部分下载器解决不了，得靠 Dockerfile 装。

## ⚠️ 其他实测踩过的坑

- **首次启动很慢**：macOS 对 600MB 的 Chrome 包首启要做完整 Gatekeeper 扫描
  （30-60 秒），之后秒开。tke 会话创建超时已设 90s 兜底。
- **某些终端模拟器下 Chrome 启动崩溃**（如 Ghostty：`Mach rendezvous failed` /
  `BUS_ADRALN`）：终端注入的进程上下文被 Chrome 继承导致。tke 拉起 chromedriver
  时已做环境清洗（env_clear 白名单）+ 脱离终端进程组。**白名单含 DISPLAY/WAYLAND_DISPLAY/
  XAUTHORITY**——Linux 有头模式 Chrome 靠它们连图形栈，早期漏了导致起不来（P-15）。
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
├── web/                    # web 会话信息 + chromedriver 日志 + 浏览器 profile
└── ios/                    # iOS 会话状态 + go-ios 日志 (tunnel/runwda/forward)
```

## 构建

```bash
./bin-proj/toolkit-engine/build-mac.sh   # 构建并输出到 bin/darwin-<arch>/tke
# 不要直接 cargo build 出包; 迭代验证可用 cargo check
```
