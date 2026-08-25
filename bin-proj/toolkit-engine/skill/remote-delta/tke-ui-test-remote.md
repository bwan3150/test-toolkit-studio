---
name: tke-ui-test-remote
description: 跟 tke-ui-test 完全一样地操作真实浏览器 / 安卓 / iOS 设备做实机检查，**但设备在远端测试服务器上**——你这台机器不用装 Chrome、adb、模拟器，架构不匹配也能用，跑在 CI 里也行。需要一个节点地址和凭据（`TKE_REMOTE` / `TKE_TOKEN`）。本机已经有设备环境的话，用 tke-ui-test。
---

# 远程 UI 实机检查（tke）

**这份和 `tke-ui-test` 是同一套东西**——同样的命令、同样的踩坑、同样的证据产物。
唯一的差别是**设备在别的机器上**：安卓真机/模拟器、iOS、无头浏览器都在远端节点，
你这边只要一个 tke 客户端和一个凭据。

适合：本机没有测试环境、架构不匹配（装不上驱动）、轻量笔记本、CI。

## 先连上

```bash
export TKE_REMOTE=https://<节点地址>     # 平台会给你
export TKE_TOKEN=<凭据>
tke remote status      # 连着谁、版本对不对得上
tke remote devices     # 节点上有哪些设备、谁在租
```

没有 `tke` 客户端就装一个（**同一个二进制**，只是配了 `TKE_REMOTE` 就走远程）：

```bash
curl -fsSL https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke/install.sh | bash
export PATH="$HOME/.tke/bin:$PATH"     # 让当前这个终端立刻能用
```

`tke remote status` 里 `version_match` 是 `false` 就先说出来——**两边版本不一致时行为可能不一样**，
抱着这个前提排查问题会白花很多时间。

## 然后就照正文那样敲

```bash
tke -d web steps '启动 ["https://example.com"]' '点击 ["登录"]' --log ./evidence
```

两个参数在远程有了新含义，**其余一模一样**：

| 参数 | 远程含义 |
|---|---|
| `-d web` / `-d android` / `-d ios` | 要**哪一类**设备（第一条命令自动租一台，后面的命令复用它） |
| `-d web:2` / `-d <设备id>` | 点名租某一台（`tke remote devices` 里的第一列） |
| `--log <目录>` | 产物**拉回本地这个目录**（截图 / log.json / report.html 都在里面） |

不带 `--log` 的话产物留在节点上，你这边什么也看不到——**要给用户看证据就得带上它**。

## 覆盖表（正文里遇到这几件事，以这里为准）

| 正文说 | 远程实际 |
|---|---|
| `curl install.sh` 装 tke **和驱动** | 只装客户端。Chrome / adb / 模拟器都在节点上，你这边一个都不用装 |
| `tke doctor --fix` 补依赖 | **不开放**——联网下几百 MB 是节点运维的事。`tke doctor` 仍可用，体检的是**节点** |
| `--headless=off` 开窗口给人手动登录 | **不可用**：服务器没有显示器。登录见下面 |
| 日志默认落 `~/.tke/logs/` | 产物在节点上，带 `--log <目录>` 才会拉回本地 |
| 浏览器有头/无头、会不会抢鼠标 | 节点一律无头，不存在这个问题 |
| 报告在本地某个路径 | 拉回来之后在 `<--log 目录>/logs/report.html` |

## 还有这些不一样

- **用完还回去**：`tke remote close`。节点会复位设备（关浏览器 / 停掉你启动过的 App），
  下一个人拿到的是干净的。不还也行——租约到期会自动回收——但**别人要等**。
- **`tke harness` / `tke security` 远程不开放**：那两条是 tke 自带 AI 的编排，属于任务层。
  命令层不跑服务端的 AI——**你自己就是那个 AI**（这也是为什么远程只计设备时长，不计 token）。
- **输出一律是 JSON**（本地不带 `--json` 时会是友好格式）。你本来就在读 JSON，不影响。
- **手动登录做不了**：节点无头，没人能在上面点。出路是照正文「碰到登录怎么办」那样，
  用 `steps` 走一遍登录流程（**凭据仍然不要写进脚本**），或者请用户在平台侧准备一台已登录的设备。
- **传文件上去**：`tke remote push <本地文件>`（待测的 APK/IPA、要回放的 `.tks`+`.tklib` 两件套）。
- **并发**：一台设备同时只有一个人租着。`tke remote devices` 显示 `available: false` 就是有人在用，
  换一台或等一会儿——这不是故障。

## 出错了先看这三样

1. `tke remote status` —— 连得上吗？版本对得上吗？租着的会话还在吗？
2. `tke remote devices` —— 你要的那类设备节点上有吗？是不是都被租着？
3. 命令报「节点拒绝」时，**理由是节点原样给的**，照着改就行；
   报「连不上节点」是网络/地址/凭据的问题，不是命令写错了。

---

**以下是与本地版完全相同的正文**（同一份源文件，没有第二处维护）：

