# 用 GitHub Actions 发布 tke 与依赖

两个 workflow，按**改动频率**分开——常改的那些一键就发，几乎不变的那些手动跑。

| Workflow | 什么时候跑 | 干什么 |
|---|---|---|
| **Publish TKE** (`tke-publish.yml`) | **日常**：改了 tke 代码或 skill 文档 | 构建四平台二进制 + 打包 skill + 刷新 VERSION |
| **Publish TKE Deps** (`tke-deps.yml`) | **基本不用**：要整体升级 Chrome 版本时 | 抓 Chrome for Testing / chromedriver / adb / aapt / go-ios |

> **依赖是一次性的活。** 四个平台各备一份就完了，之后不再动——现有那批是 2026-08-14
> 手工补齐的（Chrome for Testing Stable 152.0.7977.42）。所以 CI 的日常职责只有一件事：
> **tke 或 skill 改了，能发一个新版出去。**
>
> 真要升级 Chrome 版本时留意：`install.sh` 对**已存在的 Chrome 目录是跳过的**，
> 老用户机器上会变成"driver 升了、Chrome 还是旧的"——版本不配对，浏览器起不来。
> 得同时通知使用者删掉旧 Chrome 目录再重装。

## 一次性准备

仓库 **Settings → Secrets and variables → Actions → New repository secret**：

| 名字 | 值 |
|---|---|
| `TKC_TOKEN` | Toolkit Cloud 的上传凭证（`tkc_…`） |

没配的话 workflow 会在上传那步明确报错退出，不会静默跳过。

## 日常：**自动**发新版

**main 上动了 tke 源码或 skill，push 之后自动发一版**，不用管。监听的路径：

```
bin-proj/toolkit-engine/src/**        Cargo.toml / Cargo.lock / build.rs
bin-proj/toolkit-engine/skill/**      .github/workflows/tke-publish.yml
```

只改 `docs/` 或 Electron 那边不会触发——没必要为一篇文档等六个平台编译十几分钟。

而且**只动 skill/ 时会跳过编译**：workflow 先比一次 `HEAD^..HEAD`，源码没变就直接发
skill 与安装脚本，一分钟完事。改了 `src/` 才走六平台构建。

## 手动：Actions → Publish TKE → Run workflow

需要指定平台、换 OCR 档位、或者只想空跑验证时用，几个开关：

- **targets**：默认 `all`（四个平台）。只改了某个平台的问题时可以单选
- **ocr**：默认 `online`（快）。`full` 会连离线 tesseract 一起编，**慢很多**（从源码编译
  tesseract + leptonica），只在确实需要离线 OCR 时选
- **skill_only**：只改了 SKILL.md / 坑册时勾上，**不构建二进制，一分钟就完**
- **dry_run**：只构建打包不上传，用来验证 workflow 本身

跑完使用者侧立刻可用：

```bash
curl -fsSL https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke/install.sh | bash
```

### 上传顺序不是随便排的

**VERSION 必须最后传。** 它里面的 `build:` 戳是**破 CDN 缓存的键**（Cloudflare 缓存 4h
且不认 `no-cache` 请求头，只有变化的查询参数能破，见 P-19）。先传 VERSION 的话，使用者
会拿着新键去取还没传完的文件。

跑完有一步**复验**：从分发源真取一遍刚传的文件，验它们是 gzip 而不是 HTML——
存储平台对不存在的路径回落 200 + 网页（SPA 兜底），只看状态码会把网页当二进制装进去。

## 少见：换 Chrome / 驱动版本

Actions → **Publish TKE Deps** → Run workflow：

- **what**：`all` / `chrome+driver` / `android` / `ios`
- **chrome_channel**：`Stable`（默认）或 `Beta`
- **platforms**：逗号分隔，默认四个平台全要

**chromedriver 与 Chrome 从同一份官方清单的同一个版本取**——这是自建分发源最实在的价值：
使用者不必再自己去查版本对应关系（版本不配对是这套东西最常见的坑）。

跑完记得**再跑一次 Publish TKE**：依赖传上去了但 VERSION 没变，使用者的 CDN 缓存最长
4 小时后才会看到新驱动。workflow 结尾会提醒这件事。

## runner 的坑：**GitHub 已经没有免费的 Intel mac 了**

`macos-13` 会一直等不到 runner、卡到超时——近百次 runner-images 发布里只剩
macos-14/15/26，**全是 Apple Silicon**。所以 `darwin-amd64` 改成在 arm64 runner 上
**交叉编译** `x86_64-apple-darwin`（同一套 Apple SDK 与链接器，Rust 开箱即用）。

交叉编译的产物在本 runner 上未必能执行，所以产物验证分两档：**能执行就执行**（最强），
执行不了就**退回验架构**（`file` 比对目标平台）。两样都失败才算构建失败——
绝不是"跳过验证"。

## 各家下载源的结构（都是实测出来的，别照猜）

