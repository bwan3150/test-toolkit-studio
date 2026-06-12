# toolkit-engine (tke)

> tke = 所有测试工具的统一入口 / 协调器，与项目目录完全解耦

## 架构：三大块

```
CLI / CI / Electron App
          │ spawn
          ▼
┌───────────────────────── tke ─────────────────────────────┐
│                                                           │
│  ① 工具直通          ② 原子方法            ③ 工作流        │
│  (透传同目录二进制)   (必带 -d/--device)    (组合②完成)     │
│                                                           │
│  tke adb ...         tke refresh           tke run x.tks  │
│  tke aapt ...        tke fetch             tke run x.toml │
│  tke k6 ...          tke recognize         tke steps ...  │
│  tke ffmpeg ...      tke control           tke case ...   │
│  (放进 bin/ 即可用)                                        │
└───────────────────────────────────────────────────────────┘
          │
   stdout: JSON / NDJSON 实时事件    stderr: 日志    exit: 0/1
```

## 全局参数（与项目目录解耦）

| 参数 | 说明 |
|------|------|
| `-d/--device` | 目标设备（refresh/fetch/recognize/control 必带） |
| `--element <path>` | 元素库 element.json（缺省找 ./element.json → ./locator/element.json） |
| `--log <dir>` | 产物输出目录；**不传则 run 不保存任何产物**（CI 纯跑模式） |
| `-c/--config <toml>` | 配置文件 = 自动输入上述参数；**缺省自动读 tke 同目录的 config.toml**；优先级: CLI 参数 > --config 指定 > 同目录 config.toml（同时只用一个） |
| `--json` | 强制 NDJSON 输出（默认终端=友好格式，管道=NDJSON 自动切换） |

```toml
# tke.toml（相对路径基于本文件所在目录）
device = "f64b3b4d"
element = "locator/element.json"
log = "logs"
```

```bash
tke run smarthome_smoke.tks -c tke.toml     # 一个 -c 搞定全部参数
```

**workarea 不再存在于项目里**：原子命令用 `$TMPDIR/tke/workarea/<设备ID>/`
（跨进程共享，`recognize --cached` 依赖它）；run 工作流用每次运行独立的
临时目录，结束自动删除。

## 多端驱动（-d 决定）

| -d | 驱动 | 说明 |
|----|------|------|
| `<Android序列号>` | adb | 手机/模拟器 |
| `web` | chromedriver + Chrome for Testing (W3C WebDriver) | 网页，体验与 App 完全一致 |
| `<iOS UDID>` 或 `wda:<UDID>` | WebDriverAgent (HTTP) | iPhone/iPad；UDID 格式（25位带连字符）自动识别 |

各端底层指令映射对照（排查平台差异用）见 [driver-mapping.md](driver-mapping.md)。

**web 驱动要点**：
- 浏览器会话跨 tke 进程持久（信息存 `$TMPDIR/tke/web/`），首次使用自动拉起
- **生命周期**：会话由脚本的 `关闭` 指令控制——单脚本/steps/原子命令运行后
  **不自动销毁**（保留复用：继续调试、测多脚本联动）；**flow 结束统一收尾清场**
  （web=销毁浏览器会话；android/ios=关闭 flow 期间`启动`过的所有 App，ios 再销毁 WDA 会话）；
  `control close` 随时显式销毁；每次新建会话前自动收割孤儿 Chrome
- `启动 [URL]` 在**当前会话当前 tab 内跳转**（不开新浏览器/新 tab），
  无会话时才自动创建；**只有 启动/导航 会创建会话**，其余操作
  （fetch/点击/截图等）要求已有会话，否则报错引导先启动

**iOS 驱动要点**：
- 基础设施全自动：tke 经同目录的 go-ios 自动管理隧道/拉起 WDA/USB 转发
  （冷启动约 10s，热复用 <1s；状态与日志在 `$TMPDIR/tke/ios/`）；
  唯一一次性前置是用 Xcode 给设备装 WDA（见 [setup-notes.md](setup-notes.md)）
- 会话生命周期与 web 一致：`启动 [BundleID]` 是唯一会话创建入口，
  单脚本/原子命令不自动销毁，flow 结束统一清场
- 坐标统一为截图像素，scale（视网膜倍率）自动换算；XCUI 元素树归一化为
  uiautomator 风格 XML，识别/标注与 Android/Web 同一套代码
- **`关闭` 的语义**：web 端"应用"即浏览器会话，`关闭 [URL]` 销毁整个会话
  （含所有页面），与 Android `关闭 [包名]` 杀整个 App 对称；参数写被测站点
  URL 仅为可读性，寻址实际由 -d 决定（一个 -d web 对应一个会话）
