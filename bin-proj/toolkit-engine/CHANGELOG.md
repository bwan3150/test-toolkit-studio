# 变更记录（toolkit-engine）

只追加,不重写已有条目。每条带日期 + commit + 一句话;细节看 commit message（本仓库 commit 写得很全）。
更早历史直接看 `git log --oneline -- bin-proj/toolkit-engine`。

---

## [Unreleased]

### 2026-08-26 · 服务化定调：ADR-0022 + 远程 API 契约（**只有文档，未落代码**）
用户提出把 tke 变成可远程调用的服务：测试服务器上部署 tke + 模拟器/真机/无头浏览器，
云平台租设备下发任务/交互式探索/脱手收报告；再出 `tke-ui-test-remote` / `tke-security-test-remote`
两条远程 skill 给别家 coding agent 用（本地零安装，未来进 GitHub Action CI）。
- **ADR-0022**（用户当场拍板四条分歧点）：①tke 只做单节点 agent，调度/计费/多租户归云平台
  ②执行模型=**子进程**不做 handler in-process 化（`JsonOutput` 会 `process::exit` + 三处进程级全局态；
  而「每命令一进程」正是 skill 今天的样子，行为等价）③API 三层，**分层依据是计费模型**
  ④远程客户端=二进制 `TKE_REMOTE` 模式（选它是为了**skill 文档不分叉**）⑤远程不开 `red-team`
- **新增 INV-16**（远程只走枚举白名单、无 argv 直通；**L1 必须零 LLM 面**——否则用户白嫖平台 key）
  **INV-17**（租约即隔离、释放即复位——本地从不需要，租赁模式下不复位＝下个租户接手一台登录着的浏览器）
- **docs/remote-api.md**：三层 API（exec/tasks/artifacts）+ 租约 + 白名单与参数过滤 + 计费口径 +
  P1~P6 分阶段与验收 + 六个已知要踩的坑。复活 ADR-0009 的 `needs_decision` 五态出口（INV-3 延伸）
- ROADMAP 加「新主线：服务化」；README 导航加 remote-api.md
- **代码零改动**，全量测试与构建状态不变

### 2026-08-26 · 安全轨补齐 token 用量（计费的最后一块）
ADR-0023 D3 剩下的口子：harness 有 `Summary` 事件带全程总量，安全轨没有，
于是平台对安全任务只能计设备时长。
- **`workflow/security/usage.rs`**：领域内的用量累计件。`add(role, model, p, c)` / `merge()` /
  `is_measured()` / `to_json()`。**分角色留账**（prober / analyst / orchestrator）——
  钱花在自主探测还是对抗复核上是能指导调优的信息，合并成一个总数就再也分不开了；
  `analyst.calls` 也留着（它是每条 finding 一次，次数本身有意义）
- **三处会话都记账**：prober 跑完记 `session.total_usage()`；analyst 的 `one_shot` 改成
  连用量一起返回（**重试失败那一轮也记**——同样烧了 token，不记就是漏账）；
  交互式 orchestrator 出报告时把自己那段会话也算进去（否则对话式跑出来的账只有一半）
- **两条交付路**：无头终局 JSON 的 `usage` 字段 + `findings.json` 里的同名字段，谁先到用谁
- **没量到给 null 不给 0**（INV-9）：`is_measured()` 要求"记过账且总量>0"——
  供应商没回 usage 时全 0，那也算没量到，宁可报不知道
- **任务层两条路都收**：`usage_from_event`（harness 的 summary）+ 结果对象上的 `usage`（安全轨）
- 测试：`usage.rs` 3 条 + analyst 端到端记账（**FakeTurn 带 token 且会累计，所以这条无 key 可测**）
  + `findings.json` 落盘断言。全量 **271 绿**
- **仍待真机**：安全任务的**成功**路径端到端（要 key）——fake provider 需要进程内预排会话，
  子进程里跑不了

### 2026-08-26 · ADR-0023 平台对接 + 任务用量口子（顺带修了安全轨被判"没跑完"的真 bug）
摸了 `TOOLKIT/bug`（Go+Vue 的测试管理平台）之后定的对接方案。接缝比预想整齐：
它的 `scripts` 表注释就写着「后续 TestRun 触发自动化跑、回填结果都从这里出」，
`case_results` 已有 `bug_id`，还有一套跑熟的长作业范式（`release.Runner`）和卡片式交互。
- **ADR-0023**（用户拍板四条）：①平台是客户端，先做直连不做节点反向注册 ②自动化 run 与
  手工 run **同一个实体**（"手工用例也能交给 AI 跑"，界线已模糊，区别只在 `executor`）
  ③安全做成平台的**第七个实体**（未来还要放巡检/看门狗/HealthCheck），不塞 bug 列表
  ④设备池是**平台级**页面（跨 App）。**tke 侧几乎不用改**——平台上的两个按钮直接落到
  已有两层：回归回放走 L1（零 LLM，只计设备时长）、AI 探索/安全扫描走 L2（平台 key 记 token）
- **feat 任务终态给 `usage`**（ADR-0023 D3，计费必需）：从 `Summary` 事件抽全程 token+model。
  **测不到时是 null 不是 0**——0 会被平台读成"这次没花钱"，真相是"没测量到"（INV-9）
- **fix 一次性命令的终局判据**：`tke security --json` 无头跑完只打一个**没有 `type` 字段**的
  结果对象，P3 的"没有 done 事件=没跑完"于是把**成功**的安全扫描判成了 error
  （P3 只测了失败路径）。现在：UiEvent 流终局优先，没有则认最后一个无 type 对象的 `success`。
  这条对所有一次性命令通用，不是给安全轨打补丁
- ⚠️ **安全轨的用量仍测不到**（它不走 `Summary`），平台对安全任务只能计设备时长。已记进 ADR
- 平台侧设计文档写在 `bug/docs/11_device_cloud.md`（云设备+自动化执行闭环）与
  `12_security_entity.md`（第七个实体）。全量 **266 绿**

### 2026-08-26 · P3 落地：任务层（服务端跑 AI + SSE/WS + webhook + 五态出口）
ADR-0022 的 P3，也就是"下发完就脱手、回头收报告"那条路。
- **`src/serve/task.rs`**：`POST /v1/tasks` 起后台任务（202）→ 子进程跑 `tke harness --json` /
  `tke security` → 逐行泵事件 → 终态释放会话+复位设备+webhook 回调
- **端点**：`/v1/tasks`(POST/GET) · `/{id}` · `/{id}/events`(**SSE**，先重放再接实时) ·
  `/{id}/session`(**WebSocket**，桥 `JsonFrontend` 的双向 NDJSON——那协议本就是长连接设计的) ·
  `/{id}/report`（ui/security 两种报告名自己找）
- **五态出口复活**（ADR-0009 的条款，D6/INV-3）：passed(0)/failed(1)/needs_decision(2)/
  blocked(3)/error(4)。**headless 一看到 `awaiting_input` 就立刻终止并回传问题原文**——
  继续跑下去就等于让它自己拿主意；`interactive:true` 的任务才把问题转给 WS 那头的人。
  **没有 done 事件 = 没跑完**，退出码 0 也判 error
- **`red-team` 服务端硬拒**（D5）；`timeout_s` 收在 30~7200 硬执行，超时杀进程
- **三处实测逼出来的修正**：①参数校验必须在租设备之前（拼错的 kind 先撞上"没有 android 设备可租"，
  把"你写错了"报成"这儿没有"）②失败任务要交出 **stderr 尾巴**（最后 50 行）——这次挂的原因是
  节点没配 API key，不给尾巴调用方只知道"失败了"（P-46 同款）③**终局事件要进重放缓冲**，
  否则"任务早跑完了才来订阅"只看到一片空白，而那正是主场景
- 依赖：axum 开 `ws` + tokio-stream + futures-util
- **测试**：单测 6 + 黑盒 7（`tests/task.rs`，**不需要 API key**：测参数校验、进程起停、
  事件流与重放、五态判定、设备归还、webhook 真的发出去——测试里起了个一次性 HTTP 收听端）。全量 **264 绿**
- **待真机验**：真跑一次 AI 编排（拿到 done、出报告、WS 交互式回答问题）要节点配 `[ai]`，
  本机没有 key

### 2026-08-26 · P2 落地：`TKE_REMOTE` 客户端 + 两条 remote skill（生成式）
ADR-0022 的 P2。**一个环境变量决定走哪条路**，命令行一个字都不用改。
- **`src/remote/`**：`argv`（客户端翻译：`-d` → 租哪台、`--log` → 转发+拉回同一个相对路径）/
  `state`（会话落盘，与 web 驱动的 session_file 同套路）/ `client`（ureq，不引新的异步客户端）/
  `mod`（拦截、隐式会话、版本漂移提醒）。拦截在 **clap 之前**——先过本地 clap 等于要求两端版本严格一致
- **`tke remote status|open|close|pull|devices|push`**：显式管会话（平时不用敲，第一条命令自动租）
- **stdout 与退出码原样透传**：这是"文档不分叉"的下半截
- **`--log` 语义改了**（P1 里它是禁用旗标）：改成**全局沙箱路径参数**，两边同一个相对路径。
  **原因是实跑安全轨撞出来的**：本地 `--log logs/scan` 再 `tke report logs/scan` 靠这个路径对上，
  远程把它吃掉的话第二条命令就找不着东西了
- **无设备会话**（`platform: "none"`）：`http`/`recon`/`report`/`task` 不碰设备，没给 `-d` 就
  开一个只有工作区的会话——不占池、不互斥、**不计设备时长**。这是 ADR-0022 D3 的直接推论：
  安全轨强制租一台手机 = 让用户为没用到的设备付租金。**写 security remote skill 时才发现**
- **两条 remote skill 是生成的**：`skill/remote-delta/*.md`（只维护差异：连接方式 + 覆盖表）+
  `build-remote.sh` 把本地版正文**逐字节内联**。结构上不可能漂；publish.sh 打包前自己跑，
  四个包已进 manifest（默认全装）。Q-18 审计结论：真正要分叉的只有 4 个话题
- **测试**：客户端单测 + 黑盒 11（`tests/remote.rs`：起真节点、跑真客户端，不需要设备）+
  `tests/common/` 共享起服务器的那段。全量 **251 绿**
- **本机实测**：web（8 个产物含 report.html 拉回本地）/ 安卓真机 CPH2305（5 个产物）/
  安全轨照 remote skill 原样敲（**没租任何设备**，证据拉回 `logs/scan/evidence/`）
- **跨机没验**：都在本机回环上；网络那段耗时仍未量（Q-17 后半截）

### 2026-08-26 · P1 落地：`tke serve` 单节点（租约 + exec 白名单 + 产物）
ADR-0022 的 P1。**代码 + 三层测试 + 守卫**一起进来。
- **`src/serve/`**：`allowlist`（命令白名单 + 禁用旗标 + 宿主路径表，INV-16 的执行点）/
  `lease`（设备独占 + 目录隔离 + TTL/心跳 + 复位计划，INV-17）/ `exec`（子进程 + 参数注入 +
  分层计时）/ `routes`（9 个端点，鉴权走中间件——"忘记查"是这类代码最典型的洞）
- **`tke serve`**：`--bind/--port/--token/--root/--ttl/--exec-timeout/--web-slots/--max-upload-mb`。
  **没 token 就只准绑回环**；`--port 0` 时打印真实监听地址（这行是契约，接口测试靠它）
- **依赖只多了 5 个 crate**（axum/axum-core/httpdate/matchit/serde_path_to_error）——
  hyper/tower 栈本来就被 genai→reqwest 拖进来了，实测印证了 §10 的判断
- **`utils::sandbox::resolve_in_workspace`**：从 orchestrator 搬到 utils。远程要用同一条规则挡
  `--image /etc/passwd`，一条规则只能有一处实现
- **测试三层**（ADR-0008）：单测 30（`src/serve/**`）+ 黑盒接口 10（`tests/serve.rs`：起真二进制、
  发真 HTTP、跑真子进程，**不需要设备**）+ 真设备 e2e（`tests/e2e/serve-smoke.sh`）
- **真机实测通过**：本机 Linux amd64，**无头 Chrome 9/9**（纯 HTTP 起浏览器→打开页面→落 8 个证据
  文件→下回 52KB 截图→释放时浏览器真的被关掉）+ **安卓真机 CPH2305 8/8**
- **守卫 `check-serve-paths.sh`**（已挂 pre-commit）：扫 CLI 里带 `#[arg(long)]` 的 PathBuf 参数，
  漏登记就报红。**写它的当天就抓到两个真洞**——`refresh --out`、`control browser-download --dir`，
  两个都能读写会话工作区外。按 P-12 造了违规现场验证它真的会红
- **单测逼出一个真 bug**：`acquire` 里顺手 `retain` 掉过期租约 → 设备**绕过复位**直接给下一个租户
  （违反 INV-17）。改成过期租约照样占着设备，直到 sweep 复位完才回池
- **量了才知道**（Q-17）：进程启动 **0～1ms**，占比 <0.1%，耗时全在设备（起浏览器 690ms / 打开页面
  5.0s / 安卓一步 10.4s）。ADR-0022 D2 的重新审视触发条件**没有出现**，别为此重构

### 2026-08-26 · HTTP 框架定为 axum（`remote-api.md` §10）
上一条里「别顺手 cargo add axum」的顾虑，查完 `Cargo.lock` 后被自己推翻了。
- **决定性事实**：hyper 1.7 / hyper-util / tower / tower-http / h2 **早就在依赖图里**——
  `genai`（非可选）→ reqwest 0.13 → hyper-rustls → hyper，`--no-default-features` 下同样在。
  axum 的真实增量只有四到六个小 crate
- **选它的理由**：增量≈0 / **SSE 与 WS 都内置**（正好是 L2 的两个需求）/ tower-http 给白名单之外的
  第二层护栏（body 限制·超时·并发限制）/ `ServeDir` 覆盖 L3 产物下载
- **落选**：裸 hyper（自制请求解析＝新攻击面）、tiny_http（无 WS、每连接一线程）、
  poem-openapi（生态窄宏重，OpenAPI 改手写）、actix-web（第二套运行时）、warp（社区重心已转移）
- **配套 A**：client-only 瘦身的瓶颈**不是 axum**（<1MB），是 genai/image/rusqlite(bundled)。
  feature 布局 serve/agent/client；但「谁占那 30MB」**没量过**——P2 前先 `cargo-bloat`（Q-17 规矩）
- **配套 B**：TLS 不进 tke，交给前面的 nginx/Caddy 或平台内网 mTLS（理由与 ADR-0022 D1 同源）

### 2026-08-25 · 一次装全 skill（默认装分发源上所有 skill，manifest 驱动）
用户：skill 都是 md 不占地方，装一次就该全到位、别分开装。
- **feat manifest**：publish.sh + CI 写 `skills`（一行一个 skill 名）；install 读它决定装哪些
- **install.sh / install.ps1 默认装全部**：不带 `--skill` → 读 manifest 循环装所有 skill；
  `--skill <名>` 保留为只装一个。默认 profile=all（ui-test 要驱动），仅 `--skill tke-security-test` 时=none
- 加新 skill 自动纳入分发，不用再改 install
- 本机验：起本地分发源 → `install.sh`（无参）→ tke-ui-test + tke-security-test **都装上**

### 2026-08-25 · 往后端深挖：服务暴露 playbook + `recon detect`（治"只查表面"）
用户指出 tke security 只暴露表面（缺头/路径），挖不到 konechome 那种 Sanity 后端泄露——因为缺**服务专属知识**。
去互联网搜了真实案例，提炼成「认出服务→已知误配→精确零凭据探测」的方法论，接进代码与提示词。
- **feat `recon detect`**（`recon/detect.rs`）：从首页/JS bundle 扒后端标识——Sanity `projectId`+`dataset`、
  Supabase `<ref>.supabase.co`、Firebase `firebaseio.com`、Algolia、GraphQL 端点、S3 桶——**并在 detail 给出
  零凭据探测式**。命中=info 级线索（不是漏洞本身，AI 打一发确认才算，INV-13）。3 单测
- **doc service-playbook.md**（skill reference）：每个服务的指纹+已知误配+精确探测+**防误报**（Stripe pk_/
  Firebase apiKey/Supabase anon/Google Maps 是设计上可公开的，别报成漏洞；sk_/AKIA/service_role 才是真泄露）
- **prompt 内置深挖方向**：prober.md/orchestrator.md 加「往后端深挖」——测绘后跑 detect，认出服务用零凭据探测坐实。
  让 `tke security` 自己的 AI 也挖深，不只 skill
- 真机验：`recon detect` 对已修好的 konechome 正确返回 none（无假阳）；单测证明有 Sanity 时必抓到。全量 142 绿

### 2026-08-25 · 分发管线泛化到多 skill（tke-ui-test + tke-security-test 一行装）
skill 分发原本写死 tke-ui-test；泛化成「凡 skill/<名>/SKILL.md 存在就打一个包」，install 用 `--skill` 选装。
- **publish.sh + CI（tke-publish.yml）**：循环打所有 skill 包（各含 VERSION），自查也循环校验
- **install.sh / install.ps1 加 `--skill`**（默认 tke-ui-test）：装指定 skill；错误/提示/解压路径全参数化
- **新增 `none` profile**：只装 tke、不装任何驱动——安全 skill 用不到浏览器/adb。
  `--skill tke-security-test` 时 profile **自动=none**；none 档体检只验 `tke --version`
- **uninstall.sh/.ps1**：一并卸载 tke-security-test
- **README**：加安全 skill 装机命令
- 本机端到端验过：起本地分发源 → `install.sh --skill tke-security-test` → 只装 tke + security skill、
  提示 `/tke-security-test`；publish 打出两个包都含 VERSION、源码树无残留

### 2026-08-25 · tke-security-test skill（借调用方 AI 做黑盒安全测试，承 ADR-0010/0020/0021）
安全轨的 skill 本体：教 Claude Code/Codex 等编程 agent 用**自己的**能力 + tke 的 primitive 做安全测试。
命令全部基于本会话真机跑通的流程（konechome 两轨实测）。
- **doc `skill/tke-security-test/SKILL.md`**：红线（只测授权目标/强度档/脱敏/不产资产）+ 主流程
  （task new → recon 扫 → 顺藤摸瓜 → 自己判定 → 写 findings.json → tke report）+ 强度档纪律 + 空结果如实报
- **doc `reference/recon-and-findings.md`**：七 verb 判据 + findings.json 完整字段 + 顺藤套路
- 与 tke-ui-test 同哲学（ADR-0010）：tke 给手/眼/证据/报告，判断交给调用方；一次性检查不产 .tks
- **待做**：分发管线泛化（`skill/install.sh` 现写死 tke-ui-test，要支持多 skill 才能一行装 tke-security-test）

### 2026-08-25 · 共享任务生命周期：统一 tke report + tke task（ADR-0021 取代 0020）
用户提出更干净的模型：task/steps/report 是**领域无关**的生命周期层，一个任务是 UI 还是安全测试
是它的**属性**（记在任务目录的 task.json 里），不该拆成 `tke ui report`/`tke security report`。
- **feat `task.json` 标记 + `is_security_task`**（`workflow/task.rs`）：任务目录记 `{kind,target,mode}`；
  没标记则看有没有 findings.json 兜底
- **feat `tke task new --kind <ui|security> [--target --mode --dir]`**：建目录 + 写标记，skill/脚本的干净起点
- **feat `tke report <dir>` 统一分派**：读 task.json → security 读 findings.json 出安全报告 / 否则原设备报告。
  一条命令两轨通用
- **revert ADR-0020 的拆分**：删 `tke ui report`（回 `tke report`）、删 `tke security report` 子命令；
  skill 文档改回 `tke report`。`tke security` 起始自动写 kind=security 标记
- **steps 暂不统一**（有意，见 ADR-0021）：设备 vs URL 目标模型不同、收益低，等 skill 证明需要再议
- **doc ADR-0021**（取代 0020）+ 全量 139 绿。端到端验过：task new → 写 findings.json → report 自动出安全报告

### 2026-08-25 · tke security report primitive + 多轨命名约定 tke <track> report（ADR-0020）
为安全 skill 轨（tke-security-test，下一步）铺路：把 reporter 暴露成确定性命令，并统一多轨报告命名。
- **feat `tke security report <findings.json> [--out]`**：无 AI 纯渲染——调用方（skill/脚本/CI）自己
  收集 findings 喂进来，得到与 `tke security` 同一套品牌报告。`Finding/Severity/Category/EvidenceRef`
  加 Deserialize，可选字段给 serde 默认（喂最小结构即可）。`tke security` 仍是裸进交互（子命令与裸命令
  用 args_conflicts_with_subcommands 并存）
- **feat 命名约定 `tke <track> report`**：`tke report` 改名 `tke ui report`（↔ skill tke-ui-test），
  与 `tke security report`（↔ tke-security-test）对称；**`tke report` 保留为隐藏别名**向后兼容。
  更新 tke-ui-test skill 文档 + 顶层 help
- **doc ADR-0020**：命名约定 + 安全 skill 轨决策（承 ADR-0010）
- 全量 138 绿。skill 本体（tke-security-test）是下一步

### 2026-08-25 · orchestrator 多行消息改用 Assistant 事件（修 TUI 阶梯缩进）
真机 TUI 里 agent 的长回复（带 Markdown 项目符号）渲染成逐行右移的阶梯——因为我用了 `Notice`
事件发对话消息，而 `Notice` 的渲染走带缩进的包裹逻辑，多行就累加缩进。
- **fix agent 对用户说的话（Text 回复 + 调工具的前导说明）改用 `UiEvent::Assistant`**——
  harness 本就为「主 AI 说话」设计，`text.lines()` 逐行从第 0 列渲染，多行正确。
  短状态行（→ http / findings / 报告路径）仍用 `Notice`。token 用量取 `session.last_usage()`

### 2026-08-25 · tke security 无参进 TUI + 主 agent 开场面试（选项选择）
用户要的入口：直接 `tke security`（连 url 都不给）就进交互 TUI，**主 agent 来面试**——问测什么、
什么强度（选项选）、什么 scope。
- **feat 开场面试**：`--mode` 去掉默认值（改 `Option`），裸进时 mode/focus/target 皆未定 →
  orchestrator 开场把「已知/未知」交给 agent，提示词引导它用 `ask_user` 逐项补齐
- **feat `ask_user` 带 options**：有选项就走 `await_choice_or_text`（TUI 渲染成列表+内联输入，
  用户可选可打字，正是「tke 那种选项选择」）
