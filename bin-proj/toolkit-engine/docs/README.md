# docs 导航

入口是 `../AGENTS.md`（协议+路由）。本目录分四类：

## 权威文件（改动需谨慎）

| 文件 | 作用 | 何时更新 |
|---|---|---|
| [`INVARIANTS.md`](INVARIANTS.md) | 17 条不变量,全项目引用锚点 | 极少,必须走 ADR + 用户拍板 |
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
| [`platform-matrix.md`](platform-matrix.md) | **平台能力矩阵**：三端共有 / 平台独有 / **同名不同义** + 加新动作的检查单。面向调用方和加动作的人 |
| [`driver-mapping.md`](driver-mapping.md) | 原子指令在三端驱动的**底层**映射对照（落成什么 adb/HTTP 调用）。面向排查 |
| [`skill-integration.md`](skill-integration.md) | tke 作为 skill 融入 coding agent 工作流的**早期设计稿**。⚠️ 其中的 `tke task` 方案已被 **ADR-0010** 推翻（skill 借调用方的 AI，不内置）；实际落地物是 `skill/tke-ui-test/` + `skill/install.sh` |
| [`security-report-spec.md`](security-report-spec.md) | **`tke security` 报告规范**：段落/评级/品牌/**材料库区块**/机器可读伴生。关联 ADR-0019（生效）。可视化基线见 [`security-report-template.sample.html`](security-report-template.sample.html) |
| [`platform-integration`](adr/0023-platform-integration.md) | **与测试管理平台（TOOLKIT/bug）对接**：平台是客户端、执行分两层对应计费、tke 侧只补 `usage` 口子。平台侧设计在 `bug/docs/11_device_cloud.md` + `12_security_entity.md` |
| [`remote-api.md`](remote-api.md) | **tke 服务化远程 API 契约（v1 设计稿）**：三层 API（命令/任务/产物）、租约模型、白名单、计费口径、分阶段路线。关联 ADR-0022 + INV-16/17 |
| [`ci-publishing.md`](ci-publishing.md) | **GitHub Actions 发布**：改 tke/skill 跑 Publish TKE，换 Chrome/驱动跑 Publish TKE Deps；含各家下载源的实测结构 |
| [`publishing-to-toolkit-cloud.md`](publishing-to-toolkit-cloud.md) | 打包 skill 分发包并用 curl 一次性上传到 Toolkit Cloud 的操作手册 |
