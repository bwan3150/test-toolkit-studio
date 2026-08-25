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
| **P1** ✅ | `tke serve` 单节点：hello / health / devices / sessions 租约 / exec / artifacts / workspace | **已落地**：单测 30 + 黑盒接口测试 10 + 真设备 e2e（web 9/9、安卓真机 8/8）。见 §11 |
| **P2** ✅ | `TKE_REMOTE` 客户端模式 + build 戳握手 + `tke remote` 会话管理；两条 remote skill（**生成式**：delta + 正文原样内联） | **已落地**：客户端单测 + 黑盒 11 + 本机真机演练（web / 安卓真机 / 安全轨）。见 §12 |
| **P3** ✅ 骨架 / 🟡 AI 端到端待真机验 | 任务层：`POST /tasks` + SSE + WS 交互式 + webhook + `needs_decision` 回传 | 单测 6 + 黑盒 7（不需要 key）；**真跑一次探索要 `[ai]`，归真机验证**。见 §13 |
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

## 10. 技术选型：axum（2026-08-26 用户拍板）

### 决定性事实：hyper/tower 栈**早就在依赖图里**

```
genai（非可选） → reqwest 0.13 → hyper-rustls → hyper 1.7
Cargo.lock 已有：hyper 1.7.0 · hyper-util 0.1.17 · tower 0.5.2 · tower-http 0.6.11 · h2 0.4.12
（`--no-default-features` 下同样在）
```

所以「axum 会拉进 hyper/tower 一大坨」这个顾虑不成立——那一大坨是 AI 客户端带进来的，躲不掉。
axum 的真实增量只有 axum + axum-core + matchit 等四到六个小 crate。

### 选 axum 的四条理由（按权重）

1. **增量依赖 ≈ 0**（上面那条）。
2. **SSE 与 WebSocket 都内置**——正好是 L2 的两个需求（事件流 + 桥接 `JsonFrontend` 双向 NDJSON）。
   别的方案在这里都要再引一个 WS 库。
3. **tower-http 给白名单之外的第二层护栏**：请求体大小限制 / 超时 / 并发限制。
   INV-16 管「能执行什么」，这层管「能压多狠」，两层都要有。
4. `ServeDir` 直接覆盖 L3 产物下载。

### 落选的与理由

| 方案 | 为什么不选 |
|---|---|
| hyper 1.x 裸用 | 路由/提取/SSE/WS 全手写，且**自制请求解析是新的攻击面**——白名单之外不该再多一层自己写的解析 |
| tiny_http / rouille | WS 基本没有、SSE 勉强；每连接一线程撑不住长连接；tokio 已在图里也省不掉 |
| poem + poem-openapi | 自动出 OpenAPI 很香，但生态窄、宏重编译慢。**OpenAPI 改为手写**——契约本来就要人审 |
| actix-web 4 | 自带 actix-rt = 第二套运行时；依赖树与图里的 hyper 栈不共享，体积与编译时间双涨 |
| warp | filter 组合的类型错误难读，社区重心已转 axum |

### 配套决定 A：client-only 瘦身的瓶颈不是 axum

release 二进制现在 **30MB**。大头是 `genai`（拖 reqwest/rustls 全套）、`image`/`imageproc`、
`rusqlite`（bundled，自带整个 SQLite C 源码）、可选的 tesseract；axum 的份额估计 <1MB。

建议 feature 布局：

```toml
serve  = ["axum", "tower-http"]   # 节点侧
agent  = ["genai"]                # AI 编排（harness / security）
client = []                       # 远程客户端：无 serve、无 agent、无 image/rusqlite
```

⚠️ **「谁占那 30MB」没量过**。按 Q-17 的规矩：P2 开工前用 `cargo-bloat` 量一次再动手，
别照上面这段估计改——本项目已经有两次「说慢/说大，根因都不在猜的那个地方」的教训。

### 配套决定 B：TLS 不在 tke 里做

`hyper-rustls` 虽然已在图里，节点仍只监听 HTTP；TLS 交给前面的 nginx / Caddy，
或走平台内网 + 由反代处理 mTLS。理由与 D1 同源：证书轮换 / SNI / ACME 是运维的事，
塞进 tke 就又多一块要为多租户重新论证的东西。

## 11. P1 已落地（2026-08-26）

