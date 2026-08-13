# ADR-0007: 引擎/前端解耦（UiEvent/Frontend trait）+ TUI 手写 inline

- **状态**: 生效
- **日期**: 2026-06-24（解耦）→ 2026-07-08（inline 定稿）
- **关联**: PITFALLS P-01, commit c4433d1e→07231866→…→46475c53

## 决策
引擎只 emit UiEvent,三前端 Plain(行式,stderr)/Json(NDJSON,stdout 给 App)/Tui。
TUI 弃 ratatui Terminal,手写 inline：历史行 print 进 scrollback（终端原生宽字符/滚动/复制）,
底部钉底小窗相对重画;wrap 物理行记账;CSI 2026 原子刷新。

## 理由与代价
ratatui inline 对 CJK 不可用（P-01）;顶部固定栏与 scrollback 物理互斥,不再尝试。
Plain 走 stderr 是刻意的——json 模式下 stdout 协议不被 AI 过程输出污染。