- chromedriver 与 tke 同目录；Chrome for Testing 的存放位置/版本配对/macOS
  各类坑见 [setup-notes.md](setup-notes.md)
- 页面元素经 JS 提取后**归一化为 uiautomator 风格 XML**：resource-id=DOM id、
  content-desc=aria-label、text=直接文本、bounds=截图像素坐标（已乘
  devicePixelRatio）——元素库 xml 通道、ocr/img 通道、标注、fetch 全部直接复用
- 操作语义映射：启动 [URL]（单参数）/ 返回=history.back / 滑动=滚轮 /
  输入=原生 setter+input 事件（兼容 React/Vue）/ 主页=about:blank
- 窗口固定 1280×900 且 `--force-device-scale-factor=1`：不同机器/显示器
  渲染一致，脚本像素坐标可移植
- 注意：网页 Header 下拉菜单不产生导航历史，`返回` 只对真实跳转有效

## ① 工具直通

透传给 tke 同目录（`bin/<platform>/`）下任意二进制，新增工具零代码：

```bash
tke adb shell pm list packages    # -d 自动转 adb -s
tke k6 run load.js
tke ffmpeg -i in.mp4 out.gif
tke opencv ...                    # 自动匹配 tke-opencv
```

## ② 原子方法（必带 -d）

```bash
tke refresh -d X [--xml-only] [--ocr [lang]] [--crop "x1,y1,x2,y2" --out 元素图.png]
# 刷新页面状态到工作区 → {"success":true,"screenshot":"...","xml":"..."}
tke fetch -d X [--cached] [--interactive]
# 提取当前页面元素列表（含 xpath），直接输出 JSON 数组；--cached 用工作区已有状态
tke recognize -d X 登录按钮 [-s auto|xpath|id|text|desc|class|ocr|img] [--threshold 0.6] [--cached]
# → {"success":true,"element":"登录按钮","x":540,"y":733,"bounds":{...}}  bounds=运行时实时匹配框
tke control -d X click 540,733 / press 100,200,800 / swipe 500,1500 500,300 / input "hi" --at 300,500
# 其他: drag / swipe-dir / clear / hide-keyboard / back / home / launch / close / key
```

回放循环：`refresh → recognize --cached → control → 循环至完成`
（fetch/recognize 不带 --cached 时会自动先 refresh 一次）

## ③ 工作流

```bash
tke run <path.tks>      # 单脚本（按扩展名识别）
tke run <path.toml>     # flow 多脚本顺序执行（同上）
tke steps "启动 [...]" "点击 [{登录按钮}]" "断言 [{首页}, 存在]"
                        # 不落文件执行一串指令（编辑器逐行调试 / AI 循环用）
tke case <用例.md|"用例文字"> --script <导出.tks路径>
                        # AI 探索测试并生成脚本（透传 tester-ai 实现）
```

flow 为 TOML：

```toml
name = "冒烟测试"
scripts = ["login.tks", "devices.tks"]   # 按顺序执行，路径相对本文件
```

**输出双模式**（退出码均 0/1）：
- 终端里跑 → 人类友好进度格式（彩色、步骤序号、耗时，无时间戳噪音）：
  ```
  ▶ smarthome_smoke (10 步)
    [ 1/10] 启动 [com.konec.smarthome, ...] ... ✓ 5.9s
    [ 2/10] 点击 [{930, 2294}] ... ✓ 56ms
  ✓ 通过 (10/10 步)
    产物: logs/20260611-180925_smarthome_smoke
  ```
- 管道/重定向（App spawn、CI）→ 自动切 NDJSON 事件流（每行一个事件：
  run_start / step_start / step_end / run_end / flow_* / script_*）；
  `--json` 可强制 NDJSON
- 日志默认只输出 WARN 以上到 stderr；`-v` 输出 DEBUG

### 产物（仅 --log 时保存）

```
<log>/<时间戳>_<脚本名>/
├── log.json              完整日志: 脚本path、起止时间、每步命令/成败/完整报错/耗时
├── screenshots/
│   └── step_001.png      每步标注截图: 顶部横幅(步骤号+操作+OK/FAIL) +
│                          红框(运行时实际匹配的元素框) + 蓝点(点击坐标) + 蓝线(滑动)
└── page/
    └── step_001.xml      每步页面结构文件 (xml/wda/dom)

flow: <log>/<时间戳>_flow_<名>/ 下 flow.json + 每脚本一个子目录（结构同上）
```

