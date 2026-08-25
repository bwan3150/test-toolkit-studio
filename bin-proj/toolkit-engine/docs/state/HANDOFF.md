# 交接单

**会话时间**: 2026-08-26（第八场：**服务化定调** —— 只定方向，没写代码）
**产出 commit**: 无（文档改动**尚未提交**；上一场收尾在 `f741efaf`）

## 一句话

摸清 toolkit-engine 全貌后，把「tke 变成可远程调用的服务」这件事定了型：
**ADR-0022 生效 + 新增 INV-16/17 + `docs/remote-api.md` 契约**。代码一行没动。

## 定了什么（四条分歧点当场由用户拍板）

1. **tke 只做单节点 agent**——池化调度 / 计费 / 多租户归云平台。
   理由：tke 一旦管账号配额，就变成平台的一半，之后每条 INVARIANTS 都要为多租户重新论证。
2. **执行模型 = 子进程**，不做 handler in-process 化。
   硬事实：`JsonOutput::success/error` 直接 `process::exit`，`main.rs` 还有三处进程级全局态
   （`set_ocr_url` / `set_web_headless` / `interrupt::install`）——同进程并发不可能。
   而「每命令一个进程」**正是 skill 今天的样子**（会话靠 `web/infra.rs::session_file` 跨进程复用），
   所以这不是妥协，是行为等价；顺带让 P-10 从设计上消失。
3. **API 三层，分层依据是计费模型**（这条是本场最有价值的推论）：
   平台下发的任务用平台 key（token + 设备时长一起记用户账上），
   远程 skill 调用只计设备时长（AI 是用户自己的 Claude Code / Codex）
   → **L1 命令层必须零 LLM 面**，否则用户能通过 L1 白嫖平台 key，计费模型立刻塌。
   落成 INV-16 的延伸条款：`--copilot` 远程强制关、`harness`/`security` 不进 L1 白名单。
4. **客户端 = 二进制 `TKE_REMOTE` 模式**（不是 MCP）。选它的唯一理由是**文档不分叉**：
   `tke-ui-test-remote` ≈ `tke-ui-test` + 一段连接说明，590 行踩坑册原样复用。
   MCP 不否决，排在 HTTP 契约稳定之后（P6）。
5. **远程不开 `red-team`**（服务端硬拒）；`passive/safe/aggressive` 开放。目标归属校验归平台。

另：复活了 ADR-0009 的 `needs_decision` 五态出口——L2 headless 遇决策点不得自行决定（INV-3 延伸）。

## 改了哪些文件

- 新增 `docs/adr/0022-remote-service.md`（生效）
- 新增 `docs/remote-api.md`（契约 + P1~P6 分阶段 + 六个已知要踩的坑）
- `docs/INVARIANTS.md` 追加 **INV-16**（远程白名单 + 零 LLM 面）/ **INV-17**（租约即隔离，释放即复位）
- `docs/README.md` 导航加 remote-api.md、不变量计数 12→17（本来就已经漂了）
- `docs/ROADMAP.md` 加「新主线：服务化」+ 两条明确不做
- `docs/state/OPEN_QUESTIONS.md` 加 **Q-17/18/19**
- `CHANGELOG.md`、本文件、`STATE.md`

## 下一步：P1 `tke serve`

范围：`hello`（build 戳握手）/ `health`（复用 `doctor --json`）/ `devices`（复用 `device list`）/
sessions 租约（设备独占 + 目录隔离 + TTL 心跳 + 释放复位）/ `exec`（白名单 + 参数过滤 + 子进程）/
artifacts 下载 + workspace 上传。
**验收**：`fake:` 驱动在 CI 无设备跑通全链路；本机 web 真跑一次网页检查。
`elapsed_ms` 从第一天就拆成 网络 / 进程启动 / 设备操作三段（Q-17：先量再优化，本地已有两次教训）。

## 埋的坑 / 要注意

- **STATE 的 Last-Commit 是 `f741efaf`（上一场的），本场文档未提交**——下个会话先看 `git status`。
- 依赖选型没定：HTTP 框架（axum 会拉进 hyper/tower 一大坨，`ureq` 是同步的用不了）——
  P1 开工第一件事是权衡「二进制体积 / 编译时长 / client-only 构建」，别顺手就 `cargo add axum`。
- `Cargo.toml` 里 web 驱动用的是**同步** `ureq`（注释写明是为了避免 tokio 运行时冲突）——
  serve 是 async 的，子进程模型正好绕开这个矛盾，**别想着把它们合到一个运行时里**。
