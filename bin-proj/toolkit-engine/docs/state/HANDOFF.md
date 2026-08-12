# 交接单

**会话时间**: 2026-08-12
**产出 commit**: （本次=治理体系落地,见 CHANGELOG）

## 做完了

- 治理体系落地:INVARIANTS(12条) / PITFALLS(14条,存量搬运) / ADR 0001-0008 补录 /
  state 三件套 / ROADMAP / CHANGELOG / 守卫脚本 + git hook / AGENTS.md 改造为路由+协议入口
- tests/ 落地:cli.rs 黑盒契约(7条,含 --copilot 裸旗标回归) + e2e/smoke.sh 真机冒烟;
  测试三层放置 ADR-0008(单测就地放,别搬)
- docs 整理:tke-flow.md 更新到当前架构;codebase-map/refactor-plan/tke-overview 归档;
  docs/README.md 导航

## 没做完

- （无半成品代码）

## 埋的坑 / 需要后来人注意

- ADR/PITFALLS 是从会话记忆补录的,引用的 commit hash 都真实存在但细节若有出入以 git log 为准
- 守卫脚本挂在 studio 仓库根的 .git/hooks,但只检查 bin-proj/toolkit-engine 范围的改动
