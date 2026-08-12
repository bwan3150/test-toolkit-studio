# AGENTS.md — toolkit-engine (tke)

给在本仓库工作的任何 AI agent / 协作者的入口：**协议 + 路由 + 共享上下文**。
知识刻意粗粒度（模块级一句话职责），逐文件细节以各文件头注释为准（细粒度地图必然滞后，
教训见 docs/archive/codebase-map.md）。文档导航见 docs/README.md。

---

## ⛔ 开始之前（按顺序）

1. 读 `docs/state/STATE.md`（进度/不要碰什么）+ `docs/state/HANDOFF.md`（上个会话埋的坑）
2. 跑 `git log --oneline -10` 与 STATE 的 `Last-Commit` 比对——**对不上就停下来问人**，
   不要自行推断上个会话做了什么、不要重新实现"看起来缺失"的东西
3. 扫一眼 `docs/PITFALLS.md` 的标题；改到相关区域再细读对应条目
4. 动会碰 `docs/INVARIANTS.md` 任何一条的东西 → 先写 ADR、经用户拍板

## ⛔ 结束之前（缺一不可）

- [ ] `cargo test --no-default-features --lib` 通过
- [ ] 追加 `CHANGELOG.md`（不重写已有条目；pre-push 会强制）
- [ ] 覆写 `docs/state/STATE.md`（含 Last-Commit）与 `HANDOFF.md`
- [ ] 踩了新坑 → 追加 `docs/PITFALLS.md`；做了架构决策 → 新增 `docs/adr/NNNN-*.md`
- [ ] 改动过守卫脚本 → 先造一个故意违规的现场验证它真的会红（P-12）
- [ ] 改动未真机验证的，在 STATE/CHANGELOG 标注「待真机验」——真机验证由用户执行，
      这是本项目的审核机制：**push ≠ 完成，用户真机确认才算数**

## 路由表

| 你要做的事 | 动手前必读 |
|---|---|
| 改修复/自愈/回放 | INV-1/3/4 + ADR-0001/0006 + `runner/tksops.rs` 头注释 |
| 加/改 agent 角色 | INV-1/2/5 + ADR-0005（oneshot）+ 本文「agent 拓扑」 |
| 改提示词 | INV-5（不泄题）+ `prompt/builtin/`（改完 check-prompts 会查登记） |
| 改定位/感知 | INV-8（不用提示词打补丁）+ P-11 |
| 改 TUI/前端输出 | ADR-0007 + P-01 + INV-9（失败可见） |
| 改元素包/脚本资产 | INV-4/6/7 + ADR-0003/0004 |
| 改守卫脚本 | P-12（先造违规验证） |
| 写 fake 测试 | 本文「测试」节 + P-06（改版页换 class） |
| 不确定某设计为何如此 | `docs/adr/` 倒序翻 + `docs/PITFALLS.md` |
| 不知道下一步做什么 | `docs/ROADMAP.md` + `docs/state/STATE.md`（优先级问用户） |

## 命令

| 目的 | 命令 |
|---|---|
| 编译检查 | `cargo check --no-default-features` |
| 单测+无设备集成（push 前必过） | `cargo test --no-default-features --lib` |
| 黑盒 CLI 契约测试 | `cargo test --no-default-features --test cli`（秒级） |
| 真机 e2e 冒烟（需设备,手动） | `./tests/e2e/smoke.sh <case.tks> [device]` |
| 产二进制（真机验证用） | `./build-mac.sh` / `./build-linux.sh` / `build-win.bat`（**禁 cargo build 产二进制**，P-02） |
| Linux/CI 快速产二进制 | `./build-linux.sh --no-ocr --quiet`（跳过 tesseract，省大量编译时间） |
| 挂守卫 hook（一次） | `./scripts/install-hooks.sh` |
| 手动跑守卫 | `./scripts/check-{prompts,changelog,state,linecount}.sh` |

提交规范：沿用现状——`type(tke): 一句话——为什么`，正文写清动机与代价；
commit message 是本项目最重要的变更叙事（CHANGELOG 只是索引）。

---

## 模块表（src/，自底向上）