- **feat `set_scope` 工具**：面试结果（target/mode/focus）写回运行态，真正参数化本次测试（报告 mode
  标签、探测默认目标都用它）
- 无头（`--json`）不变：没法交互，mode/focus 用默认兜底
- 全量 137 绿。**交互真机待验**

### 2026-08-25 · tke security 收成单一入口 + 对话式 orchestrator（改对 CLI 形状）
用户纠正：`tke security` 该像 harness 一样**一个入口**（默认交互、`--json` 对接），我之前错做成
`security probe` / `security run` 两个子命令（且 `run` 与 .tks 回放语义撞车）。改回 ADR-0019 原样。
- **fix CLI 收成单一 `tke security [url]`**：删 probe/run 子命令。交互终端→对话式 orchestrator；
  `--json`/非终端→无头一次性（探测→复核→出报告，一次性 JSON）。`tke http`/`tke recon` primitive 不动
- **feat orchestrator**（`orchestrator.rs`）：安全测试的对话外壳（ADR-0002 同形），**复用 harness
  的 `Frontend` 三前端**（Plain/Json/**TUI**）——「共享 TUI」即直接用 `TuiFrontend`。工具
  recon/http/record_finding/ask_user/report/finish；**它对用户说话（只回文字不调工具）时就把话筒交回**，
  等用户下一句（REPL 回合）；有风险/升档先 `ask_user`。提示词 `agents/orchestrator.md`+tools（可覆盖）
- 全量 137 绿。**对话式交互真机待验**（无头管线已在 konechome 跑通）

### 2026-08-25 · tke security #2 完成：analyst 对抗复核 + reporter 出报告 + `tke security run` 全流程
prober 之后补齐 #2 剩两段，`tke security run <url>` 现在能一条龙：探测→复核→出 HTML 报告。
- **feat analyst**（`analyst.rs`）：对抗式复核官，单次结构化输出（INV-2，强制 report 工具）。
  逐条 finding 把**关联证据原文**喂给它据实判断（INV-13）：`keep=false` 毙假阳、
  `confirmed` 分软硬、可修正 severity/category/title/detail。0 findings 时零 LLM 调用
- **feat reporter**（`report.rs`）：**确定性**生成（不走 LLM，可单测可回归）——
  `security-report.html`（暖色计分板+严重度环形+Toolkit 品牌，亮暗自适应，风格对齐用户确认的基线）
  + `findings.json`（机器可读，含 outcome/score/counts）+ 每个**已确认**发现一份 `vuln-<id>.html`
  （疑似只进全局清单不单独出，INV-13）。全程 HTML 转义防注入
- **feat `tke security run`**：probe→analyst→report 三段编排；`probe` 保留为只探测入口
- **feat 提示词**：analyst 的 agent/tools 提示词进 builtin（可外部覆盖，同 prober）
- **feat examples/security_report_sample.rs**：无 LLM 造样例报告，验版式
- 6 个新单测（analyst 毙假阳/空跳过、reporter 评级/转义/vuln 文件）。全量 137 绿。**真机待复跑**

### 2026-08-25 · prober 收敛性修复：去重拦截 + 无进展强制收尾（真机撞出的死循环）
用户 mac 首跑 prober（konechome）暴露死循环：模型顺藤对了（认出 Framer→追到 framerusercontent
CDN），但**反复抓同一批 URL 几十次**（robots/sitemap/searchIndex），24 步没 finish 也没 record，
撞上限收场。根因：① 无「已取过」记忆；② `tool_result_bulky` 把旧大响应换占位→模型忘了→回头再抓。
- **fix 循环层去重**：记住 (方法+url / recon verb+url)→步号；重复请求直接回指旧步号，
  不再打网络、不再落证据（真机会把预算耗光）
- **fix 无进展强制收尾**：整轮都在重复 → 计数，连着两轮就强制 finish，不再空转到 max_steps；
  预算快用完时主动要求收尾。撞上限时的 summary 也据 findings 有无说人话
- **prompt 收敛纪律**：绝不重复抓、空 findings 是合格结论、死胡同就放下、Framer 数据在 JS chunk 非
  searchIndex、有预算感优先追最可疑线索
- 两个新单测钉住去重（重复不落新证据）与强制收尾（steps<10 而非撞 50）。全量 133 绿。**真机待复跑**

### 2026-08-25 · tke security P2 起步：prober 顺藤摸瓜（AI 编排 + 独立提示词体系）
#1 侦察底座真机通过后开 #2。搭出**探测官 prober**——多轮、接地、带工具的 LLM 循环
（形态学同 harness 的 orchestrator/explorer，但工具/角色/提示词另起一套，只借 provider）。
- **feat prober 循环**（`prober.rs`）：工具 http / recon / record_finding / finish；
  每轮基于刚看到的真实响应决定下一步（接地 INV-1），每个探测过 evidence（INV-14），
  findings 由 prober 显式 record 才进候选（recon 结果只是线索）。max_steps 兜底防跑飞
- **feat 独立提示词体系**（`prompt/`，同构 harness）：builtin `include_str!` 内嵌
  （`agents/prober.md` + `tools/prober/*.md`）+ 外部目录覆盖（布局同 builtin）+ 空串守卫；
  占位 `{target}/{mode}/{focus}`。**默认可用、可自定义**
- **feat 统一 Finding 模型**（`finding.rs`）：severity/category/confirmed(软硬分,INV-13)/repro/evidence，
  Serialize 进 findings.json；ProbeReport 汇总
- **feat `tke security probe <url> [--mode --focus --prompts-dir --max-steps]`**：直接跑 prober
- **fix 证据 EvidenceRef 加 Serialize**（进 findings.json 用）
- 无 AI 配置时清晰报错不 panic；prober 循环用 FakeLlm 脚本化单测（recon→http→record→finish 全链）。
  全量 132 绿。**真机由用户复验**（用他 mac 的 [ai] 配置对真实目标跑）

### 2026-08-25 · tke security：证据续写（不再覆盖）+ macOS 一键冒烟脚本
写冒烟脚本时逼出一个地基 bug：一次评估是多个进程调用（http + 各 recon verb）共用一个
`--log` 目录，但 `EvidenceDir` 每次都从 `step_001` 重编号 → **后面的探测覆盖前面的**，
证据只剩最后一条命令的。违背「一个任务一份、反复调用续写、连续编号」原则。
- **fix `EvidenceDir::new` 续写**：扫目录已有 `step_NNN` 取最大值 +1 接着排；单测钉住重开续写
- **test `tests/security-smoke.sh`**：macOS/Linux 一键冒烟——build（可 `--no-build`）→ 跑 `http`
  + 七个 recon verb（`--log` 共用一个任务目录）→ JSON 美化 → 证据一览。跑**刚构建的**
  `bin/<platform>/tke`（非 PATH 里那个）；默认目标 example.com，提示只对授权目标跑
- 全量 129 绿；本机端到端验过脚本（证据连续 step_001..016 不覆盖）

### 2026-08-25 · tke security P1 续：recon 六个 verb 补齐（地基先打牢）
在 headers 之上把侦察 primitive 补全，`recon.rs` 拆成 `recon/` 目录（一 verb 一文件）。
统一结果结构 `ReconResult`{findings, probes}——单请求=1 probe，多路径=N probe，全落证据。
六个新 verb 都可 fake 单测，`fingerprint`/`tls` 已真实网络冒烟（example.com）：
- **feat `recon fingerprint`**：从头/Set-Cookie/页面特征认技术栈（Next/Nuxt/WP/Express/Django…），info 级
- **feat `recon cors`**：带假 Origin 探 CORS——反射任意 Origin+凭据=High，通配=info
- **feat `recon graphql`**：POST 最小 introspection，看 schema 是否对外开放
- **feat `recon bundle`**：正则扫 JS 里的 AWS/Google/Slack/JWT/私钥/通用密钥，命中 High
  且**脱敏**（只留前 6 字符，承 P-45）
- **feat `recon endpoints`**：探 .env/.git/actuator/server-status/robots 等常见路径；
  **防 SPA 兜底假阳**——命中要 200 + 非 HTML + 内容签名对得上
- **feat `recon tls`**（轻量）：明文 HTTP 是否强制跳 HTTPS + HSTS；深度证书检查待接 TLS 库
- 全量 `cargo test` 128 绿（security 21 个）。**真机由用户复验**

### 2026-08-25 · tke security P1 侦察底座：HTTP 原语 + 证据 + recon headers（可 fake 单测）
P0 定契约后开写。新增 `src/workflow/security/`（第二个 agent 领域的业务逻辑层），
两个 primitive 命令，均端到端真实网络冒烟通过（example.com）：
- **feat `tke http <METHOD> <URL> [-H 'K: V'] [--data body]`**：原始 HTTP 探测。
  4xx/5xx 照收当正常响应（探测就是要看状态码），默认不跟随重定向，响应体限 2 MiB。
  `--data` 只有长名——短名 `-d` 被全局 `--device` 占用（clap 冲突，已避开）
- **feat `tke recon headers <URL>`**：安全响应头检查（HSTS/CSP/点击劫持/nosniff/Server 版本）——
  首个 curated 被动判据，safe 档可跑
- **feat HttpEngine trait**：真实 `UreqEngine`（全链路 timeout，守 Q-4）+ `FakeEngine`
  （按方法+URL 子串脚本化响应）——探测逻辑脱离网络单测，沿用 FakeDriver/FakeLlm 文化
- **feat `evidence.rs`**：`--log <目录>` 给了就把 请求/响应 落进 `evidence/step_NNN_{req,resp}.txt`
  （INV-14，无无证据的第二条路）；相对路径进 findings.json
- 加 `TkeError::NetworkError`；14 个新单测（http/evidence/recon），全量 `cargo test` 114+ 绿
- **未做**（P1 续/P2）：recon 其余 verb（cors/graphql/bundle/tls/fingerprint）、prober/analyst/reporter
  角色、`tke security` 编排、报告生成。**真机由用户复验**

### 2026-08-25 · tke security P0 设计锁（ADR-0019 + 报告 spec + INV-13/14/15）
新方向：`tke security`——探索式黑盒安全测试，作为 tke 的**第二个 agent 领域**（骨架复用 harness，
工具/角色/提示词另起一套）。产全局安全水平报告 + 每个确认漏洞一份独立报告。本条只落 **P0 设计文档**，
无 src 改动；P1 起写码。
- **doc ADR-0019** 定死：能力三层分层（`tke http`/`tke recon` primitive ⇄ AI 工具 ⇄ `tke security` 编排，
  照 device 那套）；角色 recon→prober→analyst→reporter，analyst 对抗闸门防假阳；三信息层级只改种子来源；
  **强度阶梯 `--mode`**（passive/safe/aggressive/red-team，默认 safe）+ 正交 `--focus`；五态出口
- **doc INV-13/14/15**（写入 INVARIANTS.md）：判定必须黑盒复现 / 每探测落证据无第二条无证据路 /
  强度默认最安全·升档显式·破坏不可逆需逐次确认·模式必落报告
- **doc security-report-spec.md**：全局报告 6 段 + 单漏洞报告 9 段 + **报告材料库**（概览/图表/文字卡/
  可复制执行/证据五类区块，agent 组合用）+ 机器可读 `findings.json`
- **doc security-report-template.sample.html**：可视化基线——对齐参考仪表盘风格（暖色渐变环形图 + KPI 条 +
  风险矩阵 + 攻击路径 + 带复制按钮的命令/脚本块），Toolkit 品牌，窄屏横滚，亮/暗手动切换定型

### 2026-08-22 · boot 之后把屏幕唤醒;ANR 与就绪判据都说人话
用户 mac 上跑起来了但**报告里两张截图都是纯黑**,第二步 `uiautomator dump` 连文件都
产不出来。根因是 `sys.boot_completed=1` **只说系统起来了**——屏幕可能还关着或停在锁屏,
那时候截什么都是黑的、也采不到东西。
- **fix boot 后唤醒屏幕 + 推开锁屏**（`KEYCODE_WAKEUP` + `wm dismiss-keyguard`）。
  这属于"把环境准备好",不是替人操作被测对象——锁屏不是被测对象,
  而黑着的屏幕上什么都做不了
- **fix `screen_on` 判据认三代字段**（P-50）:第一版只认 `Display Power: state=ON`,
  而 Android 15 那一行是个**对象引用**,永远匹配不上 → boot 一路等到三分钟超时
- **fix 就绪超时的错误逐项验、只说真正没过的**（P-49）:上一版一律写
  "sys.boot_completed 一直不是 1",而实际卡住的是屏幕那项——**这句写死的文案
  把我自己带偏了三轮**（去查 adb 路径、userdata 损坏、强杀残留）
- **fix boot/shutdown 的竞态**:`adb emu kill` 只是把命令发过去就返回,那台还要
  好几秒才退干净。用户 shutdown 完立刻跑脚本,`启动环境` 4 秒就"成功"——
  它把**正在关闭的那台**当成了已经在跑。现在 shutdown 等它真的消失,
  boot 的幂等检查也要问 `boot_completed`
- **feat 步骤超时时点名 ANR**:"元素反复重试或页面无响应"听起来像被测页面的毛病,
  而真相可能是 `System UI isn't responding` 那个对话框盖住了。
  现在直接报「屏幕上盖着「com.android.systemui 无响应」的系统对话框,点不到下面的元素」
- **fix `uiautomator dump` 退出码 0 不代表产出了文件**:等不到 idle 时它打印
  "ERROR: could not get idle state." 然后**正常退出**,错误要到下一步 `adb pull` 才炸,
  报的却是「failed to stat remote object】——指向文件不存在,真正的原因完全看不出来
- AVD 模板给足资源:3072MB / 4 核（软件渲染下 2 核 2GB 会让 SystemUI 一直 ANR）

### 2026-08-21 · 谁家的 AVD 用谁家的 SDK（用户已有的模拟器现在能跑了）
用户 mac 上有自己的两台 AVD（Android Studio 建的）。`tke device` 列得出来,
但**一旦他再装我们那套 SDK,这两台就跑不了了**——早先 `with_env` 无条件把
`ANDROID_SDK_ROOT` / `ANDROID_AVD_HOME` 指向 `~/.tke/android-sdk`,
于是 emulator 既找不到他的 AVD、也对不上镜像路径（他的 `config.ini` 里
`image.sysdir.1` 指的是**他那套 SDK** 里的 system-images）。
- **feat** `Toolchain`（emulator + SDK 根 + AVD 目录,**三样配套**），
  `toolchain_for(avd)` 按**这台 AVD 属于谁**选整套,而不是全局挑一个 emulator
- `list_avds` 改成**扫两边的 `<avd_home>/*.ini`**:`emulator -list-avds` 只看一个
  AVD 目录（取决于环境变量），而这里恰恰要把两套合起来;扫目录也不依赖二进制跑得起来
- `<名字>.ini` **与**同名 `.avd` 目录都在才算数——只剩 ini 的是删剩的残骸,
  列出来会让人去启动一台根本起不来的（单测钉住）

### 2026-08-21 · `tke device` 只列**立刻能用的**,未启动的折叠成一句话
用户在 mac 上看到的清单里,iOS 那边折叠了 22 台未启动的,而两台没启动的 AVD 却直挺挺
列着——因为安卓和 iOS **各写各的折叠条件**("有在跑的才折叠"),同一份清单两套规矩。
- **默认只回答"现在能测什么"**:没启动的模拟器一律不列,`--all` 才全列
- 折叠逻辑收拢到 `fold_idle_simulators` 一处,各采集函数不再自己判断
- ⚠️ **只折叠没启动的模拟器,不是所有 `ready=false`**:离线的安卓真机
  （插着但 unauthorized/offline）必须继续显示——那是"连着却用不了",
  折叠掉人就不知道该去点授权弹窗了。单测钉住这条
- 文案从 `22 台模拟器未启动 · --all` 改成
  **`tke device --all 查看其他 22 台未启动的设备`**——那行常常是人第一次看到
  `--all` 这个词,让他自己拼命令是没道理的

### 2026-08-21 · 安卓模拟器 Linux amd64 **端到端实测通过**,逼出四个真 bug
装 → 起 → 装 App → 启动 → 采集 → **按文字点击** → 页面真的跳转 → 证据落盘 → 关机,
整条链路在 Linux amd64 上跑通（截图里 "Network & internet" 被红框标出、蓝点是实际
点击位置,点完页面确实换了）。过程逼出四个 bug:
- **fix 截图是纯色（P-47）**:`-gpu swiftshader_indirect` 起得来、采得到、点得中,
  **唯独截图是纯色**（63KB vs 正常 1.7MB）;emulator 自己的 `emu screenrecord screenshot`
  也一样,说明合成器只出了背景层。改用 **`-gpu swiftshader`**（不带 `_indirect`）
- **fix 镜像换成 `default`**:`aosp_atd` 虽然小 100MB,但它**默认关掉硬件渲染**,
  截图恒为纯色（Google 让你用 AndroidX Test Screenshot API,那是进程内的,外部拿不到）。
  tke 的立身之本就是留证据,省 100MB 换"报告里全是黑图"这交换不成立
- **fix `启动 ["pkg/.Act"]` 拼出末尾多余斜杠**:tks 单参数写法把整串当包名、activity 是空的,
  而 `format!("{}/{}")` 又补了一个 → `am start` 回 `result code=-92`（START_ABORTED）
- **fix `am start` 的失败完全看不见（P-48）**:它**失败时退出码仍是 0**,错误文本走
  设备那边的 stderr,`adb shell` 不并进 stdout,tke 这层又只收 stdout——两层一叠,
  包根本没装也报成功。改成 `am start ... 2>&1` 再查 `Error:`
- **fix `platform-tools/` 缺失**:emulator 靠这个子目录认 SDK root,缺了直接
  `FATAL | Broken AVD system path`,而报到人眼前的是"起了三分钟还没就绪"
- **fix `avd:` 前缀两条路各解析各的**:`device info` 走 `DeviceManager` 没解析,
  把 `avd:tke` 原样塞进 `adb -s` → `adb: unknown host service`。统一到一个解析口
- **fix KVM 判据**:emulator 读 `/etc/group` 看你在不在 kvm 组,**不是** open `/dev/kvm`
  ——`setfacl` 那条路它不认（而且那个 ACL 会被 logind 重置）。tke 的提示改成 usermod

### 2026-08-21 · `tke doctor --fix --profile android-emu`:从 Google 官方源装模拟器
用户问能不能把模拟器镜像也放我们自己的分发源。**查了条款:不行**——Android SDK 许可
3.4 明文禁止 redistribute「the SDK or any part of the SDK」,3.1 又是 non-sublicensable。
(WebDriverAgent 能自己分发是因为它 BSD 开源,不是因为它小。)
所以改成:**我们做编排,Google 做分发**——字节来自 dl.google.com,许可关系是用户 ↔ Google,
我们只替他敲那几行命令(同 ADR-0014 的 `tke update` 就是去跑官方 install.sh)。
- **feat** `cli/android_sdk.rs`:解析 Google 仓库 XML → 挑本机的 emulator + `aosp_atd`
  系统镜像 → 下载解压 → 建 AVD。**不需要 JDK**(官方 `sdkmanager`/`avdmanager` 是 Java 写的,
  而包的直链就在清单里,AVD 本质是两个 ini 自己写更直接)
- 下载前把「这是 Google 的 SDK,下载即接受 Android SDK 许可」**说给人听**
- **feat** 卸载:`tke uninstall --all` 一并删(约 2GB);细分 `--android`。
  安卓 SDK **由 tke 自己删**不交给 uninstall.sh——它本来就是 tke 装的,而且不该等发版
- **fix 解压写成稀疏文件**:`system.img` 是稀疏镜像,`io::copy` 展开后**实占 8.1GB**,
  改成遇全零块就 seek 后降到 **1.1GB**(整套 9.0G → 2.1G)。体积统计也改成算实际占盘,
  否则卸载预览会说"9603 MB"而人量出来只有 2GB
- **fix 挑包按 revision 而不是出现顺序**:第一版"取最后一个",实测装到的是 37.1.11
- **fix `avd:` 的 kind 混用**:AVD 原来也叫 `android`,于是 doctor 把没启动的 AVD
  **算进了真机**("Android真机 可用 (1 台)"——而这台机器连 adb 都没装)。单独一个 `android-avd`

### 2026-08-21 · `cargo test --lib` 漏掉 bin crate（P-46）
`src/cli/` 属于 bin crate,而 AGENTS.md 的必过清单一直写 `--lib`——**它只测 lib**。
`cli/fix.rs` 那几个测试在 `detect_missing` 改名后就编不过了,连着几次"全绿"提交都没发现;
`json_output.rs` 的三个 doctest 也烂着(缺 `use`,而且示例里的函数会 `process::exit`)。
- 清单改成 `cargo test --no-default-features`(全量),AGENTS.md 两处都改
- 修 `cli/fix.rs` 的三处调用、`json_output.rs` 的三个 doctest(加 `use` + `no_run`)
- **教训写进 P-46**:绿灯的范围要跟你以为的范围对得上。只覆盖一半代码的命令比没有更危险

### 2026-08-21 · 安卓模拟器（AVD）起停 —— **选装**,不进依赖检查（ADR-0018）
先把账算清楚(官方仓库清单实测):`emulator` 包 351~490MB + `aosp_atd` 系统镜像
450~860MB = **一台 0.8~1.3GB**;而且 **Google 至今不发布 linux-arm64 版**。
用户拍板:iOS 模拟器是 macOS **自带**的(我们只补 21MB 的 WDA runner)、
安卓真机开发者模式很好开——**模拟器不是必经之路,做成选装**。
- **feat** `drivers/avd.rs`:定位 `emulator`(从 `ANDROID_HOME`/SDK 默认路径找,
  **不走 ToolManager**——那个只在 tke 同目录找,报错会把人引到错方向)、列 AVD、起、等就绪、关
- **feat** `-d avd:<名字>` + `启动环境` / `关闭环境`。**序列号是起来之后才有的**
  (`emulator-5554`),拿它当启动参数是循环论证,所以按 AVD 名指定;boot 后把实到的
  序列号记进 `AdbDriver::resolved`,后续每条 adb 命令都用它
- 等就绪等的是 **`sys.boot_completed`**,不是"adb 认出设备"——那时系统还在起,
  装 App / 采集都会失败(同 iOS 等 `bootstatus` 的理由)
- 无头按有没有桌面自动定(同 web 的 `--headless=auto`),无头下加
  `-gpu swiftshader_indirect`(CI 机器上没有可用的 GL)
- `device list` 把没启动的 AVD 也列出来(id = `avd:<名字>`);`doctor` 那行写
  「未安装（选装）」,**不进「下一步」催人装、退出码也不因此非 0**
- 端口这边**天然不撞**:每台 AVD 占一个从 5554 起步进 2 的控制台端口,Q-13 那类坑不存在
- **未在装了 SDK 的机器上跑过**(本机没有)——待真机验

### 2026-08-20 · Q-13 用户实测通过;③ 的判据放宽到"两边不是同一个 App"
用户 mac 上 iPhone 17 Pro + 16 Pro 并行:**端口 8149/8197 分开、两边 PID 对上、
两个端口报的前台是不同的 App、两边证据各落各的目录**。
`SIMCTL_CHILD_USE_PORT` 那个预编译 runner **确实认**（之前只是推测）。
- **判据放宽**:串台的话两个端口必然报同一个 App;至于前台是不是刚 launch 的那个不重要
  ——用户那台上原本开着 `com.example.app`,launch 的设置没顶上去,但结论照样成立
- 拉起 WDA 那步**不再报红**:拉起 runner 必然挤掉前台,采集报"现在前台是桌面"是意料之中,
  只有真起不来才该红
- `simctl launch` 的输出不再吞

### 2026-08-20 · 会话建不起来时把真原因说出来;并行③改用「开不同 App」判串台
用户重跑:①②通过（端口 8149/8197、PID 两边都对上）。③④ 的红牵出两件事:
- **fix `ensure_existing` 吞掉了 attach 的真实错误**（INV-9 违规）。原来是
  `if let Ok(conn) = self.attach_foreground(..)`,失败就换成一句泛泛的
  「无活动 WDA 会话,请先执行 启动 [BundleID]」——**而被吞的那句恰恰是有用的那句**:
  「现在前台是桌面（主屏幕）,不是你要测的 App…」。用户那台前台没 App 的模拟器
  就卡在这儿:报告有、截图没有,看不出为什么。现在原样透出去
- **③ 不再比分辨率**:iPhone 17 Pro 与 16 Pro **都是 1206×2622**,两边一样根本判不出
  是串台还是本来就同尺寸。改成给两台开**不同的 App**（设置 / Safari）,
  再问每个端口的 WDA「你前台是哪个 bundle id」——**内容层面的证据,跟型号尺寸无关**
- ④ 顺带查截图有没有落盘:③ 已经把两台前台都拉成真 App,这时还没截图就不是老原因了

### 2026-08-20 · 并行①②在用户 mac 上通过;脚本剩下的红是脚本自己的错
用户重跑:**端口 8149 / 8197 分开、两边 PID 都对上**——端口修复与
`SIMCTL_CHILD_USE_PORT` 都确认生效（那个预编译 runner 认这个变量）。
剩下两处红是脚本的毛病:
- **fix 报错行是空的**:`>/dev/null 2>"$err"` 只留了 stderr,而 **tke 的错误走 stdout**
  （`{"success":false,"error":…}`）。改成 `>file 2>&1`,报错原样打出来
- **③ 改成直接 curl WDA 的 `/screenshot`**,绕开 tke 的会话逻辑。要验的是
  「这个端口背后是哪台设备」,不该被"前台有没有 App""会话建没建起来"干扰——
  上一版走 `tke refresh`,两边都"没截到图",而那跟并行一点关系都没有。
  PNG 的宽高就在头 24 字节的 IHDR 里,python 三行读出来,连 sips 都不用
- **④ 变成 tke 层的并发验证**（两条 steps 同时跑、各出各的报告）,失败时把完整输出打出来

### 2026-08-20 · 并行修复的两个真 bug（用户实测逼出来的）
用户在 mac 上跑 `verify-sim-parallel.sh`,两台**都停在 8100**、归属校验两边都报 `?`。
脚本按设计报了红——但红的原因跟"并行"无关,是修复本身有两个洞:
- **fix 端口不再从状态文件继承**。旧版留下的状态文件里两台都写着 8100,
  「先用上次那个」于是把历史包袱一路带下去。现在端口**只认 UDID 算出来的那个**,
  被占就往后挪。另外 **8100 一律不复用**:那是 WDA 的出厂默认,任何一台没带 USE_PORT
  起来的都在那儿,认它等于认了个公共端口。`sim_port` 也改从 8101 起,有意跳过它
- **fix 归属校验查不到 PID**。iOS 里 App 在 launchd 的 label 是
  `UIKitApplication:com.facebook.…[0x…][rb-legacy]`,拿 bundle id **精确查永远查不到**
  ——于是校验一路放行,第二台**直接复用了第一台的 WDA**,正是要防的误连。
  改成列全表按子串找
- **新增诊断**:指定端口不通而默认的 8100 通 → 直说"这个 runner 不认
  SIMCTL_CHILD_USE_PORT"。不说清楚,人会去查锁屏、防火墙、WDA 版本——全是别的地方
- **脚本三处自省**:①产物比 `src/` 最后一次提交旧就直接拦下（P-42 的加强版:
  用了构建产物也可能是上一轮编的）②拉起 WDA / refresh 的报错不再 `2>/dev/null` 吞掉
  （INV-9:早先后面三步全红,真正原因被吞在第一行里）③PID 取法同步改成子串匹配

### 2026-08-20 · 上色改成例外而非常态;`tke device --all` 顶层也认
用户:「不要绿色泛滥」。一张体检表大多数时候通篇都是好的,每行都染绿等于没染——
眼睛扫过去,真正要人动手的那一两行反而淹了。
- **正常状态一律不上色**（就绪/可用/已是最新,以及平台、路径这类纯事实）
- **只有要人做点什么的行才有颜色**:`有可用更新` 绿、缺依赖红、
  查不了/不支持灰。`Tone::Ok`/`Tone::Warn` 两档随之删掉
- Web 缺 chromedriver 那行从灰改红——它跟「依赖 缺 N 项」是同一件事,该同一个颜色
- **fix** `tke device --all` 顶层也认（等价 `device list --all`）。
  `device` 既然等于 `device list`,那省下的那个词就不该反过来变成一条要记的规矩

### 2026-08-20 · doctor 的命令全收进「下一步」;并行脚本改成你指定两台
用户:「像 `tke doctor --fix` 这种建议性的一行指令,可以集中放在最后一行输出,
别插进中间位置了,太散乱」。哪行发现问题就在哪行缀一句命令,看着贴心,
实际是把一份体检表撒成了几段说明书。
- **正文里一条命令都不留**:设备行的原因简化为「缺少依赖」（缺哪个段一已经列了）,
  iOS 模拟器那行只说「缺 WebDriverAgent」,结论行不再缀「补齐：…」
- **结论区只报状态**(环境不完整 / PATH 没写进 shell / 有可用更新),
  该敲的命令统一收进最后的 **「下一步」** 块,每条 `命令  说明` 对齐成一列
- 那条八十多字符的 `export PATH` **自成两行**（说明在上、命令缩进在下）——
  硬排进同一列会把其余几行的说明推到屏幕外,为一条长命令破掉整块队形不值
- **`verify-sim-parallel.sh` 改成 `<UDID-A> <UDID-B>` 由你指定**:
  哪两台该拿来验、哪台上装着要看的 App,脚本不知道,猜错了还得从头再跑一遍。
  不给参数就列出这台机器上的模拟器（●已启动 / ○关着）让人挑;
  给的两台**型号相同会先警告**——第③步靠分辨率差异判串台,同型号是歧义结果

### 2026-08-20 · doctor 设备段按平台成对 + 双模拟器并行的自查脚本
- **顺序**:Android 真机/模拟器挨着,iOS 真机/模拟器挨着。人是按平台找这张表的
  （"我安卓那边怎么样"）,不是"先看所有真机再看所有模拟器"
- **新增** `scripts/verify-sim-parallel.sh`:验 Q-13 那个修复到底成没成。
  验的不是"能不能同时跑",是**命令有没有发到该去的那台**——两台都在跑、两条命令都报成功、
  页面也都动了,但动的可能是同一台。四步:①两台端口不同 ②每个端口的监听 PID = 那台
  WDA 的 PID ③**并发 refresh,两边分辨率各是各的** ④证据各落各的目录
- 写脚本时撞出一个坑并写进注释:**`device info` 验不了串台**——模拟器的机型/系统 tke 是
  问 simctl 拿的,压根不经过 WDA,两台串了台它照样报得对。要挑一条**必须走那个端口**的路

### 2026-08-20 · doctor 上色 + 砍掉解释性补充
用户看完第一版的评价:「有些信息该不放就不放,别乱七八糟信息都堆在一起,
**重要的是简略且保持队形**」。补充说明写得越"贴心",越是在替读者做他没要的功课。
- **砍掉**四处解释:`· tke 不负责启动`（Android 模拟器）、`· Simulator + WebDriverAgent`
  （iOS 模拟器已启动时）、`(浏览器默认仍跑无头)`、`(本机无图形界面)`（显示器环境这行
  现在只剩「有头」/「无头」两个字）
- 措辞改成状态而不是判词:`没有已连接的设备`→`尚未连接设备`,
  `没有已启动的 AVD`→`尚未启动 AVD`,`没有可用的模拟器`→`尚未创建模拟器`
- **加语气色**（`Tone`）:绿=就绪/可用/已是最新,**黄=有更新、装了但用不了**,
  **红=缺依赖**,灰=没有/查不了/不支持。颜色只分轻重,跟结论行的 ✓/!/✗ 同一套
- **队形靠三条固定**:标签一律 dim、值按语气上色、补充永远 dim。
  一行里两段亮色就分不出哪个是状态了,所以补充不跟着值一起染

### 2026-08-20 · `tke doctor` 重排：三段分组，一眼扫完
用户的原话是「之前那个什么信息都有，信息太多而且顺序混乱看起来很难受」。
问题不在少说了什么,在于**没有分组**——版本、设备、路径、提示按"想到什么加一行"的
顺序混着打,每次都要重读一遍才知道自己关心的那行在哪。
- **三段固定顺序**,每段回答一个问题:①这套工具本身（平台/依赖/Engine 版本/Skill 版本）
  ②能测什么（四端 + 显示器环境,**真机在前、模拟器在后**）③东西落在哪（Engine/Skill/日志）
- 标签列按**显示宽度**对齐（中文占两格,`{:<15}` 按字符数填必然错位）,三段共用同一列宽
- 值的格式统一成 `状态 (补充)`,补充走 dim;**用不了的整行置灰**,一眼看出哪条不可用
- 设备探测改走 `tools::discover`（`tke device list` 同一套）——同一个问题不该有两套答案。
  安卓真机与安卓模拟器靠 `emulator-` 前缀分开:「没插手机」和「没开模拟器」下一步不同
- 依赖行现在说得出分母（「7 项已就绪」/「缺 2 项 / 共 3 项」),缺的明细缩进列在它下面
- Web 不可用时**缺的几样一次说全**,不再只报第一个
- **refactor** 体检排版拆到 `cli/doctor.rs`（`fix.rs` 已 811 行超守卫阈值,只留检测与下载）

### 2026-08-20 · 模拟器 WDA 端口一台一个（关闭 Q-13）
模拟器与主机共享网络,WebDriverAgent 默认全都监听 8100——并行跑两台就会互相抢。
抢输的那台起不来还算好的,**更糟的是端口通、命令却发到了另一台设备上**:
每步都报成功,动的是别人（P-35 那一族的老毛病）。
- **fix** 每台按 UDID 定端口（`8100 + hash % 100`,**稳定可复现**——排查时 `lsof` 对得上号）,
  记进它自己的状态文件;启动时 `SIMCTL_CHILD_USE_PORT` 传给 runner
- **`--terminate-running-process`**:不杀掉已在跑的那个,launch 只是把它带到前台,
  USE_PORT 根本不生效——端口就还是撞的
- **复用前核对端口归属**:`simctl spawn <udid> launchctl list` 的 PID 与 `lsof` 的监听 PID
  必须是同一个（模拟器里的进程就是 macOS 进程）。两边有一边问不出来就放行——退回
  "端口通就算数",跟改之前一样,不会更差
- 结果按 UDID 缓存在进程内:`ensure_forward` 一步里会被调好几次,归属校验要跑两个子进程
- 三个单测钉住端口的稳定性/分散度/范围。**mac 上多台模拟器并行待真机验**

### 2026-08-20 · `--summary -` 从标准输入读：长结论不用再写临时文件
用户看到 AI 写长结论时**先落一个 /tmp/summary.md、再 `--summary-file` 指过来**,
问能不能一步到位。**那个绕路是文档教的**——`--summary-file` 的帮助里写着
「多行 Markdown 塞进命令行要跟引号搏斗,先写成文件再指过来省事得多」。
真正的问题不是"要不要文件",是**长文本进命令行难**。
- **feat** `--summary -` / `--task -` 从**标准输入**读,配 heredoc 一步到位:
  `tke report DIR --verdict pass --summary - <<'EOF' … EOF`
  （PowerShell 用 here-string）。heredoc 天然处理引号与换行
- **两个都写 `-` 直接拦下**:标准输入只能读一次,第二个会拿到空串——
  那是最难查的那种"成功"
- `--summary-file` 保留（已经有现成 .md 时用），帮助文案不再引导人去写临时文件
- **docs(skill)** 交付那节的示例整个换成 heredoc 一步式

### 2026-08-20 · 跨端检查「只用一个 `--log`」写成硬规矩
用户那次双端检查交付了**两个报告链接**（cache-mgmt + cache-mgmt-ios）,
而结论本身是合并写的（"两端功能完整可用"）——AI 做完 A 端把 B 端当成了新任务。
- **docs(skill)** 跨设备那节开头加一条硬规矩 + 正误对照:
  **从第一步到最后一步只用一个 `--log`,换设备不换目录**。
  一份报告才还原得出「在 A 上做了什么 → B 上看到了什么」这条因果链,
  而那正是跨端检查唯一要证明的东西
- 收尾那节里「证据按设备分目录」的旧说法改掉（早就不分了,文档没跟上）
- 踩坑册 C-17 补一句:**最常见的那次就是跨端检查**

### 2026-08-20 · iOS 密码明文进报告（P-45）+ 报告图片与读图优先级
用户传来的双端报告里:`输入 ["Password", "TempTest001"]` —— **注释写着"tke 会自动打码",
它没打**。
- **fix(P-45,安全)** XCUI 归一化**从来没输出过 `password` 属性**,于是 iOS 上密码一路明文
  写进 log.json / 报告 / **截图顶部横幅**。安卓原生有、web 归一化时对齐了,唯独 iOS 漏了
  ——而 target_resolver 的注释还写着「三个平台同一条路」。两个单测钉住
- **fix(报告)** 手机竖屏截图（1080×2412）按 `max-width:100%` 铺开有两三屏高,一步都看不完整。
  限到 **56vh**,点击**就地展开**成原始尺寸（纯 CSS,不引 JS——报告要能离线/转 PDF 看）
- **fix(报告)** 点图片不再**跳转**到原图（会把人从报告里带走）。原图 / 元素表 /
  **原始页面**三个链接放在图片下面那行；原始页面按前缀扫（扩展名随驱动而异,P-43）
- **fix(报告)** 顶部「设备」栏还在显示 `sim:92AA…`,而下面的分隔行早就用友好名了
  ——同一份报告里两套叫法
- **docs(skill)** 读图优先级重排：**元素表 → OCR → 读图**,上一级答得了就别用下一级。
  早先「必须读图」那段列了四条,条件宽到覆盖所有排查场景,还写着"省 token 省到不敢看结果
  是本末倒置"——**过纠正了**（那是为治 C-11「从不读图」加的）。现在明确列出
  「用元素表别读图」的四种情形（点了没反应先看 `errors`、页面变没变比两次 fetch…）

### 2026-08-20 · 有头/无头统一到一个开关
用户:「为啥有的用 --headed 有的用 --headless,统一一下」。
- **删掉 `control boot --headed`**。全局的 `--headless=<auto|on|off>` **可以写在子命令
  后面**（实测 `control boot --headless=on -d web` 就能跑），所以那个开关纯属多余——
  同一件事两个写法，用的人得先想用哪个
- tks 脚本里仍是中文参数 `启动环境 [有头]`（脚本里写 `--headless=off` 很怪），
  但走的是同一条实现
- **fix** 要开窗口而机器没有图形界面时，chromedriver 只回一句
  「Chrome instance exited. Examine ChromeDriver verbose log」——人得去翻日志才知道
  是没有 DISPLAY。改成**建会话前就拦下**并说清楚该怎么办

### 2026-08-20 · `boot` / `shutdown`：环境的起停从「魔法」变成显式一步
用户提出:浏览器现在是**第一条 web 命令顺带起来的**,脚本里看不出它什么时候起的、
以什么模式起的,AI 也无从判断"现在有没有环境";而且 `close` 语义重载
（空参=销毁会话、带参=杀 App）。拆开:
- **feat** `control boot [--headed]` / `control shutdown`;tks 指令
  `启动环境` / `启动环境 [有头]` / `关闭环境`
- **boot 管环境本身,launch 管环境里面的东西**——一条指令只做一件事,读脚本的人不用猜
- iOS 模拟器:`simctl boot` + 显示窗口 + **`bootstatus -b` 等到真的可用**
  （boot 命令返回时系统还在起,紧接着 install 会失败）;幂等（已开着不算错）
- shutdown 前**先清会话文件**:下次 boot 起来的是新系统,旧 session_id 必然失效,
  留着只会让下一条命令先撞一次"会话已死"
- **安卓模拟器明说不支持**,不假装:要 Android SDK 的 emulator 二进制 + AVD 名
- `close` 的帮助里指路到 `shutdown`;**空参行为保留**（很多脚本在用）
- **fix(P-44)** `--headless` 撞全局同名参数 → **运行时 panic**（不是编译错）。
  同一个坑第二次（上次是 `browser reset --cache`）。正解是删掉它:无头本来就是默认
- **fix** `set_web_headless` 是 `OnceLock`,main 设过之后再设**被静默丢弃**——
  `--headed` 一点作用都没有。换成 Atomic

### 2026-08-20 · 砍掉 CLI 里的教学式文案
用户:「这种解释性文案应该都移除,CLI 本来就是信息越精简越好」。
- **删** `device list` 末尾的「第一列就是 -d 要填的值」——表头已经写着 ID
- **压短** skipped 的措辞:形如「安卓未检测 · 缺 adb · tke doctor --fix」,
  **事实 + 下一步,不解释为什么**。「没查」与「没连」的区别靠"未检测"三个字带出来就够了
  （INV-9 要的是这个区别看得见,不是要展开成一句话）
- **同类一并砍**:`iOS 不支持 · 需 macOS（设备端 WDA 依赖 Xcode）` 去掉括号;
  「新终端 找不到 tke · 只有当前窗口能用（PATH 没写进 shell 配置）」压成
  「找不到 tke · PATH 没写进 shell 配置」;iOS模拟器缺 WDA 那两行并成一行;
  「更新后重新读一遍 SKILL.md——你上下文里那份还是旧的」压成「更新后重读 SKILL.md」
  （P-41 的理由写在 SKILL.md 里,这一行是提醒不是教程）

### 2026-08-19 · `tke device` 重做：四列对齐 + 平台配色 + 友好输出
用户逐条提的,全做了:
- **`tke device` 不带子命令 = `list`**——问"有哪些设备"是最常见的那次,不该多打一个词
- **四列**（ID / 系统 / 型号 / 状态）**全部左对齐**。原先型号与版本挤在一列,
  `CPH2305` 和 `iPhone 17 Pro · iOS 26.2` 长度差太多,右边整个错开
- **按显示宽度对齐**（新增 `utils/text.rs`）:`{:<w$}` 是按**字符数**填的,
  而中文占两格——这是所有中英混排表格错位的根因,3 个单测钉住
- **平台配色**:安卓绿 / 苹果蓝 / 网页黄。**用不了的整行置灰**（没启动的模拟器、
  离线的安卓）——别让人去选一个选了必然失败的
- **浏览器列两行**（无头 / 有窗口），第一列连参数一起给（`web --headless=off`），
  复制就能用；**本机开不了窗口时只列无头那行**（同 iOS 的门禁：做不到的选项不摆出来）
- **`device info` / `prop` 默认给人看的排版**,`--json` 或管道才给 JSON。
  `prop` 在终端里**只打值**——多半是要拿去接着用的,裹一层 JSON 只是碍事
- **fix** `device prop <不存在的属性>` 报 `{"success":true,"value":""}`——
  `adb getprop` 对不存在的键回空行、退出码还是 0,于是"查不到"长得像"值是空"。
  改成明确报错并给出属性名的样子（用户拿包名当属性名查时撞到）
- `info` 的 Option 字段**查不到就整行不打**,别拿 0 占位（"0 核"、"电池 0%" 比不显示更误导）

### 2026-08-19 · `tke device` 三处体验问题（用户实机反馈）
- **fix(刷屏)** `device list` 在装了 Xcode 的 mac 上列出 **24 台模拟器**,只有 1 台在跑。
  改成**默认只列 Booted 的**,另加一句「另有 N 台没启动（--all 看全部）」;
  **一台在跑的都没有时反而全列**——否则空列表会被读成"这台机器不支持模拟器"
- **fix(不该要 WDA)** `device info -d sim:…` 回一句「无活动 WDA 会话，请先执行
  启动 [BundleID]」——而用户只是想看看这台是什么。改成模拟器走 simctl:
  机型/系统版本 `simctl list` 直接有,屏幕尺寸**截一张图量**(截图不需要 WDA),
  **全程不碰 WDA**
- **fix(id 空着)** 不带 `-d` 的 `device info` 返回 `"id": ""`。adb 会自己挑唯一那台,
  但结果里不说是谁——多设备时尤其要命。改成查一次 `adb devices` 补上

### 2026-08-19 · `tke device` 的帮助文案跟上四端
`--help` 里还写着老的「[工具] 设备详细信息」,看不出 `list` 的存在。
- 顶层改成「list 看有哪些能测 / info 看某台的详情 / prop 读安卓属性」
- `info` 写明**四端都能用**、`prop` 写明**仅安卓**（它就是 `adb getprop`）
- skill 的 tke-commands 里三条并成一处（原先 info/prop 各写了两遍）

### 2026-08-19 · `install.sh` 也装 WebDriverAgent（两条路装出来的环境要一样）
用户看 `tke update` 的输出:DEPENDENCY 段里有 tke/chromedriver/adb/aapt/go-ios/chrome,
**唯独没有 WDA**——因为它只有 `tke doctor --fix --profile ios` 那条路装得上。
两条路装出来的环境不一样,人只会以为是漏了。
- **feat** `install.sh` 加 3.5 段:macOS + ios/all profile 时下 WDA runner 到 `~/.tke/wda/`,
  跟 chromedriver/Chrome 同级显示。解压后**验 `.app` 真的在**(半个解压出来的目录也是目录),
  并 `xattr -cr` 清隔离属性——不清的话 `simctl install` 会被拦
- **feat** `uninstall.sh` 跟着删 `~/.tke/wda`:跟 tke 一起装的就跟 tke 一起删,
  留着既占地方又会让下次 doctor 误判成"已装"
- `tke uninstall` 的确认清单也列上它,**但只在装了的时候列**——没装的东西列出来是噪音

### 2026-08-19 · 设备显示改成给人看的名字；device 命令补齐四端
- **删** 「正在把 WebDriverAgent 拉进模拟器」那行提示。正常流程里 `启动 [BundleID]`
  紧接着就把 App 拉回来了,那行字对用户没有任何可做的事;真出问题时 attach_foreground
  会报得很清楚
- **feat** `Controller::describe()`:四种驱动都回答「我是谁」——
  `iPhone 17 Pro · iOS 26.0（模拟器）` / `Pixel 7（安卓 14）` / `Chrome（无头）`。
  执行时问一次(要跑 adb/simctl 子进程,每步问就是每步多花几十毫秒),存进
  `ExecutionResult.device_label`,**报告显示它而不是那串 UUID**
- **fix(单测抓到的真 bug)** 换设备的判断**必须用设备 ID,不能用显示名**:
  两台同型号模拟器的 label 一模一样,混用会把跨设备那一跳吞掉——而那正是跨设备检查
  最需要在报告里还原的东西。显示用 label、判断用 id,测试夹具特意把 label 设成全一样
- **fix** `tke device info` 对 **fake 设备**报「缺少 adb」:`fake:` 在 Platform 上算
  Android(没有 Fake 平台),直接按平台判会让它去连 adb。测试设备要能离线跑才有意义
- **fix(INV-9)** `device info` 无浏览器会话时返回 `Chrome for Testing · 0×0`——
  兜底成 0 看着像个合法回答,实际意思是"根本没有会话"。改成报错并指路(同 P-27)
- `device list` 的类别换成中文(安卓 / iOS真机 / iOS模拟器 / 浏览器),
  `ios-sim` 那种是内部叫法不该摆到人面前;中文列按**显示宽度**补空格(一个汉字两格),
  交给 `{:<w$}` 会歪

### 2026-08-19 · iOS 模拟器**端到端跑通**；修三个体验问题
用户走完全流程:`doctor --fix --profile ios` 从分发源装 WDA → `启动 [BundleID]` →
`点击 ["Skip sign in (demo)"]` → `fetch` → `report --open`,**全绿**。
元素表质量很好(resource_id/xpath 都有,坐标是截图像素)。同一趟暴露三处:
- **fix** `-d sim:`(shell 变量没展开)会一路传到 simctl,回一句光秃秃的
  `Invalid device:`——没人看得出是自己的 `$UDID` 空了。改成在驱动构造时拦下并讲人话
- **fix** 「正在把 WebDriverAgent 拉进模拟器」那行**打了 4 次**:ensure_forward 在一步里
  会被调好几次(launch_app → ensure_create → ensure_existing → …)。加 `TRIED` 标志,
  **一次运行只试一次**——顺带解决"失败时一遍遍等满 15 秒超时"
- 报告里设备栏出现 `sim: · sim:xxx`(两批设备不同)是**正确行为**(跨设备要标出来),
  修掉空 UDID 之后自然就没了

### 2026-08-19 · `$VAR中文` 那个坑又犯了一次 → 加守卫拦死
打包脚本第一行就崩:`step "① 取源码（锁定 $WDA_REF）"` → `WDA_REF�: unbound variable`。
**P-42 刚记完半小时就又犯**——因为 Linux 的 bash 5 两种写法都对,本机永远测不出来。
- **fix** 改成 `${WDA_REF}`;全仓扫了一遍,只有这一处（install.sh 那处在注释里,无害）
- **feat(守卫)** `scripts/check-shell-vars.sh` 扫所有 `*.sh` 里 `$VAR` 紧跟多字节字符的写法,
  跳过注释行;进 **pre-commit 拦死**。造了反例验证它真能抓到
- **这类「本机测不出来」的坑,光写进坑册没用,得让工具替人记**(同 ADR-0010:护栏进工具)

### 2026-08-19 · 模拟器收尾：打包脚本 + skill 用法
- **feat** `scripts/package-wda-sim.sh`:编译 → 打包 → **自检 zip 结构** → 打出上传命令。
  **WDA 版本锁在脚本里**(`WDA_REF`,默认 2026-08-19 实测通过的那个 commit)——
  这正是自己分发的意义:上游变了不会突然把用户环境搞坏。顺带出一份 `WDA-VERSION` 留痕
- 自检那步专门查「zip 里第一层是不是 `WebDriverAgentRunner-Runner.app`」:
  层级错一层 tke 就找不到,而那是解压之后才会暴露的
- **docs(skill)** 补「iOS 模拟器」一节,**重点写第一次要先 `启动 [BundleID]`**:
  拉 WDA 会挤掉前台 App,直接 `fetch` 采到的是桌面那一屏(tke 会认出来并报错);
  以及 iOS 包名怎么查(`simctl listapps`,`tke app` 是安卓专属)
- **docs** 发布布局补 `wda/` 一层;写明那份 `.app` 是 fat 包,**Intel 与 Apple Silicon 共用**

### 2026-08-19 · 附着到「桌面」也得报错（同一个坑的第二个变种）
用户实跑:模拟器上还没装 WDA 时跑验证——tke 把 WDA 拉起来(挤掉了前台 App),
然后附着到了**桌面**,采到一堆 Fitness/通讯录/文件 图标,接着"找不到那个按钮"
卡满 20 秒超时。
- **fix** 上一条只判了「附到 WDA 自己」,**没判「附到桌面」**（`com.apple.springboard`）。
  两种都附得上、`/status` 也正常,采到的却是一屏跟被测功能毫无关系的东西。
  实测第二种更常见
- **feat** 拉起 WDA 时先打一行:「会暂时挤掉前台 App」——不然人只看到"我的 App 怎么没了"
- **fix(验证脚本)** 给了 bundle-id 时:先触发一次 fetch 让 WDA 拉起来(顺带挤掉),
  **再**把 App 拉回前台;②之后加一道桌面识别,别让人对着一屏系统图标纳闷

### 2026-08-19 · iOS：没有会话就附着当前前台 App
用户实跑:模拟器上 WDA 拉起来了(`/status` 通),`fetch` 却报「无活动 WDA 会话」。
- **feat** `ensure_existing` 找不到会话时,**附着当前前台 App** 建一个(不带 bundleId)。
  「App 已经开着,我只想看看这一页」是最常见的诉求,不该逼人先 `启动 [BundleID]`
  ——那会重启 App、把要看的现场毁掉
- **INV-9** 附上之后**要确认附到了谁**:模拟器第一次拉起 WDA runner 时它自己会被带到
  前台、把用户的 App 挤到后台,这时附着成功、`/status` 也正常,但采到的是 WDA 那个
  空白测试界面——不检查就是一份「页面上什么都没有」的假结论。附到 WDA 自己身上时
  直接报错并指路

### 2026-08-19 · iOS 模拟器改走**预编译 WebDriverAgent**（ADR-0017 修订）
用户提出「锁定 idb 版本、自己当客户端」,理由是 brew 上的版本我们控制不了。
这个前提一立,账就反过来了——**同样是「自己分发 + 锁版本」,WDA 全面胜出**:
协议 HTTP+JSON(客户端代码真机那套现成)、归一化现成、分发物 21MB(idb_companion 是 77MB)。
- **实测(`scripts/probe-wda-prebuilt.sh`)**:`.xctestrun` 里**没有本机绝对路径**→可分发;
  **`simctl launch` 直接就起得来**→连 xcodebuild 和 .xctestrun 都不用带,`/status` 回 WDA 16.3.0
- **feat** 模拟器连接时若 8100 不通,tke 自己 `simctl install` + `simctl launch` 拉起来并等就绪
- **feat** `tke doctor --fix --profile ios` 下载预编译 runner 到 `~/.tke/wda/`;
  `TKE_WDA_APP` 可顶掉(自己编译的、或试别的版本)
- **删掉 idb 驱动与 AX 归一化**:不留两套——那意味着两份归一化、两条调试路径
- 已知限制:端口写死 8100,多台模拟器同时跑会撞(要并行得传 USE_PORT)
- ⚠️ **分发源上还没有那个 zip**:传上去之前,用 `TKE_WDA_APP` 指向自己编译的产物
  (verify-ios-sim.sh 会自动找 /tmp/wda-build 里那份)

### 2026-08-19 · `tke doctor --fix --profile ios` 替你把 idb 装上
用户提出「idb 也该走 tke 的 fix/doctor,不该让人自己 brew」。查了 `idb_companion --help`:
它**没有任何 UI 操作参数**(grep tap/touch/describe/accessibility/hid 一条不匹配),
只管 boot/erase/create 和**起 gRPC 服务**——点击与元素采集全在 Python 前端那一半。
所以单独分发那个二进制没用。
- **feat** `--fix` 时替用户跑 `brew tap/trust/install` + `pip install --user fb-idb`,
  每条**先打出来再执行**(在别人机器上装东西不该是黑箱);复验以「现在装上了没有」为准,
  不以「命令有没有报错」为准
- **失败不改退出码**:doctor 的退出码说的是「必需依赖齐不齐」,而 idb 只影响 iOS *模拟器*
  ——安卓/网页/iOS 真机都不靠它。但必须打出来(INV-9),否则用户会以为模拟器能用了
- `idb_present()` **两半都要有**:`idb_companion`(服务端)+`idb`(客户端),
  光有 companion 一个动作都做不了

### 2026-08-19 · iOS 模拟器链路**真机验通**；顺手修掉产物静默少一份（P-43）
用户在 mac 上跑通了 `verify-ios-sim.sh`:模拟器列得出、7 个元素采得到、
**坐标是截图像素量级(366pt×3=1098)**、`点击 ["Skip sign in (demo)"]` 页面真的变了
——scale 换算、语义定位、AX 归一化、证据落盘全部成立。ADR-0017 的模拟器路线到此闭环。
- **fix(P-43)** 唯一没过的一项:`raw_pages/` 空着。收集那段写的是
  `for ext in ["html","xml"]` 白名单,而模拟器的 AX 原文是 `.json`——**不在名单里
  就当它不存在**,不报错不提示。改成按 `current_raw_page.` **前缀扫**,
  驱动用什么后缀是驱动的事,收集方不该维护一份清单
- 两个单测钉住,夹具里特意放了个叫「以后某个新驱动的后缀」的扩展名
- 更普遍的那条也记进坑册:**凡是「加了新东西要记得同步改另一处」的地方都在等着漏**,
  能靠约定自动发现的就别写清单——清单会漏,而且漏了是静默的(同族 P-31)

### 2026-08-19 · 构建完不同步 → 敲的还是旧 tke（P-42）
用户实跑撞到:`build-mac.sh` 报 Build successfully,紧接着 `tke device list` 说
"unrecognized subcommand",`-d sim:` 被当成安卓序列号去连 adb。**编译的确实是新代码**
(警告里都能看到新文件名),只是敲的 `tke` 是另一个文件——构建产物落在仓库
`bin/<platform>/`,而 PATH 里是安装器装的 `~/.tke/bin/tke`。
- **fix(验证脚本用产物路径)** `verify-ios-sim.sh` 自己算出 `<repo>/bin/<platform>/tke` 再调,
  不用 `tke` 这个名字——验的必须是"刚改的这份代码",不能是碰巧在 PATH 里的那个
- **fix(构建脚本只提示不覆盖)** `build-mac.sh` / `build-linux.sh` 打一行
  「你敲的 tke 不是刚构建的这个」。**先写成了自动同步过去,被用户当场拦下**:
  `command -v tke` 那个是用户日常在用的,构建脚本没资格替他换掉
- **fix(验证脚本)** `verify-ios-sim.sh` 把 `{"success":false,…}` 这种错误对象也当成
  "采到元素了",然后在下一行 `[:6]` 炸出 KeyError——报错报在无关的地方。改成必须是非空数组
- **fix(脚本自身的两个坑,都在 P-42 里记了)** ①`$BASH_SOURCE` 是相对路径,
  `cd` 走之后就指不到——先解析成绝对路径再 cd；②`"$VAR（中文）"` 在 macOS 自带的
  bash 3.2 下会把全角括号的字节吃进变量名(路径变空、括号烂掉),中文文案里一律写 `${VAR}`。
  **Linux 的 bash 5 都不复现,恰恰因此最容易漏**
- 清掉 6 条 `whitespace symbol '\u{3000}' is not skipped` 警告(字符串续行后的全角空格
  不会被跳过,改用 `\u{3000}` 显式写)

### 2026-08-19 · 把「四种设备」这件事查漏补齐
用户问 fetch/refresh 是不是也同步到四种设备了。**采集/操作是通的**——它们全走
`Controller` 的穷尽匹配,漏一个编译都过不去。但**按 Platform 分叉或 `_ =>` 兜底的地方
会静默漏**,查出两个真缺口:
- **fix(harness 向导)** 早先是自己 adb 列一遍、iOS **让人手打 UDID**、模拟器压根不出现。
  改成一律走 `tools::discover`——跟 `tke device list` 同一个来源。同一个问题不该有两套答案,
  手打 UDID 这道门槛本身也没必要。没查成的那几类**也摆进选项**(INV-9):
  不然人只会以为"设备没连",而真相是"没装 adb / 没装 idb"
- **fix(doctor)** iOS 那栏只查 go-ios(真机),模拟器要的 idb 没人查。补一行状态;
  **不进"缺 N 项"**——idb 是 brew 装的,不归 `--fix` 管,把补不了的东西算进缺失
  只会让结论变成死路
- flow 收尾那句"iOS 关完销毁 WDA 会话"的注释补上模拟器的情形(空操作,无害)
- **docs** skill 的 tke-commands 写明 fetch/refresh **四种设备同一套**:
  换个 `-d` 就行,输出格式、坐标口径、`--interactive`/`--ocr`/`--wait-text` 全一样,
  底层(uiautomator / XCUI / AX 树 / DOM)已经归一化掉了

### 2026-08-19 · iOS 模拟器改走 idb（ADR-0017）
上一条(5dd97961)让模拟器也走 WDA,但那要求用户自己编译一个 WDA 的 Xcode 工程。
看到用户另一个会话的实跑记录后改了路线:**模拟器走 idb,真机继续 WDA**。
判据只有一件事——设备上要不要跑一个**签名过的** runner:真机两边都要(换不来好处),
模拟器 idb 完全不要(`idb_companion` 直调 CoreSimulator)。
- **feat** 新增 `Driver::IosSim`(`drivers/idb/`):点击/长按/滑动/输入/清空/按键/返回/
  主页/启动/关闭/采集全部落到 `idb ui *` 与 `xcrun simctl`
- **feat** AX 树 → uiautomator XML 归一化(约 80 行)。**拿用户实采的真实数据当夹具**,
  4 个单测:文字提取、坐标换算、密码框识别、以及「**traits 不能判可点击**」
  ——实测 `Scrollable` 连 StaticText 都带,一律当可点会把满屏文字标成能点的
- 坐标 scale = **截图宽 ÷ AXApplication 宽**(实测 1206÷402=3),自己算得出来,
  不用像 WDA 那样再问一次接口。换算在驱动层做完——让调用方 AI 自己乘 dpr 是把工具的活
  推给它,迟早算错
- `subrole: AXSecureTextField` 直接对上 `is_password`,证据打码无缝接上
- `has_soft_keyboard` **不含模拟器**(有意):它用电脑键盘、软键盘不弹,
  而 `idb ui text` 本来也不依赖键盘——白等一次就是白等几百毫秒 × 每个输入框
- `tke device list` 在「列得出模拟器但没装 idb」时明说**列得出来但操作不了**,
  并给出 brew 那三条命令
- **docs** platform-matrix 扩成四列(安卓/iOS真机/iOS模拟器/网页)+ 两条接入路对照;
  skill 的 steps-syntax 同步
- ⚠️ 本机 Linux,**模拟器实跑验不了**:归一化(有真实夹具)和平台路由已验,
  点击/采集要在 mac 上跑一次

### 2026-08-19 · iOS 模拟器接入（`-d sim:<udid>`，tke 侧已通,待 mac 实测）
- **feat** `-d sim:<udid>` 识别为 iOS 并走**模拟器路径**:模拟器与主机共享网络,
  WDA 就在 `127.0.0.1:8100`,不建隧道、不做端口转发。**WDA 协议那一整套(点击/采集/
  XCUI 归一化/截图)原封不动复用**——真机和模拟器只有"怎么连上"这一步不同
- 必须靠 `sim:` 前缀识别:模拟器 UDID 是标准 UUID(36 位),真机是 25 位,
  裸 UUID 会被一路认成**安卓序列号**然后拿 adb 去连(加了两个单测钉住)
- 连不上时**说清楚该怎么办**:tke 拉不起模拟器里的 WDA(go-ios 只对真机有效,
  模拟器没有 lockdown 通道),报错里直接给 xcodebuild 那条命令
- **docs** platform-matrix 加「iOS:真机与模拟器是两条接入路」对照表,写明
  为什么非 WDA 不可(simctl 点不了也读不到元素树)、idb 为什么不划算、
  以及 **tke 不需要内置 WDA 源码**(它只说 WDA 的 HTTP 协议)
- ⚠️ 本机是 Linux,**模拟器这条路我验不了**:能验到的只有平台路由和报错文案

### 2026-08-19 · `tke device list`：一条命令看清这台机器能测什么
- **feat** 新增 `tke device list`,统一列出安卓 / iOS 真机 / **iOS 模拟器** / 浏览器,
  第一列就是 `-d` 要填的值。管道里给 JSON,终端里给对齐的表
- **INV-9** 某一类查不了会**单独说明原因**（"没装 adb —— 是没查,不是没连"）:
  空列表和"没连设备"长得一模一样,不说清楚人只会去插拔数据线。skipped 单独成段,
  不混进设备表（混进去会被读成"这些是设备"）
- 模拟器条目带 `sim:` 前缀:模拟器 UDID 是标准 UUID(36 位),而 tke 认 iOS 靠的是
  真机 UDID 的形状(25 位),不加前缀会被当成安卓序列号
- **docs** skill 里 `adb devices` 全部换成 `tke device list`——删掉 CLI 直通(ADR-0016)后
  那条指引依赖"用户自己 PATH 里有 adb",不该这么假设
- 排版:只对 ASCII 的前两列做对齐,中文靠后不参与列宽（`{:<w$}` 按字符数填充,
  中文占两格,混进来必然错位——TUI 那次的老账）

### 2026-08-18 · skill 过期就自己更新，**并且重读自己**
用户实跑看到:doctor 报 skill 过期,AI 停下来问「要我跑 tke update 吗」——
问的是一件不用问的事,而且**跑完它手上那份文档还是旧的**。
- **docs(skill)** SKILL.md 改成:看到过期**直接更新**(幂等、十几秒、已装依赖跳过),
  更新完 **`cat` 一遍自己**。加了唯一要先看一眼的例外:依赖缺 Chrome 时先跟用户说
- **feat(P-41)** 同一句话**由 tke 自己说**:`hint()` 在 skill 过期时缀「更新后重读
  SKILL.md」,doctor 里多打一行完整版。只写在 SKILL.md 里没用——**手上文档旧的那个 AI,
  恰恰就是看不到新指示的那个**(ADR-0010:护栏进工具)
- 那行提醒有个「必须短」的单测,加这句会撑破 40 字符。**不是删断言**,是分两支:
  skill 过期那支放宽到 60 并断言必须含"重读 SKILL.md",tke-only 那支仍 <40 且断言不许提它

### 2026-08-18 · skill 踩坑册与文档跟上这两天的改动
- **docs(skill)** 踩坑册加 4 条,全是这两天新修出来的:C-22「点了没反应先看页面报错」、
  C-23「原生对话框挡住所有操作且 fetch 采不到」、C-24「iframe 能采到但要认出跨域标记」、
  C-25「别绕过 tke 操作设备(直通已删)」;C-16 补上同名元素**优先可点击**(P-35)、
  C-8 补一句"没有第二条路了"
- **docs(skill)** SKILL.md 确认结果那节加 `errors` / `dialog` 两个字段的读法;
  tke-commands.md 补 iframe 与对话框在采集里的表现
- **docs** `driver-mapping.md` 补齐 hover/select/browser-*/对话框/页面报错/iframe 六行,
  修掉 key 那行的过时描述;与新的 `platform-matrix.md` 互相指路并说清分工
  (一份面向排查"怎么实现的",一份面向调用"有没有/一不一致")