### 端点（`tke serve --port 8787 --token <t>`）

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/v1/hello` | 版本 + host_os + arch + **白名单命令清单**（省得调用方靠猜） |
| GET | `/v1/health` | 复用 `tke doctor --json`，不新造判据 |
| GET | `/v1/devices` | 池 + `leased_by` / `available`（平台调度靠它判断还能不能派） |
| POST/GET | `/v1/sessions` | 建租约（201）/ 列活跃租约 |
| GET/DELETE | `/v1/sessions/{sid}` | 查 / 释放（释放带**复位回执**） |
| POST | `/v1/sessions/{sid}/heartbeat` | 续租 |
| POST | `/v1/sessions/{sid}/exec` | L1 命令层 |
| GET | `/v1/sessions/{sid}/artifacts/{*path}` | 下载产物（`?list=true` 列目录） |
| PUT | `/v1/sessions/{sid}/workspace/{*path}` | 上传（APK/IPA、两件套） |

**没有 token 就只准绑回环**——一个不设防的端口能操作真机，绑 `0.0.0.0` 等于把机器送人。

### 实测数字（Q-17：先量再优化）

Linux amd64，本机无头 Chrome + 一台安卓真机（CPH2305 · Android 15），经 HTTP 调用：

| 动作 | 耗时 | 其中 spawn |
|---|---|---|
| 一条会失败的命令（走完全链路即返回） | 3 ms | **0～1 ms** |
| `control boot`（起无头 Chrome） | 690 ms | ~1 ms |
| `steps 启动 [URL]`（真打开页面 + 落证据） | 5.0 s | ~1 ms |
| `fetch --interactive`（web） | 49 ms | ~1 ms |
| `steps 按键 [KEYCODE_HOME]`（安卓真机） | 10.4 s | ~1 ms |

**结论：进程启动不是瓶颈**（个位数毫秒，占比 <0.1%），耗时几乎全在设备本身。
ADR-0022 D2「子进程执行」的重新审视触发条件（"进程启动开销成为主要成本"）**没有出现**，
在有更强的证据之前不要为此重构。

### 三层测试

| 层 | 在哪 | 覆盖 | 跑法 |
|---|---|---|---|
| 单测 | `src/serve/**` 内 `#[cfg(test)]` | 白名单三道关 / 租约独占·TTL·复位计划 / 参数注入顺序 / 超时杀进程 | `cargo test --no-default-features --lib serve::` |
| 接口 | `tests/serve.rs` | 起真二进制、发真 HTTP、跑真子进程：鉴权、租约 HTTP 语义（409 vs 404）、白名单在 HTTP 层的拒绝、上传下载沙箱、会话间隔离。**不需要设备** | `cargo test --no-default-features --test serve` |
| e2e | `tests/e2e/serve-smoke.sh` | 唯一测不到的那环：**接口调用真的把设备操作了**——租真设备→操作→取证据→释放并确认复位 | `./tests/e2e/serve-smoke.sh web` |

### 守卫

`scripts/check-serve-paths.sh`（已挂进 pre-commit）：扫 `src/cli/**` 里带 `#[arg(long)]` 的
`PathBuf` 参数，凡是既没进路径参数表、也没进禁用清单的就报红——因为**给命令加参数的人
根本不会想到 serve**。写这条守卫的当天它就抓到两个真洞：`refresh --out` 与
`control browser-download --dir`，两个都能读写会话工作区外。

### P1 有意没做的

- **`tke file` 不进白名单**：它的 `push`/`pull` 把宿主路径和设备路径混在同一批参数里，
  语义要单独设计，先不开（`app` 没有 `install` 子命令，所以不存在宿主路径）。
- **TLS**：节点只监听 HTTP，交给前面的反代（§10 配套决定 B）。
- **fake 设备的跨进程状态**：`drivers::fake` 的页面脚本是进程级注册表，子进程里是空的，
  所以接口测试用的是 `task new` / `device` 这类不碰设备的命令。想让 CI 也覆盖"设备命令"
  这条路，得先让 fake 驱动的状态可落盘——**留到需要时再做**，e2e 已经覆盖了真设备那条。

## 12. P2 已落地（2026-08-26）

### 客户端：一个环境变量决定走哪条路

```bash
export TKE_REMOTE=https://<节点>   # 不带协议就补 http://
export TKE_TOKEN=<凭据>
tke -d web steps '启动 ["https://example.com"]' --log logs/check   # 跟本地一模一样
```

拦截发生在 **clap 之前**（远程要原样转发命令；先过本地 clap 等于要求两端版本严格一致）。
不在白名单里的命令拿不到拦截，照旧走本地。

**只有两个参数在远程有了新含义，其余原样转发：**

| 参数 | 远程含义 | 为什么这么设计 |
|---|---|---|
| `-d web` / `-d android` / `-d <设备id>` | 租哪一类 / 哪一台（服务端注入回子进程） | 平台关键字与设备 id 分开认——远程点名一台是常事，猜错会把人租到别的机器上 |
| `--log <相对目录>` | **照样发给节点**（沙箱进会话工作区），跑完把这棵子树拉回本地**同一个相对路径** | 本地写 `--log logs/scan` 再 `tke report logs/scan`，两条命令靠这个路径对上。把它吃掉，第二条就找不着了——实跑安全轨时撞出来的 |

其余由服务端接管的（`--cache`/`--current-dir`/`--json`/`--copilot`/`--headless`）就地吃掉**并说一声**
（静默 = 让人以为它生效了，INV-9）；`--config`/`--prompts-dir` 当场拒绝（那是节点的事）。

**会话是隐式的**：第一条命令自动租一台、落盘记住（`~/.tke/remote/<节点>.json`，与 web 驱动的
`session_file` 同一个套路）、后续命令复用并续租。显式管理用 `tke remote <status|open|close|pull|devices|push>`。

**stdout 与退出码原样透传** —— 这是"文档不分叉"的下半截：调用方看到的东西跟本地一样。

### 无设备会话（计费模型的一部分）

`http` / `recon` / `report` / `task` / `doctor` 不碰设备。没给 `-d` 时客户端开一个
**只有工作区的无设备会话**（`platform: "none"`）：不占池、不互斥、不复位、**不计设备时长**。

这不是优化，是 ADR-0022 D3 的直接推论：安全轨远程调用如果强制租一台手机，
用户就要为没用到的设备付租金。**写 security remote skill 时才发现这个洞**。

### 两条 remote skill：生成式，单一源头

```
skill/remote-delta/tke-ui-test-remote.md        ← 只维护差异（连接方式 + 覆盖表）
skill/build-remote.sh                            ← delta + 本地版正文**逐字节内联**
→ skill/tke-ui-test-remote/                      ← 生成物，不进 git；publish.sh 打包前自己跑
```

`tke-ui-test-remote` = 84 行 delta + 585 行原版正文 = 669 行；`tke-security-test-remote` 232 行。
**结构上不可能漂**——正文只有一处源头。四个包已经在 manifest 里，`install.sh` 默认全装。

Q-18 的审计结论：真正需要分叉的只有 4 个话题（装/连、`doctor --fix` 补依赖、有头登录、日志落点），
做成 delta 里的**覆盖表**（"正文说 X → 远程实际是 Y"）。赌注成立。

### 实测（本机 Linux amd64）

| 场景 | 结果 |
|---|---|
| `tke -d web ... --log logs/x` | 起无头 Chrome → 打开页面 → 8 个产物（含 report.html）拉回本地 |
| `tke -d android ...` | 租到 CPH2305 · Android 15，5 个产物拉回本地 |
| 安全轨（照 remote skill 原样敲） | **没租任何设备**，`task new` → `recon headers` → 证据落 `logs/scan/evidence/` 并拉回 |
| `tke remote status / close` | 版本对得上；释放带复位回执，设备回池 |

### P2 有意没做的

- **`--log` 只接受相对路径**（绝对路径当场拒并说清楚）：绝对路径既过不了沙箱，
  也没法在两边表示同一个位置。
- **没有后台心跳**：单次命令的进程活不到下一条命令。改为**每条命令前续租一次**，够用。
- **跨机没验**：全部实测都在本机回环上。网络那段的耗时仍未量（Q-17 的后半截）。

## 13. P3 已落地（2026-08-26）

### 端点

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/tasks` | 起任务（**202**，异步）。body：`kind`(ui/security) / `target` / `testcase` / `mode` / `interactive` / `max_rounds` / `timeout_s` / `callback_url` |
| GET | `/v1/tasks` · `/v1/tasks/{id}` | 列表 / 单个（含 `outcome` + `exit_code` + `detail`） |
| GET | `/v1/tasks/{id}/events` | **SSE**：先重放已发生的，再接实时的 |
| GET | `/v1/tasks/{id}/session` | **WebSocket**：事件推给你，你的回答写进任务进程 stdin |
| GET | `/v1/tasks/{id}/report` | ui → `report.html`，security → `security-report.html`（自己找，调用方不用记） |
| POST | `<callback_url>` | 终态 webhook：outcome + 报告地址 |

### 五态出口（复活 ADR-0009 的条款）

`passed`(0) / `failed`(1) / `needs_decision`(2) / `blocked`(3) / `error`(4)。

**决策点不得自行决定**（D6 / INV-3）：headless 任务一看到 `awaiting_input` 就**立刻终止**并把
问题原文回传（继续跑下去就等于让它自己拿主意了）；`interactive: true` 的任务才把问题转给
WebSocket 那头的人。

**没有 `done` 事件 = 没跑完**，即使退出码是 0 也判 `error`——编排本该以 done 收束。

### 三处实测逼出来的修正

1. **参数校验必须在租设备之前**。一个拼错的 `kind` 先撞上"没有 android 设备可租"（409），
   把"你写错了"报成了"这儿没有"，调用方会往错误的方向查。
2. **失败任务要交出 stderr 尾巴**（最后 50 行）。任务挂了的时候它是唯一线索——
   这次实测挂的原因是节点没配 API key，不给尾巴调用方只知道"失败了"（P-46 同款）。
   stderr 不进事件流（它是节点日志不是协议），但进 `detail.stderr_tail`。
3. **终局事件要进重放缓冲**。原本 `task_end` 只广播不落缓冲，于是"任务早跑完了才来订阅"
   的人只看到一片空白——而这恰恰是"下发完关掉终端、回头再看"的主场景。

### 强度与预算

`red-team` **服务端硬拒**（D5，`check_mode`）；`timeout_s` 收在 30~7200 秒之间硬执行，
超时杀进程并判 `error`。任务结束（含失败、含超时）一律释放会话并复位设备（INV-17）。

### 还没验的

**真跑一次 AI 编排**（拿到 `done`、出报告、WS 交互式回答问题）要节点配 `[ai]`——
本机没有 key，全部归真机验证。已测的是任务层骨架：参数校验、进程起停、事件流与重放、
五态判定、设备归还、webhook 回调。
