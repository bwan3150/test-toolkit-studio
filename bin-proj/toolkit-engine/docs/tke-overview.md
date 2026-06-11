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
| `-c/--config <tke.toml>` | 配置文件 = 自动输入上述参数；CLI 显式参数优先 |
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

## 元素库（element.json, schema v2）

key = 元素名（唯一主键，人类可读，脚本里 `{元素名}` 直接引用，**不设独立 id**）。
一个完整元素 = 多端标识 + 两条通用通道，**字段可 null 不可缺**：

```json
{
  "Devices入口": {
    "desc": "首页 Devices 卡片，点击进入设备管理页",
    "xml":  { "xpath": null, "resource_id": null, "text": null,
              "content_desc": "Devices", "class_name": null },
    "wda":  null,
    "dom":  null,
    "img":  "img/devices.png",
    "ocr":  "Devices"
  }
}
```

- `xml`=Android / `wda`=iOS / `dom`=Web：多端 App 一个元素三套标识
- `img` 路径**相对 element.json 所在目录** → 元素库（json+img/）自包含可搬云端
- `ocr`：OCR 识别用文字
- **不存 bounds/clickable**（换设备/分辨率即失效）；标注和返回的 bounds
  一律来自运行时实际匹配到的元素（xml=元素框, ocr=文字框, img=模板尺寸）
- auto 策略兜底顺序：xml → ocr → img（拿不到 xml/dom 的页面靠 ocr/img）
- 元素图用 `tke refresh --crop "x1,y1,x2,y2" --out img/xxx.png` 截取

## .tks 脚本

```
步骤:
启动 [com.example.app, .MainActivity]
点击 [{Devices入口}]          # {元素名} 经元素库定位; {元素名}&xpath 指定策略
点击 [{930, 2294}]            # {x,y} 直接坐标
等待 [{某元素}]               # 等元素出现(30s超时); 等待 [2000] 为固定毫秒
断言 [{Settings入口}, 存在]
关闭 [com.example.app]
```

每一行 = 一步。命令：启动 关闭 点击 按压 滑动 定向滑动 输入 清理 隐藏键盘 返回 等待 断言

## 设备工具命令

`ocr`（图片文字识别）/ `file`（设备文件系统）/ `app`（应用管理）/ `device`（设备信息）

> 旧版 controller/fetcher/recognizer/run project 命令及 -p/--project 参数已全部移除，
> Toolkit Studio 重构时按本文档新接口对接。
