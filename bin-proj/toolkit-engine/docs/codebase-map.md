# toolkit-engine (tke) 代码地图

> 本文是 toolkit-engine 的权威架构与代码参考：分层、每个文件夹/文件的职责、核心数据流、
> 数据模型与 element.json schema、tks 语言规范、关键约定，以及**待确认/可疑点**。
> 之后任何开发、重构、排障都以本文为索引。所有签名均以源码为准（已逐文件核对）。

---

## 0. 一句话定位

tke = **所有自动化测试工具的统一入口/协调器**。一个 Rust CLI 二进制，四大功能块：

| 块 | 命令 | 实现层 |
|----|------|--------|
| ① 直通 | `tke adb …` / `tke k6 …` / `tke ffmpeg …` | `passthrough`（透传同目录二进制，零代码扩展） |
| ② 原子方法 | `tke refresh / fetch / recognize / control`（必带 `-d`） | `atomic`（编排 drivers+engines 完成单步动作） |
| ③ 工作流 | `tke run x.tks / x.toml` / `tke steps …` / `tke harness …` | `workflow`（组合原子能力跑脚本 / AI 探索） |
| ④ 自有工具 | `tke ocr / file / app / device / element` | `tools`（tke 内置实用工具） |

设计主线：**tks 脚本是中心契约**——手写 / AI 生成（未来：录制）三种方式*生产* tks，`run` 唯一*回放* tks。
回放可移植性依赖「元素名 + element.json 元素库」（多通道兜底），而非裸坐标。

---

## 1. 分层架构（自底向上，依赖单向）

```
┌─ cli ─────────────────────────────────────────────────────────┐
│ 命令翻译层：只解析 CLI 参数 → 调库，不含业务逻辑（含 EventPrinter / help）│
└──────────────────────────────┬────────────────────────────────┘
        ② atomic        ③ workflow        ④ tools        ① passthrough
        refresh/fetch    run/steps/        file/app/      ToolManager
        recognize/       case/flow         device/        (二进制定位/直通)
        control          + agent(AI)       element
└──────────────────────────────┼────────────────────────────────┘
┌─ engines（纯逻辑，无设备 IO）──┴─ drivers（对接层，每驱动一种协议）─────┐
│  fetcher   XML→UIElement       │  Controller → adb / web / wda          │
│  recognizer 多通道识别          │  (三端归一化为同一套 uiautomator XML)    │
│  ocr       在线/离线文字识别     │                                        │
└──────────────────────────────┬────────────────────────────────┘
┌─ models（数据模型） + utils（workarea/config/json_output/xml）───────────┐
└───────────────────────────────────────────────────────────────┘
```

职责边界：
- **drivers** = 对接（每个驱动只对接一种协议：adb 子进程 / W3C HTTP / WDA HTTP）。
- **engines** = 纯逻辑（解析 XML、匹配元素、OCR），不碰设备、不起进程。
- **atomic** = 编排（drivers + engines 完成"一步"），自身不实现协议/算法。
- **workflow** = 把原子能力组装成多步流程 + 产物 + 事件。
- **cli** = 参数翻译，禁止业务逻辑。

---

## 2. 目录 / 文件地图（每个文件的职责）

### 入口
| 文件 | 职责 |
|------|------|
| `main.rs` | clap `Cli`/`Commands` 定义；加载 config；合并全局参数（device/element/log，CLI > config）；路由到各 handler |
| `lib.rs` | crate 根：模块声明 + 统一导出（crate 根可直接 `use tke::X`）+ `TkeError` 错误枚举 |

### models/（数据模型，纯结构）
| 文件 | 职责 / 关键类型 |
|------|------|
| `mod.rs` | 统一导出 |
| `point.rs` | `Point{x,y}`、`Bounds{x1,y1,x2,y2}`（+ `center()/width()/height()/is_visible()`），均 `Copy` |
| `ui_element.rs` | `UIElement`（页面解析出的元素，见 §5）+ `center()/is_visible()/matches_text()/to_ai_text()` |
| `locator.rs` | `Platform{Android,Ios,Web}` + `from_device()`；`LocatorStrategy{XPath,ResourceId,Text,ContentDesc,ClassName,Ocr,Img,Auto}`；`AndroidLocator/IosLocator/WebLocator`；`Locator`（元素库条目）+ `structural(platform)`；`ElementLibrary{elements:HashMap}` |
| `tks_types.rs` | `TksCommand`(12)、`TksParam`(7)、`TksStep`、`TksScript`（见 §6） |
| `device_info.rs` | `DeviceInfo` + `HardwareInfo/BatteryInfo/NetworkInfo` |
| `execution.rs` | `ExecutionResult`、`StepResult`、`AppInfo`、`CurrentFocus`（见 §5） |

