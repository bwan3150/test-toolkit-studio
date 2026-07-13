# tke run 的 AI 辅助驾驶(定位自愈 + 分诊)

`tke run xxx.tks` 纯回放时,App 小改版(文案微调/控件换了类型/位置挪动/页面局部重构)会让脚本某步
按原定位找不到元素。AI 辅助驾驶让回放不因此中断——某步元素**连续定位失败**(重试第 3 次)触发
**分层判断**(每层一次 LLM 调用,命中即停):

| 层 | 判断 | 动作 |
|---|---|---|
| 1 同页找回 | 就是它,换了位置/层级/文案 | 救活继续跑 |
| 2 同页替代 | 原元素没了,但有**功能等价**的入口能进下一步 | 救活继续跑,报告标注"→替代「XX」" |
| 3 前面走偏 | 本步开始就不在该在的页(前面某步没生效/点歪) | 不救,失败报错带诊断 |
| 4 路径重构 | 功能入口整体迁移,原路径不存在了 | 不救,失败报错带诊断 |
| 5 App 问题 | 元素消失/不可交互——**脚本可能测出了真实缺陷** | 不救,失败报错带诊断 |

## 起始态对齐(开跑前)

无「启动」步的脚本,回放起点完全取决于设备当前停在哪。copilot 开启时,`tke run` 开跑前:

1. 脚本**有「启动」步 → 跳过**(冷启动自会对齐)
2. 起始参照 = .tklib pages「起始页」(desc+特征集);老脚本退回 `# 起始标志:` 头注释;都没有 → 跳过并警告
3. 当前页**本地匹配**(与「断言页面」同一套命中率投票,零 AI 成本)命中 → 直接跑
4. 不匹配 → `navigate` 轻导航回起始页(纪律:最短路径 + **禁改账号/数据状态**)→ **实测复验**
5. 复验仍不匹配 → **不开跑**(在错误页面上回放可能产生副作用),报错说清当前页面与建议

**登录态等前提:查得出、说得清、不代办**——自动登录既要凭据又改变账号状态,是纪律红线。
对齐失败的报错会明确提示"登录态/权限/特定数据类前提请人工处理后重跑"。
flow(.toml)不做对齐:脚本间连续性是有意设计(web 会话保留可测联动)。

## 步内分层(运行中)

- 第一段(层 1)只看「元素库里当初的样子」vs「当前页面」,窄而准;没把握才进第二段
- 第二段(层 2~5,分诊)额外喂**脚本全文**:判断层 3 的素材是"当前页面能看到更早/更晚步骤的
  元素→前步没生效/跳步了";层 2 只在高把握时替代,涉及提交/支付/删除等不可逆步骤宁可失败
- 层 2 的替代元素**不写元素库**(它不是"它",写进原元素名会污染库);层 1 的修正只写解包临时副本
- 后三层的诊断拼进该步报错:`🩺 AI 分诊:疑前面步骤走偏(…)/疑路径已整体重构(…)/疑 App 问题(…)`
- harness 的 replay/repair 只用层 1(那边有轨迹报告+编排官决策体系,分诊会与之打架)

## 与 harness 修复的分界(重要)

| | tke run 辅助驾驶 | tke harness 的 replay/repair |
|---|---|---|
| 目的 | 小改动不打断回放 | 修复并更新脚本资产 |
| 改 .tks | **否** | 是(断点续探写回) |
| 改 .tklib | **否**(修正只落解包出的临时副本) | 是(自愈修正回包) |
| 产出 | 执行报告里标注哪些步靠 AI 通过 | 更新后的两件套 |

`tke run` 的自愈**不改任何脚本资产**——报告里标注"这些元素的原定位在当前 App 上已失效",
是否更新脚本由人(或 harness 修复流程)决定。

## 开关

默认**开启**。三档优先级:CLI > config > 默认。

```bash
tke run foo.tks --copilot false   # 本次关闭
```

```toml
# config.toml
copilot = false                    # 全局关闭

[ai]                               # 自愈是 LLM 调用,需配置 [ai] 才真正生效
provider = "anthropic"
model = "claude-sonnet-4-6"
```

未配置 `[ai]` 时自愈调用会失败返回 null,回放行为等同关闭(不会报错卡住)。

## 报告(NDJSON / log.json / 终端)

- `step_end` 事件与 log.json 的 `steps[]` 新增可选字段 `healed`(值 = 被 AI 找回的元素名):
  ```json
  {"event":"step_end","index":3,"command":"点击 [{登录按钮}]","success":true,"healed":"登录按钮",...}
  ```
- `run_end` 事件新增可选字段 `healed`(汇总数组,空则省略):
  ```json
  {"event":"run_end","success":true,"healed":["第4步「登录按钮」"],...}
  ```
- 终端(Pretty)逐步显示 `🩹 AI 辅助:「登录按钮」原定位失效,已按当前页面找回`,
  结尾汇总 `🩹 AI 辅助驾驶介入 N 处`。

App 侧(handlers)消费 NDJSON 时按可选字段处理即可,老版本事件流不含这些字段。

## 实现位置

- 触发点:`src/workflow/tks/interpreter/target_resolver.rs`(ElementHealer 钩子,重试第 3 次回调)
- LLM 实现:`src/workflow/agent/runner/healer.rs`(LlmElementHealer:pick_same 层1 + triage 层2~5;
  `copilot_healer` 装配工厂)
- 提示词:`builtin/messages/verify/heal_pick.md`(层1)/ `heal_triage.md`(层2~5,可经 prompts_dir 覆盖)
- 装配:`src/cli/workflow/run.rs::healer_factory` → `ScriptRunner/FlowRunner::with_healer_factory`
  (解包 .tklib 后以临时副本路径 + 脚本原文延迟构造)
- 诊断出口:`ElementHealer::take_diagnosis`(取走即清空)→ ScriptRunner 拼进失败步 error
- 步超时:healer 在场时非滚动查找步 20s→60s(给 LLM 挑选留时间)
- 起始态对齐:`tksops::align_start`(pub,CLI 开跑前调;AlignOutcome 四态,Failed 不开跑)
- 测试:`repair_tests.rs` 六条——`run_copilot_heals_without_touching_assets`(层1,资产零改动)/
  `run_copilot_triage_replaces_with_equivalent_element`(层2,替代救活不落库)/
  `run_copilot_triage_diagnoses_wrong_page`(层3,诊断进报错)/
  `align_start_navigates_back_to_start_page` / `align_start_skips_scripts_with_launch_step` /
  `align_start_fails_cleanly_when_navigation_cannot_reach`。
  fake 测试装配注意:改版页面节点 class 必须≠库条目(fake::node 全是 Button),
  否则 recognizer 的 class_name 结构容错会"意外命中",heal 根本不触发
