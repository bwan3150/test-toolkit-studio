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
- ~~skill 集成 阶段0~~ ✅ **原型已落地并实测通过**(2026-08-12,ADR-0010:借调用方 AI)。
  下一步候选:平台自包含(Q-6)、护栏命令化(若实测脚本质量不行)、Android 端跑通、intent 契约
- **跨设备/跨平台测试**:flow per-script device ✅ 已落地;
  **ADR-0011(设备降为工具级参数,AI 按语义选)待拍板**;
  还缺重试断言与跨设备数据传递(Q-7)
- ~~Linux 构建脚本缺口~~ ✅ 已补 `build-linux.sh`（2026-08-12,带依赖预检 + `--no-ocr` CI 模式）
- **App 侧接入**:handlers 消费新 NDJSON（healed/对齐输出）——等用户解冻
- 文档债:docs/tke-cli-manual.md 整体过时重写;codebase-map.md 废弃或重生成

## 新主线：服务化 / 远程能力（ADR-0022,2026-08-26 用户拍板）
把 tke 变成「可被远程调用的单节点测试 agent」——测试服务器上部署 tke + 模拟器/真机/无头浏览器,
云平台租设备下发任务,或让用户自己的 coding agent 通过远程 skill 调用（本地零安装）。
- **P1 `tke serve` 单节点**（租约 + exec 白名单 + 产物）
- **P2 `TKE_REMOTE` 二进制客户端 + 两条 remote skill**（文档复用不分叉）
- **P3 任务层**（服务端 AI 跑 harness/security + SSE/WS + webhook + needs_decision 回传）
- P4 平台对接 / P5 部署形态(Docker/mac 节点/GitHub Action) / P6 MCP 网关(可选)
完整契约与验收见 `remote-api.md`。**P1+P2 就交付核心价值,别先做 P3。**

## 源码沙盒：让 harness 拿着图纸测（ADR-0025，2026-08-28 用户拍板）
节点上按 App 存 repo 工作副本（git worktree，与租约同生命周期），凭据用平台换发的
一小时只读 token。**红线见 INV-19：源码只用于定位，不用于判定。**
- **P1 `changed_surfaces`** —— 只做"这次改动碰了哪些界面"的聚焦。
  判据：同一用例的探索轮数 / token 降幅。**降不下来就停**，别把三阶段做完才发现不值钱
- P2 `find_locator` + `find_route`（顺带提升 `.tks` 的抗改版能力）
- P3 接 CI 产物（apk/ipa/预览 URL）装上测
**不做**：节点上构建移动端（那是重造 CI）、部署编排。

## 明确不做 / 缓做
- 自动登录/改账号状态类"代办"（INV-12）
- 知识库/mem0 真实接入（留了口子,未配置则跳过）
- 多节点调度/计费/多租户——**归云平台,不进 tke**（ADR-0022 D1）
- 远程 `red-team` 强度档（ADR-0022 D5;本地仍可用）
