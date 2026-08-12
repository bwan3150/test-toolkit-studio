# 变更记录（toolkit-engine）

只追加,不重写已有条目。每条带日期 + commit + 一句话;细节看 commit message（本仓库 commit 写得很全）。
更早历史直接看 `git log --oneline -- bin-proj/toolkit-engine`。

---

## [Unreleased]

### 2026-08-12 · tests 与 docs 整理
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
