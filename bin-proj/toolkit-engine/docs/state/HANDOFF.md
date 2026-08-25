# 交接单

**会话时间**: 2026-08-25（第七场：**tke security 从零到上线** —— 第二个 agent 领域）
**产出 commit**: `94270671` 之后到 `HEAD`（见 CHANGELOG 的 Unreleased 段；本场约 40 个提交）

## 一句话

这一整场从零做出了 **tke 的第二个 agent 领域：安全测试**——设计锁 → 侦察 primitive → 对话式 AI agent →
共享任务生命周期 → skill → 深挖 playbook → **CI 发版上线**。全程真机验证、142 测试绿。

## 做出来的东西（自底向上）

1. **primitive**（`src/workflow/security/`）：`tke http`（原始探测，4xx/5xx 照收/不跟重定向/体限2MiB）+
   `tke recon <verb>` 八个（headers/fingerprint/**detect**/cors/graphql/bundle/endpoints/tls）。
   `HttpEngine` trait（UreqEngine+FakeEngine 可脱网单测）+ `evidence.rs`（`--log` 落 `evidence/step_NNN`，**续写不覆盖**）。
2. **AI 角色**：`prober`（自主顺藤，去重+无进展强制收尾）、`analyst`（对抗复核，oneshot 强制结构化，毙假阳分软硬）、
   `reporter`（**确定性**出 HTML+findings.json，无 LLM）。提示词 `security/prompt/`（builtin+外部覆盖）。
3. **对话外壳** `orchestrator`：`tke security` 默认进 **TUI**（复用 harness `Frontend`），**主 agent 开场面试**
   （目标/强度/scope，选项选择），`--json`/非终端→无头一次性。
4. **共享生命周期**（ADR-0021）：`tke task new --kind <ui|security>` 写 `task.json` 标记；
   **`tke report <dir>` 按 kind 自动分派** UI 报告 vs 安全报告——两轨一条命令。
5. **skill** `tke-security-test`（ADR-0010 借调用方 AI）+ **service-playbook.md**（往后端深挖：Sanity/Supabase/
   Firebase/S3/Algolia/Hasura 的指纹+已知误配+**精确零凭据探测式**+防误报）。
6. **分发**：多-skill 管线泛化（manifest 驱动），**默认一行装全部 skill**；`--skill` 只装一个。CI 已发版上线。

## 关键决策（都进了 ADR）

- **ADR-0019** security 领域 + 三层能力分层 + 强度阶梯 + INV-13/14/15。
- **ADR-0020**（已被 0021 取代）：曾想 `tke ui report`/`tke security report` 拆分。
- **ADR-0021**：领域即数据——task/report 领域无关，靠 `task.json` 分派。取代 0020。

## 真机验过的 / 待验的

- ✅ **真机过**：P1 七 verb + 无头 `security --json` 在 konechome 出报告；我（当调用方 AI）手动走完两轨
  （security 顺藤到 Framer bundle·无泄露·评级B；UI 无头浏览器截图）；`recon detect` 对已修的 konechome 正确返回 none。
- 🟡 **待用户真机验**：**对话式 TUI 交互手感**（选项选择/追问节奏/插话）——只有真 TTY 能暴露。
  上一场 prober 死循环就是真机跑才逼出来的（已修），交互这块同理，值得他实跑一遍 `tke security`。

## 下一步候选

- playbook 再覆盖几类服务（Contentful/Strapi/Elasticsearch/MongoDB/Directus）——加一条 playbook + 一个 detect 正则即可，不改架构。
- 注入子系统（opt-in，检测非利用）、源码灰盒、endpoints 吃 OpenAPI、tls 深度证书（需 TLS 库）。
- `--json` 与 Electron app 联调。

## 坑（这场踩的，已进 PITFALLS）

- **P-52**：TUI 里 AI 对用户说的多行话要用 `UiEvent::Assistant` 不是 `Notice`——Notice 走带缩进的包裹，多行成阶梯。
- 深度不在"更聪明的模型"，在**喂给它的攻击知识（playbook）+ 把线索递到手上的工具（detect）**——AI 会推理但没领域知识就是空推。