| 模块                  | 职责 |
|-----------------------|------|
| `models/`             | 纯数据结构（Point/UIElement/Locator/tks 类型/DeviceInfo），不含逻辑。 |
| `utils/`              | 基础设施（config、params 统一参数表、interrupt 统一中断、workarea、tklib 元素包打包/解包）。**最底层，禁止 import 上层模块**。 |
| `drivers/`            | 设备协议对接：`adb`(Android) / `web`(chromedriver) / `wda`(iOS)，`Controller` 按设备 id 分发。`fake:` 前缀 = 测试假驱动（`fake.rs`，脚本化页面+事件记录）。 |
| `engines/`            | 纯逻辑引擎：`fetcher`(XML→元素树) / `recognizer`(元素定位) / `ocr`（含进程级 OCR 来源注册表）。 |
| `atomic/`             | 原子方法（refresh/fetch/recognize/control）：把 drivers+engines 组合成单步能力。 |
| `tools/`              | 自有 CLI 工具（file/app/device/element）。 |
| `workflow/tks/`       | .tks 脚本引擎：解析器 + 解释器 + 单步执行（编辑器调试用）。**与 `workflow/agent/runner`（AI 编排）是两回事**，曾同名 "runner" 已改名。 |
| `workflow/`（顶层）   | `script_runner` 完整脚本执行（事件+产物）、`flow` 多脚本顺序执行、`artifacts` 产物落盘。 |
| `workflow/agent/`     | AI harness（`tke harness`），见下表。 |
| `passthrough/`        | 外部二进制定位与直通。 |
| `cli/`                | 参数翻译层，**禁止业务逻辑**（已知违例：`cli/workflow/harness.rs` 直调 AdbDriver 列设备，待修）。 |

## workflow/agent/ 子模块表

| 子模块        | 职责 |
|---------------|------|
| `provider/`   | genai 封装：`LlmSession`（历史压缩/页面省略/大结果滚动省略/可重试错自动重试/Fake 后端）。genai 类型不外泄。 |
| `prompt/`     | 提示词体系：`builtin/*.md` 编译期内嵌（agents/tools/messages 三类），`--prompts-dir` 可外部覆盖；解析为空有守卫（防"漏登记→静默空串"）。 |
| `perception/` | 页面采集 + 元素解析 + 元素库对照 + 渲染给 AI 的元素列表。 |
| `execution/`  | AgentAction → 设备动作 / 元素落库 / .tks 行生成。 |
| `runner/`     | agent 编排：`orchestrator`(主 AI REPL) / `testrun`(explore 全流程) / `flow`(驱动循环) / `asserter`(踩实) / `supervisor`(finish 把关) / `diagnose`(诊断回放·测量仪器) / `healer`(定位自愈) / `reflect`(重探指导+优化官) / `tksops`(replay/repair/optimize 路径化工具) / `verify`(marker 推导+回放基建) / `fmt`+`ctx`(共用件)。 |
| `ui/`         | 引擎/前端解耦：`UiEvent`/`UiCommand` + Plain(行式)/Json(NDJSON 给 app)/Tui(ratatui) 三前端；提问/授权经 `supports_prompts()`。 |
| `transcript/` | conversation.jsonl 全景记录（按 agent 作用域分栏）。 |

## agent 拓扑

- **orchestrator** 是唯一与用户对话的主 AI（REPL），经颗粒化工具调度一切：
  `explore`（→ explorer 驱动循环，内含 asserter 每次导航踩实、supervisor finish 把关）、
  `replay_tks`(诊断回放+失败报告)/`resume_explore`(断点续探原语)/`optimize_tks`(optimizer)、文件增删改查（写/改/删需授权）。
- 多轮带工具 = explorer / optimizer / orchestrator（genai function calling）；
  单次结构化 = asserter / supervisor / reflector(命名) / verify(marker) / healer / advisor——
  统一走 `runner/oneshot.rs`：单个 report 工具 + ToolChoice 强制调用，schema 供应商侧校验；
  **禁止**新增"提示词求 JSON + 文本解析"的角色（那是已淘汰的老路）。
- **接地规则（几十版医生换来的教训）**：多轮 agent 必须**接地在它正在改变的状态上**（explorer 每轮看真实页面），
  不接地的任务一律拆成单次调用。旧「医生」（多轮编辑 .tks 文本、看过期 trace）因此被整体删除——
  修复 = ①定位级自愈（`runner/healer` 单次挑选，Healenium 式，解析失败时基于当前实时页面修正元素库）
  ②断点续探原语（`tksops::resume_explore`：explorer 从设备当前页面走完目标，前缀+新尾巴写回；
  失败不写回、不自带验证）。**修复流程由编排官编排**（replay_tks 拿结构化失败报告(逐步轨迹) → 判断:续探/
  navigate 复位对齐起始态/问用户 → resume_explore → replay_tks 验证）;
  `navigate` 是轻导航原语(不产脚本不做断言)——复原/对齐起始态专用,别拿 explore 凑合——没有一键黑盒 repair，
  修复决策必须留在与用户对话的层。
- **页面契约（规范形式）**：元素包的 `pages` 节存「页面」实体（desc+特征文字集+截图），
  `断言页面 ["起始页"/"完成页"]` 指令按**命中率投票**(≥60%)匹配——探索 finalize 自动落页面
  并首尾插断言步，起始/终点校验就是脚本自身的普通步骤（回放器真实执行、失败信息带 desc/命中率）。
  头注释 `# 目标标志:`/`# 起始标志:`/`# 起始页:` 降级为老脚本兜底——判据一律来自真实页面，禁止 LLM 发明。
