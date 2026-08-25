# ADR-0020: 多测试轨的命名约定（`tke <track> report`）+ 安全 skill 轨

- **状态**: 生效（用户 2026-08-25 拍板）
- **日期**: 2026-08-25
- **关联**: ADR-0010（skill 借调用方 AI）/ ADR-0019（security 领域）

## 背景

tke 有两种「谁来驱动 AI」的模式，已在设备/UI 轨验证：
- **tke 自带 harness**：tke 内置 AI 编排（`tke harness` / `tke security`）。
- **skill 借调用方 AI**（ADR-0010）：把 tke 的确定性 primitive 当「手和眼」借给 Claude Code/Codex，
  调用方用**自己的** agent loop 驱动，tke 不内置 AI。设备轨的 `tke-ui-test` 已上线验证有效。

安全轨（ADR-0019）做完自带 harness（`tke security`）后，自然要补 skill 轨（`tke-security-test`），
让编程 agent 用自己的能力 + tke 的 http/recon primitive 做安全测试。这引出一个命名问题：
skill 的调用方需要一个**确定性出报告**的命令，而设备轨那个命令叫 `tke report`（通用名），
一旦有第二种报告就会含糊。

## 决策

### 1. 命名约定：`tke <track> report`

每条测试轨的收尾/报告命令一律 `tke <track> report`，轨名与 skill 名对齐：
- 设备/UI 轨：`tke ui report`（= 原 `tke report`，**保留 `tke report` 为隐藏别名**向后兼容）↔ skill `tke-ui-test`
- 安全轨：`tke security report` ↔ skill `tke-security-test`
- 未来新轨 X：`tke X report` ↔ skill `tke-X-test`，照抄这套。

交互入口保持各自历史名（`tke harness` / `tke security`），不强行统一——改 harness 名代价大且无收益。

### 2. reporter 暴露为确定性 primitive

`tke security report <findings.json>`：**无 AI，纯渲染**。调用方（skill / 脚本 / CI）自己收集
findings（用 http/recon 探、用自己的脑子判），喂进来就得到与 `tke security` **同一套**品牌报告
（security-report.html + findings.json + 每个确认漏洞 vuln-*.html）。单一实现，符合「一个问题不该有两套答案」。
`Finding`/`Severity`/`Category`/`EvidenceRef` 加 `Deserialize`；可选字段给 serde 默认，喂最小结构即可。

### 3. 安全 skill 轨 `tke-security-test`（承 ADR-0010）

调用方 AI 用 `tke http`/`tke recon` 探测、`tke security report` 出报告；**只做一次性检查 + 留证据 +
出报告，不产 .tks、不回放**（与 `tke-ui-test` 同边界）。复用 tke-ui-test 的一行装/六平台分发/CI 发版管线。
**本 ADR 落地了命名约定与 report primitive；skill 本体是下一步。**

## 理由与代价

- 为什么两轨都留：自带 harness 给「让 tke 自己跑」的人；skill 给「我在 coding agent 里，想用自己的
  agent 能力 + tke 的手眼」的人。设备轨已证明二者都有需求。
- 为什么 report 做成 primitive 而非只在 harness 内部：skill 轨没有 tke 的内层 AI，必须能从外部拿到报告；
  且 CI/脚本也受益。reporter 本就是确定性的，暴露是小工。
- 代价：`tke report` 改名引入一个隐藏别名（长期保留，不删——已发布 skill 与用户脚本在用）。
- 重审触发：若某轨的交互入口也要统一到 `tke <track>`（如把 harness 改名），届时重议。
