# ADR-0018 安卓模拟器是选装的：tke 不分发、不进依赖检查

**日期**：2026-08-21
**状态**：已采纳（用户拍板）
**相关**：ADR-0012（下载只在 tke fix 里）、ADR-0017（iOS 模拟器）

## 背景

iOS 模拟器打通之后（ADR-0017），很自然的下一问是「安卓模拟器要不要也做成开箱即用」。
先把账算清楚——官方 `repository2-3.xml` 与 `sys-img` 清单里的**实测数字**：

| 宿主 | 官方 emulator | 包大小 |
|---|---|---|
| macOS arm64 | ✅ | 417 MB |
| macOS amd64 | ✅ | 490 MB |
| Linux amd64 | ✅ | 351 MB |
| Windows amd64 | ✅ | 456 MB |
| **Linux arm64** | ❌ **Google 至今不发布** | — |
| **Windows arm64** | ❌ | — |

系统镜像（`aosp_atd`，Google 给自动化用的精简版，比 `google_apis` 省约 40%）：
API 34 arm64-v8a 599 MB、API 36 x86_64 746 MB。**一台的总成本 0.8~1.3 GB。**

## 决策

**安卓模拟器是选装能力**：tke 不分发它、不检测缺失、没装不算「环境不完整」。
装了就能用（`-d avd:<名字>` + `启动环境`），没装就当这条路不存在。

理由（用户的原话）：

- **iOS 模拟器是 macOS 自带的**——Xcode 装了就有，我们只补一个 21MB 的 WDA runner，
  所以那条路值得做成开箱即用
- **安卓真机的开发者模式很好开**——插上就能测，模拟器不是必经之路
- 让每个人为一条**备选**路径先下 1GB，不划算

这比 ADR-0012 更进一步：那条说的是「下载只在 `tke fix` 里发生」，
这条说的是**根本不进依赖检查**——`doctor` 只写一句「未安装（选装）」，
不进「下一步」催人装，退出码也不因此非 0。

## 落地

- `drivers/avd.rs`：定位 `emulator`（**不走 ToolManager**——那个只在 tke 同目录找，
  而这是用户自己装的 SDK 的一部分，报错说"要和 tke 放在同一目录"会把人引到错的方向）、
  列 AVD、起、等就绪、关
- `-d avd:<名字>`：**序列号是模拟器起来之后才有的**（`emulator-5554`），
  拿它当启动参数是循环论证。所以按 AVD 名指定，boot 之后把实到的序列号记进
  `AdbDriver::resolved`，后续每条 adb 命令都用它
- 等就绪等的是 `sys.boot_completed`，**不是"adb 认出设备"**：那时系统还在起，
  装 App / 采集都会失败（同 iOS 那边等 `bootstatus` 的理由）
- 无头：`headed=None` 时按有没有桌面自动定（同 web 的 `--headless=auto`），
  无头下加 `-gpu swiftshader_indirect`——CI 机器上没有可用的 GL
- `device list` 把没启动的 AVD 也列出来（id = `avd:<名字>`），有在跑的就只提一句
  还有几台闲着（同 iOS 模拟器，别刷屏）

## 实测（2026-08-21，Linux amd64）

装 → 起 → 装 App → 启动 → 采集 → 按文字点击 → 页面跳转 → 证据落盘 → 关机，全通。
冷启动 61 秒（KVM 加速），采集与点击的表现跟真机没有区别。

**镜像从 `aosp_atd` 换成了 `default`**：ATD 小 100MB，但它**默认关掉硬件渲染**，
截图恒为纯色。Google 的说法是改用 AndroidX Test Screenshot API——那是 instrumentation
**进程内**的东西，我们从外面 `screencap` 拿不到。tke 的立身之本是留证据（ADR-0010），
这个交换不成立。

**渲染后端必须是 `-gpu swiftshader`，不能是 `swiftshader_indirect`**（P-47）：
后者起得来、采得到、点得中，唯独截图是纯色——合成器只出了背景层。

## 代价 / 已知限制

- **Linux arm64 没有官方 emulator**。要在那一档跑安卓模拟器只能用 redroid 之类的
  容器方案（需要宿主内核有 binder/ashmem，因此 mac/Windows 的 Docker 里也跑不了）。
  暂不支持，如实说。
- 端口不像 iOS 那样要自己管：每台 AVD 占一个从 5554 起、步进 2 的控制台端口，
  序列号就是 `emulator-<端口>`，adb 天然分得清（**Q-13 那类坑在这边不存在**）。