### utils/（基础设施）
| 文件 | 职责 |
|------|------|
| `config.rs` | `TkeConfig{device,element,log,ai,knowledge}` + `AiConfig` + `KnowledgeConfig`；`load()` 解析（相对路径基于 config 文件目录） |
| `workarea.rs` | `Workarea`：设备缓存区 `$TMPDIR/tke/workarea/<device>/`（跨进程持久）或 run 临时区 `$TMPDIR/tke/run-<ts>-<pid>/`（用完删）；文件名固定 `current_screenshot.png` / `current_ui_tree.xml` |
| `xml.rs` | `escape_attr()`（归一化 XML 时的属性转义，web/wda 共用） |
| `json_output.rs` | `JsonOutput`：所有 CLI 的统一 JSON 输出；`error()` 返回 `!`（打印 `{success:false,error}` 并退出 1） |

### passthrough/（① 直通 + 二进制定位）
| 文件 | 职责 |
|------|------|
| `manager.rs` | `ToolManager`：`resolve(name)`（同目录找 `name`→`name.exe`→`tke-name`→`tke-name.exe`）/ `passthrough(name,args,device)`（继承 stdio、以工具退出码退出；adb 特例注入 `-s`）/ `list_available()`（扫描同目录可执行，供 --help） |

> ToolManager 是**唯一的二进制定位器**：adb / chromedriver / go-ios / tke-opencv 等程序化调用全部共用它。

### drivers/（对接层；三端归一化为同一套 uiautomator XML）
| 文件 | 职责 |
|------|------|
| `mod.rs` | `Controller`：按 `Platform::from_device` 选 `Driver{Adb,Web,Wda}`，向上暴露统一 API（见 §4.1）。注意 web/ios 用 `device_id.unwrap()`（这两个平台只在 device_id 为 Some 时匹配，安全） |
| `adb.rs` | `AdbDriver`：调 adb 子进程（`screencap`/`uiautomator dump`/`input tap/swipe/text`/`am start`）；输入法智能切换（中英文检测，优先 Appium Unicode IME）；`clear`=40 次删除键；`hide_keyboard`=KEYCODE_BACK；`press`=同点 swipe |
| `web/mod.rs` | `WebDriver`：W3C WebDriver over HTTP(ureq)；会话持久化到 `$TMPDIR/tke/web/<device>.json`（跨进程）；坐标按 devicePixelRatio 换算；tap=pointer、swipe=wheel 滚轮、input=原生 setter+input 事件（兼容 React/Vue）；只有 navigate 能创建会话 |
| `web/infra.rs` | chromedriver 进程生命周期：定位（ToolManager）、空闲端口、`env_clear`+脱离终端进程组（防 Chrome 崩溃）、Chrome for Testing 定位、固定窗口 1280×900 + scale=1、孤儿收割 |
| `web/normalize.rs` | `DOM_WALK_JS`（浏览器内递归提取可见元素）+ `dom_elements_to_xml`（→ uiautomator XML） |
| `wda/mod.rs` | `WdaDriver`：WDA over HTTP(ureq，经 go-ios 转发)；状态持久化 `$TMPDIR/tke/ios/<udid>.json`；坐标按 scale（视网膜倍率）换算；tap=/wda/tap、press=touchAndHold、swipe=dragfromtoforduration、back=左缘右滑手势；只有 launch 能创建会话 |
| `wda/infra.rs` | go-ios 管理：`tunnel start`(iOS17+,全设备共享守护) + `runwda`(经 testmanagerd 拉起 WDA) + `forward`(USB 8100)。前置：Xcode 装过 WDA App（一次性） |
| `wda/normalize.rs` | `normalize_xcui_xml`（XCUI /source → uiautomator XML，仅保留可见有面积元素） |

