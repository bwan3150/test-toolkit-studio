# docs 导航

入口是 `../AGENTS.md`（协议+路由）。本目录分四类：

## 权威文件（改动需谨慎）

| 文件 | 作用 | 何时更新 |
|---|---|---|
| [`INVARIANTS.md`](INVARIANTS.md) | 12 条不变量,全项目引用锚点 | 极少,必须走 ADR + 用户拍板 |
| [`ROADMAP.md`](ROADMAP.md) | 方向与下一步候选 | 阶段推进时 |

## 状态文件（每次会话必读,结束时必写）

| 文件 | 性质 |
|---|---|
| [`state/STATE.md`](state/STATE.md) | 进度快照,覆写 |
| [`state/HANDOFF.md`](state/HANDOFF.md) | 交接单,覆写 |
| [`state/OPEN_QUESTIONS.md`](state/OPEN_QUESTIONS.md) | 未决问题,增删 |

## 归档类（只追加,永不重写）

| 路径 | 作用 |
|---|---|
| [`adr/`](adr/) | 架构决策,一决策一文件,编号递增;废弃改状态字段,不删文件 |
| [`PITFALLS.md`](PITFALLS.md) | 踩坑记录 |
| [`../CHANGELOG.md`](../CHANGELOG.md) | 变更索引(细节在 commit message) |
| [`archive/`](archive/) | 停止维护的旧文档 |

## 活参考（内容漂移就修）

| 文件 | 作用 |
|---|---|
| [`tke-flow.md`](tke-flow.md) | 当前流程速览(mermaid 图) |
| [`setup-notes.md`](setup-notes.md) | 环境搭建/部署/换机的坑 |
| [`driver-mapping.md`](driver-mapping.md) | 原子指令在三端驱动的底层映射对照 |
| [`skill-integration.md`](skill-integration.md) | tke 作为 skill 融入 coding agent 工作流的**早期设计稿**。⚠️ 其中的 `tke task` 方案已被 **ADR-0010** 推翻（skill 借调用方的 AI，不内置）；实际落地物是 `skill/ui-check/` + `skill/install.sh` |
| [`publishing-to-toolkit-cloud.md`](publishing-to-toolkit-cloud.md) | 打包 skill 分发包并用 curl 一次性上传到 Toolkit Cloud 的操作手册 |