- **fix(P-40,INV-9)** 核对矩阵时发现 **web 的 `key_event` 也是 `_ => Ok(())`**
  (上一轮只修了 iOS):认不出的键报成功却什么都不做。改成报错并列出支持的键;
  web 顺带支持单个字符(`按键 ["a"]` 是有意义的请求)

### 2026-08-18 · 平台能力矩阵 + 删掉 CLI 直通（用户拍板）
- **docs** 新增 `docs/platform-matrix.md`:三端共有/平台独有/**同名不同义**三张表 +
  「加新动作的检查单」;skill 的 steps-syntax.md 同步一份精简版
- **fix(INV-9)** 写文档时炸出:iOS 的 `key_event` 对认不出的键 `_ => Ok(())`——
  `按键 ["TAB"]` 什么都不做却报成功。改成明确报错
- **refactor(ADR-0016)** 删掉 CLI 直通(`tke adb shell …`)。它是操作设备的第二条路,
  绕过证据留存、坐标换算和唯一的动作映射——「点得中但什么都没留下」多一条入口。
  保留 `ToolManager::resolve`(内部定位 adb/chromedriver/go-ios/tke-opencv)和
  `tke <path.tks>` 便捷路由;未知命令报错并指路
- **feat** 补上删除前盘出的唯一缺口 `tke app log`(logcat):按**包名的 PID** 过滤而不是
  grep 包名——崩溃堆栈那几行不含包名,grep 会把最有用的一段滤掉;默认 `*:W` 200 行,
  拉全量会把 AI 上下文冲爆;取一次就返回(不 follow,CLI 挂着等日志只能被超时杀掉)

### 2026-08-18 · browser 能力收进 control 层（用户拍板）
用户指出:control 层就是所有原子指令的入口,浏览器独有能力也该在它下面。
理由比命名更硬——`execute_action` 的注释写着「**唯一的 ControlAction → 设备映射**,
`tke control` / tks 解释器 / AI agent 都经此执行」,而上一版把 browser 能力放在 CLI 里
**直接调 Controller**,等于绕过了这个单一来源:steps 和 agent 都用不上这些能力。
- **refactor** 删掉顶层 `tke browser` 子命令组,四条平铺进 control(统一 `browser-` 前缀):
  `control browser-reset|browser-eval|browser-viewport|browser-download`
- **refactor** 新增 `ControlAction::{BrowserReset,BrowserEval,BrowserViewport,BrowserDownload,Dialog}`,
  经 `execute_action` 分发;输出也跟着统一成 JsonOutput(之前那版自己 println,风格也不一致)
- **fix(同类问题)** tks 的三条对话框指令原本也直接调 controller、绕过了统一映射,一并改回
  `execute_action`;新增 `control browser-dialog accept|dismiss [--text]` 补上 control 侧入口
- 真浏览器实测:eval/reset/viewport/dialog/download 五条全过,tks 侧 `对话框输入` 照常

### 2026-08-18 · web 能力补齐:页面报错可见 + 干净态 + eval + 视口 + 下载
- **feat(页面报错,P-38)** 每步自动收 console.error / 未捕获异常 / 加载失败的请求
  (`POST /log`,chromedriver 扩展端点,无需额外 capability),写进 StepResult/StepEnd/
  报告/终端。「点了没反应」最常见的真因就在这儿,而页面结构和截图里都看不见。
  噪音控制:只留 SEVERE、滤 favicon、每步最多 3 条、单条截 300 字
- **feat(browser 子命令组)** `tke -d web browser reset|eval|viewport|download`
  - `reset` 回到「首次访问」:cookie/localStorage/sessionStorage/IndexedDB/缓存全清。
    浏览器会话跨命令复用,不清的话你以为在测新用户、其实是老用户视角
  - `eval` 在页面里跑 JS(不写 return 当表达式)。边界:观察和造前置,不代替用户操作
  - `viewport` 走 CDP `Emulation.setDeviceMetricsOverride` 而非 `/window/rect`(P-39):
    后者改的是窗口,实测设 390x844 量到 390x757,差一截就跨过断点了
  - `download --dir [--wait N]`:无头 Chrome 默认不落盘。判据是「有文件且无 .crdownload」
    ——**不能用"有没有新增"**,CLI 每条命令都是独立进程,记不住基线(实测踩过)
- **docs** skill 的 tke-commands.md / steps-syntax.md 同步写上——**能力不写进文档等于
  不存在**,这条已经踩过三次

### 2026-08-18 · 原生对话框被 WebDriver 自动点成「取消」,全程没提示
- **fix(P-37)** `unhandledPromptBehavior: "ignore"`,别让 WebDriver 替人做决定;
  每步后探测 `/alert/text` 写进 StepResult/StepEnd/报告/终端;下一步执行前拦截并
  讲人话(否则冒出来的是 `unexpected alert open`,AI 会去改定位、重试、绕路)
- **feat** 三条指令:`确认对话框` / `取消对话框` / `对话框输入 ["文本"]`(填完自动确定)
- **fix(顺带炸出的真 bug)** `session_alive()` 用 `GET /url` 探活,而对话框挂着时它同样回
  `unexpected alert open` → 判定**会话已死**,撞上对话框的下一条命令直接报「无活动浏览器
  会话」,AI 连把它点掉的机会都没有。改成含 `unexpected alert` 的照样算活着
- 真浏览器实测:confirm 确认→CONFIRMED、取消→CANCELLED、prompt 填字→张三;
  跨批次也能把遗留对话框处理掉

### 2026-08-18 · iframe 里的东西一个都采不到
- **feat(P-36)** 同源 iframe 递归采集:内部 rect 相对它自己的视口,累加 iframe 位置 +
  边框宽度;视口裁剪换成 iframe 自己的尺寸;xpath 前缀 `iframe[1]>>…`(内部 xpath
  拿到主文档找必然落空)。跨域采不到的**留一条标记**(INV-9),拼进该 iframe 自己那条
  记录,不另 push(否则同一个 iframe 出现两次)
- 支付/第三方登录/验证码/富文本都常住 iframe——不进去的话 AI 看到的是一张空页面
- 真浏览器实测:同源内部按钮点得中(unpaid→PAID);跨域出标记

### 2026-08-18 · 文字定位一直点在标题上,还报成功
用户实跑一小时的登录测试,结论写着「点 Sign in 均无任何反馈,是个死表单」——
**表单是好的,每次都点在 `<h1>Sign In` 标题上**(DOM 里它在按钮前面)。
- **fix(P-35)** `find_by_text` 原是 `.find()` 取 DOM 序第一个匹配、不看能不能点。
  改成按「更像用户会点的那个」排序:**可点击优先** → 精确匹配优先 → 自身文字短优先
  → DOM 序;一个可点击候选都没有时照旧返回第一个(断言文本存在不需要可点击)
- AI **没写错**:`fetch --interactive` 只输出 clickable/focusable,标题根本不在它看到的
  清单里。所以这不是提示词问题(INV-8),必须在定位层修

### 2026-08-18 · Ctrl+C 在确认提示处按了没反应
用户实跑:`tke uninstall` 停在 `继续？[y/N]`,连按十次 Ctrl+C 不动,**还得敲回车才退**。
- **fix(P-34)** 全局中断只置标志、等循环到检查点再停——这对跑步骤是对的,但此刻主线程
  阻塞在 `read_line`,**没有任何循环会去查那个标志**。新增 `interrupt::prompting()`
  (Drop guard)包住阻塞读 stdin 的那段,期间 Ctrl+C 直接 `exit(130)`;
  `tke uninstall` 与 `tke doctor --fix` 两处确认都接上
- **fix(二次硬退)** 监听改 `loop`,第二次 Ctrl+C 立即退出(第一次没停下来说明当前步骤
  很长或卡住了),提示语补「再按一次立即退出」

### 2026-08-18 · 装完开新 tab 又 not found:PATH 判断看错了地方
用户实跑撞到:装完那个 tab 里 `which tke` 有,**开新 tab 就 not found**,而 `tke doctor`
还一路绿灯写着「✓ 全局已就绪」。
- **fix(install.sh 根因,P-33)** PATH 段原先用 `command -v tke` 判断"装没装"——那看的是
  **当前进程的 PATH**,而它可能只是刚才临时 `export` 的(上一条改动恰恰教 AI 这么做),
  于是脚本认为已就绪、**一个 rc 文件都没写**。改成只看 **rc 文件的内容**;bash 同时写
  `.bashrc` **和 `.bash_profile`**(macOS 终端开的是登录 shell,只读后者);rc 不存在就创建
- **fix(doctor 不许撒谎,INV-9)** 新增「新终端」一项,同样只查 rc;不持久时结论从
  「全局已就绪」降级为「当前窗口可用 · 新终端里还找不到 tke」,并给出补写命令。
  体检报的是**这台机器**行不行,不是**这个窗口**行不行
- `install.ps1` 无此问题:它读写的是**注册表里的用户级 Path**,本来就是持久层

### 2026-08-17 · 删掉误导人的 check-env.sh;安装方法写进 skill
用户实跑撞到:新机器上 `tke doctor` 报 command not found,AI 翻到 skill 里的
`scripts/check-env.sh`,被它那句「构建 bin-proj/toolkit-engine/build-mac.sh」带偏,
最后卡住问用户要源码——**而普通用户手上根本没有源码**。
- **fix(删遗留物)** `check-env.sh` **零引用**(SKILL.md 第 0 步早就统一成 `tke doctor` 了),
  内容还停在开发者视角。留着就是个误导源,删掉;README/skill-integration/publishing 三处
  引用同步清理
- **docs(安装方法进 skill)** SKILL.md 第 0 步最显眼处写明:报 `command not found` 就是
  **没装**,一条 curl 装好;并说清**装完当前终端仍找不到**是正常的(安装器只写 rc 文件),
  同会话要 `export PATH="$HOME/.tke/bin:$PATH"`。Windows 给的是**先落地再执行**的写法——
  `install.ps1` 开头有 `param()` 块,`irm … | iex` 会报错
- **docs** 坑册 C-21;tke-commands 速查开头也补上安装那两条
- **验证** 用 `env -i` 造了个干净环境(PATH 里没有 tke)端到端跑通:
  command not found → curl 安装 → export → `tke --version` 可用

### 2026-08-17 · 报告能装下 AI 那份完整总结(表格/卡片) + pages 改成元素库
用户拿真实报告对照:对话框里 AI 写了漂亮的对照表+列表+注意事项,报告里却成了一句流水账。
- **诊断** Markdown 渲染**是生效的**,但两处逼着 AI 压缩:①**不支持表格**(AI 做对照最爱用)
  ②一大段多行 Markdown 塞进命令行要跟引号和换行搏斗
- **feat(表格 + 标题)** Markdown 子集补上 GFM 表格与 `#` 小标题;表格窄屏可横向滚动,
  不撑破页面
- **feat(`--summary-file`)** 从文件读总结,AI 先 Write 一个 md 再指过来即可
- **style(任务/结论独立成卡片)** 从 header 里挪出来,做成报告**最上面的一块**——
  人打开第一眼就是"要验什么"和"结论是什么",挤在标题行旁边等于藏起来
- **fix(pages 语义,之前理解错了)** `pages/` 改存**元素表 JSON**(等于"这一页的元素库":
  有什么、能点什么、在哪、xpath 是什么),而不是 tke 内部那份归一化 XML;
  `raw_pages/` 才是原文(DOM/uiautomator/XCUI)。报告的坐标反查同步改成读 JSON
  (顺带比抠 XML 属性更稳),老证据的 XML 仍然认
- **验证** 照用户那份总结的形态实测:1 张表(3 表头 6 行)+ 1 个小标题 + 3 个列表项 +
  加粗 + 行内代码,全部渲染;卡片确实在 header 之外。lib 80/80(+2) + CLI 27/27

### 2026-08-17 · `raw_pages/` 原始页面 + 总结按 Markdown 渲染
- **feat(raw_pages)** 每步另存一份**驱动直给的原始页面**(web=DOM outerHTML / 安卓=uiautomator
  原文 / iOS=XCUI 原文),与 `pages/`(tke 筛选归一化后的元素表)并列。
  实测同一页:**原始 1151 个标签 → 元素表 74 个**。用途:①某元素定位不到时,
  分得清"被 tke 筛掉了"还是"页面上压根没有" ②将来页面改版,对着两份原文才看得出改了什么
  (脚本持久化的底料)。取不到就跳过——它是参照物,缺了不影响执行
- **docs(不再把页面灌进对话)** SKILL.md 讲清:**想回看页面直接读 `pages/step_NNN.xml`**,
  比再跑一次 `fetch` 便宜得多、也不会把一坨 JSON 灌进对话框(`fetch` 留给"要看**现在**这一刻")
- **feat(总结按 Markdown 渲染)** `--summary` 现在支持段落/列表/加粗/行内代码——AI 写总结
  天然是这个格式。**在 Rust 侧转成 HTML,不往报告里塞 JS 渲染器**:报告得离线能看、
  内网能看、转 PDF 也能看,塞 JS 等于给交付物加"必须有肯执行脚本的浏览器"这个前提。
  也没引 pulldown-cmark——AI 用到的就那几样标记,为这点东西多一个依赖不划算
  (同 `tke fix` 用 curl 而非 reqwest 的理由)
- **安全** Markdown 渲染**先整体转义再认标记**:summary 是 AI 生成的文本,直接拼进 HTML
  就是注入口子。加了测试:`<script>` 必须被转义、`**<img onerror>**` 加粗生效但标签转义、
  没配对的 `**` 不吞字符
- **验证** 端到端实测:目录出现 raw_pages/;markdown 结论正确渲染成 p/strong/ul/li/code。
  lib 78/78(+3) + CLI 27/27

### 2026-08-17 · 用户实跑一轮的六条反馈:成败语义/报告开头/元素噪音/主动调用
用户拿真实后台跑了一轮(7 步,其中 1 步定位没命中、第 6 步用坐标点回来了),逐条反馈:
- **fix(成败语义,最要紧)** 报告顶上写着「失败」——可任务明明验完了。**tke 判断不了任务成没成**:
  一步没命中只是过程里的**无效尝试**,换个方式点中了就没事。现在:①步骤级措辞 `步失败`→`步未成`
  ②整体徽章**不再由步数推导**,没人给结论就只写"已完成" ③新增 `tke report --verdict
  pass|fail|blocked`——**`fail` 专指被测对象真有问题**(功能坏了/复现了 bug/用户说的属实),
  `blocked` 是没验成。加了回归测试锁住"一步未成 ≠ 任务失败"
- **feat(报告开头写需求)** `tke report --task "用户的原话" --summary "一句话结论"`,
  显示在报告最上面——没有它,人打开报告只看到一串点击,不知道当初想验什么
- **feat(`--open`)** 生成后用系统默认浏览器打开(mac `open` / Linux `xdg-open` /
  Windows `cmd /C start`);**无图形界面自动跳过**并说明,不报错
- **fix(元素噪音,信息爆炸的根因)** `fetch --interactive` 一屏刷出 43 个元素、276 行 JSON,
  其中 30 个是 `svg`/`path`/`rect` —— 它们 `clickable=true` 只因为 **`cursor:pointer`
  会被子元素继承**,一个图标按钮能刷出四五条。现在:①图形构件一律排除(除非自带 aria/title)
  ②`cursor:pointer` 只在**没有可点祖先**时才算。实测某页 43 → 21 个,且全是 a/button/input
- **fix(INV-9,静默丢步)** 认不出的指令**被静默跳过**——五条里错一条,那条悄悄不执行、
  其余照跑,结果是"少做了一步却显示成功"(实测:AI 写了不存在的 `refresh`,只收到一句
  "没有可执行的有效指令")。现在整批拦下并**列出所有可用指令**
- **style(降噪)** 每条命令前的 `WARN 未找到元素库文件` 降为 debug——**没有元素库是常态**
  (skill 明令不建),真用 `{元素名}` 时定位那步会实打实报错
- **docs(提高调用率)** SKILL.md 新增「什么时候该用(**不必等用户开口**)」:改完前端/UI、
  修完影响界面的 bug、改了会体现在界面上的后端逻辑、交付之前——都该自觉跑一遍再交结论;
  frontmatter 的 description 同步强化(那是 Claude Code 决定何时加载 skill 的唯一依据)。
  坑册 C-19/C-20

### 2026-08-17 · 安装进度条接到名字后面 + 体检只报状态(用户两条反馈)
- **style(进度条)** 原来是两行:先打一句「下载中（几百 MB）」,curl 的进度条再自己占一行。
  现在**接在名字后面同一行**、完成后原地变对钩:
  curl 的 `-#` 走 stderr、用 `\r` 原地刷新,把 `\r` 拆成帧、逐帧重画整行就拼上了
  (`\033[K` 清上一帧残尾;`PIPESTATUS[0]` 取 curl 的退出码)
- **fix(INV-9)** 上面这么做会**连失败原因一起擦掉**——curl 的报错也走 stderr、且是
  **追加在进度条同一行右侧**(所以 `grep '^curl:'` 抓不到,要不锚行首)。
  改成 `tee` 留一份、失败时把原因摆出来,由调用方缩进展示,不与「下载失败」那句重复
- **style(体检只报状态)** `浏览器  无头运行 · --headless=off 可开窗口（手动登录时用）`
  → `浏览器  无头运行`(无桌面时补一句「本机无图形界面」——那是环境事实,不是用法)。
  **用法归 `--help`**,doctor 只报状态和基础信息
- **fix(Windows)** install.ps1:去掉「下载中（几百 MB）」;顺带修一个复制粘贴 bug——
  Chrome 装好后打印的居然是 skill 路径。PowerShell 的进度是顶部横幅(其固有形式),
  做不成 bash 那种行内拼接,为此手写整个下载循环不值得(且本机无法真机验 Windows)
- **style(收尾去重)** 装完的结论**体检已经说过了**(`✓ 全局已就绪` / `✗ 环境不完整 · 补齐：…`),
  安装器再说一遍是重复。现在只补一句体检不会讲的:`在 Claude Code 中输入 /tke-ui-test 以调用`;
  失败分支整段删掉(doctor 的输出已经完整,只留退出码)。Windows 那边三行(还带一句示例台词)
  收成同样一行
- **验证** 伪终端下实测成功/失败两条路径:进度条原地刷新→对钩;失败时进度条擦净、
  原因缩进显示。lib 74/74 + CLI 27/27
- **fix(P-32,进度条"出来很慢")** 上面那条流水线里的 `tr` 是**块缓冲**的:要攒够 4KB
  进度帧才吐给下游,于是**整个下载期间一帧不显示**、最后才一次性跳到 100%。
  改用 bash 内建 `read -r -d $'\r'` 切帧(管道里不留会缓冲的外部命令;
  `stdbuf -oL` 能治 GNU 的 tr,但 **macOS 没有 stdbuf**)。
  **实测第一帧 9.25s → 0.28s**,全程均匀刷新。另外先把 `· <名字>` 打出来占位——
  建连接/握手那几秒 curl 一个字节都不输出,不占位就是盯着空白等

### 2026-08-17 · `tke update` / `tke uninstall` + 更新提示收敛(用户反馈"这个不好看")
用户:"这个不好看,就直接告诉用户有可用更新,然后提示用什么一行指令更新就好了……
这么看是不是应该有个 tke update 和 tke uninstall 的指令?"
- **style(提示收敛)** 原来三行(本地/分发源对比 + 路径 + 一条 100+ 字符的 curl)压成一行:
  体检里是 `skill  可用更新 · 20260817-040034`(版本号走 dim),**更新命令只在结论区出现一次**
  (`! 有可用更新　更新：tke update`)——tke 和 skill 谁旧都是同一条命令,说两遍是噪音。
  `steps` 缀的那行同样收成 `! skill 有可用更新　更新：tke update`
- **feat** `tke update` / `tke uninstall`:**不另起一套逻辑,就是去跑官方 install.sh / uninstall.sh**
  (重写一遍只会多一条必然分叉的路径)。`uninstall` 支持 `--logs/--chrome/--all/--dry-run`,
  默认问一句
- **feat(exec 交接)** 用户原话:"执行这个 curl 指令然后立刻放手,让 sh 脚本来替换自己"——
  **Unix 用 `exec` 把本进程替换掉**:tke 就此消失、bash 接管同一个 PID 与前台,
  输出/Ctrl+C/退出码全部照常,而且**没有任何进程还占着 tke 的可执行文件**。
  Windows 没有 exec,只能 spawn 等待,故 `install.ps1` 补了"删不掉就改名"的兜底
  (Windows 允许重命名运行中的文件)
- **安全** **不用 `curl … | bash`**:分发平台对不存在的路径回落 200 + HTML(P-19),
  管道执行会把网页喂给 bash。先落地、**验文件头**(`#!` / `<#`)再执行;加了 CLI 契约测试
- **docs** 安装器结尾从 curl 卸载命令改成 `升级 tke update · 卸载 tke uninstall`
- **验证** `uninstall` 走通 exec 交接;`update` 完整链路实测(下载→装→体检→"全局已就绪");
  指向 SPA 路径时如实拒绝执行。lib 74/74 + CLI 27/27(+2)
- **style(参数砍到最少,用户:"这些是干啥的,不能简单点吗")**
  `tke update` **零专属参数**——装的时候已经选过一次 profile,更新时按**现场装了什么**推断
  (只装了 adb 的人不该因为一次 update 被拖 600MB 的 Chrome);
  `tke uninstall` 只留 `--all`,**`--dry-run` 删掉**——它想解决的是"先看看会删什么",
  而这本就该由唯一那次确认提示直接列出来(顺带修了个口径不一致:清单里的安装目录
  原来取"当前 tke 在哪",与脚本实际删的 `~/.tke/bin` 可能不是一个地方)。
  `--profile`/`--logs`/`--chrome`/`--base-url` 保留但从 help 隐藏

### 2026-08-17 · 浏览器默认无头 + 凭据不落进证据(ADR-0015)
用户:"能否默认跑无头?**有头会和用户抢鼠标**。"顺着这条又定了凭据怎么处理。
- **feat(默认无头)** `Auto` 从"按桌面探测"改为**恒定无头**。无头与有头渲染早已验证一致
  (1280×813,bounds 零差异),日常没理由弹窗口抢焦点。要看着它跑用 `--headless=off`
- **fix(登录流程的命脉)** 新增 `HeadlessMode::explicit()`:**Auto 不再算"显式要求"**。
  否则「`--headless=off` 开窗口 → 用户手动登录 → 下条命令不带参数(Auto=无头)」
  会被判成会话模式失配 → **销毁会话 → 登录态没了**。现在 Auto = "沿用现有会话"
- **feat(凭据脱敏,硬护栏)** 实测:`输入 ["密码","hunter2"]` 的明文会进**三处**——log.json、
  report.html、**以及标注截图的顶部横幅(烧进像素)**,而报告正是拿去分享的
  (本会话就把报告传过公网)。四层一起堵:①采集层**密码框永远不取 value** ②`UIElement::is_password`
  与安卓 uiautomator 原生 `password="true"` 同名对齐,三平台一条路 ③判据取**焦点所在元素**
  (按"坐标上有什么"会漏:`输入 ["密码",…]` 常命中 `<label>`,点它同样能聚焦到密码框)
  ④命令原文经 `utils::redact` 打码后才落盘
- **决策(打码的失败方向)** 结构不符预期一律**整条打掉**,绝不原样退回——
  测试里逮到过:`输入 ["密码, "hunter2]`(值没闭合)引号恰好偶数,"只替换最后一对"
  把明文留在了外面。**少打一次码就是泄一次密码**
- **docs** SKILL.md 新增「浏览器默认无头」「碰到登录怎么办」两节(默认让用户自己登、
  用户主动给凭据才代填);坑册 C-18;C-7 补上"Auto 不触发销毁"的说明
- **fix(顺手)** `启动 ["file:///…"]` 被当成域名拼上 `https://` → ERR_NAME_NOT_RESOLVED;
  改为认 `://` 与 `data:`(本地 HTML 是最省事的测试页)
- **验证** 真实密码框实测:`alice` 原样保留、密码在命令/log/报告/**截图横幅**全为 `••••••`,
  明文一个文件都搜不到。lib 75/75(+5) + CLI 25/25

### 2026-08-17 · 修 CI 漏编(P-31,用户报"doctor 用不了") + 体检/安装文案重做
用户装完最新版后 `tke doctor` 报「doctor 可执行文件缺失」——那是 passthrough 的报错,
说明**发出去的二进制根本没有这个命令**,而 CI 全绿、skill 包照发。
- **fix(P-31,CI 静默漏编)** `changes` job 用 `git diff HEAD^ HEAD` 判断要不要编译,**只比最后一个提交**。
  而那次 push 推了 `feat(src/…)` + `docs(STATE 收尾)` 两个 → 只看到后者 → 判定"只动文档" →
  **跳过六平台编译**。本项目的收尾惯例正是最后补一个 docs 提交,所以这个坑会
  **稳定复现在每一次带收尾提交的功能发布上**。改为比**整个 push 范围**
  (`github.event.before..github.sha`,`fetch-depth: 0`),取不到 before 时**默认编译**;
  并把本次改动的文件列表打进日志(条件本身要能被看见)
- **style(体检)** 文案专业化 + **结论行移到最后**(对钩是结论,不是中间某项检查):
  `✓ all 需要的依赖都在`→`依赖 已就绪 · all` + 末尾 `✓ 全局已就绪`;
  `无（adb 可用但没连设备）`→`设备 未连接`;`0.7.4-beta（与分发源一致）`→`版本 已是最新 · 0.7.4-beta`;
  `证据落点`→`日志落点`(去掉"已有 N 次检查");`有桌面 → 浏览器默认有头…`→`运行环境 有头环境`;
  iOS 门禁那两行啰嗦提示收成对齐的一行
- **style(安装)** `装好了 在 Claude Code 里直接提需求，或 /tke-ui-test` →
  `全局已就绪，在 Claude Code 中输入 /tke-ui-test 以调用`
- **style(卸载)** `保留 检查记录(--logs) · Chrome(--chrome)` 没人看得懂那个括号是什么意思 →
  `已保留 日志 <路径> · Chrome` + 下一行明写`重跑并加 --logs / --chrome 可一并删除（--all 全删）`
- **注** install.sh 里的体检仍用别名 `fix --check`:它跑的是**刚下下来那个** tke,
  万一分发源上还是旧版,用新名字会直接报"命令不存在"（别名永久保留正是为此）

### 2026-08-17 · `tke doctor`:把「本地是不是旧版」变成看得见的一行(ADR-0014,关闭 Q-11)
Q-11 的代价已经付过一次:用户装好的 skill **装完就不动**,没有任何东西告诉他有新版——
一整场会话改的四个修复,他重跑时拿到的仍是两天前的旧文档,**必然得出"没改善"的结论**。
- **fix(根因)** `fix --check` 其实一直在联网比版本,但只比 **tke 二进制的版本号**——
  而版本号**只在 bump 时才变**、SKILL.md 却天天改。**改成比 VERSION 里的 `build` 戳**
  (每次发布都变),`publish.sh` 把这份 VERSION **一起打进 skill 包**,装完就有据可查
- **feat(更名)** `tke fix` → **`tke doctor`**(体检,**不下载任何东西**);`doctor --fix` 才补依赖。
  `tke fix` / `tke fix --check` **保留为别名且语义不变**——已发布的 install.sh、用户脚本、
  以及**用户机器上那份旧 SKILL.md** 里全是老写法,它们不会因为我们改名就自己更新
  (正是本次要解决的问题,不能自己再犯一遍)
- **feat(挂在 steps 上)** 调用方 AI 每次操作设备都走 `steps`,**这是唯一保证被看见的位置**——
  指望人想起来跑体检正是踩坑的原因。三条克制:结果**缓存 4h**(每批都问、每 4h 才真联网一次,
  超时 5s)、打 **stderr**(stdout 是给 Electron 的 NDJSON)、`--json` 时闭嘴
- **决策(只提醒不代劳)** 发现不一致只打印一行 + 更新命令,**绝不自己覆盖二进制**——
  覆盖运行中的可执行文件在三平台各有各的坑(Windows 锁文件/Linux ETXTBSY/macOS 签名),
  install.sh 已经踩平并验证过
- **边界(宁可漏报不误报)** 老安装器装的 skill 没有版本文件 → 报"装了,但没有版本信息",
  **不当成过期**;本地自编的 tke(`unknown`)不参与比对。误报会让人学会忽略提醒,比不报更糟
- **不违反 ADR-0012** 只 `curl` 一个几十字节的 VERSION,不下载任何依赖;"唯一会下载的命令"
  仍然只有那一条。ADR-0012 已加指向
- **验证** 模拟 08-13 的旧 skill → doctor 与 steps 都如实报出并给更新命令;一致时安静;
  `tke fix --check`(install.sh 用的那条)行为不变。lib 70/70(+4) + CLI 25/25(+2)
- **ci** CI **不走 publish.sh**(自己打包),同样的顺序问题:VERSION 原本在 skill 打包之后才生成。
  已把 VERSION 生成提到前面并一起打进 skill 包,**加了一条自查**——包里没有 VERSION 就让 CI 红,
  否则这套会静默失效(装到用户机器上永远看不出过期,而这正是它要解决的问题)

### 2026-08-15 · 证据组织重做:一个任务一份报告(用户反馈"这种组织方式很乱")
用户看完上一份报告的评价:**"log 和 report 的组织方式很乱"**。确实——每调一次 `tke steps`
就建一个 `steps_<时间戳>/`,各带自己的 `screenshots/`/`page/`/`log.json`,外面再拼一份
全流程报告。调十次就是十个目录 + 十一份报告,人要审得先挑出哪份是总的。
- **feat(布局)** 新增 `Layout::Task`:**`--log` 指的就是任务目录本身**,反复调用**续写同一份证据**——
  `<任务>/{report.html, screenshots/, pages/, log.json}`,**步骤跨批次连续编号**(step_001…005),
  一个任务**始终只有一份 report.html**。`page/` 更名 `pages/`
- **范围** 只改 `steps`(一次性检查);`run`/`flow`/`harness` 保持 `<名>_<时间戳>/`——
  它们每次是**独立的一次回放**,分目录才对得起"跑第二遍和第一遍比一比"(用户拍板)
- **feat(log)** `TaskLog{batches:[…]}` 累积每批,**读-改-写**而不是覆盖;兼容读旧的单批格式
- **feat(跨设备不再分目录)** 每批自带 `device`,报告里标出来并**按时间排成一条线**——
  正好还原"平台侧做了什么 → 手机侧看到什么"。此前教人分 `web/`+`phone/`,反而把因果链切断、
  还要额外汇总一次。skill 文档同步改掉
- **feat(截图内嵌)** 任务报告**默认自包含**:单个 html 直接转发,对方不需要那个目录。
  内嵌走**缩放 + JPEG**(宽 960/质量 82)——报告容器只有 880px,内嵌更大的像素**一个字都不会更清楚**。
  5 步实测 **1.7MB(原图) → 598KB**。点报告里的截图可跳原图;`tke report --full-image` 出原图版
  - ⚠️ 实测教训:**光转 JPEG 几乎不省**(PNG 对大片纯色压得很好,JPEG 在文字锐边上还吃亏),
    真正省体积的是**缩放**。第一版用 1280 宽只从 1.1MB 降到…没降,量了才发现
- **feat(边界)** `next_step_index()` 按**文件**扫编号而不是数 log.json 的步数:中途 Ctrl+C 的批次
  可能没写完 log 但截图已经落了,漏算会**直接覆盖上一批的证据**(静默丢证据,加了回归测试挡)
- **验证** 三次独立 `steps` 调用 → 连续 5 步 / 3 批 / 一份 598KB 自包含报告,无死链。
  lib 65/65(+3) + CLI 23/23

### 2026-08-15 · 探索式使用不再把报告搞乱(用户追问"一步一步探索会不会分成两个任务")
不会——同一个 `--log` 就是同一个任务。但顺着这个问题实测了一遍"每次只走一步"的极端情况,
发现报告确实会被搞乱,以及两个真 bug:
- **fix(噪音)** 批次分隔行改为**只在有信息量时才插**:换设备、或中间停了 ≥60s。
  探索式会产生一长串"1 步"批次,每批插一行的话人看到的全是"AI 分几次调的"——
  那是工具的实现细节,不是这次检查发生了什么。实测 5 次单步调用:分隔行 **5 → 0**
- **fix(标题/目录指错)** Task 布局下 `dir` 就是任务目录,而渲染仍照旧取 `parent()` →
  **报告标题变成 "logs"、"打开检查目录"跳到 `~/.tke/logs`**。改为按 `prefix` 是否为空区分两种布局
- **feat(空档标注)** 两批间隔 ≥60s 时标一行「间隔 N 分钟」——那多半是**人在中间做了什么**
  (手动登录、去后台改配置),而这件事在证据里没有任何痕迹,只剩这个时间空档
- **docs** SKILL.md 讲清"`--log` 目录名 = 任务身份,一个字都不要改",并明确探索式怎么用;
  新增坑册 C-17(中途改名 → 一次检查散成几份报告,**且不会报错**)
- **验证** 单元测试锁住三种情况(同设备连续=0 行 / 换设备=1 行且写明设备 / 间隔 6 分钟=标出空档);
  真机换设备实测确实插行。lib 66/66(+1) + CLI 23/23

### 2026-08-15 · 语义定位这条路上的四个洞:实测走一遍才发现它一直是断的
上一场把 SKILL.md 从坐标掉头到语义定位(90d9dcad),但**没有人真的用新版走过一遍**(Q-9)。
这次自己当调用方 AI 实跑,一个普通的"搜索→进条目→点内链"链路就把路上的洞全撞出来了——
**四个洞环环相扣,单修任何一个这条路都还是断的**。
- **fix(P-28,感知层)** 读屏专用元素(`sr-only`/`screen-reader-text`)**人看不见却带着那行文字**,典型是 **1×1 像素**,却通过了 `width>0&&height>0` 的可见性过滤进了元素表,还**排在真输入框前面被先命中** → `输入 ["Search Wikipedia", …]` 点在那个 1×1 幽灵点上。同时真正的输入框**一个字都没有**(没直接文本、没 placeholder,可见名称来自 `<label for>`,而采集只认 `aria-label`/`placeholder`)。**两处一起修**:排除人点不到的(≤1px/`opacity:0`/`clip` 裁没的) + 补齐可及名称(`aria-labelledby`→`.labels`→`title`)
- **fix(INV-9,误导错误)** 上面那个点空,驱动报的是「当前没有聚焦的输入框(**请先点击输入框**)」——把人和 AI 都引向"那我先点一下",也就是**引回坐标路线**;真实原因是上一步点空了。改为回报焦点落在什么标签上 + 指出多半是定位命中了同名非输入元素
- **fix(P-29,平台白等)** `atomic/control.rs` 的 `Input` 点击后固定 `sleep(500ms)` **等软键盘**——那是移动端才有的东西,web 上纯白等(占该步耗时 ~38%)。下沉到 `Controller::has_soft_keyboard()`。**与 P-27 同族**:一个语境下正确的等待被搬到不需要它的语境,不报错、只是悄悄变慢
- **docs(P-30,文档坑)** 文字定位**只看得见视口内的元素**,目标在折叠下方时直接失败(还白等满 ~6s、整批中断)。破解它的 `滚动查找 ["文字", 方向]` **能力一直都在、且不需要元素库**,但 `steps-syntax.md` 把它标成"需要元素库",而这个 skill 明令不建元素库 → **调用方 AI 一次都没用过它**。与 90d9dcad **同型**(能力早就有,只是没告诉 AI / 告诉错了)
- **验证(本机无头,前后对照)** 同一步 `输入 ["Search Wikipedia", …]`:修前**失败**(点到 1×1 幽灵)→ 修后 **1315ms 通过**→ 去掉 500ms 白等后 **886ms**;整批 3 步**纯语义、零坐标、零 fetch**。`点击 ["Memory safety"]`:不滚 = 失败+白等 **9.1s**,先 `滚动查找` = **0.4s** 找到 + 点中
- **量到的 token 事实** 一次 `fetch --interactive` = **32KB(≈8K token)**,而 fetch 本身只要 237ms——**贵的从来不是时间,是每次重新 fetch 的那张表**。这就是坐标路线烧 token 的根因,与上一场的诊断对上了

### 2026-08-15 · `fetch --wait-text`:把"等文字出现"从提示词变成子命令(ADR-0013,关闭 Q-8)
skill 不建元素库 → 重试断言用不了 → 等异步下发只能让调用方 AI 手写 shell 轮询,
护栏全在措辞里。三个坑(忘超时/忘判命中/误加 `--interactive`)每个都产生**假结论**,
**第三个是写 SKILL.md 的我自己第一版就踩的**。
- **feat** `tke -d <设备> fetch --wait-text <文本> [--timeout <秒>]`:**出现即刻返回**(不是死等满)、正常输出元素表退出 0;超时**非零退出**,`||` 与 `$LASTEXITCODE` 天然接得住。查**全量**元素(不受 `--interactive` 影响),多候选 `"A|B"`,匹配口径与 `滚动查找` 共用 `utils::scroll`
- **docs** SKILL.md / pitfalls.md / steps-syntax.md / tke-commands.md 里的手写轮询范例**全部替换**;新增坑册 C-15(视口外要先滚动查找)、C-16(点到人看不见的同名元素)
- **依据** ADR-0010 早就写过"护栏退化的出路是做成必须调用的子命令,不是把提示词写更长";这次 `滚动查找` 被文档写错而无人使用,是**靠文档传递能力有多脆**的独立佐证
- **验证** 命中 0.45s 退出 0 / 超时 5.45s 退出 1;CLI 契约测试 +2。lib 62/62 + CLI 23/23

### 2026-08-15 · 审计:wda/web 没有 adb 同款"无限挂"(关闭 Q-4)、移动端没有 P-27 式白等(Q-10)
- **Q-4 关闭** `web`/`wda` 全部 **17 处 ureq 调用 1:1 都带 `.timeout()`**,所有等待循环都是有界的 `(0..N).any(…)`、无 `loop{}`。根本差异:adb 是 spawn 子进程(**没有任何自带超时**,故 P-03 要全链路兜底),web/wda 走 HTTP 由 ureq 兜底;外部进程(chromedriver/go-ios)是 spawn 后台化 + 有界轮询探活,不阻塞
- **Q-10 部分回答** 移动端**没有** P-27 那种"读不到值→等满"的静默退化——`adb`/`wda` 的 `tap` 后**根本不等**(web 才有 `wait_ready`)。反而查出**反向**同族问题(P-29:web 吃了移动端的软键盘等待,已修)
- **留给真机的** adb 每次采集要 **6 次进程往返**(`screencap`+`pull`+`rm`、`dump`+`pull`+`rm`),`input_text` 另有固定 500ms(输入法切换,adb 特有且有理由)。这是安卓侧"慢"的最大嫌疑,但**本机无设备量不了**——按 P-27 的教训**先量再改**,量法与结论记在 Q-10
用户反馈"等待太多、拖慢整体速度"。顺着量下去,发现**真凶不是 AI 写的等待,是 tke 自己在白等**。
- **fix(P-27,静默退化)** `WebDriver::execute()` 返回的是**已剥掉 `{"value":…}` 外壳**的结果,但两处调用方又多解了一层:①`wait_ready` 里 `document.readyState` **永远读不到 `complete`** → **每次点击都白等满 20×200ms + 400ms** ②`center_into_viewport` 里视口尺寸永远读不到 → 一直用硬编码兜底 1280×800,**坐标夹紧会算错**。两处都不报错、只是悄悄退化——**量了耗时才逼出来**(点击 4899ms vs 单次采集 110ms、原子点击 14ms)
- **效果** 每步 **4899ms → ~750ms(6.5×)**。按用户那次 47 步算:3.8 分钟 → 约 35 秒
- **feat** 文字定位补上**隐式等待**(12×500ms,与元素定位同一套):此前只采集一次、找不到就失败,调用方只能到处垫 `等待 [1s]` 兜底。现在**元素已在就立刻返回**、没渲染完才等,且能等够 6 秒(比死等 1 秒更可靠)
- **docs** SKILL.md 与坑册 C-14:**默认别写 `等待`**——定位自带隐式等待、点击也会等到页面就绪;只有"没有对应元素的过程(动画/toast)"和"后端异步下发(用轮询更准)"才需要。示例里那些 `等待 [1s]` 全删了(**是我在示例里带头写的**)
- **验证** 延迟 2.5s 才渲染的元素:不写等待照样点中;真实跨站跳转:确实等到新页面就绪(内容已是 iana.org)——**快的是不该等的地方,该等的一秒没少**

### 2026-08-14 · 治 token 爆炸的根因:从坐标路线掉头到语义定位(用户实测反馈)
用户跑一个跨端任务烧光了一整个 opus 会话。拉他的报告数出实据:**20 个批次 / 47 步,平均每批只有 2.35 步,其中 22 步(47%)是「等待」;坐标操作 23 步、语义操作 0 步。**
- **诊断** 根因**不是 tke 不能干,是 SKILL.md 把 AI 引到了最费 token 的那条路上**:用坐标就必须先 `fetch` 全量元素表,而坐标一变就失效 → 每两三步重新 fetch 一次 → 20 批 × 大 JSON。**语义定位的能力 tke 早就有**(`resolve_text` 在每步执行时实时刷新页面并按文字定位),实测 `点击 ["Learn more"]` 直接可用
- **docs(最要紧的一改)** SKILL.md 掉头:**首选 `点击 ["保存"]`,坐标降为兜底**。文字在**执行那一刻**才解析,所以能**一次传五六步**而不怕页面变——批次数掉下来,fetch 次数跟着掉。且 `点击 ["保存"]` 可读可复用,顺带解决用户担心的"坐标不利于后期发展"。**这不违反 ADR-0010**:文字定位不产 `.tklib` 资产
- **feat(真能力缺口)** 新增 **`选择`** 指令:原生 `<select>` 展开后选项由**浏览器绘制**、DOM 里不可见(`getBoundingClientRect` 为 0),点击路线**根本走不通**——用户只好绕道 python 读页面。现在直接走 DOM 设值 + 派发 input/change 事件(不派发的话 React/Vue 收不到)
- **fix** `<select>` 采集特判:此前只取**直接文本节点**,而文字全在子 `<option>` 里 → text 恒为空;option 自身又因不可见被过滤 → **AI 完全不知道有哪些选项**。现在带出当前值 + 全部可选项(`options` 字段),选错时报错也会把可选项列出来
- **fix** 全流程报告**跨批次连续编号**:此前每批各自从 1 开始,读起来像好几段互不相干的测试拼在一起(`01 02 | 01 02 | 01 02 03 04`)
- **docs** 坑册加 C-12(坐标烧 token 且不可复用)、C-13(原生 select 点不开);steps-syntax 首选写法改为文字
- **测试** lib 62/62 + CLI 21/21 + bin 3/3;`选择` 指令实测:按文字定位下拉框→选中→fetch 确认值真的变了;选不存在的项报错会列出可选项

### 2026-08-14 · 安装/卸载输出精简(用户逐条反馈)
- **change** 分节标题统一成**英文大写**:`SKILL` / `DEPENDENCY` / `DOCTOR` / `REMOVED`(不再用中文标题)
- **change** 文案砍到最短:头部三项挤成一行(`tke 0.7.4-beta · darwin-arm64 · all`);Chrome 那句"已在 …（换版本先删这个目录）"删掉;PATH 两行并一行;结尾一句话 + 一行卸载命令
- **change** `tke fix` 的「环境/状况」**两段并一段**——分两段会与安装器的分节套在一起、还把平台报了两遍;安装器也不再单开「体检」节
- **change** 卸载**只报实际发生的事**:不存在的默默跳过(不再列"没有检查记录""没有安装 Chrome"),保留了什么压缩成结尾一句 `保留 logs(-Logs) · chrome(-Chrome)`
- **change** 卸载器用回 **ENGINE** 的 LOGO(品牌只有一个,不另做 UNINSTALL 字样)
- **feat** Chrome 下载**显示进度条**(几百 MB,静默会让人以为卡死);其余小文件仍静默。PowerShell 侧临时开 `$ProgressPreference`,只对大文件开——管道执行时它会刷屏
- **fix** PowerShell 又扫出 4 处 `$变量中文`(P-24 那个坑),已全部加花括号;自查命令已在 PITFALLS

### 2026-08-14 · 安装/卸载体验:LOGO + 配色 + 一行卸载
- **feat** 安装器加 TOOLKIT ENGINE 的 ASCII LOGO,输出改成**符号 + 颜色**(`▸` 分节 / `✓` `!` `·`),**不用 emoji**——等宽终端里对不齐、SSH/CI 日志里常变方块。`tke fix` 的输出同步到同一套(用户反馈"CLI 输出也不好看")
- **feat** **一行卸载**:`uninstall.sh` / `uninstall.ps1`。默认删 skill + tke/驱动 + PATH 行,**默认保留**检查记录(跑过的证据)与 Chrome(几百 MB);`--logs` / `--chrome` / `--all` 显式加。带 `--dry-run` 先看会删什么;改 rc 文件前先备份
- **fix(用户发现)** macOS 上不该找 `libc++.so`——那是 **Linux 版 aapt** 的运行时依赖(RUNPATH 含 `$ORIGIN`)。无条件装会在 mac/Windows 上请求一个不存在的文件、拿到 404
- **fix(PowerShell 两个标识符坑,P-24)** ①变量名**不区分大小写**:`$T`(颜色)被参数 `$t` 覆盖→标题打两遍;局部 `$logs` 覆盖 switch 参数 `$Logs`→赋值直接报类型错 ②变量名**可以含中文**:`$Ye试运行` 整个被当变量名、那三个字消失——**与 bash 的 P-20 如出一辙**。③函数名 `Remove-Item-Reported` 撞内置 `Remove-Item` 致参数绑定错乱
- **fix(P-25)** `Invoke-WebRequest .Content` 可能是 **byte[]**:版本号显示成 `116`(那是 `'t'` 的 ASCII),更坏的是 `build:` 戳解析不出来、**破 CDN 缓存的键悄悄失效**而表面正常
- **验证** 装了 pwsh 7.6.4 在本机真跑:install/uninstall 两个 ps1 语法通过 + 模拟 Windows 环境跑通(落地名正确补 `.exe`、DLL 保持原样);bash 版走完整安装→卸载闭环,试运行确认一个字节没删、logs 默认保留、rc 改前有备份

### 2026-08-14 · 宿主机能力门禁:做不了的组合直接拦下并说清
- **feat** 新增 `utils::capability`:**iOS 只在 macOS 放行**,Windows/Linux 上碰 iOS 设备直接拦下,报错说清**为什么**(设备上的 WDA 要用 Xcode 装一次,Xcode 只有 mac 有)、**这台机器能做什么**(web/安卓)、以及**逃生口**
- **落点** 门禁放在 `Controller::new` —— 所有设备操作的**唯一必经之路**,`control`/`run`/`steps`/`harness` 一处覆盖,不会漏
- **feat** 源头也不摆做不到的选项:`list_devices`(给编排官的)与交互式向导在非 mac 上**不列 iOS**——摆出来只会让人/AI 选一次、撞一次门禁、再回来重选
- **feat** `tke fix` 在非 mac 上不报"缺 go-ios"(补上也用不了),并说明原因
- **fix(误导措辞)** `tke fix --check --profile ios` 在 Linux 上原本显示"✅ ios 需要的依赖都在"——**这台机器压根做不了 iOS**,说"依赖都在"是骗人的。说明被 early return 跳过了,已提到列缺失之前,措辞改成"没有可补的依赖——这台机器做不了 iOS"
- **留了逃生口 `TKE_ALLOW_IOS=1`**,因为**这条界线是产品决策不是技术极限**:go-ios 本身跨平台、运行期也不需要 Xcode(经 testmanagerd 拉起 WDA),真正卡住的是那次一次性安装。"WDA 已装好的设备接到 Linux CI"技术上是通的,不该被堵死
- **docs** SKILL.md 的设备表加"哪些机器能做"一列;`tke fix --check` 会告诉你这台机器能做什么
- **测试** 4 条新单测(web/android 恒放行、iOS 按宿主机分且报错要说清原因与替代、逃生口、可选平台列表);`profile_scopes_what_is_checked` 随行为变更改成按宿主机分支断言。lib 61/61 + CLI 21/21 + bin 3/3

### 2026-08-14 · Windows 这条路补通（同事主力平台）
- **feat** **`install.ps1`**:Windows 一键安装器,与 install.sh 一一对应。此前 Windows 同事**根本装不上**——`install.sh` 是 bash,而那句"请用 install.ps1"指向的文件压根不存在
- **feat** **体检并进 `tke fix --check`**:除了列缺失依赖,还报安卓设备/版本比对/证据落点/有头还是无头。**一份 Rust 实现三平台通用**——`check-env.sh` 是 bash,Windows 用户跑不了,而 Windows 恰恰是"同事跑完 Claude Code 要验一遍"的主力。SKILL.md 第 0 步已统一成这条
- **docs** SKILL.md 与坑册的 shell 片段**补 PowerShell 版本**(轮询、Select-String、`$env:USERPROFILE\.tke\logs\`):Windows 上 Claude Code 用 PowerShell,`grep -q` / `for i in $(seq)` 直接跑不了
- **fix** CI 与 `publish.sh` 都只发 `install.sh`——`install.ps1` 不带上等于没做,已补
- **验证** 本机装了 pwsh 7.6.4 专门验这个(没跑过的脚本等于没写,今天已吃过一次亏):语法解析通过 + 抽出核心函数真跑——文件头校验(真 gz 通过 / HTML 被拦下)、gzip 解压内容正确、build 戳解析、落地名补 `.exe` 的规则;`$Profile` 作为参数名(PowerShell 自动变量)实测在脚本作用域内可用。云上那份取回来再验一次语法
- **change(措辞)** 版本比对不再摆 ⬆️ 箭头:本地可能是刚编的、比分发源还新,箭头会让人以为该更新。改成如实报"不一致"

### 2026-08-14 · 平台补到六个 + 摸清上游的三条边界
- **feat** CI matrix 加 **linux-arm64**(`ubuntu-24.04-arm` runner)与 **windows-386**(`i686-pc-windows-msvc` 交叉编译),构建步骤支持 `--target`
- **deps** 补齐 **win32 全套**(chromedriver + Chrome 152 + adb/aapt + 两个 DLL,都是 i386)与 **linux-arm64 的 go-ios**(ELF aarch64)
- **fix(差点传错)** win32 的 go-ios 我一开始是从 amd64 直接拷的——**上游的 go-ios Windows 包只有 64 位**(PE32+ x86-64),32 位跑不了。逐个 `file` 验架构时抓出来,已移除
- **事实(实测,非推断)** ①Chrome for Testing **只出 5 个平台**(linux64/mac-arm64/mac-x64/win32/win64),`linux-arm64` 与 `win-arm64` 直连 **404** ②Google 的 platform-tools **不出 arm64 Linux 版**(三种命名全 404) ③go-ios 的 Windows 包只有 64 位
- **feat** `tke fix` 知道这些边界:arm64 Linux 上直说"上游没有官方驱动,请 `apt install chromium-driver adb` 再软链到 tke 同目录",而不是让人对着下载失败反复试;32 位 Windows 不再报"缺 go-ios"(报了也补不上)
- **决定** **windows-arm64 有意不做**:Windows on ARM 自带 x64 模拟,windows-amd64 那套直接能跑;而 Chrome for Testing 也没有 arm64 Windows 版,单出一份只多一套要维护的东西

### 2026-08-14 · Windows 的 adb 还缺两个 DLL（用户提醒）
- **fix(Windows 上 adb 直接起不来)** `adb.exe` **直接依赖 `AdbWinApi.dll`**,USB 还要 `AdbWinUsbApi.dll`(由前者**运行时加载,不在导入表里**)。我第一版只传了 adb.exe,Windows 上根本跑不起来——**跟 Linux 版 aapt 缺 libc++.so 是同一类问题**,是用户想起来问才发现的
- **verify** 用 `objdump -p` 把四个 Windows 二进制的导入表都查了一遍:`aapt.exe` / `chromedriver.exe` / `ios.exe` **都自包含**(只用系统 UCRT 与系统 DLL),只有 adb 需要补。两个 DLL 已上传
- **feat** `tke fix` 的**伴生文件按平台分**:Linux 带 `aapt`+`libc++.so`,Windows 带 `aapt`+两个 DLL,mac 带 `aapt`。另外「adb.exe 在但 DLL 不在」这种半装状态(从别处拷 adb 过来最容易出现)现在也会被检出并补齐

### 2026-08-14 · 补齐四平台依赖 + 修 Windows 落地名
- **fix(Windows 上必炸)** `tke fix` 下载的二进制**落地时没补 `.exe`**——分发源上统一叫 `adb.gz`,Windows 落成一个没有扩展名的 `adb`,**根本执行不了**。现按平台补回扩展名(`libc++.so` 这类本身带点的不动)
- **deps** 手工补齐 **darwin-amd64 / windows-amd64** 两个空白平台:chromedriver + Chrome for Testing(Stable **152.0.7977.42**,driver 与 Chrome 同版本配对)+ adb + aapt + go-ios。逐个验过解压出来的架构:mac 是 Mach-O(chromedriver x86_64、其余 universal),win 是 PE32/PE32+(adb/aapt 是官方原样的 32 位)
- **注意** 现有 darwin-arm64(149) / linux-amd64(151) **有意不动**——`install.sh` 对已存在的 Chrome 目录是跳过的,升 driver 不升 Chrome 会版本不配对起不来。各平台内部配对即可,跨平台不必一致
- **docs** `install.sh` 里指向了一个**不存在的 `install.ps1`**,改成实话:Windows 手工放 tke.exe + `tke fix` 补依赖
- **change** CI 定位按用户要求收窄:`tke-deps.yml` 降级为"要整体升 Chrome 版本时才跑",**CI 的日常职责只剩「tke/skill 改了能发新版」**

### 2026-08-14 · GitHub Actions 发布流水线
- **feat** `tke-publish.yml`(常用):四平台构建 tke(darwin-arm64/darwin-amd64/linux-amd64/windows-amd64)+ 打包 skill + 刷新 VERSION,一键发到分发源。开关:`targets` 选平台、`ocr`(online 默认/full 含离线 tesseract/none)、**`skill_only`** 只改文档时一分钟发完、`dry_run` 验流程
- **feat** `tke-deps.yml`(低频):抓 Chrome for Testing + chromedriver + adb + aapt/libc++.so + go-ios。**driver 与 Chrome 从同一份官方清单的同一版本取**——版本必然配对,这是自建分发源最实在的价值
- **重要** **上传顺序:VERSION 最后传**。它的 build 戳是破 CDN 缓存的键,先传的话使用者拿新键去取还没传完的文件(P-19)。传完还会**从分发源真取一遍复验是 gzip 而不是 HTML**
- **fix(不跑就发现不了)** go-ios 的 zip **三个平台三种结构**:linux 里是 `ios-amd64`+`ios-arm64` **两个架构**,mac 是单个 `ios`,win 是 `ios.exe`。原写法 `find -o | head -1` 取目录遍历顺序,**在双架构包上会选错架构**;已改成按架构名优先级逐个找
- **验证** 三段下载逻辑**全部从 YAML 里抽出来本地实跑**:android(adb/aapt/libc++.so 三件到位)、ios(三平台各拿对架构,linux 那个确认是 x86-64)、chrome+driver(Stable 152.0.7977.42,driver 解压后版本一致、chrome zip 解压即 `chrome-linux64/` 结构)。**CI 脚本不本地跑一遍等于没写**
- **docs** `docs/ci-publishing.md`:两个 workflow 怎么用、Secret 怎么配、各家下载源的实测结构

### 2026-08-13 · 全流程报告:一次检查一份,不再是一堆碎报告
- **fix(设计缺陷)** AI 做一次检查要调很多次 `tke steps`(看页面→操作→再看→再操作),每次留下一个 `steps_<时间戳>/` 和一份独立 report.html。**人要审核时面对十几份碎报告,根本没法读**(用户提)
- **feat** `steps` 每批跑完**自动重建**父目录的 `report.html`:所有批次按时间接成一条时间线,每批带批次头(序号/设备/时刻/步数/跳回单批链接)。**AI 什么都不用做**
- **feat** `tke report <目录> [--embed]` 显式汇总:跨设备时证据分在 `web/` 与 `phone/` 子目录,自动重建只管到各自那层,收尾跑一次就把两台设备的批次**按时间交错**排成一条线——正好还原"平台侧做了什么 → 手机侧看到什么"的因果
- **取舍** 全流程报告默认**相对链接**引用截图(3 批 12K,重建极快);`--embed` 内嵌成单文件(420K)供贴工单/发群。单批报告仍然内嵌,它本来就要能单独发
- **refactor** 提出 `Ctx{run_dir,prefix,img}` 与共用 `BASE_CSS`:两份报告长得不一样会让人以为是两个工具出的
- **测试** 新增 5 条,钉的是**会读错因果**的地方:跨子目录必须**按时间**排(不是按目录名)、步数跨批累计、空目录要报错而不是产出骗人的空报告
- **样例** https://cloud.test-toolkit.app/sl/preview/guest/test/AI_Reference/tke-session-report-sample.html
- **测试** lib 57/57 + CLI 契约 21/21

### 2026-08-13 · 读图策略:该看的时候必须看(坑册 C-11)
- **docs** SKILL.md 此前只强调「每步都读图会让 token 爆掉」,**引导过头**——AI 可能一张图不看就下结论。现在给出**必读判据**:下最终结论前、结果与预期不符时、操作后页面没如预期变化时、要判断布局/颜色/选中态/图表/图片时
- **docs** 新增坑册 **C-11「从不读图 → 拿'元素存在'冒充'功能可用'」**,与 C-9(每步读图爆 token)互为反面、双向交叉引用。文本能证明"节点在控件树里",**证明不了"用户看到的这一屏是对的"**——渲染失败/被遮挡/颜色错/图没加载,元素表里全都一样
- **docs** 算清成本再讲取舍:一张图约上千 token,二十步几万确实爆;但**一次检查读两三张关键的可以忽略**。省 token 省到不敢看结果是本末倒置
- **docs** 指明读**标注截图**(`steps --log` 已存好,路径在输出的 `screenshot` 字段)——带操作横幅/元素框/点击点,比重新 `refresh` 一张信息量大

### 2026-08-13 · `tke fix`:一条命令补齐运行依赖(ADR-0012)
- **feat** `tke fix` 从分发源补齐 chromedriver / Chrome for Testing / adb(+aapt+libc++.so) / go-ios。`--profile web|android|ios|all`、`--check` 只看不下(缺东西时**退出码非 0**,CI 可判)、`-y` 免确认、`--base-url` 换源
- **decision** **下载只在这条命令里发生**,普通命令缺依赖只报错指路。一条 CLI 命令突然静默拖 600MB,在内网/离线/CI/按流量计费的机器上都是灾难,企业还有合规问题——**要不要下是使用者的决定**(用户拍板)
- **fix(误导报错)** 缺 chromedriver 时先跑 `fetch` 会报「无活动浏览器会话,请先执行 启动」——**指错方向**,让人撞第二堵墙才看到真原因。现在先分清"还没启动"和"根本装不了"
- **fix(误导报错)** 缺 Chrome 时只报 `session not created (日志: …)`,完全看不出缺的是浏览器本体。现在检测到没有 Chrome for Testing 就补一句说明 + `tke fix --profile web`
- **fix(自己犯的假成功)** 第一版 Chrome 解压失败(zip 关了 deflate 特性),但**半个解压出来的目录留在那儿**,复验只看目录存在就报「✅ 补齐了」+ 退出码 0。判据已改成**可执行文件在不在**,且解压失败会清掉半成品。**一路在防的假成功,自己犯了**
- **fix** `zip` crate 补 `deflate` 特性——Chrome 官方包是 deflate 压的,只留 stored 会报 "Compression method not supported"
- **refactor** 新增 `utils::deps`:Chrome 路径(`CHROME_REL`)与工具探测**驱动层和 fix 共用一份**。各写一套会出这种怪事:fix 说装好了、驱动却找不到
- **choice** 下载走 `curl` 子进程而非 Rust HTTP 客户端:reqwest 是 `ocr-online` 的可选依赖,CI 的 `--no-default-features` 构建里没有,而 fix 必须在任何构建下都能用;tke 本来就是"调外部工具"的架构
- **fix** 手写的 `cli/help.rs` 又漏了新命令(P-16 同款),测试抓住
- **实测** 空目录只放一个 tke → `tke fix -y --profile web` → chromedriver 20MB + Chrome 600MB 装齐 → **用装出来的环境真跑通一次网页检查**;android 那套也验了(含顺带的 aapt/libc++.so)、幂等、非交互不确认不下载
- **测试** lib 54/54 + CLI 契约 19/19(新增 3 条)

### 2026-08-13 · 报告三连:点了什么 · AI 写的评语 · 相关文件按钮
- **feat** **「点了什么」**:脚本里写的是 `点击 [{299, 242}]`,光看这行没人知道点的是啥。报告从**执行时的页面结构**反查该坐标命中的元素(取**最内层**那个),展开可看 class/text/resource-id/xpath/范围/可点击性,来源带平台前缀(web/android/ios)
- **fix(会悄悄标错)** 反查必须用**上一步**的 xml——每步存的是**动作执行后**的页面(点完早跳走了),拿本步的查会把"点了什么"标成"点完到了哪"。专门加测试钉住
- **feat** **点空必须说出来**:坐标没命中任何元素 → 红标「点了个空处,这一步多半没起作用」。tke 本身仍报 success(驱动层不校验),这是眼下唯一能拦住这种**假成功**的地方
- **feat** **`.tks` 支持行内注释** → 成为报告里的「这一步在干什么」。`点击 [{927,112}] # 点保存,验证会落库` 原样进报告。**写指令的 AI 是当时唯一知道意图的人**,不写下来这信息就永远丢了
- **fix(会切坏指令)** `#` 只在**引号外**才算注释:URL 锚点 `"https://x/#/list"`、文本 `"话题#标签"` 都是数据;且要求 `#` 前面是空白(`KEYCODE#1` 不算)。5 个测试钉这几种
- **change(自省)** 一度写了套**规则生成的评语**("点击了链接「X」"),被用户否掉——定型文不够灵活,更坏的是**让人以为读懂了其实没有**。改成只显示 AI 真写的那句,没写就不占位置
- **feat** 顶部「相关文件」从一行链接改成**一排按钮**(查看原始日志/截图序列/页面 XML/打开执行目录),文案说「点了会看到什么」而不是裸文件名;删掉页脚两句废话
- **feat** 顶部补:设备、脚本路径、起止时刻、run_dir;chips 改成通过/失败/AI找回/总步数/耗时(失败与 AI 找回只在非零时出现)
- **feat** `ExecutionResult` 加 `device`、`StepResult` 加 `note`(都是可选字段,App 的 NDJSON 消费不受影响)
- **测试** lib 52/52 + CLI 契约 16/16;样例 https://cloud.test-toolkit.app/sl/preview/guest/test/AI_Reference/tke-report-sample.html

### 2026-08-13 · `--log` 时自动生成人看的 `report.html`
- **feat** 一次运行的 log.json + 截图序列缝成**一个自包含 HTML**:顶部结论(通过/失败·N/N 步·耗时)+ 每步命令/成败/耗时/报错/标注截图。`steps` 与 `run` 共用这条路径,**不用加任何参数**
- **取舍** 截图 **base64 内嵌**(单文件发给同事/贴工单也能看图,人最需要的);页面结构 xml **不内嵌**(动辄几百 KB、只有 AI 排障才看),留相对链接、原目录打开可用
- **取舍** CSS 全内联不引 CDN——离线/内网/断网 CI 里打开都一样;支持 `prefers-color-scheme` 深色
- **fix(自省)** 报告生成失败只 `warn` 不中断:证据本体(log.json/截图)已经落好了,不该因为报告生不出来把整次运行判失败
- **测试** 4 个单测挑的都是**会悄悄坏掉**的地方:HTML 转义(一个带 `<` 的报错就能把报告打歪)、失败信息必须出现(INV-9)、截图读不到时报告照出、汇总数字正确
- **样例** https://cloud.test-toolkit.app/sl/preview/guest/test/AI_Reference/tke-report-sample.html

### 2026-08-13 · skill 默认装用户级 + 证据默认落 `~/.tke/logs`
- **change** `install.sh` 默认 `--user`(`~/.claude/skills`,装一次所有项目通用),`--project` 才装进当前仓库。此前反过来——每换一个项目就得重装一次(用户提)
- **change** 证据默认落 **`~/.tke/logs/<任务简称>/steps_<时间戳>/`**,不再往被检查的项目里写。它是一次性检查的过程产物,**不该混进人家仓库、也不该逼人加一条 `.gitignore`**。时间戳子目录 tke 自动建,AI 只给任务简称
- **docs** 同时给 AI 留了改写口子:证据要跟 PR 走 / 工具链只能读项目内文件时,改用 `--log .tke-ui-test/`,**那时才提醒用户加 `.gitignore`**
- **feat** `check-env.sh` 新增「证据落点」一段,直接报 `~/.tke/logs` 及已有几次记录——人找证据不用问 AI
- **实测** `~` 展开正常、目录自动创建、体检计数正确;两个脚本 `bash -n` 过

### 2026-08-13 · skill 更名 ui-check → **tke-ui-test**（用户定名）
- **breaking(分发)** 目录、frontmatter `name`、斜杠命令、分发包名全部改:`skill/tke-ui-test/`、`/tke-ui-test`、`skill/tke-ui-test.tar.gz`。**三者必须一致**才认得出
- **fix** `install.sh` 装完自动清除旧的 `ui-check` 目录——不清的话两个 skill 同时在册、description 几乎一样,AI 会乱挑、用户也看不出该用哪个
- **注意** 云上 `skill/ui-check.tar.gz` 是旧路径,下次 publish 会传新名;**老包不会自动消失**,需要时手动删
- **实测** 三个脚本 `bash -n` 过;本机重装 + 体检全绿(tke 0.7.3-beta / chromedriver 151 / Chrome 就位)

### 2026-08-13 · skill 拆出踩坑册 + 澄清「不产 .tks」
- **docs** 新增 `reference/pitfalls.md`(C-1~C-10):**专收"会得出假结论"的坑**——不是跑不起来,是跑起来了结论是错的。主文件只留"怎么做",坑册收"为什么会错",**新踩的坑往坑册加、不再撑大 SKILL.md**(用户提)。SKILL.md 214 → 173 行
- **docs** `reference/tks-syntax.md` → **`steps-syntax.md`**:旧名暗示"要写 .tks",而这个 skill **只把指令当 `steps` 的命令行参数用**,不产脚本资产、不建元素库——产可回放脚本是 `tke harness` 的活,**两个东西**(用户强调)
- **fix(误导)** steps-syntax 里原写着"等异步结果**必用**重试断言"——但断言的目标必须是元素、**需要元素库**,skill 里根本用不了。改成指向 shell 轮询 + 坑册 C-1/C-3
- **docs** README 补**斜杠调用**:`/ui-check <任务>`;斜杠名 = 目录名 = frontmatter `name`,三者一致才认得出。装进 `~/.claude/skills/` 后**当场生效,不用重启会话**(本机实测:拷进去后 skill 立刻出现在可用列表里)

### 2026-08-13 · skill 补跨设备检查（在 A 上做，去 B 上验）
- **docs** `SKILL.md` 新增「跨设备检查」一节,针对"平台建场景 → 手机 App 验收"这类真实需求。此前只有两句话「按语义分别指定、不确定就问」,不够用
- **fix(重要)** **轮询找内容必须用全量 `fetch`,不能加 `--interactive`**——要验收的名字往往是**不可点击的文本标签**(标题/列表项文字),`--interactive` 只输出可点击元素会漏掉,于是等到超时、报假失败。我第一版片段就是这么写错的,实测才发现(example.com 的 `Example Domain` 正是只在全量里)
- **docs** 另外四条都是"别骗自己"类的:①**先验起点**(动手前确认 B 上还没有,否则看到的可能是旧数据=假成功)②轮询要有超时且**退出后必须判断有没有命中**(否则"循环跑完"被当成"通过")③手机侧**先下拉刷新**(App 只在进页面时拉一次,别把没刷新当成没下发)④**验"能用"不只是"能看到"**(点进详情、执行一次,只验列表有这行是漏检)
- **docs** 平台侧登录:tke 的 web 会话是独立 Chrome 实例、**不共享用户日常浏览器的登录态**,又不许代登 → 停下来让用户在那个有头窗口里自己登,**中途别 `control close`**(会连登录态一起销毁)
- **背景** `.tks` 的重试断言(`断言 [{元素}, 存在, 15s]`)**在 skill 里用不了**——它需要元素库,而 skill 明令不建元素库。所以跨设备等下发只能用 shell 轮询。是否给 tke 加个不依赖元素库的 `fetch --wait-text` 待定(Q-8)
- **实测** 本机 Linux + web 验了轮询片段的正反例:命中即刻退出、未命中如实报未出现

### 2026-08-13 · 两件套平台自包含（Q-6 关闭）
- **feat** `tke run foo.tks` 不带 `-d` 时,从同名 `foo.tklib` 的 `meta.json` 读**录制平台**兜底:web → `device="web"`(零参数即可回放)/ android → 放行走默认 adb 设备 / ios → 仍要求显式给,但报错附上录制时的 UDID 便于对照。平台认不出或没有元素包,照旧报缺设备
- **feat** `tklib::read_meta()`:zip 随机存取只读 meta.json,不解整包;全程 `Option`——读元信息失败绝不把回放拦下来
- **背景** meta.json 里的 platform 此前**只写不读**(注释写着"给后续留钩子")。INV-7 承诺「两件套拷到别的机器直接能跑」,差的就是这一口气:脚本不记平台,而元素包早就记了
- **实测** 本机 Linux 无头:造 web 两件套 → `tke run case.tks`(不带 `-d`)→ 提示「按元素包记录的平台回放：web」→ 浏览器实跑 **2/2 步、退出码 0**。lib 39/39 + CLI 契约 16/16(新增 2 条)

### 2026-08-13 · publish.sh 默认只打 tke
- **feat** 日常发布只打 `tke + skill + install.sh + VERSION` 四个文件;驱动/Chrome 改为显式 `--with-drivers` / `--with-chrome` / `--full`。**驱动几乎不变,云上已有的不会因为没重传而消失**,每次都传纯属浪费(用户提)
- **注意** `VERSION` 仍每次必传——它是破 Cloudflare 缓存的键(P-19),不传则使用者拿不到新 tke

### 2026-08-13 · 修 shell 变量名吞中文（macOS bash 3.2 崩溃）
- **fix** `publish.sh` 在 mac 上跑到一半崩 `line 67: SRC: unbound variable`——`$SRC` 后面紧跟中文逗号,**macOS 自带 bash 3.2 会把中文字节当成变量名的一部分**。全项目扫了一遍,`publish.sh`/`install.sh`/`check-env.sh`/`build-linux.sh` 共 6 处一并改成 `${VAR}`(P-20)
- **注意** 这不是 locale 问题:我在 Linux 上 `LC_ALL=C` 都复现不出来,是 bash 版本差异。**同一个坑在同一个脚本里犯过两次**(用户此前修过 `${pkg}`,commit 1d4d5e92),所以这次加了自查命令进 PITFALLS

### 2026-08-13 · 分发上线 Toolkit Cloud + 自动更新检查
- **feat** skill 体检加**版本检查**:跟分发源比对 `VERSION` 第一行,落后就提示更新命令;3s 超时、失败静默(离线/内网照常用)。解决"skill 一直用着旧 tke"
- **fix(重要)** 安装器**逐个校验文件头**(gz 的 `1f8b` / zip 的 `PK` / 版本号以 `tke ` 开头):分发平台对**不存在的路径返回 200 + 前端 HTML**(SPA 兜底),`curl -f` 只对 4xx/5xx 生效、完全拦不住——漏传一个文件就会把网页当二进制装进去(P-19)
- **fix(重要)** **Cloudflare 缓存 4h 且不认 `no-cache` 请求头**,传了新文件使用者仍下到旧的。现在 `VERSION` 里带 `build: <时间戳>`,install.sh 先破缓存取 VERSION、再用 build 戳作为所有下载的 `?b=` 键——发新版自动破缓存,同版本仍命中 CDN(P-19)
- **feat** Linux 依赖收齐并上传:adb(platform-tools 37.0.1)、aapt + **libc++.so**、go-ios v1.3.2、chromedriver/Chrome 151.0.7922.138。**Linux 版 aapt 单独拿出来跑不了**(缺 libc++.so),但其 RUNPATH 含 `$ORIGIN`,放 tke 同目录即可——两个脚本都已带上这个依赖
- **docs** 下载路径是 `/sl/preview/<mount>/<key>`(不是 `/<mount>/<key>`,后者是 SPA 页面);平台**不支持 Range 请求**(520),别用 `curl -r` 探文件头
- **实测** 端到端全通:从云端一行安装(含 170M Chrome)→ 体检全绿 → **用装出来的 tke 实跑检查 3/3 步通过、证据齐全**

### 2026-08-12 · skill 一键安装器
- **feat** `skill/install.sh`:`curl -fsSL <BASE_URL>/install.sh | bash` 一行装齐——按平台自动取 skill 文件 + tke + 对应驱动 + Chrome for Testing,写 PATH,自动跑体检。`--profile web|android|ios|all` 按需装(只测网页就不必拖安卓/iOS 工具);`--user`/`--project` 选装到哪;幂等,重复跑只覆盖不装重
- **feat** `skill/publish.sh`:把 skill 与二进制打包成约定布局到 `dist/`,`aws s3 sync` 上去即可。**把配对好的 chromedriver 与 Chrome 放同一批**——使用者不必再去查版本对应关系,这是自建分发源最实在的好处
- **fix(自省)** 安装器最初"体检失败也照样说装好了"——已改成如实反映:环境不完整时明确列出缺什么并**非 0 退出**(INV-9 的精神,自己写的脚本也该守)
- **实测** 本地 http server 模拟 S3 + 临时 HOME 全流程验证:缺 Chrome → 警告 + 退出码 1;Chrome 就位 → 体检全绿 + 退出码 0;**用装出来的 tke 实跑一次检查流程,3/3 步通过、截图序列与 log.json 齐全**
- **docs** `skill/README.md` 补一行安装 + 分发源布局说明(维护者视角)

### 2026-08-12 · skill 补完备性(用户质疑内容太薄,属实)
- **skill** 补 `reference/tke-commands.md`(元素采集/OCR、**安卓 app focus/list 拿包名+Activity**、file 文件系统、device 信息、原生直通、排查日志位置)与 `reference/tks-syntax.md`(全部指令+参数写法+重试断言)。主文件保持精简(AI 必读),细节按需读——分发物只有 skill 目录 + tke 二进制,没有源码可查,所以必须自包含
- **skill** 主文件补关键缺口:**安卓不知道包名就查 `app focus`/`app list`,别猜**(此前完全没提,安卓场景会卡死);图标无文字用 `fetch --ocr`;体检脚本路径写明确
- **skill** 新增 `skill/README.md`(给人读的安装说明):skill 目录两种装法、tke 及**同目录依赖**(chromedriver 必须与 tke 同目录,不搜 PATH)、Chrome for Testing 按平台落点与 macOS 三个坑、验证、常见问题

### 2026-08-12 · ADR-0011 harness 侧落地：设备成为工具参数
- **feat** `explore`/`navigate`/`replay_tks`/`resume_explore`/`optimize_tks` 五个设备类工具各加 `device` 参数——**编排官按任务语义决定每一步跑在哪台上**;不传则沿用默认(`-d`/向导),单设备场景照旧
- **feat** 新增 `list_devices` 工具:枚举 Android 设备 + web + iOS 说明 + 当前默认设备。**"按语义选设备"的前提是先知道有什么**
- **feat** 交互向导加「由 AI 决定」选项;**无默认设备不再拒绝启动**(此前报「需要指定操作目标」直接退出)——编排官会 list_devices/问用户
- **feat** 无设备时调设备类工具 → 明确报「先 list_devices…拿不准就 ask_user 问用户」。此前设备落成空串、被当 Android,只得到一句莫名其妙的「adb 缺失」(INV-9)
- **prompt** 编排官提示词加「设备怎么选」一节:不确定就问**绝对不要猜**(打错设备有真实副作用)、跨设备=多次 explore 各指定 device + `save_file` 写 flow.toml 串起来、**别把多台设备塞进一个 .tks**(脚本没有设备维度、回放不了)、等异步用重试断言
- **实现要点** `AgentRunOptions::with_device()` 造设备覆盖副本(平台按新设备重新推断,否则会拿上一台的平台去操作)

### 2026-08-12 · skill 定位纠正 + 重试断言 + run 设备必填
- **skill 定位纠正（用户）**:`skill/ui-test/` → **`skill/ui-check/`**。此前把 harness 的目标(可复用资产)错塞进了 skill——去掉「先 verify 后 explore / 产两件套 / 回放验证」那一套。**skill 只做:把设备操控+查看能力交给调用方 AI,并留下可复核的证据**,是改完代码后的一次性检查手段(类比单测/API 测试)。用坐标操作,不建元素库
- **发现** 证据落盘**零改动就有**:`tke steps '点击 [{640, 380}]' --log <dir>` 即产标注截图 + 页面 xml + log.json,用坐标不需要元素库。SKILL.md 据此改用 `steps` 而非 `control`(control 什么都不留)
- **feat 重试断言** `断言 [{提示}, 存在, 10s]`——第三参数=最长等待,在这段时间内反复采页重判,一成立就通过。用于等异步结果(后台下发/跨设备同步/请求返回);不给该参数则行为不变(采一次判一次)
- **fix** 步超时对**自带时长的命令**放宽:`断言`/`等待` 的步预算 = 自身时长 + 20s。此前 `等待 [30s]` 会被 20s 步超时掐死——代码注释早写了"等待命令也放宽"但**实现里根本没有**(P-08 同类)
- **feat** `tke run <.tks>` **必须显式 -d**(用户拍板):.tks 不记平台,不给会被当 Android、web 用例只得到一句「adb 缺失」。校验放在脚本/元素包检查**之后**——文件不存在、缺两件套是更基础的问题,先报那个(测试逮住过一次顺序退化)
- **feat** flow 校验:无全局 `-d` 时,逐项检查是否自带 device,缺的直接点名报错
- **实测** 重试断言对照(页面加载 15s 后才出现提示):不带等待 → **失败**「元素不存在」3.2s;带 25s 重试 → **通过** 10.9s。同时验证步超时放宽生效(25s 断言没被 20s 掐死)
- **test** lib +2(flow TOML 解析)、CLI 契约 13→14(run 设备必填);serialize 往返样例加重试断言

### 2026-08-12 · flow 支持跨设备（per-script device）
- **feat** flow 的 `scripts` 每项可指定设备:`{ path = "a.tks", device = "phoneA" }`,不指定则沿用全局 `-d`;纯字符串列表**完全向后兼容**。跨设备测试的表达方式定为「一个 .tks = 一个设备上的一段流程(两件套自包含,INV-7),跨设备 = 串成 flow」——用户场景:A 手机改设置 → B 手机验收 / web 后台下发 → 手机端看
- **fix** flow 收尾清场改为**按设备分组**:此前只按全局 `-d` 清一台,跨设备时其余设备会留下孤儿浏览器/App
- **adr** `ADR-0011`(**提案,待拍板**):设备从「会话级全局」降为「工具级参数」,harness 启动不再强制选设备、AI 按任务语义选、不确定问用户。关键取舍:**一次 explore 仍只跑一个设备**(explorer 内部零改动),跨设备靠编排官多次调用 + flow 串——因为**回放层没有设备维度**,多设备混合脚本回放不了
- **test** flow TOML 两种写法解析 + 老格式兼容(单测 2 条)

### 2026-08-12 · `control close` 可省包名（web）
- **feat** `tke -d web control close` **省略包名即销毁会话**（浏览器 + chromedriver + 会话文件 + 孤儿收割）——替掉此前要人手敲的 `rm -f $TMPDIR/tke/web/*.json` + `pkill Chrome`。web 分支本就忽略这个参数（`Controller::stop_app` → `close_session`），只是 CLI 强制要求填一个没意义的值
- **feat** 移动端省略包名 → 明确报错（不拿空串去 force-stop）
- **test** CLI 契约 +2（11→13）;文档里的手工清理命令全部替换

### 2026-08-12 · 无头坐标可移植性**已验证** + 两个真机撞出的修复
- **✅ 关键结论(用户 mac 实测 + 本机 Linux 对照)**:`mac 有头 = mac 无头 = Linux 无头 = 1280x813`,
  元素 bounds `diff` 零差异。**像素坐标跨模式、跨平台可移植——「本地录、CI 回放」成立**。
  这是 skill/CI 路线最大的未知,现已结掉
- **fix** 会话跨命令复用导致 `--headless` **静默不生效**(P-18):`SessionInfo` 增记 `headless`,
  复用前比对,模式不符则销毁旧会话 + 明确报错要求重新 launch。
  用户对照实验正是被这个坑出**假阳性**(两次结果一致其实是同一个浏览器)
- **fix** `--platform web` 不连带定 device → 下游按 Android 推断报「adb 缺失」(用户实测发现)。
  web 是唯一「设备 id 就是平台名」的端,现补成 `device="web"`,与交互式向导那条路径拉齐
- **验证** `tke harness` 在 mac 上跑通(有头,2 轮出两件套);无头 harness 待用户重跑

### 2026-08-12 · skill 模式落地 + 无头实测通过
- **adr** `ADR-0010` 生效(用户拍板):**skill 借调用方的 AI**——Claude Code 直接调 tke 原子命令,tke 退回成设备操作原语 + 证据产出器。**`tke task`(ADR-0009)取消**,该 ADR 标为已被取代(一行代码没写过)。`tke harness` 内置 AI 保留(App/纯 CLI 用户),两条路并存
- **skill** 新增可分发原型 `skill/ui-test/`(SKILL.md + check-env.sh):先 verify 后 explore、主循环用 `fetch --interactive` 文本元素表省 token、结束条件=`tke run` 回放通过(硬证据)
- **fix** `element add --lib foo.tklib` **包不存在时建新包**——此前 .tklib 只有 harness finalize 会造,用原子命令攒两件套第一步必失败(P-17)
- **实测(本机 Linux/amd64 无头)** 全链路通过:装 Chrome for Testing + chromedriver 151.0.7922.138 → 无头启动/采集/操作 → 落库建包 → 写 .tks → **`tke run` 5/5 步通过、退出码 0**;标注截图(横幅+红框+蓝点)、log.json、page/*.xml 齐全,**无头下中文渲染正常**
- **实测数据** 无头截图 **1280x813**(window-size 1280x900 减去 87px 浏览器 UI 高度,说明 headless=new 在模拟真实窗口)。**有头对照本机做不了**(无 DISPLAY、无 xvfb)——待 mac 上跑同样命令比对
- **未验** `tke harness` 的完整无头探索(需 `[ai]` key,本机无)。但 harness 与 run/原子命令**共用同一条 `WebDriver::start_new_session`**,驱动层无头已验
- **发现** 记 Q-6:`.tks` 不记平台,`tke run foo.tks` 不带 `-d` 按 Android 推断 → web 脚本报「adb 缺失」;而 tklib 的 meta.json 已存 platform,「拷走即跑」还差这一口气

### 2026-08-12 · web 无头支持（为无头服务器 / docker CI 铺路，**真机未验**）
- **feat** `--headless=auto|on|off`(全局参数 + config `headless`)。**auto 默认**:mac/win 恒有桌面;Linux 看 `DISPLAY`/`WAYLAND_DISPLAY`,都没有 → 无头。无头用 `--headless=new`(完整渲染路径,与有头一致;老实现的精简渲染截图对不上)
- **feat** 容器/root 自动加 `--no-sandbox --disable-dev-shm-usage`(探测 `/.dockerenv`、`/run/.containerenv`、uid==0);普通桌面保留沙箱
- **fix** `find_chrome_binary` 此前**只认 mac-arm64 硬编码路径**,Linux/Windows 上永远找不到 Chrome(只能回退系统 Chrome、版本可能不配对)。改为跨平台:搜索根=tke 同目录 + `<data_dir>/tke`,相对路径按 Chrome for Testing 官方 zip 原样结构(解压即用,便于自建 S3 镜像)
- **fix** `env_clear` 保留列表补 `DISPLAY`/`WAYLAND_DISPLAY`/`XAUTHORITY`——Linux **有头**模式下 Chrome 靠它们连图形栈,清掉直接起不来(P-15;mac/win 不看这些所以一直没暴露)
- **fix** `--headless` 裸旗标会吞掉后面的子命令(`tke --headless run x.tks` 里 `run` 被当成值)——加 `require_equals` + `value_parser` 白名单。`--copilot` 踩过同类坑,这次由**黑盒 CLI 契约测试当场逮住**(P-16)
- **test** 单测 4 条(HeadlessMode 解析/定案)+ CLI 契约 4 条(帮助登记/裸旗标不吞子命令/无效值明确报错/off 可接受);lib 32→36
- **注意** 手写帮助(`cli/help.rs`)不会自动收录新参数,靠 `help_lists_headless` 契约测试兜住
- **未验** ①有头录/无头回放的**像素坐标是否一致**(决定"本地录、CI 回放"成不成立) ②docker 系统库与中文字体(下载器解决不了,得靠 Dockerfile)

### 2026-08-12 · ADR-0009 拍板生效
- **adr** ADR-0009 提案 → **生效**（用户拍板）:headless 一次性模式命名定为 **`tke task`**(顶层命令,非 `harness --headless` 旗标——两者出口语义与 `ask_user` 行为不同,做成旗标会让"会不会阻塞问人"取决于一个 flag)
- **不变量** INV-3 补延伸条款:「对话层」不限定必须是 tke 自己的 REPL,外部 agent 调用时调用方即对话层;**headless 一旦自行决策即违反 INV-3**。这是本 ADR 的失效红线,写进不变量当锚点
- **状态** 契约已定、**实现未开始**;下一步做阶段 0(零改动包 `tke run`)还是直接阶段 1(`tke task`),待用户定

### 2026-08-12 · Linux 构建脚本
- **build** 新增 `build-linux.sh`:依赖预检(cc/cmake/pkg-config,缺了直接给 apt 命令)+ `--no-ocr`(走 `--no-default-features`,CI 用,跳过 tesseract 源码编译)+ `--quiet`;去掉 mac 专属 codesign,但保留「先删后拷」——Linux 上的理由是覆盖运行中二进制会 ETXTBSY(与 P-02 同做法不同因);产物 `--version` 跑不起来就 exit 1
- **实测** Linux/amd64 两条路径都通过:`--no-ocr` 9m33s / 28M;完整(含 tesseract) 3m17s / 34M(**注意:这是 tesseract-rs 已在 cargo 缓存里的增量耗时,冷机首次会久得多**);两者版本号注入均正确、落点 `bin/linux-amd64/tke`
- **实测** OCR 门控对照:`--no-ocr` 产物调 `tke ocr` 报 `ocr-offline feature not enabled`,完整产物报图像解码错——证明 feature 确实生效;两者都是明确报错 + 退出码 1(不静默,INV-9)
- **订正** 此前"没有 Linux 构建"的说法不准:`build-mac.sh` 的 case 本就有 Linux 分支,真正缺的是命名可发现性、依赖预检、CI 跳 OCR 开关

### 2026-08-12 · skill 集成设计（只有文档,无代码）
- **docs** 新增 `docs/skill-integration.md`:tke 作为 skill 融入 coding agent(Claude Code)工作流的设计稿——verify/explore 两动作分离、intent 意图契约、report 硬软证据分级 schema、skill 布局与安装、四阶段路线;首版范围 Web+Android
- **adr** 新增 `ADR-0009`(**提案,待拍板**):headless 一次性任务模式 `tke task`——五态出口+退出码,决策点不静默降级而结构化回传给调用方(调用方 agent 即 INV-3 所说的"对话层")。背景:Plain 前端 `supports_prompts()=false` 但 `await_answer` 仍阻塞读 stdin,非交互下属未定义行为
- **待办** 记入 ROADMAP:Linux 构建脚本缺口(现只有 mac/win),skill 若要落到 Linux 开发机/CI 需先补
- **test** 新增 `tests/cli.rs` 黑盒 CLI 契约测试(7 条:--copilot 裸旗标回归/两件套缺包/JSON error 契约等,spawn 真二进制,秒级)+ `tests/e2e/smoke.sh` 真机冒烟(需设备手动跑);测试三层放置定稿 ADR-0008;pre-push 纳入 CLI 契约测试
- **docs** 整理:`tke-flow.md` 更新到当前架构(去医生/repair_tks,补 resume_explore/navigate/页面契约/run 辅助驾驶);`codebase-map/refactor-plan/tke-overview` 归档进 `docs/archive/`;新增 `docs/README.md` 导航;引用同步

### 2026-08-12
- **治理** 落地项目治理体系:INVARIANTS/PITFALLS/ADR(0001-0007 补录)/state 三件套/ROADMAP/CHANGELOG/守卫脚本+hook;AGENTS.md 改造为路由+协议入口

### 2026-07-13
- **feat** `7c4138c9` 起始态对齐输出瘦身（compact 前端/顶格/分段空行）
- **fix** `db33162c` 辅助驾驶设备缺省不再静默失效（INV-9 的由来之一）
- **feat** `b7e30a2d` tke run 起始态对齐——开跑前导航回起始页,失败拒跑
- **feat** `75842dda` tke run AI 辅助驾驶——定位自愈两段分诊,不改脚本资产（ADR-0006）

### 2026-07-06 → 07-12（修复重建线,详见 ADR-0001/0004/0005）
- **refactor** `57ed54e7` 删除医生 agent,修复重建为断点续探
- **feat** `1f6d49b1` 定位级自愈 + workarea 并发竞态修复
- **feat** `1e58b91a` 页面契约:「断言页面」指令统一起始/终点校验
- **refactor** `87ef690a` 单次 agent 全部迁到 oneshot 强制工具调用
- **refactor** `181246bc` 删一键黑盒 repair,修复决策交编排官
- **feat** `a8fe8c07` navigate 导航原语;`e27eb6e8` replay 失败报告带逐步轨迹
- **feat** `8ac12699` explorer 提问经参谋中转 + 卡住升级梯度
- **fix** TUI 手写 inline 定稿系列（`07231866`→`46475c53`,ADR-0007）
