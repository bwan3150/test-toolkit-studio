# ROADMAP（粗粒度,方向由用户拍板,顺序可调）

## 已完成的大块
1. ✅ 引擎/前端解耦 + TUI（ADR-0007）
2. ✅ 编排官 REPL + 工具颗粒化（ADR-0002）
3. ✅ 修复重建:接地三件套（ADR-0001/0004）
4. ✅ 元素包两件套（ADR-0003）
5. ✅ 无设备测试层（FakeLlm/FakeDriver）
6. ✅ tke run AI 辅助驾驶（ADR-0006）

## 进行中 / 下一步候选（做之前跟用户确认优先级）
- **分诊层真机验证与调优**（Q-1）:拿真实改版 App 逼出 replace/wrong_page 等路径
- **探索质量债**（Q-2）:web 小图标落点、滚动查找策略、平台化工具/提示词
- **skill 集成**（设计稿 `docs/skill-integration.md` + **ADR-0009 已拍板生效**）:让 Claude Code
  等 coding agent 调 tke 做 UI 验收。首版 Web+Android。
  阶段 0(零改动包 `tke run`,验证工作流价值) → 阶段 1(`tke task` headless,契约已定可开工)
  → 阶段 2(intent 契约)。**下一步做哪个阶段问用户**
- ~~Linux 构建脚本缺口~~ ✅ 已补 `build-linux.sh`（2026-08-12,带依赖预检 + `--no-ocr` CI 模式）
- **App 侧接入**:handlers 消费新 NDJSON（healed/对齐输出）——等用户解冻
- 文档债:docs/tke-cli-manual.md 整体过时重写;codebase-map.md 废弃或重生成

## 明确不做 / 缓做
- 自动登录/改账号状态类"代办"（INV-12）
- 知识库/mem0 真实接入（留了口子,未配置则跳过）
