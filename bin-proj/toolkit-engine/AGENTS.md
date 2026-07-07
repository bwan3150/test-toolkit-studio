# AGENTS.md — toolkit-engine (tke)

给在本仓库工作的任何 AI agent / 协作者的共享上下文。**刻意粗粒度**（模块级一句话职责），
描述漂移时改这里一处即可；逐文件细节以各文件头注释为准，不在这里重复（细粒度地图必然滞后，
教训见 docs/codebase-map.md）。

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

## 测试（能在低层表达就放低层）

```bash
cargo test --no-default-features --lib     # 单测 + 无设备集成（秒级，CI 可跑）
```

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
- `docs/codebase-map.md` 是旧的逐文件细粒度地图，多处滞后——**模块职责以本文件为准**。
