# 不变量（INVARIANTS）

全项目引用锚点。每条都是真实教训换来的，**改任何一条必须先写 ADR 并经用户拍板**。
引用方式：代码注释 / commit / ADR 里写 `INV-N`。

---

## INV-1 接地原则

**多轮 agent 必须接地在它正在改变的状态上。** explorer 每轮看真实页面→成立；
旧「医生」编辑脚本文本却只看过期 trace→对看不见的设备做脑内手术，结构性失败，
几十版调不好后整体删除（57ed54e7）。不接地的任务一律拆成单次调用（asserter/supervisor/healer 这类稳）。

## INV-2 单次 agent 统一 oneshot 强制工具调用

单次结构化输出角色（asserter/supervisor/reflector/verify/healer/advisor）一律走
`runner/oneshot.rs`：单个 report 工具 + ToolChoice 强制，schema 供应商侧校验（87ef690a）。
**禁止**新增"提示词求 JSON + 文本解析"的角色——那是已淘汰的老路。

## INV-3 修复决策留在与用户对话的层

没有一键黑盒 repair（181246bc 删除）。修复 = 编排官编排：replay_tks 拿结构化失败报告（逐步轨迹）
→ 判断（续探 / navigate 复位 / 问用户）→ resume_explore → replay_tks 验证。
工具是原语，决策在对话层。

**延伸（ADR-0009，2026-08-12 拍板）**：「对话层」指的是与用户对话的那一层，**不限定必须是 tke 自己的 REPL**。
`tke task`（headless）被外部 agent 调用时，调用方就是对话层——所以 headless 遇到决策点必须
**结构化回传**（`outcome: needs_decision`），不是自行决定。**headless 一旦开始自行决策，即违反本条**。

## INV-4 tke run 纯回放不改脚本资产

`tke run` 的 AI 辅助驾驶（自愈/分诊/对齐）**不写 .tks / .tklib**——修正只落解包临时副本，
报告里标注（75842dda）。回写权只属于 harness（replay_tks 回包、resume_explore 写回）。
替代元素（分诊层2）在任何路径都不落库——它不是"它"，写进原元素名会污染库。

## INV-5 提示词不泄题

提示词 / AI 可见 schema **禁止写死具体测试内容**（产品名/型号/网站/URL）——tke 是通用 agent，
写死即泄题。测试内容只能来自用户输入与运行时上下文。

## INV-6 判定依据来自真实页面

起始/终点校验的规范形式 = 元素包 pages 实体 + `断言页面` 指令按命中率投票（≥60%，1e58b91a）——
校验是脚本自身的普通步骤，回放器真实执行。**禁止让 LLM 凭空发明标志**；
头注释标志只是老脚本兜底。

## INV-7 两件套自包含，无共享元素库

一个测试 = `foo.tks` + `foo.tklib`，拷走即跑（2026-07-03 定稿，共享库彻底删除）。
每个脚本的定位宇宙就是自己的 tklib，新脚本永远污染不了旧脚本。
运行期「解包→操作→回包」，下游对 tklib 无感知。

## INV-8 修复归层

感知/定位问题（同名点错、找不到）修 `perception`/`engines`/`drivers`，
**不许拿提示词打补丁**（教训：web 同名元素靠提示词怎么调都没用，根因是缺唯一 DOM 路径 512400b3）。
平台怪癖修 `drivers/`；agent 行为改 `prompt/builtin/*.md`；护栏改 `runner/`。

## INV-9 失败必须可见，增强不得更严

质量闸门（supervisor/asserter/marker/desc）失败必须 emit Warn——静默下线极难排查。
**增强功能的前提容忍度必须 ≤ 宿主命令**：宿主容 device=None，增强层就不得把它当硬前提
静默 Skip（db33162c 教训：不带 -d 时对齐/自愈全程失效且零线索）；增强被环境跳过时必须可见。

## INV-10 分层依赖方向

`utils` 最底层禁止 import 上层；`cli/` 只做参数翻译禁止业务逻辑；
`workflow/script_runner`（纯回放层）不反向依赖 `workflow/agent`（AI 层）——AI 能力经
trait 钩子/工厂由装配层注入（ElementHealer 即范例）。frontend(App) 只经 IPC 与 handlers 交互。

## INV-11 收尾不毁证据，承重步不许删

teardown（关浏览器/退 App）是收尾不是测试内容，终点校验必须在收尾前完成。
断言步（踩实）与启动步不许被优化官当冗余删（护栏在代码里）。

## INV-12 登录态等前提：查得出、说得清、不代办

自动登录既要凭据又改变账号状态，是 navigate 纪律红线。起始前提不满足 → 诊断 + 停下 + 交还用户，
不硬修（同理：不幂等操作的脚本可重复性有天然上限，出口是警告+商量）。