- 目标标志(marker)首次推导后持久化在 .tks 头注释 `# 目标标志: `，replay/repair/optimize 共用同一判定基线。

## 元素包（.tklib）——没有共享元素库

一个测试 = 两个文件，拷到别的机器直接能跑：`foo.tks`（人读脚本）+ `foo.tklib`（元素包，
zip 容器：meta.json + element.json + img/ 模板图）。**共享元素库已彻底删除**（2026-07-03 定稿）——
每个脚本的定位宇宙就是自己的 tklib，新脚本永远污染不了旧脚本。
运行期是「解包→操作→回包」（像 docx）：装配层（tksops/`tke run`/finalize）把 tklib 解包到
cache、element_path 指过去，recognizer/元素工具/回放器对 tklib 无感知；repair 落了新元素再回包。
诊断/定位回放一律写 cache 临时 .tks，**绝不覆盖用户脚本**（会抹掉 marker 头/尾注）。
全局 `--element` 参数已删除：`tke run foo.tks` 强制要求旁边有同名 `foo.tklib`，缺包直接报错；
`tke element add` / `tke recognize` 用各自的局部 `--lib`（接受 .tklib 或裸 element.json）。

## 修复归层（重要）

- **感知/定位问题**（同名元素点错、元素找不到）修 `perception`/`engines`/`drivers`，**不许拿提示词打补丁**——历史教训：web 同名元素靠提示词怎么调都没用，根因是感知层缺唯一 DOM 路径。
- **平台怪癖**（键盘、弹窗、DPR 换算）修 `drivers/`。
- **agent 行为**（何时断言、何时 finish、探索策略）改 `prompt/builtin/*.md`；把关/护栏逻辑改 `runner/`。
- **错误分类**：可重试（元素没找到、断言失败、LLM 限流/5xx/超时）vs 终止（设备连接断、配置错、用户取消）。LLM 层自动重试在 `provider/session.rs`；修复流程只重试"测试层面"失败。

## 测试（能在低层表达就放低层，三层放置见 ADR-0008）

```bash
cargo test --no-default-features --lib        # ① 单测 + 无设备集成（#[cfg(test)] 就地放 src 内）
cargo test --no-default-features --test cli   # ② 黑盒 CLI 契约（tests/cli.rs，spawn 真二进制）
./tests/e2e/smoke.sh <case.tks> [device]      # ③ 真机 e2e（tests/e2e/，需设备，手动跑）
```

**单测不要搬进 tests/**——它们要访问 crate 私有项（ADR-0008）。CLI 参数/输出协议类问题只有 ② 测得到。

- **FakeLlm**：`LlmSession::new_fake(system, tools, turns)` 直接注入；深层自建会话的子 agent
  （asserter/supervisor/doctor…）用 `AiConfig{provider:"fake", model:<scope>}` +
  `provider::enqueue_fake_role_session(scope, role, turns)` 按角色注入。
- **FakeDriver**：设备 id 写 `fake:<名>`；`drivers::fake::install(device, pages)` 装页面脚本
  （uiautomator XML，感知层真实解析）、`events()` 断言动作序列、`remove()` 清理。
  页面推进语义：tap/switch 进一页、back 退一页、launch 回第 0 页。
- 范例：`runner/drive_tests.rs`（驱动循环闭环/卡死止损/把关时序）、`provider/session.rs` tests（压缩/省略/配对）。
- **真机验证**：改完跑 `./build-mac.sh`（输出到 `../bin/`），终端跑 `tke harness`（用户开发循环）。

## 红线（What NOT to do）

- **不用 `cargo build` 产二进制**——用 `./build-mac.sh`（bin 落点/签名由它管）；`cargo check`/`cargo test` 随便用。
- **提示词 / AI 可见的 schema 禁止写死具体测试内容**（产品名/型号/网站）——那是泄题，tke 是通用 agent。
- **断言步（踩实）是承重点**：优化官不许当冗余删；**启动步**不许删（护栏在代码里，改护栏前先懂为什么）。
- **teardown（关浏览器/退 App）是收尾不是测试内容**：终点校验要在收尾前完成，收尾会销毁校验证据。
- **不许静默吞错**：质量闸门（supervisor/asserter/marker/desc 生成）失败必须 emit Warn——供应商抖动时把关体系"悄悄下线"极难排查。
- **文件落点**：运行中间产物走 `params.cache_root()`；AI 交付文件必须经 `resolve_in_workspace`（工作区沙箱，拒绝绝对路径/`..`）。
- `docs/archive/` 是停止维护的旧文档（含旧细粒度地图）——**模块职责以本文件为准**。
