# tke 作为 skill 融入 coding agent 工作流（设计稿）

> 状态:**设计中,未实现**。契约部分见 [`adr/0009-headless-task-mode.md`](adr/0009-headless-task-mode.md)（待拍板）。
> 第一版范围:**Web + Android**（iOS 暂缓,WDA 环境成本拖慢首版）。

## 1. 要解决的问题

Claude Code 这类 coding agent 写完一个功能后,**没有任何办法知道它在真实设备上能不能用**——
只能靠自己写的单测自证,而实现与测试出自同一个模型,盲点重合。
单测/API 测试覆盖不到"点进去、填上、提交、看到结果"这条真实链路。

tke 提供的是**外部证据**:一台真机或真浏览器上,这个流程走得通还是走不通,
卡在哪一步、当时屏幕长什么样、是产品 bug 还是脚本过时。

第二个价值同等重要:**产物是资产**。一次探索产出的 `foo.tks + foo.tklib` 两件套(ADR-0003)
可提交进 repo,之后每次改动**先回放、不重新探索**——秒级、零 LLM、零不确定性。
"探索一次、回放无数次"的经济学,决定了这套东西能不能长期用下去。

## 2. 两个动作,必须分开暴露

| 动作 | 何时 | 成本 | 需要 |
|---|---|---|---|
| **verify** | 已有 `<feature>.tks` 两件套 | 秒级,无 LLM | 设备/浏览器 |
| **explore** | 没有脚本,或功能是全新的 | 分钟级,数万 token | 设备 + `[ai]` 配置 |

**默认策略写死在 skill 里:先找脚本 → 有就 verify → 没有才 explore → 探索产物落进
`tests/ui/` 并建议 commit。** 若只暴露一个入口,调用方每次都会去烧 AI 探索。

verify 走现成的 `tke run`(已合规:NDJSON + 退出码 + 定位自愈 + 起始态对齐,ADR-0006),
explore 需要 ADR-0009 的 headless 模式。

## 3. 意图契约（intent）

调用方最有价值的输入不是自由文本,是结构化意图。**关键杠杆:Claude Code 刚写完这段 UI 代码,
它知道锚点叫什么**——把 `anchors` 给到 tke,定位就从"看截图猜"变成精确匹配,
顺带治 P-11(按 text 撞同名点错元素)。

```yaml
# tests/ui/<feature>.intent.yaml
goal: <一句话说清要验证什么>
platform: web            # web | android
entry: <起始 URL,或 Android 的 package/activity>
preconditions:           # tke 只诊断不代办(INV-12);不满足 → outcome: blocked
  - <例:需要已登录的测试账号会话>
anchors:                 # 精度杠杆:写代码时埋的稳定锚点
  - {role: <这个元素干什么用>, testid: <data-testid / resource-id / a11y label>}
expectations:            # 会被落成断言步(承重,优化官不许删,INV-11)
  - <期望出现的结果>
forbidden:               # 禁止操作(破坏性动作、改账号状态)
  - <例:不得删除任何数据>
budget: {max_rounds: 20, timeout_s: 600}
```

> ⚠️ **INV-5 边界**:intent 是**运行时输入**,写具体业务内容合法。
> 但**绝不可**把这里的示例内容写进 `prompt/builtin/*.md`——那是提示词,写死即泄题。
> 实现时最容易踩的就是这一步("顺手加个例子让 AI 好懂")。

## 4. 报告契约（report）

一次调用写一个 JSON。**硬软证据分字段是这份 schema 的核心约束**(ADR-0009 决策 4):
若合并成单个 `success`,调用方会把 AI 的乐观判断当绿灯汇报给用户,这是本方案最危险的失效模式。

```jsonc
{
  "schema_version": 1,
  "task":   { "goal": "...", "platform": "web", "device": "..." },
  "outcome": "passed | failed | needs_decision | blocked | error",

  "evidence": {
    "replay":    { "ran": true, "steps_total": 12, "steps_passed": 12,
                   "assertions": [ /* 回放器真实执行的断言步 —— 硬证据 */ ] },
    "agent_judgement": { "reached_goal": true, "basis": "...",
                         "page_contract_hit_rate": 0.8 /* 软证据:LLM 判断 */ }
  },

  "failure": {                    // outcome=failed 时
    "step": 5, "line": "点击 {提交按钮}", "error": "...",
    "screenshot": "<path>", "page_elements": "<path>",
    "triage": "app_issue | script_stale | wrong_page | path_changed | unknown"
  },
  "needs_decision": {             // outcome=needs_decision 时
    "question": "...", "options": ["..."], "context": "...", "screenshot": "<path>"
  },
  "blocked_by": [ "..." ],        // outcome=blocked 时:未满足的前提(登录态等)

  "artifacts": { "tks": "<path>", "tklib": "<path>", "conversation": "<path>" },
  "warnings": [ /* INV-9:被跳过的增强、失败的质量闸门,必须可见 */ ],
  "usage": { "rounds": 8, "tokens": 41000, "duration_ms": 213000 }
}
```

