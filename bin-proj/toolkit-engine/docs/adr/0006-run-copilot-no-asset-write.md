# ADR-0006: tke run AI 辅助驾驶——救活当次执行,不改脚本资产

- **状态**: 生效
- **日期**: 2026-07-13, commit 75842dda→b7e30a2d→db33162c→7c4138c9
- **关联**: INV-4 / INV-9 / INV-12, PITFALLS P-06/P-07/P-08

## 背景
App 小改版不该打断纯回放;但 run 是回放不是修复,不该有回写权。

## 决策
两段分诊（pick 同元素找回→triage: replace/wrong_page/path_changed/app_issue/unknown）;
层1/2 救活继续跑,层3-5 只出诊断进报错。修正只落解包临时副本,报告标注（healed 字段）。
开跑前起始态对齐（本地投票→navigate→实测复验,失败拒跑）。登录态只诊断不代办。
开关 copilot 默认开。harness 路径只用层1（那边有轨迹报告+编排官体系）。

## 理由与代价
run 与 harness 的语义分界:报告 vs 修复。代价:healer 在场时步超时放宽到 60s（真死页面判定变慢）。
