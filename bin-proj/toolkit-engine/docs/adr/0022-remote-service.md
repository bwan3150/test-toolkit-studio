# ADR-0022: tke 服务化——单节点 agent + 子进程执行 + 二进制远程客户端

- **状态**: 生效（2026-08-26 用户拍板四条分歧点）
- **日期**: 2026-08-26
- **关联**: 新增 **INV-16 / INV-17**；复活 **ADR-0009** 的 `needs_decision` 回传条款（INV-3 延伸）；
  承 ADR-0010（skill 借调用方 AI）/ ADR-0011（设备是参数）/ ADR-0016（无 CLI 直通）/
  ADR-0019+INV-15（安全强度阶梯）/ ADR-0021（共享任务生命周期）；PITFALLS P-10
- **契约文档**: [`../remote-api.md`](../remote-api.md)

## 背景

tke 现在只能在「有设备的那台机器」上用。目标场景是：几台测试服务器上部署
tke + 安卓模拟器/真机 + iOS 模拟器/真机 + 无头浏览器，用户**不需要自己的测试环境和测试电脑**——
从云平台租一台设备下发任务或交互式探索，脱手，结束收报告；或者让自己的 coding agent
（Claude Code / Codex）通过远程 skill 调用，本地零安装、架构不匹配也能用；未来塞进 GitHub Action。

摸清现状后，三条事实决定了方案形状：

1. **CLI handler 不是库函数，是进程**：`JsonOutput::success/error` 直接 `process::exit`，
   `main.rs` 还有三处进程级全局态（`set_ocr_url` / `set_web_headless` / `interrupt::install`）。
   同进程内并发跑两条命令不可能。
2. **而「每命令一个进程」正是 skill 今天的样子**：会话靠 `web/infra.rs::session_file`（按 device_id）
   与 WDA 端口跨进程复用。所以子进程模型不是妥协，是与本地**行为等价**。
3. **一大半已经有了**：`tke task new` + `task.json` + `tke report <dir>`（ADR-0021，领域无关）、
   `--json` 双向 NDJSON（`JsonFrontend`，本为 Electron 长连接设计）、`tke doctor --json`（健康）、
   `tke device list`（清单）、自包含报告 HTML、`web:1/web:2` 多槽位、`fake:` 可无设备回归。

## 决策

### D1 tke 只做「单节点 agent」，池化/调度/计费/多租户留给云平台

`tke serve` 的语义是**一台机器的能力被远程调用**：单节点、单租户、不认识"用户"、不做跨节点分配。
节点向平台上报能力（宿主 OS + 设备清单 + 版本），平台做 registry / scheduler / 计费 / 报告存档。
一旦 tke 开始管配额与账号，它就变成平台的一半，之后每条 INVARIANTS 都要为多租户重新论证。

### D2 执行模型 = 子进程，不做 handler 的 in-process 化

`serve` 收请求 → fork 一个 `tke` 子进程 → 收 stdout 的 JSON → 回给调用方。
不重构 `JsonOutput`、不动全局态。附带收益：进程级隔离让 P-10（同秒共享工作区互相覆盖）从设计上消失。
**重新审视触发条件**：当单命令的进程启动开销被实测证明是远程链路的主要成本时（而不是网络 RTT 或设备本身），
再考虑把热路径内联——那时的前置工作是让 handler 返回 `Result` 而不是 `exit`。

### D3 API 三层，且**分层的依据是计费模型**（用户 2026-08-26 拍板 3）

| 层 | 内容 | 谁的 AI | 计费 |
|---|---|---|---|
| **L1 命令层** | `POST /sessions/{sid}/exec` 单端点执行白名单 argv（control/http/recon/device/fetch/refresh/recognize/steps/run/task/report） | **无 AI**（调用方自己的 agent，ADR-0010） | 只计设备租赁时长 |
| **L2 任务层** | `POST /tasks`（服务端跑 `tke harness` / `tke security`）+ 事件流 + 交互式 WS | **平台的 key** | 设备时长 + token，一起记用户账上 |
| **L3 产物层** | 产物下载 / 工作区上传（APK/IPA、`.tks`+`.tklib` 两件套） | — | — |

