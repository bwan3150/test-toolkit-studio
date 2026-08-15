---
Last-Updated: 2026-08-15
Last-Commit: 741d86cc
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
| **skill 版本过期无提示** | ❌ 已知缺口 | **Q-11,优先级高**:本机装的是 8-13 旧版,repo 已到 8-15。改完 SKILL.md 的收益可能根本到不了用户手上。**用户重跑前必须先重装 skill** |
| skill（给 AI 设备操控+证据） | ✅ 可用,**跨设备待用户 mac 实测** | **ADR-0010**。**只做一次性检查+留证据,不产 .tks/.tklib、不回放**(与 harness 是两个东西)。`skill/tke-ui-test/`:主文件精干 + `reference/pitfalls.md` 踩坑册(新坑往里加,别撑大主文件)。`/tke-ui-test` 斜杠可调 |
| 跨设备/跨平台测试 | ✅ 已实现,**AI 侧真机未验** | ADR-0011 全套:flow per-script device / 重试断言 / 设备成为工具参数 + list_devices;动态值传递未做(Q-7) |
| 宿主机能力门禁 | ✅ 本机实测 | iOS 只在 macOS 放行(门禁在 `Controller::new`,control/run/steps/harness 一处覆盖);留 `TKE_ALLOW_IOS=1` 逃生口——界线是产品决策不是技术极限 |
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