| 来源 | 结构 |
|---|---|
| Chrome for Testing | 官方 zip 解压出来就是 `chrome-linux64/` 这种目录，**正是我们的约定**，直接转存不重新打包 |
| chromedriver | zip 里是 `chromedriver-<plat>/chromedriver`，要取出来单独 gz |
| platform-tools | `platform-tools/adb`；**aapt 不在这里**。Windows 版还必须带上 `AdbWinApi.dll` + `AdbWinUsbApi.dll`——`adb.exe` 直接依赖前者，USB 靠后者（运行时加载，不在导入表里），少一个 adb 就起不来 |
| build-tools | `aapt` 在这儿。Linux 版 aapt 单独跑不了（缺 `libc++.so`），但 RUNPATH 含 `$ORIGIN`，与 tke 同目录就能加载 → 两个一起带 |
| go-ios | **三个平台三种结构**：linux 的 zip 里是 `ios-amd64` + `ios-arm64` **两个架构**，mac 是单个 `ios`，win 是 `ios.exe`。必须按架构名**优先级逐个找**——`find -o \| head -1` 取目录遍历顺序，在那个双架构包上会选错 |

最后一条是跑之前没料到的：光看 release 页面的资产名（`go-ios-linux.zip`）会以为里面是单个
二进制。**CI 脚本不本地跑一遍就等于没写。**

## 文件命名：分发源上一律不带 `.exe`

`bin/windows-amd64/adb.gz` 解压出来的是 `adb.exe` 的内容，但**源上就叫 `adb.gz`**。
落地时由 `install.sh` / `tke fix` 按平台补扩展名。

这样定是因为：源上按平台改名的话，取的那一端也得写平台分支，两处都要维护。
（`tke fix` 早期版本漏了这一步，Windows 上会落成一个没有扩展名的 `adb`，根本执行不了。）

## 六个平台，但不是每个都齐 —— 上游决定的，不是漏传

| 平台 | tke | chromedriver / Chrome | adb / aapt | go-ios |
|---|---|---|---|---|
| darwin-arm64 | ✅ | ✅ | ✅ | ✅ |
| darwin-amd64 | ✅ | ✅ | ✅ | ✅ |
| linux-amd64 | ✅ | ✅ | ✅ | ✅ |
| **linux-arm64** | ✅ | ❌ **上游没有** | ❌ **上游没有** | ✅ |
| windows-amd64 | ✅ | ✅ | ✅ | ✅ |
| **windows-386** | ✅ | ✅ | ✅ | ❌ **上游只出 64 位** |

三条硬事实（都是实测出来的，不是猜的）：

- **Chrome for Testing 只出 5 个平台**：`linux64` / `mac-arm64` / `mac-x64` / `win32` / `win64`。
  `linux-arm64` 与 `win-arm64` 直连 404。
- **Google 的 platform-tools 不出 arm64 Linux 版**（`linux_aarch64` / `linux-arm64` /
  `linux_arm64` 三种命名全 404）。
- **go-ios 的 Windows 包只有 64 位**（zip 里那个 `ios.exe` 是 PE32+ x86-64），
  32 位 Windows 跑不了，所以 `windows-386` 目录里**有意不放** go-ios。

`tke fix` 知道这些边界：在 arm64 Linux 上会直说"上游没有官方驱动，请
`apt install chromium-driver adb` 并软链到 tke 同目录"，而不是让人对着下载失败反复试；
32 位 Windows 上也不会去报"缺 go-ios"——报了也补不上。

**windows-arm64 有意不做**：Windows on ARM 自带 x64 模拟，`windows-amd64` 那套直接能跑，
而 Chrome for Testing 也没有 arm64 Windows 版，单出一份只会多一套要维护的东西。

## 分发源布局（install.sh 与 tke fix 都按这个取）

```
tke/
├── install.sh
├── VERSION                       # 第一行 tke 版本(体检比对)；build 戳=破缓存键
├── skill/tke-ui-test.tar.gz     # 包内含一份 VERSION 副本(ADR-0014,doctor 据此判断 skill 是否过期)
├── bin/<platform>/               # darwin-arm64 / darwin-amd64 / linux-amd64 / windows-amd64
│   ├── tke.gz
│   ├── chromedriver.gz
│   ├── adb.gz  aapt.gz  libc++.so.gz
│   └── go-ios.gz
└── chrome/chrome-<mac-arm64|mac-x64|linux64|win64>.zip
```

## 本地发布（不走 CI）

`bin-proj/toolkit-engine/skill/publish.sh` 干的是同一件事，适合快速迭代：

```bash
./bin-proj/toolkit-engine/build-mac.sh
cd bin-proj/toolkit-engine/skill && ./publish.sh
export TKC_TOKEN=<token>
curl -fsSL https://cloud.test-toolkit.app/script/upload.sh | bash -s -- dist/ tookit-engine-resource:tke/
```

只想更新二进制、不动 skill：`… -- dist/bin dist/VERSION tookit-engine-resource:tke/`
（源目录**不带**尾斜杠会保留目录名本身；写成 `.` 会让 key 混进 `./` 而 502，见 P-21）。