## 元素库（element.json, schema v3）

顶层 = `elements`（key=元素名，唯一主键，脚本里 `{元素名}` 直接引用，
不设独立 id）。每个元素按**平台名**分通道 + 两条通用通道，**字段可 null 不可缺**：

```json
{
  "elements": {
    "Devices入口": {
      "desc": "首页 Devices 卡片，点击进入设备管理页",
      "android": { "xpath": null, "resource_id": null, "text": null,
                   "content_desc": "Devices", "class_name": null },
      "ios": null,
      "web": null,
      "img": "img/devices.png",
      "ocr": "Devices"
    },
    "产品菜单": {
      "desc": "官网顶部导航 Products",
      "android": null,
      "ios": null,
      "web": { "css": null, "xpath": null, "id": null,
               "text": "Products", "aria": null, "tag": null },
      "img": null,
      "ocr": "Products"
    }
  }
}
```

- 平台通道按 `-d` 自动选用：`-d <序列号>`→android、`-d web`→web、`-d <UDID>`→ios
- ios 通道字段：`{xpath, name, label, value, class_name}`（name=accessibility id，
  label=accessibility label，匹配引擎映射: name→resource-id, label→content-desc, value→text）
- `img` 路径**相对 element.json 所在目录** → 元素库（json+img/）自包含可搬云端
- **不存 bounds/clickable**（换设备/分辨率即失效）；标注和返回的 bounds
  一律来自运行时实际匹配到的元素（结构=元素框, ocr=文字框, img=模板尺寸）
- auto 策略兜底顺序：平台结构标识 → ocr → img
- 元素图用 `tke refresh --crop "x1,y1,x2,y2" --out img/xxx.png` 截取

## .tks 脚本

```
步骤:
启动 [com.example.app, .MainActivity]   # Android; web 为 启动 [https://...]
点击 [{Devices入口}]          # {元素名} 经元素库定位; {元素名}&xpath 指定策略
点击 [{930, 2294}]            # {x,y} 直接坐标
等待 [{某元素}]               # 等元素出现(默认30s超时); 等待 [{某元素}, 90] 自定义超时秒数; 等待 [2000] 为固定毫秒
断言 [{Settings入口}, 存在]
关闭 [com.example.app]
```

每一行 = 一步。命令：启动 关闭 点击 按压 滑动 定向滑动 输入 清理 隐藏键盘 返回 等待 断言

## ④ 自有工具

`ocr`（图片文字识别）/ `file`（设备文件系统）/ `app`（应用管理）/ `device`（设备信息）/
`element`（元素库管理）

**element add——按坐标取元素落库**（测试人员定位元素的主要入口）：

```bash
tke element add "帮助菜单" --at 786,57 --desc "顶部导航 Help" -d web -c tke.toml
```

自动完成三件事：① 取该坐标处最小可见元素（带 text/desc/id 标识的优先），按 `-d`
平台写入对应结构通道（android/ios/web）；② 按元素 bounds 从当前截图 **crop 出
模板图**存 `<库目录>/img/<元素名>.png` 并填 img 通道；③ ocr 通道取结构文本，
无文本的图标类元素自动对 crop 图跑 OCR 兜底。已有元素则合并更新（img/ocr 已有值
时不覆盖，`--force` 强制；落库即可被 recognize/脚本 `{元素名}` 引用）。

## src 目录结构（四大模块）

```
src/
├── passthrough/   ① 直通: ToolManager(通用二进制透传) + adb/aapt 路径管理
├── atomic/        ② 原子: refresh/fetch/recognize/control
│   ├── controller/    设备驱动 (adb / wda / web)
│   ├── fetcher/       UI XML 解析 (元素提取/xpath 生成)
│   └── recognizer/    元素识别引擎 (xml/ocr/图像三通道)
├── workflow/      ③ 工作流: run/steps/case + 产物/事件
│   └── runner/        .tks 解析器 + 解释器
├── tools/         ④ 自有工具: ocr/file/app/device/element
├── models/  utils/(workarea/config/json_output)
├── handlers/      CLI 命令处理器（镜像四大块分目录）
├── lib.rs  main.rs
```

`tke --help` = 原子指令 + 工作流 + 自有工具（静态）+ 当前二进制目录下
所有可直通二进制（动态扫描，末尾列出）；不存在的二进制会提示
"xxx 可执行文件缺失或不完整"。

> 旧版 controller/fetcher/recognizer/run project 命令及 -p/--project 参数已全部移除，
> Toolkit Studio 重构时按本文档新接口对接。
