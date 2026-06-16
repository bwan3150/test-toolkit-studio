# toolkit-engine 重构实施计划：统一规范 + 解耦

> 目标：把 tke 的各功能（回放 / AI / 未来录制）做成**相对解耦、但都依赖同一套共享底座**。
> 底座是"规范的单一来源"——长在代码里，不靠各功能自觉遵守。
> 本文是动手前的蓝图：决策 → 目标架构 → 分阶段实施 → 约束与风险。配合 `codebase-map.md` 使用。

## 实施状态（2026-06-16）

**Phase 0–3 全部完成**，四块共享底座（参数层 / 操作模型 / tks 契约 / RunArtifacts）已落地，全程编译+冒烟+往返测试通过、对外行为保持。

| Phase | 状态 | commit |
|------|------|--------|
| 0 参数层 | ✅ | `44f2d07`（Params + element/ocr 单一来源）/ `56cc749`（编排层持 Arc<Params>） |
| 1 操作模型统一 | ✅ | `74c0184` |
| 2 tks 序列化（双向契约） | ✅ | `c224871` |
| 3 RunArtifacts 复用 + case 接入 | ✅ | `06e0e3e`（命名+序列化器）/ `75b05bf`（case 产物同构） |
| 4 清理（贯穿） | 🟡 部分 | 已随各阶段去重（DEFAULT_ELEMENT_PATHS / direction_cn 等）；vestigial 字段(TksScript.case_id/details)、Fetcher 疑似死方法待后续顺手清 |

对外可见的行为变化仅两处：① run/steps/flow 产物目录名 `<时间戳>_<名>` → `<名>_<时间戳>`；② case 产物改用 RunArtifacts 运行目录（详见 Phase 3）。

---

## 0. 设计决策（已确认）

| # | 决策 | 要点 |
|---|------|------|
| **1A** | 统一操作模型 | 单一 `ControlAction`（坐标级原子操作）+ **单一执行器**；tks 解释器 / `control` CLI / AI / 录制都走它；`等待 / 断言 / 循环` 归"工作流控制层"，**不进**设备操作枚举 |
| **2A** | tks 双向契约 | 补完整序列化 `TksStep::to_source()`，与 parser 互逆、往返一致；生产者构造 `TksStep` 结构体，不手拼字符串 |
| **3-ii** | 没有 project 概念，只有参数 | `--config` 一个参数总集（显式 `log/element/scripts/...`）；新增**参数层**：CLI+config 解析一次 → 统一参数表，模块**查表**取参（取代逐层透传）；case 复用 `RunArtifacts` |
| **4A(+a)** | element 路径归参数层单一来源 | 默认查找 `./element.json → ./locator/element.json` **保留**，逻辑只写在参数层一处；recognizer / element.rs 只接收解析结果 |
| **5** | 保留 online+offline 两种 OCR | 离线"词级定位"改进列 **TODO**；当前离线仅文字提取、识别走在线；OCR 服务 URL **可配**（进参数层，默认现 URL） |
| **6A** | 保守清理死代码 | grep 核验后删确认死的；"预留未接线"件保留 + 标 TODO；跟着大改造顺手做 |

## 1. 目标架构

