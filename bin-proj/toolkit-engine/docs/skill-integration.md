# tke 作为 skill 融入 coding agent 工作流（设计稿）

> 状态:**原型已落地并实测通过**（2026-08-12）。方向见 [`adr/0010-skill-borrows-caller-ai.md`](adr/0010-skill-borrows-caller-ai.md)
> ——**skill 借调用方的 AI**,tke 不带 AI、不需要 API key。
> 可分发原型在 [`../skill/tke-ui-test/`](../skill/tke-ui-test/)。
> ~~ADR-0009 的 `tke task`~~ 已取消（被 ADR-0010 取代）。
>
> ⚠️ **定位已于 2026-08-12 由用户纠正**（本文早期版本把 harness 的目标错塞进了 skill）:
> **skill 只做「把设备操控/查看能力交给调用方 AI + 留下可复核的证据」**,
> 是改完代码后的一次性检查手段(类比单测/API 测试);
> **不产 .tks/.tklib 资产、不做回放**——那是 harness 的活(目标是未来可复用)。
> 下面第 2/3/4 节的两件套/verify-explore/intent 属于早期设想,**已不适用于 skill**,
> 保留仅作 harness 侧参考。
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
| **explore** | 没有脚本,或功能是全新的 | 分钟级,token 花在调用方 | 设备（**不需要 `[ai]`**） |

**默认策略写死在 skill 里:先找脚本 → 有就 verify → 没有才 explore → 探索产物落进
`tests/ui/` 并建议 commit。** 若只暴露一个入口,调用方每次都会去烧 AI 探索。

**两者都只用现成命令**:verify=`tke run`(NDJSON + 退出码 + 标注截图产物);
explore=调用方 AI 驱动 `fetch`/`control`/`element add` 原子命令,tke 侧零新增(ADR-0010)。
注意 `--copilot` 定位自愈需要 `[ai]`,skill 场景下不可用——回放失败直接报错,交调用方判断。

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

## 4. 证据与产物（实测结构）

不需要专门的报告 schema——调用方 AI 直接读命令输出和产物目录。`tke run --log <dir>` 产出:

```
<log>/<脚本名>_<时间戳>/
├── log.json              每步:命令/成败/error/耗时/截图与页面文件名
├── screenshots/step_NNN.png   标注截图:顶部横幅(操作+成败) + 元素框(红) + 点击点(蓝圈白心) + 滑动轨迹
└── page/step_NNN.xml     每步页面结构
```

**证据分级仍然成立,只是变简单了**:退出码 0 + 断言步通过 = **硬证据**（回放器真实执行）;
调用方 AI 自己"觉得走通了"= 软证据,不作数。skill 把结束条件定死在硬证据上。

`triage`（`app_issue` / `script_stale` / …）现由调用方 AI 依据 log.json + 截图判断,
SKILL.md 里给了分诊表。tke 侧 ADR-0006 的分诊层仍服务于 `tke run --copilot`（需 `[ai]`）。

## 5. skill 形态与安装

可分发原型在本仓库 [`../skill/tke-ui-test/`](../skill/tke-ui-test/),使用者复制到自己项目的
`.claude/skills/` 下即可:

```
<项目>/.claude/skills/tke-ui-test/
├── SKILL.md                 # 主循环 + 先verify后explore + .tks 语法 + 护栏 + 红线
```

没有 verify.sh / explore.sh——**流程写在 SKILL.md 里由调用方 AI 执行**,包一层脚本反而挡住它。

放**项目内** `.claude/skills/`(而非 `~/.claude/skills/`):UI 脚本资产本就该跟代码同仓,
团队 clone 即得,skill 与它驱动的 `tests/ui/*.tks` 一起演进。

**前置体检**(`tke doctor`)不满足就明确报出来,别让调用方撞进去猜:
`tke` 在不在 PATH / **chromedriver 是否与 tke 同目录**(ToolManager 只搜同目录,不回退 PATH)/
Chrome for Testing 在不在(按官方 zip 原样结构找)/ 当前会走有头还是无头 / `tests/ui/` 在不在。
**不查 `[ai]`——skill 模式不需要**。

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

## 6. 进度

| 阶段 | 内容 | 状态 |
|---|---|---|
| 0 | skill 原型（`skill/tke-ui-test/`）+ 本机无头全链路实测 | ✅ 2026-08-12 |
| 1 | ~~`tke task` headless~~ | ❌ 取消（ADR-0010：不需要内层 AI） |
| 2 | intent 意图契约（结构化输入替代自由文本） | 待定 |
| 3 | 护栏命令化（把 asserter/页面契约做成必须调用的子命令） | 待定,触发条件见 ADR-0010 |

**实测结论（Linux/amd64 无头）**：装 Chrome for Testing + chromedriver → 无头启动/采集/操作 →
`element add` 落库建包 → 写 .tks → `tke run` 5/5 步通过。标注截图、log.json、page xml 齐全,
无头下中文渲染正常。撞出两个真缺口:tklib 建包（已修,P-17）、平台自包含（Q-6,待定）。

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

- ~~headless 命名~~ ❌ 整个 `tke task` 已取消（ADR-0010）
- **平台自包含**(Q-6):`.tks` 不记平台,`tke run foo.tks` 不带 `-d` 按 Android 推断 →
  web 脚本报「adb 缺失」。tklib 的 meta.json 已存 platform,要不要让 run 据此兜底?
- **护栏会不会退化**:asserter/supervisor/页面契约现在只是 SKILL.md 里的两条要求。
  若实测脚本质量不行,出路是把护栏做成必须调用的子命令(**不是把提示词写更长**,ADR-0010)。
- 探索产物的 commit 时机:skill 自动落盘 + 建议,还是必须调用方显式确认?
- Android 上 `entry` 的表达(package/activity vs deeplink),以及冷启动净化策略。
- intent 契约(第3节)是否还需要——调用方 AI 自己就有上下文,结构化 intent 的边际收益待观察。
