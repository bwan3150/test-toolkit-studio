# ADR-0023: 与测试管理平台对接——平台是客户端，tke 只补计量口子

- **状态**: 生效（2026-08-26 用户拍板四条）
- **日期**: 2026-08-26
- **关联**: 承 ADR-0022（服务化：D1 单节点边界 / D3 计费分层 / D5 red-team / D6 决策回传）；
  INV-16 / INV-17；平台侧设计见 `TOOLKIT/bug/docs/11_device_cloud.md` 与 `12_security_entity.md`
- **对接方**: `TOOLKIT/bug`（Go + Vue 的测试管理平台：bug / release / case / suite / run / script 六大列表）

## 背景

ADR-0022 把 tke 做成了"可远程调用的单节点 agent"，并明确**调度/计费/多租户归云平台**（D1）。
那个平台就是 `TOOLKIT/bug`。摸过之后发现接缝比预想整齐：

- 它的 `scripts` 表原始注释就写着「后续 TestRun 触发自动化跑、回填结果都从这里出」，
  已有 `script_type='ui_auto'` / `repo_path_or_link` / `version` / `is_current` / `trigger_command`；
- `case_results` 已有 `result / failure_reason / executor / executed_at` 和 `bug_id` 外键；
- 它有一套跑熟的"长作业"范式（`release.Runner`：服务端持进度、常驻 SSE、谁都能中断、
  跑在 background ctx 上），测试执行照抄即可（只是不能是全局单例）；
- 它的 AI 会话**已有卡片式交互**（`app_picker`/`finish_confirm` + `cards/:id/answer`），
  正好接 D6 的 `needs_decision` 回传。

也就是说：**平台侧要建的东西不少，tke 侧几乎不用改。**

## 决策

### D1 平台是客户端，tke 不认识平台

对接方向是**平台 → 节点**（用户拍板：先做直连，不做节点反向注册）。
tke 不新增"平台"概念、不存 app_id/user_id、不做回调之外的主动上报。
节点反向注册（用户自带的、NAT 后面的真机墙）留到有真实需求时再议——
`device_nodes.conn_mode` 在平台侧留了位置，tke 侧一行代码都不用先写。

### D2 执行分两层，与计费一一对应（不新增第三条路）

平台上的两个按钮直接落到 ADR-0022 已有的两层：

| 平台动作 | tke 层 | 成本 |
|---|---|---|
| 回归执行（用例已有两件套） | **L1 命令层** `exec` → `tke run` | 零 LLM，只计设备时长 |
| 生成脚本 / AI 探索 / 安全扫描 | **L2 任务层** `POST /v1/tasks` | 平台 key，token 记用户账 |

**不为平台新增专用接口。** 传两件套用 `PUT …/workspace/**`，拉证据用 `GET …/artifacts/**`，
都是现成的。平台要的"一次 run 跑 N 条用例"是平台侧的编排，不是 tke 的新原语——
把循环塞进 tke 会让它开始理解"用例"，那是平台的领域。

### D3 补一个口子：任务终态必须给出用量

**这是本 ADR 唯一要求 tke 改的东西。** 计费模型（ADR-0022 D3）说平台任务的 token
计入用户账单，但现在任务终态只有 `outcome` 和 `detail`，**没有用量**——平台拿不到数，
`device_leases.token_usage` 填不上。

任务终态（`GET /v1/tasks/{id}`、`task_end` 事件、webhook 回调）新增 `usage`：

```json
"usage": {"prompt_tokens": N, "completion_tokens": N, "total_tokens": N, "model": "..."}
```

来源是引擎已有的 `Tokens`（`Summary` 事件带「全程总量」）。**没有用量时给 null 而不是 0**——
0 会被当成"这次没花钱"，而真相是"没测量到"（INV-9 的精神：查不了要说出来）。

⚠️ **安全轨现在测不到用量**：`tke security --json` 的无头路径不走 `Summary` 事件，
只打一个结果对象。所以安全任务的 `usage` 恒为 null，平台对它**只能计设备时长（=0）**。
要按 token 计费得先让安全轨也汇总用量——独立的一件事，不在本 ADR 范围。

**顺带修掉的一个真 bug**：安全轨那个结果对象**没有 `type` 字段**，
于是一次**成功**的安全扫描会被判成"没跑完"（P3 只测了失败路径）。
现在的规则是：UiEvent 流的终局优先，没有的话认最后一个无 `type` 的结果对象的 `success`——
这条对所有"一次性命令"类任务通用，不是给安全轨打的补丁。

### D4 安全轨的产物按"发现"落库，tke 侧不变

平台把安全做成第七个实体（一行 = 一个 finding），而不是塞进 bug 列表。
tke 侧不用动：`findings.json` 本来就是机器可读的伴生文件（ADR-0019 / `security-report-spec.md`），
每条有稳定 id，平台按 id 幂等 upsert（同一个巡检每周跑一次，不能攒出 52 行）。

`red-team` 继续硬拒（ADR-0022 D5），平台界面上不放这个档位。

### D5 决策点走平台的卡片，不做新协议

`outcome: needs_decision` + 问题原文/选项 → 平台渲染成一张卡 → 用户点选 →
带着答案重新下发任务。**tke 侧零改动**：D6 定的就是"回传给与用户对话的那一层"，
平台的卡片机制正好是那一层。

## 理由与代价

**为什么不让 tke 直接写平台的库 / 不给 tke 加 app_id。**
那会让 tke 认识"用户""应用""用例"，D1 的边界当场失守，而且每个客户平台都要 tke 适配一次。
现在的形状是：tke 回答"这台机器能做什么、这条命令跑出了什么"，语义归属全在平台。

**代价与风险：**
- **对接面变宽了但没变形**：平台会同时用到 L1/L2/L3 三层，任何一层的破坏性改动都会打到平台。
  `GET /v1/hello` 的 build 戳比对因此更重要——版本漂移要在平台界面上看得见。
- **`usage` 是新的公开字段**，一旦平台开始计费就不能随便改形状。
- **iOS 只能落 macOS 节点**（宿主机能力门禁）：平台调度要按 `host_os` 路由，
  否则会浪费一次调度才被节点拒。这是 tke 有意保留的门禁，不为平台放开。
- **计时以平台为准**：节点重启会丢租约内存态。tke 的 `GET /v1/sessions` 只作对账，
  不承诺是计费真相。

**重新审视触发条件**：出现第二个对接方，且它要的东西无法用现有三层表达时——
那时才考虑给 tke 加"集成层"，而不是现在预造。