### engines/（引擎层，纯逻辑无设备 IO）
| 文件 | 职责 |
|------|------|
| `fetcher/mod.rs` | `Fetcher`：递归解析 uiautomator XML → `Vec<UIElement>`；生成语义化 xpath（resource-id→content-desc→text→父路径→class+instance）；算 sibling_index、z_index（按面积）；过滤系统 UI / 不可见元素 |
| `recognizer/mod.rs` | `Recognizer`：加载 element.json；`find_element_detailed(name,strategy)` → `(Point, Bounds)`；**auto 兜底：结构标识(resource_id→xpath→text→content_desc→class_name) → OCR → 图像**；阈值默认 0.60 |
| `recognizer/xml.rs` | 结构标识精确匹配（在 Fetcher 解出的元素里按字段 ==） |
| `recognizer/ocr.rs` | 在线 OCR 文字匹配（精确→包含），URL 硬编码 `https://ocr.test-toolkit.app/ocr` |
| `recognizer/image.rs` | 调 `tke-opencv` 二进制做模板匹配，bounds 由模板尺寸反推 |
| `recognizer/text.rs` | 纯文本查找（`UIElement::matches_text`） |
| `ocr/mod.rs` | `ocr(data, online, param)` 统一入口（online→URL；offline→语言码，spawn_blocking 跑 tesseract） |
| `ocr/types.rs` | `OcrResult{texts}`、`OcrText{text,bbox,confidence}` + 在线 DTO |
| `ocr/online.rs` | reqwest POST base64 图到 OCR 服务 |
| `ocr/offline.rs` | tesseract-rs；**整图作为一个文本块返回，无词级 bbox，confidence 假定 0.9**（feature `ocr-offline`） |

### atomic/（② 原子方法，编排层）
| 文件 | 职责 / 关键签名 |
|------|------|
| `refresh.rs` | `Refresh::new(device)?` + `run(RefreshOptions)`→`RefreshResult`：采集截图+XML 到 workarea（可选 OCR / crop 元素图） |
| `fetch.rs` | `Fetch::run(device, FetchOptions{cached,interactive_only})`→`Vec<UIElement>`（cached=false 先 refresh） |
| `recognize.rs` | `Recognize::new(device, element)?` + `find(name, strategy, cached)`→`(Point,Bounds)`（cached=false 先采集） |
| `control.rs` | `Control::new(device)?` + `execute(ControlAction)`→`serde_json::Value`；`ControlAction` 枚举见 §4.1 |

