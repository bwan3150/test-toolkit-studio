---
Last-Updated: 2026-08-27
Last-Commit: 40d35c22
---

# 当前状态

## 大局

北极星 = 测试领域专精的 Claude Code（ADR-0002）：tke 是能操作 Android/iOS/Web 的设备 AI agent,
探索产 .tks+.tklib 两件套,可无 AI 回放,回放坏了由编排官编排修复。
Electron App（studio）只是 tke 的外围封装——**当前主线只做 toolkit-engine**（用户拍板 2026-07-13）。

**新主线（2026-08-26 拍板）：服务化**。tke 要能被远程调用——测试服务器上部署 tke + 模拟器/真机/
无头浏览器，云平台租设备下发任务或交互式探索、脱手收报告；再出两条 remote skill 给别家 coding agent
（本地零安装，未来进 CI）。

**2026-08-27 状态：跑通了。** 平台侧（`~/Documents/GitHub/TOOLKIT/bug`）已经能
建 case → 挂两件套 → 组 suite → 下发到云设备 → 回写用例结果 → 产物落对象存储，
全程实测（3/3 回放通过、各 14 个产物）。内网机器走反向通道（ADR-0024）接进来，
真实的公网平台 + 内网 Arch 机器之间验过。

## 各能力线状态

| 能力线 | 状态 | 备注 |
|---|---|---|
| 编排官 REPL + 颗粒化工具 | ✅ 真机验过 | ADR-0002 |
| 探索（explorer+asserter+supervisor） | ✅ 可用 | 质量债见 ROADMAP |
| 修复（自愈+断点续探+页面契约） | ✅ 真机验过"效果还可以" | ADR-0001/0004 |
| TUI 手写 inline | ✅ 定稿 | ADR-0007 |
| tke run 辅助驾驶（对齐+自愈+分诊） | 🟡 对齐链真机通过;分诊层2-5 真机未逼出 | ADR-0006 |
| 无设备测试层（FakeLlm/FakeDriver） | ✅ 36/36 + CLI 契约 11/11 | 秒级 CI 回归 |
| web 无头（无头服务器/docker CI） | ✅ **坐标可移植性已验证** | mac有头=mac无头=Linux无头=1280x813,bounds 零差异。会话模式失配已修(P-18) |
| App 侧（handlers/frontend） | ⏸ 冻结 | healed 字段等新 NDJSON 待接 |
| **skill 已上线可用** | ✅ 用户实测过一轮并已调优 | 一行装:`install.sh`/`install.ps1`;六平台分发齐备;CI 自动发版。**用户实测反馈已消化**(语义定位/选择指令/提速 6.5×),**效果待他重跑验证** |
| **语义定位链路** | ✅ 本机 web 实测打通 | 2026-08-15:实跑撞出四个洞并全修(P-28 幽灵 sr-only / 可及名称缺采 / 误导错误 / P-30 视口+文档坑)。修完同一批 **3 步纯语义、零坐标、零 fetch**。**安卓侧未验**(无设备) |
| `fetch --wait-text` | ✅ 本机实测 | ADR-0013,关闭 Q-8。出现即返回/超时非零退出;skill 里的手写 shell 轮询范例全部替换。**跨设备真机待验** |
| **浏览器默认无头 + 凭据脱敏** | ✅ 本机 web 实测 | ADR-0015:有头抢鼠标 → 默认无头;`Auto` 不算显式要求(否则手动登录的会话会被销毁);密码框的值在**采集/命令/报告/截图横幅**四处一律 `••••••`。**移动端密码框判据待真机验** |
| **证据组织(一个任务一份)** | ✅ 本机实测 | 用户反馈"组织方式很乱"后重做:`--log` 就是任务目录、反复调用续写、步骤连续编号、`pages/`、报告默认压缩内嵌自包含(1.7MB→598KB)。**只改 steps**,run/flow/harness 维持时间戳目录。跨设备不再分目录 |
| **自我管理 `tke update`/`uninstall`** | ✅ 本机实测 | ADR-0014 补充:**就是去跑官方 install.sh/uninstall.sh**;Unix 用 **exec 把自己替换掉**(放手,不占着自己的二进制),Windows spawn+改名兜底;**先验文件头再执行**(不用 `curl \| bash`,P-19 会喂 HTML 给 bash) |
| **doctor 报告排版** | ✅ 本机实测(Linux) | 用户说"信息太多而且顺序混乱"→ **三段分组**:①工具本身(平台/依赖/Engine/Skill 版本)②能测什么(四端+显示器,真机在前)③落点(Engine/Skill/日志);标签列按显示宽度对齐、值统一 `状态 (补充)`;**正文一条命令都不留**,该敲的全收进末尾「下一步」块;**上色是例外**(正常不上色;绿=有更新/红=缺依赖/灰=查不了——"不要绿色泛滥"),队形三条:标签一律 dim、值上色、补充永远 dim。设备探测改走 `tools::discover`(与 `device list` 同一套)。排版拆到 `cli/doctor.rs` |
| **版本新鲜度 `tke doctor`** | ✅ 本机实测 | **Q-11 已关闭**(ADR-0014):`tke fix`→`tke doctor`(fix 保留别名);比 **build 戳**而非版本号;`steps` 每批提醒(缓存 4h/stderr/--json 闭嘴);只提醒不自更新。⚠️ **要等下一次发布把 VERSION 打进 skill 包后才对用户生效** |
| skill（给 AI 设备操控+证据） | ✅ 可用,**跨设备待用户 mac 实测** | **ADR-0010**。**只做一次性检查+留证据,不产 .tks/.tklib、不回放**(与 harness 是两个东西)。`skill/tke-ui-test/`:主文件精干 + `reference/pitfalls.md` 踩坑册(新坑往里加,别撑大主文件)。`/tke-ui-test` 斜杠可调 |
| **iOS 模拟器** | ✅ **端到端跑通并已分发**;**多台并行已实测通过**(2026-08-20) | ADR-0017。`-d sim:<UDID>`,与真机**同一套 WDA**,只有"怎么连上"不同(真机 USB 隧道 / 模拟器直连本机端口,**一台一个**,Q-13 已关)。`doctor --fix --profile ios` 下预编译 runner(21MB,arm64+x86_64 fat 包)到 `~/.tke/wda/`,tke 自己 `simctl install/launch`——**不碰 Xcode、不装 brew、不编译**。路线拐过一次弯(WDA→idb→WDA),原因见 ADR-0017「修订」 |
| **设备发现 `tke device list`** | ✅ 本机+用户实测 | 四端统一(安卓/iOS真机/iOS模拟器/浏览器),第一列就是 `-d` 的值;**查不了的那类单独说明原因**("没装 adb 是没查不是没连")。harness 向导也改走它——同一个问题不该有两套答案 |
| **报告可读性** | ✅ 本机实测 | 手机竖屏图限高 56vh + **点击就地展开**(纯 CSS,不引 JS);图下方三个链接:原图/元素表/**原始页面**;设备栏显示友好名。`--summary -` **从标准输入读**(heredoc 一步给长 Markdown,不用再写临时文件) |
| **安卓模拟器(AVD)** | ✅ **Linux amd64 端到端实测通过**;mac 待验 | **ADR-0018:选装**——iOS 模拟器 macOS 自带,而这套要 1GB 上下(emulator 包 351~490MB + aosp_atd 镜像 450~860MB),安卓真机又很好开。**不分发、不进依赖检查**,doctor 只写「未安装（选装）」。`-d avd:<名字>` + `启动环境` 起它(等 `sys.boot_completed`),起来后就是普通 adb 设备。**装**走 `doctor --fix --profile android-emu`——**从 Google 官方源下**(SDK 许可 3.4 禁止我们转发),不需要 JDK;解压写成**稀疏文件**(不然 system.img 实占 8.1GB,现在 1.1GB);卸载 `uninstall --all`(约 2.5GB)。**镜像用 `default` 不用 `aosp_atd`**(后者关了硬件渲染,截图恒为纯色);**`-gpu swiftshader` 不能带 `_indirect`**(P-47,截图纯色)。实测链路:装→起(61s)→装App→启动→采集→按文字点击→页面跳转→证据落盘→关机。**Google 不发布 linux-arm64 emulator**,那一档只能用 redroid |
| **环境起停 boot/shutdown** | ✅ 本机实测 | `control boot [--headless=off]` / `control shutdown`;tks `启动环境 [有头]` / `关闭环境`。**boot 管环境本身,launch 管环境里的东西**——早先浏览器是被第一条 web 命令顺带起来的,脚本里看不出它何时起、以什么模式起。iOS 模拟器 `simctl boot` + bootstatus 等就绪;安卓模拟器见下一行(ADR-0018,选装) |
| **iOS 密码脱敏** | ✅ 已修(P-45) | XCUI 归一化**从来没输出 password 属性**,于是 iOS 上密码明文进 log/报告/截图横幅——而注释还写着"三平台同一条路"。安卓原生有、web 已对齐,**唯独 iOS 漏了整整一个平台**。两个单测钉住 |
| **设备显示名** | ✅ 本机实测 | `Controller::describe()`:`iPhone 17 Pro · iOS 26.0（模拟器）`/`Pixel 7（安卓 14）`/`Chrome（无头）`。报告显示它而不是 UUID;**换设备的判断仍用设备 ID**——同型号模拟器 label 会撞(单测钉着) |
| 跨设备/跨平台测试 | ✅ 已实现,**AI 侧真机未验** | ADR-0011 全套:flow per-script device / 重试断言 / 设备成为工具参数 + list_devices;动态值传递未做(Q-7) |
| 宿主机能力门禁 | ✅ 本机实测 | iOS 只在 macOS 放行(门禁在 `Controller::new`,control/run/steps/harness 一处覆盖);留 `TKE_ALLOW_IOS=1` 逃生口——界线是产品决策不是技术极限 |
| **CLI 直通** | ❌ **已删除**(2026-08-19) | ADR-0016(用户拍板):它是操作设备的第二条路,绕过证据留存/坐标换算/唯一动作映射。保留 `ToolManager::resolve`(内部定位 adb/chromedriver/go-ios)与 `tke <path.tks>` 便捷路由;删前盘出唯一缺口 logcat,补了 `tke app log` |
| 分发源六平台齐备 | ✅ 依赖全 / ⏳ 二进制待 CI | 依赖六平台已手工补齐(linux-arm64 只有 go-ios、win32 没有 go-ios——上游就没有),**一次性活不再动**;tke 二进制只有 mac-arm64+linux-amd64,darwin-amd64/windows 等 CI 跑 |
| 依赖补齐 `tke fix` | ✅ 本机端到端实测 | ADR-0012:唯一会联网下载的命令;普通命令缺依赖只报错指路。空目录只放 tke → fix → 跑通网页检查 |
| 两件套自包含（拷走即跑） | ✅ 本机实测通过 | Q-6 关闭:缺 `-d` 时从 tklib 的 meta.json 读平台兜底(web 零参数回放/android 走默认设备/ios 仍需显式) |
| **tke security（安全测试新领域）** | 🟢 **从零到上线全做完·CI 已发版**（primitive+prober/analyst/reporter+对话 TUI+task/report 生命周期+skill+深挖 playbook+一行装全 skill）;🟡 对话式 TUI 交互手感真机待验(Q-15) | **ADR-0019** + INV-13/14/15 + `security-report-spec.md` + 基线 `security-report-template.sample.html`。第二个 agent 领域,骨架复用 harness。**唯一入口 `tke security [url]`**(改对了:曾错做成 `security probe`/`security run` 子命令,用户纠正——`run` 是 .tks 语义):**默认对话式编排**(复用 harness `Frontend`/TUI),**`--json`/非终端→无头一次性**(探测→复核→出报告)。三层:`tke http`/`tke recon` primitive(可脚本,7 verb) ⇄ AI 工具 ⇄ orchestrator。强度阶梯 passive/safe/aggressive/red-team(默认 safe)+`--focus`。**已落**(`src/workflow/security/`):http 原语 + `HttpEngine`(Ureq+Fake 可脱网测) + `evidence.rs`(续写不覆盖,INV-14) + recon 七 verb(bundle 密钥脱敏/endpoints 防 SPA 假阳/tls 轻量) + **prober**(自主顺藤,去重+无进展强制收尾,真机死循环已修) + **analyst**(对抗复核,oneshot 强制结构化,毙假阳分软硬,INV-13) + **reporter**(确定性出 `security-report.html`+`findings.json`+每确认漏洞 `vuln-*.html`,转义防注入) + **orchestrator**(对话 REPL:recon/http/record_finding/ask_user/report/finish,说话即交回话筒)。提示词 `security/prompt/`(builtin+外部覆盖)。冒烟脚本 `tests/security-smoke.sh`;样例 `examples/security_report_sample.rs`。全量 137 绿。**真机实测**:P1 七 verb + 无头 `security --json` 都在 konechome 跑通出报告。**待验**:对话式 TUI 交互真机手感。**待做**:`--json` 与 Electron 联调、注入子系统(opt-in)、源码灰盒、endpoints 吃 OpenAPI、tls 深度证书、report 里方法/边界/风险矩阵等区块补全 |
| **服务化 / 远程 API（P1+P2+P3）** | ✅ **已落地并真机实测通过**（web 9/9 + 安卓真机 8/8）；P2 起未做 | **ADR-0022** + **INV-16/17** + `docs/remote-api.md`（§11 = P1 实况）。`tke serve` 单节点：9 个端点（hello/health/devices/sessions×4/exec/artifacts/workspace），**没 token 只准绑回环**。`src/serve/`：allowlist（三道关）/ lease（独占+隔离+TTL+复位计划）/ exec（子进程+注入+分层计时）/ routes（鉴权走中间件）。**测试三层**：单测 30 + 黑盒接口 10（`tests/serve.rs`，起真二进制发真 HTTP，**不需要设备**）+ 真设备 e2e（`tests/e2e/serve-smoke.sh`）。**守卫** `check-serve-paths.sh` 已挂 pre-commit，写它当天抓到两个真洞（`refresh --out`、`control browser-download --dir` 能读写工作区外）。**单测逼出一个真 bug**：acquire 顺手 retain 掉过期租约 → 设备绕过复位给下一个租户（违反 INV-17），已改成"复位完才回池"。**量了**（Q-17）：进程启动 0～1ms、占比 <0.1%，耗时全在设备 → D2 子进程模型的重审条件没出现。依赖只多 5 个 crate。**有意没做**：`tke file` 不进白名单（宿主路径与设备路径混在一起）、TLS 交反代、fake 驱动跨进程状态。**P2 也已落地**（`docs/remote-api.md` §12）：`TKE_REMOTE` 一设，命令行一个字不改就发到节点（拦截在 clap 之前）；`-d` → 租哪台、`--log` → **两边同一个相对路径**（转发+拉回，实跑安全轨撞出来的：吃掉它 `tke report logs/scan` 就找不着东西）；stdout/退出码原样透传；隐式会话落盘复用；`tke remote status|open|close|pull|devices|push`。**无设备会话**（`platform:none`）——http/recon/report/task 不碰设备就不租、**不计设备时长**（ADR-0022 D3 的直接推论，写 security remote skill 时才发现）。**两条 remote skill 是生成的**：delta（连接+覆盖表）+ 本地版正文逐字节内联，结构上不可能漂；四个包已进 manifest 默认全装（Q-18 结论：真正要分叉的只有 4 个话题）。全量 **251 绿**；本机实测 web/安卓真机/安全轨三条都通。**P3 任务层也已落地**（§13）：`POST /v1/tasks`(202) → 子进程跑 `harness --json`/`security` → SSE(先重放再实时)/WS(桥 JsonFrontend 双向 NDJSON)/report/webhook。**五态出口复活**(ADR-0009 条款)：passed(0)/failed(1)/needs_decision(2)/blocked(3)/error(4)；**headless 见 `awaiting_input` 立刻终止回传问题**(D6/INV-3)，`interactive:true` 才转给人；**没有 done 事件=没跑完**(退出码 0 也判 error)。`red-team` 硬拒、timeout 硬执行、终态必释放会话+复位。实测逼出三处：参数校验要在租设备之前 / 失败要交出 stderr 尾巴(P-46 同款) / 终局事件要进重放缓冲。全量 **264 绿**。🟡 **待真机验**：真跑一次 AI 编排(done/报告/WS 交互)要节点配 `[ai]`，本机没 key(Q-20)。**下一步 = P4 平台对接 / P5 部署形态** |

| **平台对接（TOOLKIT/bug）** | 📐 方案已定（ADR-0023），tke 侧只补了 `usage` | 用户拍板四条：①平台是客户端(先直连,不做节点反向注册) ②自动化 run 与手工 run **同一实体**（"手工用例也能交给 AI 跑"，区别只在 `executor`）③安全是平台的**第七个实体**（未来放巡检/看门狗/HealthCheck），不塞 bug 列表 ④设备池是**平台级**页面（跨 App）。**tke 侧几乎不用改**：回归回放走 L1（零 LLM·只计设备时长）、AI 探索/安全扫描走 L2（平台 key·记 token）。已补 `usage`（从 `Summary` 抽，测不到给 null 不给 0）；**顺带修了真 bug**——安全轨无头输出没有 `type` 字段，成功的扫描被判成"没跑完"。✅ **安全轨用量已补齐**（`security/usage.rs`：prober/analyst/orchestrator 三处会话分角色记账，走「终局 JSON 的 usage」+「findings.json 的 usage」两条交付路；没量到给 null 不给 0）。**D6（形状变了）**：AI 不下发 key，改走**平台 AI 网关**——用户的 key 一步不离开平台，计量也变成网关侧权威的（tke 回的 usage 只作对账）。为此把 `[ai].base_url` 放开给所有 provider 并**保留原生适配器**（走 OpenAI 兼容会丢 anthropic 思考块）。凭据经 `TKE_AI_*` 环境变量交给子进程（argv 会被 `ps aux` 看见）、stderr 尾巴脱敏。**`meta` 透传**做归账：任务与会话都收、原样回到视图/列表/webhook——设备租赁与 AI 计费共用同一条路。**D7 节点主动报到**（用户拍板：轮询不稳定且容易炸服务器）：`tke serve --platform/--platform-token/--advertise`，一个端点幂等 upsert、**第一次心跳即注册**、每次带**全量**设备清单（事件式漏一条就永久错位，全量会自愈）、周期由平台回、连不上平台不影响节点干活。**本机端到端验过**：真 tke 报到 → 平台 online + 设备进池 → 杀节点 → 56s 自动判 offline。节点仍**零业务凭据**（不认识 App/用户、不持 AI key、产物是平台去拉）。平台侧实现见 `bug` 仓库 commit `e0c522a`（迁移 116 三张表 + `api/node`）+ 设计 `bug/docs/11_device_cloud.md`。**✅ 2026-08-27 全链路实测通过**：case → 两件套 → suite → run → 下发回放 → 回写用例结果 → 产物落对象存储（3/3 passed、各 14 个产物）。实跑逼出的都在 P-56~P-59 与 bug 仓库的提交里 |
| **反向通道（内网节点）** | ✅ **真实内网机器 + 公网平台验过** | **ADR-0024**。心跳只解决了"平台知道有这台机器"，任务通道仍是平台 → 节点的 HTTP，节点必须够得着 —— 而真机大多插在办公室的机器上（内网、IP 会变、没有公网入口），用户明确不要隧道也不要 VPN。`tke serve --link`：节点主动连平台，**连上即注册、断开即注销**，之后所有指令在这条连接上跑。关键设计：**帧就地拼成 Request 交给已有的 axum Router**（Router 本身是 `tower::Service`），七个 handler 一个字没改，不会出现"HTTP 一套 / WS 一套"的双份实现；鉴权也不绕过中间件（绕过等于给自己开个不走鉴权的入口）。二进制正文走 b64。两条路**不自动切换**（自动切换会让"走的哪条"变成运行时才知道的事）。实跑撞出五个洞：`tkeclient` 三个出口漏改一个 → **整个后端 panic**（补上之余加了兜底：没有传输方式时返回能查下去的错误）／重连时旧连接的协程把新连接刚标的在线覆盖掉／判离线用 `last_seen_at` 超时而反向通道**不发心跳** → 连着却被判死／后端重启后 `assigned` 的任务永远占着设备／`conn_mode` 的 CHECK 约束按默认命名猜错了名字，DROP IF EXISTS 什么也没删。**只在平台单副本下正确**（节点连在哪个副本，任务就得在那个副本派）——用户说暂时没多副本计划 |
| **多台安卓模拟器** | ✅ | `tke doctor --fix --profile android-emu --emulators N`：镜像只下一次，已装过只补配置。第一台仍叫 `tke`（改名会打断所有现成的 `-d avd:tke`）。两次踩同一类坑：参数化漏了指路文件那一处 → 第二台建了却认不出来；判"已装"只看文件在不在 → 坏掉的指路文件（指向别的 AVD，emulator 直接 FATAL）永远修不好，改成校验 `path=` 落点 |
| **设备池会变** | ✅ | `set_pool` 这个函数一直没人调用，池子进程启动时扫一次就定死 —— 后来起的模拟器、插上的真机都看不见。现在每 30 秒重扫，变了才写回；**在用的设备不许消失**（重扫时它可能恰好没被扫到），合并逻辑抽成 `merge_pool()` 并加了回归 |

## 本次会话不要碰

- App/website 侧（用户明确:先只做 tke）
- `docs/archive/` 归档文档（已停止维护,模块职责以 AGENTS.md 为准）
- 医生复活类方案——读 ADR-0001 再想

## 环境

- 构建:`./build-mac.sh`（禁 cargo build 产二进制,见 PITFALLS P-02）
- 测试:`cargo test --no-default-features --lib`（~80s,32 个）
- 真机循环:用户 mac 上 build 后终端跑,反馈贴回会话
