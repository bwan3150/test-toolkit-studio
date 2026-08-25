# tke 远程服务 API 契约（v1，设计稿）

> 状态：**设计稿，未落地一行代码**。决策见 [`adr/0022-remote-service.md`](adr/0022-remote-service.md)（生效）。
> 约束锚点：**INV-16**（白名单 + 零 LLM 面）/ **INV-17**（租约即隔离，释放即复位）/
> INV-3（决策回传）/ INV-15（强度阶梯）。
> 本文是**契约**，不是教程；实现推进时内容漂移就修（属于「活参考」类）。

## 0. 一张图

```
[用户 / Claude Code / Codex / GitHub Action]
        │  租设备、下发任务、拿报告
        ▼
[云平台]   账号 / 凭据 / 计费 / 设备池调度 / 报告存档 / webhook / 目标归属校验
        │  内网，一台节点一个 token
        ▼
[tke serve]  单节点 · 单租户 · 无调度（ADR-0022 D1）
        ├─ 安卓真机 ×N / AVD（`-d <serial>` / `-d avd:<名>`）
        ├─ iOS 真机 / 模拟器（**必须 macOS 节点**，门禁在 Controller::new）
        └─ 无头 Chrome（`-d web:1` … `web:N`，session file 按 device_id 分槽）
```

## 1. 执行模型

一个请求 = 一个 `tke` 子进程（ADR-0022 D2）。`serve` 只做：鉴权 → 白名单校验 → 注入目录参数 →
fork → 收 stdout 的 JSON → 回。**不在同进程调用 handler**（`JsonOutput` 会 `process::exit`，
且 `set_ocr_url` / `set_web_headless` / `interrupt::install` 是进程级全局态）。

与本地行为的差异**只允许有两处**：网络 RTT、白名单裁剪。出现第三种差异就是 bug。

## 2. 鉴权与握手

- 每个节点持一个 token（平台发），`Authorization: Bearer <token>`。tke 不认识"用户"。
- `GET /v1/hello` 返回 `{tke_version, build_stamp, host_os, api_version}`。
  客户端**比 build 戳**（沿用 ADR-0014 的判据，不比版本号），不匹配立刻说清楚——
  skill 过期不提示已经坑过一次（Q-11 / P-41），远程会放大它。

## 3. 租约（session）

```http
POST /v1/sessions
{ "capabilities": {"platform": "android", "os_min": "13"}, "ttl_s": 1800 }
→ 200 { "session_id": "...", "device": {"id":"f64b3b4d", "label":"Pixel 7（安卓 14）"},
        "workspace": "/v1/sessions/{sid}/workspace", "expires_at": "..." }

POST   /v1/sessions/{sid}/heartbeat     # 续租；断心跳 → 回收
DELETE /v1/sessions/{sid}               # 释放：打包产物 + **复位设备**（INV-17）
GET    /v1/sessions/{sid}               # 状态/剩余时长（计费口径：租赁时长从这里出）
```

- **一个设备同时只有一个租约**——这就是"租赁"的技术实体。
- 每个 session 一个隔离目录；`--log` / `--cache` / `--current-dir` 由服务端注入（INV-16/17）。
- `capabilities` 里的 `platform=ios` 只能落在 macOS 节点；平台按节点上报的 `host_os` 路由。

## 4. L1 命令层（零 LLM 面，只计设备时长）

```http
POST /v1/sessions/{sid}/exec
{ "argv": ["control", "click", "--at", "100,200"], "timeout_s": 30 }
→ { "exit_code": 0, "stdout": {...}, "stderr": "", "elapsed_ms": 412 }
```

**一个端点覆盖全部基础能力**，这样 tke 命令语法演进时 API 不用跟着改版，
远程 skill 文档 = 本地 skill 文档（ADR-0022 D4 的核心理由）。

### 白名单（枚举，INV-16）

| 组 | 命令 |
|---|---|
| 设备原语 | `control *` / `refresh` / `fetch` / `recognize` |
| 安全原语 | `http` / `recon *` |
| 工具 | `device` / `app` / `file` / `element` / `ocr` |
| 脚本 | `steps` / `run` |
| 生命周期 | `task new` / `report` |
| 环境 | `doctor`（**只读**，`--fix` 不开放：联网下载是节点运维的事） |

**不在白名单**：`harness` / `security`（它们是 L2 的活，走 L1 就是白嫖平台 key）、
`update` / `uninstall` / `fix`、以及任何形式的 argv 直通。

### 参数过滤（INV-16）

- 拒：`--config` / `--prompts-dir` / 绝对路径 / 含 `..` 的路径 / `TKE_ALLOW_IOS` 类逃生口
- 强制注入：`--log` / `--cache` / `--current-dir` → session 目录
- 强制关：`--copilot`（它要 `[ai]`，属服务端 LLM 面）
- `-d` 必须等于本 session 租到的设备 id

## 5. L2 任务层（平台的 AI，计 token + 设备时长）