**由此得出一条硬约束：L1 必须是零 LLM 面。** 走 L1 的请求不得触发任何服务端 LLM 调用——
`--copilot` 在远程 L1 强制关闭（它需要 `[ai]`，ADR-0010 已说明 skill 场景下本就不可用），
`harness` / `security` 的 AI 编排不在 L1 白名单里。否则用户能通过 L1 白嫖平台 key，计费模型立刻塌。

### D4 远程客户端 = 二进制客户端模式（用户拍板 2）

同一个 `tke` 二进制，检测到 `TKE_REMOTE` / `TKE_TOKEN` 就把白名单子命令转成 HTTP，
而不是本地执行；分发一个 client-only 构建（无 OCR、无驱动依赖，全平台交叉编译）。

**这么选的核心理由是文档不分叉**：`tke-ui-test-remote` ≈ `tke-ui-test` + 开头一段「怎么连远端」。
590 行踩坑册、语义定位、证据组织全部原样复用。若选 MCP，命令语法与 MCP 工具 schema 会变成两份
各自演进的契约，两条 skill 立刻分叉成四份要维护的文档。
**MCP 网关不否决，但排在 HTTP 契约稳定之后**（见 remote-api.md 的 P6）。裸 `curl` 路径永远保留为兜底（CI/冷门架构）。

### D5 安全轨：`red-team` 不对远程开放（用户拍板 4）

远程 API 接受 `passive` / `safe` / `aggressive`，**`red-team` 服务端硬拒**（不是参数校验提示，是拒绝执行）。
INV-15 的阶梯不变，只是最高一档不经过网络暴露——破坏性/不可逆向量需要"人就在那台机器前"这个物理前提。
目标归属校验（谁允许你扫这个域名）属于平台侧，不在 tke。

### D6 决策点结构化回传（复活 ADR-0009 条款）

L2 任务在服务端 headless 跑，遇到决策点（要凭据、前提不满足、要做不可逆操作）
**不得自行决定**，以 `outcome: needs_decision` + 问题/选项/上下文/现场快照终止并回传，
由平台推给用户。INV-3 的对话层此时是平台 UI；**headless 一旦开始自行决策，本 ADR 失效**。

### D7 新增两条不变量

- **INV-16 远程执行只走白名单，无 argv 直通**（见 INVARIANTS.md）
- **INV-17 租约即隔离，释放即复位**（见 INVARIANTS.md）

## 理由与代价

**代价与风险：**
- **服务端执行 argv = 潜在 RCE**。靠 INV-16 的枚举白名单 + 参数过滤（拒 `--config`/`--prompts-dir`/
  绝对路径/`..`；`--log`/`--cache`/`--current-dir` 由服务端强制注入）+ `-d` 必须是本 session 租到的设备。
  ADR-0016 删掉 CLI 直通那一刀在这里是天然对齐的——**没有第二条路**是这套设计的前提。
- **脱敏成了唯一防线**：报告/截图现在要过网。INV-15 的凭据脱敏（P-45 iOS 那次的教训）需要补 API 层回归测。
- **多一条执行路径要维护**：远程与本地的行为差异必须收敛到「网络 RTT + 白名单」这两点，
  任何第三种差异都是 bug。版本握手对 **build 戳**（沿用 ADR-0014 的判据），
  client/node 不匹配直接说清楚——skill 过期不提示已经坑过一次（Q-11、P-41），远程会把它放大。
- **设备"脏"状态是租赁模式独有的新问题**：本地从不需要，远程必须做（INV-17）。
- iOS 只能落在 mac 节点（门禁在 `Controller::new` → `utils::capability::check`）；
  逃生口 `TKE_ALLOW_IOS=1` **服务端不暴露**。

**放弃了什么：** 同进程高性能执行（D2）、MCP 的零安装（D4）、远程 red-team（D5）。前两条是排序不是否决。
