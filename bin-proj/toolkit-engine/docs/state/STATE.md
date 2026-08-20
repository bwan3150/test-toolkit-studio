---
Last-Updated: 2026-08-20
Last-Commit: 86bbbcb5
---

# 当前状态

## 大局

北极星 = 测试领域专精的 Claude Code（ADR-0002）：tke 是能操作 Android/iOS/Web 的设备 AI agent,
探索产 .tks+.tklib 两件套,可无 AI 回放,回放坏了由编排官编排修复。
Electron App（studio）只是 tke 的外围封装——**当前主线只做 toolkit-engine**（用户拍板 2026-07-13）。

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
| **doctor 报告排版** | ✅ 本机实测(Linux) | 用户说"信息太多而且顺序混乱"→ **三段分组**:①工具本身(平台/依赖/Engine/Skill 版本)②能测什么(四端+显示器,真机在前)③落点(Engine/Skill/日志);标签列按显示宽度对齐、值统一 `状态 (补充)`、用不了的整行置灰。设备探测改走 `tools::discover`(与 `device list` 同一套)。排版拆到 `cli/doctor.rs` |
| **版本新鲜度 `tke doctor`** | ✅ 本机实测 | **Q-11 已关闭**(ADR-0014):`tke fix`→`tke doctor`(fix 保留别名);比 **build 戳**而非版本号;`steps` 每批提醒(缓存 4h/stderr/--json 闭嘴);只提醒不自更新。⚠️ **要等下一次发布把 VERSION 打进 skill 包后才对用户生效** |
| skill（给 AI 设备操控+证据） | ✅ 可用,**跨设备待用户 mac 实测** | **ADR-0010**。**只做一次性检查+留证据,不产 .tks/.tklib、不回放**(与 harness 是两个东西)。`skill/tke-ui-test/`:主文件精干 + `reference/pitfalls.md` 踩坑册(新坑往里加,别撑大主文件)。`/tke-ui-test` 斜杠可调 |
| **iOS 模拟器** | ✅ **端到端跑通并已分发**(2026-08-19 用户实测) | ADR-0017。`-d sim:<UDID>`,与真机**同一套 WDA**,只有"怎么连上"不同(真机 USB 隧道 / 模拟器直连本机端口,**一台一个**,Q-13 已关)。`doctor --fix --profile ios` 下预编译 runner(21MB,arm64+x86_64 fat 包)到 `~/.tke/wda/`,tke 自己 `simctl install/launch`——**不碰 Xcode、不装 brew、不编译**。路线拐过一次弯(WDA→idb→WDA),原因见 ADR-0017「修订」 |
| **设备发现 `tke device list`** | ✅ 本机+用户实测 | 四端统一(安卓/iOS真机/iOS模拟器/浏览器),第一列就是 `-d` 的值;**查不了的那类单独说明原因**("没装 adb 是没查不是没连")。harness 向导也改走它——同一个问题不该有两套答案 |
| **报告可读性** | ✅ 本机实测 | 手机竖屏图限高 56vh + **点击就地展开**(纯 CSS,不引 JS);图下方三个链接:原图/元素表/**原始页面**;设备栏显示友好名。`--summary -` **从标准输入读**(heredoc 一步给长 Markdown,不用再写临时文件) |
| **环境起停 boot/shutdown** | ✅ 本机实测 | `control boot [--headless=off]` / `control shutdown`;tks `启动环境 [有头]` / `关闭环境`。**boot 管环境本身,launch 管环境里的东西**——早先浏览器是被第一条 web 命令顺带起来的,脚本里看不出它何时起、以什么模式起。iOS 模拟器 `simctl boot` + bootstatus 等就绪;**安卓模拟器明说不支持**(要 SDK 的 emulator + AVD 名) |
| **iOS 密码脱敏** | ✅ 已修(P-45) | XCUI 归一化**从来没输出 password 属性**,于是 iOS 上密码明文进 log/报告/截图横幅——而注释还写着"三平台同一条路"。安卓原生有、web 已对齐,**唯独 iOS 漏了整整一个平台**。两个单测钉住 |
| **设备显示名** | ✅ 本机实测 | `Controller::describe()`:`iPhone 17 Pro · iOS 26.0（模拟器）`/`Pixel 7（安卓 14）`/`Chrome（无头）`。报告显示它而不是 UUID;**换设备的判断仍用设备 ID**——同型号模拟器 label 会撞(单测钉着) |
| 跨设备/跨平台测试 | ✅ 已实现,**AI 侧真机未验** | ADR-0011 全套:flow per-script device / 重试断言 / 设备成为工具参数 + list_devices;动态值传递未做(Q-7) |
| 宿主机能力门禁 | ✅ 本机实测 | iOS 只在 macOS 放行(门禁在 `Controller::new`,control/run/steps/harness 一处覆盖);留 `TKE_ALLOW_IOS=1` 逃生口——界线是产品决策不是技术极限 |
| **CLI 直通** | ❌ **已删除**(2026-08-19) | ADR-0016(用户拍板):它是操作设备的第二条路,绕过证据留存/坐标换算/唯一动作映射。保留 `ToolManager::resolve`(内部定位 adb/chromedriver/go-ios)与 `tke <path.tks>` 便捷路由;删前盘出唯一缺口 logcat,补了 `tke app log` |
| 分发源六平台齐备 | ✅ 依赖全 / ⏳ 二进制待 CI | 依赖六平台已手工补齐(linux-arm64 只有 go-ios、win32 没有 go-ios——上游就没有),**一次性活不再动**;tke 二进制只有 mac-arm64+linux-amd64,darwin-amd64/windows 等 CI 跑 |
| 依赖补齐 `tke fix` | ✅ 本机端到端实测 | ADR-0012:唯一会联网下载的命令;普通命令缺依赖只报错指路。空目录只放 tke → fix → 跑通网页检查 |
| 两件套自包含（拷走即跑） | ✅ 本机实测通过 | Q-6 关闭:缺 `-d` 时从 tklib 的 meta.json 读平台兜底(web 零参数回放/android 走默认设备/ios 仍需显式) |

## 本次会话不要碰

- App/website 侧（用户明确:先只做 tke）
- `docs/archive/` 归档文档（已停止维护,模块职责以 AGENTS.md 为准）
- 医生复活类方案——读 ADR-0001 再想

## 环境

- 构建:`./build-mac.sh`（禁 cargo build 产二进制,见 PITFALLS P-02）
- 测试:`cargo test --no-default-features --lib`（~80s,32 个）
- 真机循环:用户 mac 上 build 后终端跑,反馈贴回会话