```http
POST /v1/tasks
{ "kind": "ui" | "security", "target": "...", "testcase": "...",
  "mode": "passive|safe|aggressive",     # red-team 服务端硬拒（ADR-0022 D5）
  "budget": {"max_rounds": 30, "timeout_s": 1800}, "callback_url": "..." }
→ { "task_id": "..." }

GET /v1/tasks/{id}                 # 状态 + outcome
GET /v1/tasks/{id}/events          # SSE：直接透传现有 NDJSON 事件流
WS  /v1/tasks/{id}/session         # 交互式：桥接 JsonFrontend 双向 NDJSON（本就是长连接协议）
GET /v1/tasks/{id}/report          # 自包含 HTML（无外链，可直接给链接）
POST <callback_url>                # 终态 webhook：outcome + 报告 URL + 用量
```

服务端跑的是 tke **自带 AI** 的 `tke harness` / `tke security`（节点持 `[ai]`）。

**出口五态**（复活 ADR-0009，见 ADR-0022 D6）：
`passed` / `failed` / `needs_decision` / `blocked` / `error`。
遇到决策点（要凭据、前提不满足、要做不可逆操作）**不得自行决定**——
`needs_decision` + 问题/选项/上下文/现场快照终止回传，平台推给用户（INV-3、INV-12）。

**证据分级不合并**（ADR-0009 第 4 条仍成立）：「回放通过」（硬证据）与「AI 判定达成」（软证据）分字段。

## 6. L3 产物层

```http
GET /v1/sessions/{sid}/artifacts/**     # 截图 / log.json / page/*.xml / report
PUT /v1/sessions/{sid}/workspace/**     # 上传 APK / IPA / foo.tks + foo.tklib（两件套自包含，INV-7）
```

上传落点过现有的 `resolve_in_workspace` 沙箱（拒绝绝对路径与 `..`）。

**脱敏是这层唯一的防线**：报告/截图现在要过网，INV-15 的凭据脱敏（P-45 iOS 明文那次）
必须补一组 API 层回归测——本地漏了只是本地，远程漏了是数据外泄。

## 7. 计费口径（ADR-0022 D3）

| 路径 | 谁的 AI | 计什么 |
|---|---|---|
| 平台下发任务（L2） | 平台 key | 设备租赁时长 + token，一起记用户账上 |
| 远程 skill 调用（L1） | 用户自己的 Claude Code / Codex | **只计设备租赁时长** |

这是 L1 零 LLM 面的原因，不是洁癖。

## 8. 分阶段落地

| 阶段 | 内容 | 验收 |
|---|---|---|
| **P0** ✅ | ADR-0022 + 本契约 + INV-16/17 | 用户拍板（2026-08-26 已拍） |
| **P1** | `tke serve` 单节点：hello / health(`doctor --json`) / devices(`device list`) / sessions 租约 / exec / artifacts | `fake:` 驱动在 CI 无设备跑通全链路；本机 web 真跑一次网页检查 |
| **P2** | `TKE_REMOTE` 客户端模式 + build 戳握手；`tke-ui-test-remote` / `tke-security-test-remote`（复用主文档，只加「怎么连远端」） | 另一台零环境机器上 Claude Code 跑通一次 UI 检查并拿到报告 |
| **P3** | 任务层：`POST /tasks` + SSE + webhook + WS 交互式 + `needs_decision` 回传 | 下发一个 harness 任务，关掉终端，收到报告链接 |
| **P4** | 节点注册/心跳/能力上报；平台侧池化调度与计费对接 | 平台上租一台安卓，页面里对话式探索 |
| **P5** | 部署形态：Docker（Linux + web + AVD/redroid）、mac mini 节点（iOS）、systemd/launchd、GitHub Action | Action 里一个 step 跑通 |
| **P6** | 可选：MCP 网关；`tke steps` 统一吃 http/recon（ADR-0021 暂缓的那条——远程「每步一 RTT」后可能真的需要合批） | — |

P1+P2 就交付了「轻量/架构不匹配的电脑也能测」这个核心价值；P3 才是「脱手 + 收报告」。**别先做 P3。**

## 9. 已知要踩的坑（开工前读一遍）

1. **iOS 只在 mac 节点**；`TKE_ALLOW_IOS=1` 服务端不暴露。
2. **设备"脏"状态**：本地从不需要复位，远程必须做（INV-17）——否则下一个租户接手一台登录着的浏览器。
3. **版本漂移**：client / node build 戳不一致必须明说；沉默会让人得出"没改善"的假结论（Q-11 的教训）。
4. **进程启动开销**：远程链路成本要**先量再优化**（本地已有教训：两次"太慢"根因都不在猜的地方）。
   分层计时（网络 / 进程启动 / 设备操作）从 P1 就埋进 `elapsed_ms`。
5. **并发压设备**：`web:N` 多槽位已支持，安卓/iOS 一机一租约；节点的槽位上限要上报，别让平台超卖。
6. **安全轨授权**：目标归属校验在平台侧；tke 侧只保证 `red-team` 拒绝 + 强度落报告（INV-15）。