### workflow/（③ 工作流）
| 文件 | 职责 |
|------|------|
| `events.rs` | `RunEvent` 枚举（NDJSON，`tag="event"`）：RunStart/StepStart/StepEnd/RunEnd/FlowStart/ScriptStart/ScriptEnd/FlowEnd |
| `artifacts.rs` | `RunArtifacts`：`create(log_root, name)`→`<log>/<时间戳>_<名>/`；`save_step()` 写**标注截图**(横幅+元素红框+点击蓝点+滑动线) `screenshots/step_NNN.png` + `page/step_NNN.xml`；`write_log<T:Serialize>()` 写 `log.json`；`run_dir` 字段 pub |
| `script_runner.rs` | `ScriptRunner`：`run(path)/run_lines(lines)`；逐步执行→（--log 时）每步 save_step + 累积 StepResult→失败即停→write_log→清理 workarea(web 会话不销毁)→发 RunEvent；`validate_script_path` |
| `flow.rs` | `FlowDef{name?,scripts}`(TOML)、`FlowResult`、`FlowRunner`：多脚本顺序执行（脚本失败不中断 flow）；结束统一清场（web 销毁会话 / android 关所有启动过的 App / ios 再销毁 WDA 会话） |
| `runner/mod.rs` | `Runner`（单步执行，编辑器调试用）+ 导出 `ScriptParser/ScriptInterpreter/ActionTrace` |
| `runner/parser/mod.rs` | `ScriptParser`：文本 → `TksScript`（正则切 `命令 [参数]`） |
| `runner/parser/constants.rs` | 中文命令→`TksCommand`、中文方向→英文方向 映射表 |
| `runner/parser/parameter_parser.rs` | `[...]` 内参数解析（`{元素名}`/`{x,y}`/引号文本/数字/方向/布尔） |
| `runner/parser/syntax_highlight.rs` | 编辑器语法高亮信息 |
| `runner/interpreter/mod.rs` | `ScriptInterpreter`（持 controller+recognizer+workarea+`ActionTrace`）；`interpret_step` 分发到 CommandExecutor；`ActionTrace{captured,points,bounds,element_name}`（供截图标注） |
| `runner/interpreter/command_executor.rs` | `CommandExecutor`：每个 `TksCommand` → 直接调 `Controller` 方法（execute_launch/click/press/swipe/input/wait/assert…） |
| `runner/interpreter/target_resolver.rs` | `TargetResolver`：`TksParam` → `Point`（元素→先 capture_ui_state 再 `recognizer.find_element_detailed`，记 bounds 到 trace） |
| `runner/interpreter/param_extractor.rs` | `ParamExtractor`：从 `TksParam` 提取 text/number/duration/direction |
| `agent/` | ③ 对话式设备 AI agent（`tke harness`，像 coding agent）。子模块：provider(genai 对接)/prompt(可自定义提示词)/tools/perception/execution/knowledge/transcript/interaction/**runner**。详见 agent/mod.rs 注释 + `docs/tke-flow.md` |
| `agent/runner/` | 主 AI 调度层。`orchestrator`(主 AI 会话循环+工具调度+文件操作+授权) · `testrun`(explore：驱动→.tks 写工作区+sidecar，自收尾) · `tksops`(路径化 replay_tks/repair_tks/optimize_tks) · `flow`(驱动循环) · `doctor`/`reflect`/`verify`/`supervisor`/`asserter`(测试专长子 agent) · `options`/`interrupt`。**无固定流水线、无常驻运行态**——.tks 都是工作区文件，主 AI 按需调度 |

### tools/（④ 自有工具）
| 文件 | 职责 |
|------|------|
| `element.rs` | `add_element(device, element_path, name, desc, x, y, cached, force)`：按坐标取最小可见元素→建当前平台结构通道→crop 模板图存 `img/`→OCR 兜底→合并写库（见 §4.4） |
| `device.rs` | `DeviceManager`：设备详细信息（硬件/电池/网络/prop） |
| `file.rs` | `FileManager`：Android 文件系统（tree/find/cat/ls/mkdir/rm/cp/mv/pull/push） |
| `app.rs` | `AppManager`：应用管理（list 三方应用/uninstall/当前焦点/launch/stop） |

### cli/（翻译层，参数→调库）
| 文件 | 职责 |
|------|------|
| `mod.rs` | 按四大块组织 + 重新导出命令 Args/Commands |
| `help.rs` | `build_help()` 总览（手工排版，直通清单动态扫描） |
| `passthrough.rs` | 直通 `handle(args, device)`（拒绝路径样参数，调 ToolManager） |
| `atomic/{refresh,fetch,recognize,control}.rs` | 各原子命令的 clap Args/Commands + `handle(args, device, …)` |
| `workflow/{run,steps,case}.rs` | 工作流命令 handle；`printer.rs`=`EventPrinter`（人读 Pretty / 机读 NDJSON 双模式，`auto(force_json)`：非 TTY 或 --json → NDJSON） |
| `tools/{device,file,app,element,ocr}.rs` | 各工具命令 handle |

> **cli 一致模式**：每个命令一个 `Args`/`Commands`（clap）+ 一个 `handle(args, device, [element/log/json…]) -> Result<()>`，只解析参数并调库层，末尾 `JsonOutput`/`EventPrinter` 输出。

---

## 3. 四端归一化映射（关键！三端统一的根基）

三个驱动都把页面归一化成**同一套 uiautomator 风格扁平 XML**，坐标统一为**截图像素**：
`<node class resource-id content-desc text clickable enabled bounds="[x1,y1][x2,y2]" />`

| 归一化属性 | Android (uiautomator) | Web (DOM) | iOS (XCUI) |
|------------|------------------------|-----------|------------|
| `class` | class | 标签名 tag | XCUIElementType* |
| `resource-id` | resource-id | DOM id | name（accessibility id） |
| `content-desc` | content-desc | aria-label | label |
| `text` | text | 直接文本 ownText | value\|label |
| `bounds` | 原生像素 | CSS × devicePixelRatio | 逻辑点 × scale |

因此 **fetcher / recognizer / 截图标注 / element.json 全部三端通用**，无需为每端写一套。
element.json 的平台通道与此映射互为正逆（落库时反推，识别时正用，见 `Locator::structural`）。

---

## 4. 核心数据流

### 4.1 多端驱动分发（Controller 统一 API）
`Controller::new(device_id: Option<String>)` → `Driver{Adb|Web|Wda}`，方法 match 分发：

| 方法 | 同步? | 说明 |
|------|------|------|
| `capture_ui_state(workarea)` / `capture_xml_only(workarea)` | **async** | 采集截图+XML / 仅XML，**写入 workarea 文件**（不返回字符串） |
| `tap/swipe/press/input_text/key_event/back/home/launch_app/stop_app/clear_input/hide_keyboard` | sync | 设备操作 |
| `get_device_info` | sync | 设备信息 |

`ControlAction`（atomic/control.rs，注意与 TksCommand **不完全对齐**）：
`Click{point}` · `Press{point,duration_ms:u32}` · `Swipe{from,to,duration_ms:u32}` · `SwipeDir{from,direction,distance:i32,duration_ms:u32}` · `Input{text,point:Option<Point>}` · `Clear` · `HideKeyboard` · `Back` · `Home` · `Launch{package,activity}` · `Close{package}` · `Key{code}`

### 4.2 `tke run x.tks` 回放全链路
```
ScriptRunner.run(path, log, on_event)
 └ ScriptParser.parse_file → TksScript{steps}
 └ Workarea::temp_for_run()                     # 临时工作区，用完删
 └ RunArtifacts::create(log, stem)              # 仅 --log
 └ for step in steps:
      ScriptInterpreter.interpret_step(step)
        └ CommandExecutor.execute_X(params)     # 按 TksCommand 分发
             └ TargetResolver.resolve(param)    # {元素名}/{x,y}/文本 → Point
                  └ capture_ui_state()          # 元素定位前先采集当前页
                  └ Recognizer.find_element_detailed()  # auto 多通道，返回 (Point,Bounds)
             └ Controller.tap/swipe/...         # 直接调驱动
      （--log）save_step → 标注截图 + page xml → StepResult
      失败即停；步间 100ms
 └ write_log(ExecutionResult) → log.json
 └ workarea.cleanup()                           # web 会话不销毁（由脚本 关闭 控制）
 └ RunEvent::RunEnd
```
> 注意：**tks 执行走 `command_executor` 直接调 Controller**；而 `tke control` / AI agent 走 `Control::execute(ControlAction)`。两条并行的"操作设备"路径，殊途同归到 Controller（见 §7 可疑点）。

### 4.3 元素识别 auto 兜底（回放鲁棒性核心）
`Recognizer.find_element_detailed(name, Auto)`：取 `Locator.structural(platform)`，依次尝试
**resource_id → xpath → text → content_desc → class_name**（任一字段存在且匹配即返回）→
**OCR 通道**（locator.ocr，在线识别文字）→ **图像通道**（locator.img，tke-opencv 模板匹配）。
返回的 `Bounds` 来自当前页面实际匹配框（结构=元素框 / ocr=文字框 / img=模板尺寸），供截图标注。

### 4.4 元素落库 `tke element add "名" --at x,y`
`tools::element::add_element`：① 采集页面（cached 跳过）；② 取包含坐标的**最小可见元素**（有 text/desc/id 标识的优先，≤4 倍面积内）；③ 按平台写结构通道（android/ios/web，是 §3 映射的逆向）；④ 按 bounds 从截图 crop 模板图存 `<库目录>/img/<名>.png` 填 img 通道；⑤ ocr 通道取结构文本，无文本的图标对 crop 图跑 OCR 兜底（>30% 屏幕面积的大容器不跑）；⑥ 合并写库（img/ocr 已有值不覆盖，`--force` 强制）。

### 4.5 原子回放循环（CI / AI 用）
`refresh → recognize --cached → control → 循环`。fetch/recognize 不带 `--cached` 会自动先 refresh。
设备缓存工作区 `$TMPDIR/tke/workarea/<device>/` 跨进程共享，使 `--cached` 能复用上一条命令的采集结果。

### 4.6 产物 / 事件 / 输出
- **产物**（仅 `--log`）：`RunArtifacts` → `<log>/<时间戳>_<名>/{log.json, screenshots/step_NNN.png, page/step_NNN.xml}`。flow：`<log>/<时间戳>_flow_<名>/` 下每脚本再起一个 `RunArtifacts`（即嵌套时间戳目录）+ `flow.json`。
- **事件**：`RunEvent`（NDJSON）。`EventPrinter::auto`：终端→Pretty 进度格式；管道/`--json`→NDJSON。
- **退出码**：成功 0 / 失败 1；JSON 输出统一走 `JsonOutput`。

---

## 5. 核心数据模型（精确字段，已 grep 核对）

**UIElement**（页面解析出的元素）
```
index:usize, class_name:String, bounds:Bounds, text:Option<String>,
content_desc:Option<String>, resource_id:Option<String>, hint:Option<String>,
clickable/checkable/checked/focusable/focused/scrollable/selected/enabled:bool,
xpath:Option<String>(Fetcher 生成), z_index:Option<usize>,
parent_index:Option<usize>(skip), depth:usize(skip), sibling_index:usize(skip)
```

**ExecutionResult**（log.json）
```
success:bool, case_id:String, script_name:String, start_time:String, end_time:String,
steps:Vec<StepResult>, error:Option<String>,
script_path:Option<String>, run_dir:Option<String>, launched_packages:Vec<String>
```
**StepResult**：`index:usize, command:String, success:bool, error:Option<String>, duration_ms:u64, line:Option<usize>, screenshot:Option<String>(相对run_dir), xml:Option<String>(相对run_dir)`

**element.json（schema v3）**——顶层 `elements`（key=元素名，唯一主键）
```json
{
  "elements": {
    "登录按钮": {
      "desc": "可选说明",
      "android": { "xpath":null, "resource_id":null, "text":null, "content_desc":"Login", "class_name":null },
      "ios":     null,   // { xpath, name, label, value, class_name }
      "web":     null,   // { css, xpath, id, text, aria, tag }
      "img": "img/登录按钮.png",   // 相对 element.json 所在目录
      "ocr": "Login"
    }
  }
}
```
- 平台通道按 `-d` 自动选用；字段可 null 不可缺。
- `Locator::structural(platform)` 把 ios/web 通道统一映射成 AndroidLocator 形态供匹配引擎用
  （ios: name→resource-id, label→content-desc, value|label→text；web: id→resource-id, aria→content-desc, tag→class）。
- **不存 bounds/clickable**（换设备即失效）；运行时实时匹配。

---

## 6. tks 脚本语言规范

一行一步：`命令 [参数1, 参数2]`（也支持无括号 `返回` / 空格式）。在 `步骤:` 行之后解析，`#` 注释。

**命令**（`TksCommand`，12 个）：`启动 关闭 点击 按压 滑动 定向滑动 输入 清理 隐藏键盘 返回 等待 断言`
**参数**（`TksParam`）：`{元素名}` / `{元素名}&策略` / `{x,y}` / `"文本"` / 数字 / 方向(上下左右) / 布尔(存在/不存在)

| 写法 | 含义 |
|------|------|
| `点击 [{登录按钮}]` | 元素名→元素库定位 |
| `点击 [{930, 2294}]` | 裸坐标（不可移植） |
| `输入 [{用户名}, "abc"]` | 先聚焦元素再输入 |
| `按压 [{元素}, 1000]` | 长按 1000ms |
| `定向滑动 [{x,y}, 上, 距离]` | 从点向方向滑 |
| `等待 [2000]` / `等待 [{元素}]` / `等待 [{元素}, 90]` | 固定毫秒 / 等元素出现(默认30s) / 自定义超时 |
| `断言 [{元素}, 存在]` | 校验存在/不存在 |
| `启动 [包名, .Activity]` / `启动 [https://…]` / `启动 [bundleId]` | android/web/ios |

> **当前无"AST→文本"反向序列化**（parser 只单向 文本→AST）。录制/AI 生成 tks 需自己拼字符串——见 §7。

---

## 7. 关键约定

- **workarea**：与项目目录解耦，全在 `$TMPDIR/tke/`。设备缓存区 `workarea/<device>/`（持久，原子命令用）；run 临时区 `run-<ts>-<pid>/`（用完删）；web/ios 会话状态另存 `web/<device>.json`、`ios/<udid>.json`。
- **配置优先级**：CLI 显式参数 > `--config` 指定 > tke 同目录 `config.toml` > 无。config 相对路径基于 config 文件所在目录。
- **element 默认查找**：`./element.json` → `./locator/element.json`（`Recognizer::new` 与 `tools/element.rs` **各硬编码一份**，见可疑点）。
- **产物命名**：`<时间戳YYYYMMDD-HHMMSS>_<脚本名>/`；flow 为 `<时间戳>_flow_<名>/` 下每脚本再嵌套。
- **会话生命周期**：web/ios 单脚本/原子命令跑完**不自动销毁**（便于联动/调试）；flow 结束统一清场。`关闭`/`control close` 显式销毁。
- **坐标系**：对外一律截图像素，driver 内部各自换算（web÷dpr、ios÷scale、android 原生）。

---

## 8. 待确认 / 可疑点 / 改进机会

> 这些是通读中发现的、对后续重构（尤其"统一规范 + 解耦功能"）有价值的点。

1. **`DEFAULT_ELEMENT_PATHS` 重复硬编码**：`recognizer/mod.rs:19` 与 `tools/element.rs:14` 各一份 `["element.json","locator/element.json"]`。应收敛到单一来源。
2. **两套"操作设备"分发**：`atomic/control.rs`（`ControlAction`→`Control::execute`）与 `workflow/interpreter/command_executor.rs`（直接调 `Controller`）并存。AI agent 走前者、tks `run` 走后者——同一批 Controller 方法被两层各包一遍。可考虑统一。
3. **`TksCommand` 与 `ControlAction` 操作集不一致**：TksCommand 有 `断言/等待` 无 `Home/Key/Drag`；ControlAction 有 `Home/Key/Drag` 无 `断言/等待`。两个"操作枚举"各自演化，未对齐。
4. **tks 无反向序列化**：只有 parser（文本→AST），没有 serializer（AST→文本）。录制器、AI 生成器要产 tks 只能手拼字符串，易与 parser 失配。**建议补 `TksStep::to_source()` 作为双向契约。**
5. **离线 OCR 不可定位**：`ocr/offline.rs` 把整图当一个文本块返回（bbox=全图，confidence 假定 0.9）。OCR 识别通道实际依赖在线服务（URL 硬编码 `recognizer/ocr.rs:10`）。
6. **adb `hide_keyboard` = KEYCODE_BACK**：可能误触返回导航（代码在 input_text 里特意不隐藏键盘以规避）。
7. **adb `press`(长按) = 同点 swipe**：并非真长按，部分控件可能不识别。
8. **Fetcher 疑似死代码**：`optimize_ui_tree`/`generate_tree_string`/`extract_ui_elements_with_size`/`infer_screen_size_from_xml`/各 `filter_*` 似未被调用，待确认清理。
9. **flow 产物嵌套时间戳**：实际是 `<log>/<ts>_flow_<名>/<ts>_<script>/`（flow.rs 注释写的是 `<脚本名>/`，与实现不符）。
10. **`TksScript.case_id/script_name/details` 基本是空字段**：parser 只填 `steps`，这三个恒为空（`script_name` 由 script_runner 回退到文件名）。vestigial，待确认用途或清理。
11. **`DeviceInfo.android_version` 跨平台复用**：web 存 "Chrome for Testing" 在 model、iOS 版本塞进 `android_version` 字段——字段名语义不准。
12. **agent（AI case）产物未遵循 RunArtifacts 规范**：当前自建 `conversation.jsonl`/`*.screens` 放 `--script` 同级，未用 `--log`/`RunArtifacts`/`step_NNN`/`ExecutionResult`。**这是已知待统一项**（见下"规划方向"）。
13. **无 project 根目录概念**：当前 `--log`/`--element` 各自独立。拟引入的 `<project>/{logs,elements,scripts}` 统一规范尚未落地。

---

## 9. 规划方向（讨论中，未实现）

目标：**功能解耦 + 规范统一**。把"规范"沉淀成共享模块（单一来源），各功能（回放/录制/AI）解耦地依赖它：
- **tks 契约**：补 序列化（与 parser 配对）→ 录制/AI 产出的 tks 与 run 字节级同构。
- **ProjectLayout**：config 配一个 project 根，派生 `logs/elements/scripts`，按需创建；run/steps/case 统一命名 `<name>_<时间戳>`。
- **RunArtifacts 升为共用**：case 复用它（log.json/screenshots/page + 额外 conversation.jsonl），与 run 同构。
- 录制器与 AItester 共享"tks 生成会话"骨架（落库+生成行+产物），只换"决策来源"（人 / AI）。

> 详细方案见团队讨论记录；落地前需逐项确认，确保不破坏现有子工具功能。
