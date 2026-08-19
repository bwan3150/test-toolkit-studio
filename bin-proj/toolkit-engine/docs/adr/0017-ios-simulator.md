# ADR-0017 iOS 模拟器走**预编译的 WebDriverAgent**（曾短暂走过 idb）

- 日期：2026-08-19
- 状态：**已落地并在 mac 上实跑验通**（2026-08-19）
- 关联：ADR-0011（设备是工具的参数）、`docs/platform-matrix.md`

## 背景

真机那条路是 go-ios + WebDriverAgent，已经通了。模拟器一开始也想照搬——
`sim:` 前缀 + 直连 `127.0.0.1:8100`，代码都提交了（5dd97961）。

但那条路要求**用户自己编译一个 WDA 的 Xcode 工程**并跑起来，因为 go-ios 帮不上忙
（它走 USB/lockdown，模拟器没有）。这跟 tke「装好就能用」的定位差得很远。

后来看到用户另一个会话的实跑记录：那个 AI 为了点一下模拟器，先试 `simctl`（点不了）、
再试 AppleScript（被辅助功能权限挡死）、最后 `brew install idb-companion` 成功驱动。
这提示了另一条路。

## 决策（**已修订**）

> **最初的决定是「模拟器走 idb」**，理由是它 `brew install` 一条命令、免签名，
> 而 WDA 要用户自己编译 Xcode 工程。那个理由在「**我们自己分发预编译产物**」这个
> 前提下就不成立了——用户拍板要锁版本、自己掌控依赖之后，重新算账，WDA 全面胜出。
> 下面保留原始分析（它仍然解释了「为什么真机不用 idb」），修订后的结论在最后。

**模拟器走 idb，真机继续走 WDA。**

判据是一件事：**设备上要不要跑一个签名过的 runner**。

| | 真机 | 模拟器 |
|---|---|---|
| WDA | 要 runner，**必须签名**（Apple 硬要求） | 要 runner（免签名），但得编译 Xcode 工程 |
| idb | 要 runner，**同样必须签名**——换不来好处 | **不要**，`idb_companion` 直调 CoreSimulator |

所以真机换 idb 是净亏（丢掉已跑通的链路 + 重写归一化 + 照样要签名），
模拟器换 idb 是净赚（`brew install` 一条命令 vs 编译一个工程）。

## 实测依据（真实设备数据，不是推演）

`idb ui describe-all` 的输出质量足以支撑语义定位：

```json
{"role":"AXButton","AXLabel":"Skip sign in (demo)","frame":{"x":132.33,"y":393.33,...},"enabled":true}
{"role":"AXTextField","subrole":"AXSecureTextField","AXValue":"Password",...}
```

- `AXLabel` 就是可见文字 → `点击 ["Skip sign in (demo)"]` 能用
- `subrole: AXSecureTextField` → 直接对上 tke 的 `is_password`，证据打码无缝接上
- 坐标单位是**点**：`AXApplication` 宽 402，截图宽 1206 → scale 3.0
  **不用再调接口问 scale**（WDA 那边要问 `/wda/screen`）
- `idb ui tap` 用的也是点，与 describe-all 同一套坐标系（实测点 (201,402) 命中）

一个坑：**`traits` 不能用来判可点击**——实测 `Scrollable` 连 `StaticText` 都带，
一律当可点会把满屏文字都标成能点的（P-35 那条「可点击优先」就废了）。看 `role`/`type` 才准。

## 边界

- **`Platform` 仍然只有三个**。模拟器跑的是同一套 iOS App，元素库的 ios 通道、
  定位策略、归一化目标格式全都复用。多出来的只是**驱动**（`Driver::IosSim`），
  跟 `Driver::Fake` 一样：有驱动，没平台
- 归一化多了一份（AX 树 → uiautomator XML，约 80 行）。这是这个决定的**唯一新增成本**，
  拿真实数据当夹具写了 4 个单测
- idb 是外部依赖（brew 装，77MB）。`tke device list` 会在**列得出模拟器但没装 idb** 时
  明说「列得出来但操作不了」——这两件事看起来一样，不说清楚人会以为是设备问题

## 修订（2026-08-19，实测后）

用户提出：brew 上的版本什么时候更新我们控制不了，不如自己锁版本。
这个前提一立，账就反过来了——

| | idb + 自研 gRPC 客户端 | **预编译 WDA runner** |
|---|---|---|
| 协议 | gRPC → 要引 tonic + prost + h2 | **HTTP + JSON**，`ureq` 已在用 |
| 客户端代码 | 全新写 | **真机那套现成**，且已验证 |
| 元素归一化 | AX 树（新写） | XCUI（现成） |
| 分发物 | `idb_companion` 77MB | `.app` **21MB** |
| 起法 | companion 起 gRPC 服务 + Python 客户端 | **`simctl launch` 直接起** |

`idb_companion --help` 里**没有任何 UI 操作参数**（tap/describe/accessibility 全无），
它只管 boot/erase/create 和起 gRPC 服务——所以「只分发那个二进制」根本不成立，
点击与元素采集全在 Python 前端那一半。

而 WDA 这边实测（`scripts/probe-wda-prebuilt.sh`）：

- `xcodebuild build-for-testing` 出 `WebDriverAgentRunner-Runner.app`（21MB）
- `.xctestrun` 里**没有本机绝对路径** → 产物可以分发
- **`simctl launch` 直接就起来了**，`/status` 返回 WDA 16.3.0
  ——连 `xcodebuild` 和 `.xctestrun` 都不用带（XCTest bundle 一般要 xcodebuild
  带一堆环境变量，模拟器上不需要）

**修订后的结论**：模拟器走我们自己分发的预编译 WDA runner，
`tke doctor --fix --profile ios` 下载到 `~/.tke/wda/`，连接时若 8100 不通就
`simctl install` + `simctl launch` 拉起来。idb 驱动与 AX 归一化**已删除**——
不留两套，那意味着两份归一化、两条调试路径。

端口写死 8100 是已知限制：模拟器与主机共享网络，多台同时跑会撞。
单台够用；要并行得给每台传 `USE_PORT`。

## 自查（`scripts/verify-ios-sim.sh` 一条命令跑完）

- `tke device list` 列出 `sim:<udid>` ✅
- `fetch --interactive` 出元素表，**坐标在截图像素量级**（实测 366pt × 3 = 1098）✅
- `steps '点击 ["某个按钮"]'` 后页面真的变了（**坐标换算对不对全看这一步**）✅
- 证据四件套落盘 ✅（`raw_pages/` 起初是空的，见 P-43——收集方认扩展名白名单，
  而 AX 原文是 `.json`）

验证脚本**用仓库里的构建产物**，不用 PATH 里那个 tke：拿装好的发布版去验，
验的就不是刚改的代码（P-42）。