```
┌─ 解耦的功能层（各自核心逻辑独立，横向互不依赖）──────────────────┐
│   回放 run        AI case        未来：录制 record                 │
└──────────────────────┬──────────────────────────────────────────┘
                       │ 都只依赖↓
┌─ 共享底座（单一来源，规范长在代码里）────────────────────────────┐
│  ① 参数层    CLI+config 解析一次 → 统一参数表（含 element 默认/ocr-url）│  [#3 #4 #5]
│  ② 操作模型  单一 ControlAction + execute_action()；工作流控制单列一层 │  [#1]
│  ③ tks 契约  parser ⇄ serializer 双向，语法单一来源                  │  [#2]
│  ④ 产物      RunArtifacts 全功能复用，run/case/录制同构              │  [#3]
└──────────────────────┬──────────────────────────────────────────┘
┌─ 能力层  原子(control/fetch/refresh) + 驱动(adb/web/wda) + 子工具直通 ─┐
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. 分阶段实施（按依赖排序，每阶段独立可编译、不破坏现有命令）

### Phase 0 — 参数层（地基中的地基）  [#3 #4 #5-url]  ✅ 已完成

**目标**：所有参数在一处解析，模块查表取参，取代 `device/element/log` 顺着函数签名层层透传。

**新增**：`src/utils/params.rs` —— `Params` 结构体（lib 类型），字段为**已解析**的参数：
```
device, element(已解析含默认查找), log, scripts, ocr_url, json, verbose,
ai: AiConfig, knowledge: KnowledgeConfig, …
```
+ `Params::resolve(cli_values, config) -> Params`：合并优先级 **CLI 显式 > config > 默认**；
  element 默认查找（`./element.json → ./locator/element.json`）只在此实现一次；ocr_url 默认现 URL。

**改**：
- `main.rs`：构建一次 `let params = Params::resolve(...)`，向各 handler 传 `&Params`（一个引用取代多个透传参数）。
- 各 `cli/*/handle(...)`：签名收敛为 `handle(args, &params)`，内部查表取 device/element/log/...。
- `recognizer/mod.rs`、`tools/element.rs`：删除各自的 `DEFAULT_ELEMENT_PATHS`，改为接收参数层解析好的元素库路径（落实 #4）。
- `recognizer/ocr.rs`：OCR URL 改为从参数取（落实 #5-url），默认值仍是现 URL。

**实现选择（默认）**：`Params` 以**引用传递**（`&Params` 作为唯一上下文参数向下传）——即"持有参数表的引用并查它"，比全局单例更安全可测。若你更想要"零透传的全局表"，可改用 `OnceCell` 全局，二选一（动手时定）。

**兼容**：纯参数解析重构，**对外 CLI/输出 schema 零变化**；不引入 `project`。
**验证**：`cargo check`；`tke --help`、`tke fetch -d X -c cfg.toml`、`tke recognize` 行为与现状一致。

---

### Phase 1 — 统一操作模型  [#1]  ✅ 已完成

**目标**：设备操作只有一套表示 + 一个执行器；tks 执行、`control` CLI、AI、录制都走它。

**核心动作**：抽出**单一执行器** `execute_action(controller: &Controller, action: ControlAction) -> Result<Value>`，
作为"坐标级操作 → 设备"的唯一实现：
- `atomic/control.rs`：`Control::execute` = `Control::new` + 调 `execute_action(&self.controller, action)`（行为不变，只是把 match 体抽出去）。
- `workflow/interpreter/command_executor.rs`：每个设备类命令改为「`TargetResolver` 解析元素得坐标（保留，含 ActionTrace 记录）→ 构造 `ControlAction` → 调 **同一个** `execute_action(&self.controller, …)`」，**不再各自直调 Controller**。

**归类**：`等待 / 断言`（及未来 `循环`）属**工作流控制层**，留在 command_executor / 解释器，**不进** `ControlAction`、不走 `execute_action`。

**操作集对齐**：以 `ControlAction`（Click/Press/Swipe/SwipeDir/Drag/Input/Clear/HideKeyboard/Back/Home/Launch/Close/Key）为设备操作全集；tks 的 `定向滑动` 等映射到对应变体。

**兼容**：`tke control X`、`tke run x.tks` 行为与现状一致（同样最终调 Controller 方法）。
**验证**：`cargo check`；对比改动前后 `tke control click`、`tke run` 一个样例脚本的行为。

---

### Phase 2 — tks 序列化（双向契约）  [#2]  ✅ 已完成

**目标**：`TksStep`/`TksScript` 可渲染回文本，与 parser 互逆。

**新增**：序列化器（放 `runner/parser/` 旁，与 parser 配对，或挂在 `tks_types.rs`）：
- `TksParam::to_token() -> String`：`{元素名}` / `{元素名}&策略` / `{x,y}` / `"文本"` / 数字 / 方向(中文) / 布尔(存在/不存在)。
- `TksStep::to_source() -> String`：`命令 [参数, …]`（命令用中文，对齐 `constants.rs` 的映射）。
- 单元测试保证 `parse(serialize(script))` 与原 AST 一致（往返）。

**改**：生产者（AI case、未来录制）构造 `TksStep` → `to_source()`，**删除手拼字符串**。

**兼容**：纯新增 + 内部替换，对外无影响。
**验证**：往返单测；AI case 产出的 .tks 能被 `tke run` 正常回放。

---

### Phase 3 — RunArtifacts 全功能复用 + case 接入  [#3 产物，消费 #1 #2]  ✅ 已完成

**目标**：case 与 run 产物同构；命名统一 `<name>_<时间戳>`；脚本落 `scripts` 参数目录。

**改**：
- `artifacts.rs`：`RunArtifacts::create` 命名改为 `<name>_<时间戳>`（无 name 用纯时间戳）。run/steps/flow 顺带统一（产物目录名顺序变，属可接受的行为变更）。
- `workflow/agent/`（case）：
  - 产物改用 `RunArtifacts`（`params.log` 为根）：每轮即一步 → `screenshots/step_NNN.png` + `page/step_NNN.xml`；累积 `ExecutionResult` → `write_log` 写 `log.json`；`conversation.jsonl` 写进**同一运行目录**。
  - 生成的 `.tks` 经 **Phase 2 序列化器** 写到 `params.scripts`（命名同运行目录）。
  - 设备动作经 **Phase 1 的 execute_action** 执行。
- 弃用 case 自建的 `*.screens/round_NNN`、`--script 同级 conversation`。

**兼容**：`tke run/steps/flow` 仅产物目录命名顺序变；**RunEvent / JsonOutput 输出 schema 不变**（Electron 依赖，见 §3）。
**验证**：真机跑一个 case → 得到与 run 同构的运行目录 + 脚本（你在真实设备验证）。

---

### Phase 4 — 清理（贯穿各阶段）  [#6 #4 残项]  🟡 部分完成

- grep 核验 Fetcher 疑似死方法（`optimize_ui_tree`/`generate_tree_string`/`extract_ui_elements_with_size`/`infer_screen_size_from_xml`/`filter_*`）与 `TksScript.case_id/details` → 确认死的删，预留件标 TODO。
- 确认 `DEFAULT_ELEMENT_PATHS` 重复已随 Phase 0 消除。
- 改到哪清到哪，不单开一轮。

---

### 未来 — 录制 record（本次不实现，仅留位）

录制 = 平台相关的**输入捕获**（Android `getevent` / Web 注入监听 / iOS 较难）**独立** + 共享底座的**输出处理**（落库 + `to_source` 生成 tks + RunArtifacts 产物）。
底座（Phase 0~3）就位后，录制器只需实现"捕获人操作 → 构造 TksStep/元素落库"，其余全复用。

---

## 3. 硬约束与风险

**硬约束（不可破坏）**：
- **对外输出 schema 稳定**：`RunEvent`（NDJSON）与 `JsonOutput` 的 JSON 形状被 Electron App 消费，**不得改字段名/结构**。产物目录名变化通过事件里的 `run_dir` 字段传出，App 跟随即可。
- **其他子工具功能不受影响**：refresh/fetch/recognize/control/run/steps/flow/ocr/file/app/device/element 的 CLI 行为保持。每阶段 `cargo check` + 冒烟（`tke --help`、各 `--help`、无设备报错路径）。
- **不引入 project 概念**（决策 3-ii）。

**风险**：
- **Read 工具会 garble 某些文件**（control.rs / ui_element.rs / execution.rs / config.rs 等）——改前**务必 grep/sed 核对真实签名**，以 `codebase-map.md` §5/§4 为准。
- 命名 `<name>_<时间戳>` 是 run/steps/flow 唯一对外可见变化（产物目录名），属决策内可接受项。
- 大改造跨多文件：严格按阶段推进，每阶段编译通过 + 冒烟后再下一阶段；每阶段可独立 commit。

## 4. 推进方式

- 顺序：**Phase 0 → 1 → 2 → 3**，清理(Phase 4)贯穿。
- 每阶段：动手前先按本文 + codebase-map 核对签名 → 实现 → `cargo check --no-default-features` → 冒烟 → commit。
- 真机相关验证（case 实跑）由你在真实设备完成并反馈。
- 本文与 `codebase-map.md` 同步维护：架构/接口若在实现中调整，回写更新。
