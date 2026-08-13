# ADR-0005: 单次 agent 统一 oneshot 强制工具调用

- **状态**: 生效
- **日期**: 2026-07-12（87ef690a）
- **关联**: INV-2

## 决策
所有单次结构化角色统一走 runner/oneshot.rs：单个 report 工具 + ToolChoice 强制,
schema 供应商侧校验,模型没走工具时带提醒重试一次。告别"提示词求 JSON + 文本手术"。

## 理由与代价
解析层错误几乎清零,schema 即契约。代价:每个角色要写 schema——这是收益不是负担。