`triage` 直接复用 `tke run` 辅助驾驶的分诊层(ADR-0006)。这对调用方是**最有行动价值的字段**:
`app_issue` → 回去改代码;`script_stale` → 触发续探修复脚本。
顺带一提,这个 skill 场景恰好能逼出 OPEN_QUESTIONS 的 **Q-1**(分诊层 2-5 真机质量未知)——
coding agent 天天改页面,是天然的"改版"现场发生器。

## 5. skill 形态与安装

```
<项目>/.claude/skills/ui-test/
├── SKILL.md                 # 触发条件 + 策略(先 verify 后 explore) + 职责边界
├── scripts/check-env.sh     # 前置体检
├── scripts/verify.sh        # 包 tke run
├── scripts/explore.sh       # 包 tke task(阶段1后)
└── reference/{intent,report}-schema.md
```

放**项目内** `.claude/skills/`(而非 `~/.claude/skills/`):UI 脚本资产本就该跟代码同仓,
团队 clone 即得,skill 与它驱动的 `tests/ui/*.tks` 一起演进。

**前置体检**(`check-env.sh`)查四件事,不满足就明确报出来,别让调用方撞进去猜:
`tke` 在不在 PATH / `[ai]` 配没配(explore 才需要) / 设备就绪(Web: chrome+chromedriver;
Android: `adb devices` 有货) / `tests/ui/` 在不在。

**安装 tke 本身**:按平台跑 `./build-linux.sh` / `./build-mac.sh` / `build-win.bat` 产二进制
(禁 `cargo build`,P-02),产物落 `bin/<platform>/`,放进 PATH;
AI 配置走 `-c <config.toml>` 的 `[ai]` 段(敏感 key 别上命令行)。

**CI 上建议 `./build-linux.sh --no-ocr --quiet`**:跳过离线 OCR(tesseract 从源码编译,很慢),
且不需要 cmake/pkg-config。实测 Linux/amd64:`--no-ocr` 9m33s / 产物 28M。

**代价说准**:`--no-ocr` 产物里 `tke ocr` 子命令**依然存在**(CLI 定义不受 feature 门控),
调用时明确报错 `ocr-offline feature not enabled` + 退出码 1(不是静默,符合 INV-9)。
影响面是**依赖 OCR 文字增强的用例**(给无 text/content-desc 的图标补可读文字):
这类脚本在 CI 产物上会与本地行为不一致,必须用完整构建跑。选 `--no-ocr` 前先确认
目标用例不吃 OCR 通道。

## 6. 分阶段路线

| 阶段 | 内容 | tke 改动 |
|---|---|---|
| **0** | 纯 skill 包装现有 `tke run`;脚本先用交互式 `tke harness` 手工产 | 零 |
| **1** | `tke task` headless 一次性模式 + 报告 schema + 五态退出码(ADR-0009) | 主要工程量 |
| **2** | `--intent <yaml>` 意图契约接入 | 中等 |
| **3** | MCP server 化(中途双向交互) | 暂不做,触发条件见 ADR-0009 |

阶段 0 的意义是**在接口定死之前拿到真实使用反馈**——契约一旦发布,改起来贵。
且符合本项目"push ≠ 完成,用户真机确认才算数"的审核机制。

## 7. 红线与已知风险

- **职责边界**:tke 只产报告和脚本,**不改代码**。改代码是调用方的事。
- **并发隔离**(P-10):调用方可能并行开多个,每次调用必须传独立 `--cache` / `--current-dir`。
- **登录态**(INV-12):最大的失败来源。tke 不代办,skill 必须把"提供已登录会话/测试账号"
  的责任明确压给调用方,并在前置体检里查。
- **成本可见**:explore 是分钟级、数万 token。skill 不能让调用方同步傻等,
  应后台跑 + 轮询报告文件,或在 SKILL.md 里写明代价让它先问用户。
- **前置埋点比事后描述更有效**:skill 里应带一条约定——写 UI 代码时顺手埋稳定的
  `data-testid` / `resource-id` / a11y label。这比事后向 tke 描述页面层级的收益高一个数量级。

## 8. 未决

- headless 模式命名:`tke task` vs `tke harness --headless`(前者更清晰,后者少一条顶层命令)。
- 探索产物的 commit 时机:skill 自动落盘 + 建议,还是必须调用方显式确认?
- Android 上 `entry` 的表达(package/activity vs deeplink),以及冷启动净化策略。
