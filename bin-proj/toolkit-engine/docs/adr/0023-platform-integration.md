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

**安全轨已补齐**（2026-08-26 同日）：它不走 `Summary` 事件，所以另建了一份领域内的累计件
（`workflow/security/usage.rs`），把 prober / analyst（每条 finding 一次）/ orchestrator
三处会话的 `total_usage()` 汇总起来，**分角色留账**——钱花在自主探测还是对抗复核上
是能指导调优的信息，合并成一个总数就再也分不开了。
交付走两条路，谁先到用谁：①无头终局 JSON 的 `usage` 字段 ②`findings.json` 里的同名字段。

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

## D6（2026-08-26 追加）：AI 走平台网关，凭据不下发到节点

**背景**：平台的 App 下本来就有用户自己填的 AI API Key，账要记在那个 App 名下
（层级：App 总用量 > 个人用量 > session 用量，设备租赁与 AI 计费分开记录但共用一套组件）。
最初的做法是"平台把 App 的 key 随任务交给节点"，为此还给凭据做了一路防护
（只走环境变量、不进 argv、stderr 尾巴脱敏）。

**用户提出更好的形状**：平台出一个 **AI 网关**，转发原生请求并做 fallback；
节点只调平台的网关地址。于是：

- **用户的 key 一步都不离开平台** —— 前面那一路防护变成纵深防御，而不是唯一防线。
- **计量在平台侧、是权威的**：网关看得见每一次请求与响应的 usage，不再依赖节点自报。
  tke 仍然回报 `usage`（对账用），但**账以网关为准**。
- **fallback / 多供应商 / 配额闸门**都落在平台已有的适配器层
  （`internal/service/ai/adapter/` + `resilience.go` 超时+重试+退避），不用在 tke 里重造。
- 节点拿到的是**短期任务令牌**（scoped to app+task，可撤销、可设预算上限），不是长期密钥。

### tke 侧为此改了一处

`[ai].base_url` 原本被硬限制在 doubao/qwen 上，并且**强制换成 OpenAI 适配器**。
现在放开给所有 provider，且**保留各家原生适配器**——只换端点，不换协议。

这一条是必须的：走 OpenAI 兼容协议会丢掉 anthropic 的思考块
（历史坑：genai 丢思考块 → anthropic 必须 4.6 + adaptive），而思考块正是 harness 质量的来源之一。
所以平台网关应当做**原生透传**（`/ai/proxy/<provider>/...`），而不是把一切压成 OpenAI 格式。

### 交付路不变

`POST /v1/tasks` 的 `ai` 字段照旧，只是内容变了：

```json
"ai": {"provider": "anthropic", "model": "...", "base_url": "https://平台/api/v1/ai/proxy", "api_key": "<短期任务令牌>"}
```

节点把它们经**环境变量**交给子进程（`TKE_AI_*`）——argv 会被 `ps aux` 看见，
配置文件会落到磁盘上。令牌同样过 stderr 脱敏。

### 代价

- **平台成了远程任务的单点**：网关挂了，节点上的 AI 任务跑不动。可接受——
  任务本来就是平台下发的。
- 多一跳延迟，相对模型本身的耗时可忽略。
- 网关要支持 tke 用到的能力（工具调用 / 强制工具 / reasoning），
  **不是简单的 HTTP 转发就够**——原生透传的好处正在于此：不理解内容，只转发。
