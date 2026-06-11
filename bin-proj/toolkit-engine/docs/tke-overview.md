# toolkit-engine (tke)

> tke = 所有测试工具的统一入口 / 协调器

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
│  tke adb ...         tke fetch             tke run script │
│  tke aapt ...        tke recognize         tke run flow   │
│  tke k6 ...          tke control           tke run ai     │
│  tke ffmpeg ...                            tke run step   │
│  (放进 bin/ 即可用,                                        │
│   零代码扩展)                                              │
└───────────────────────────────────────────────────────────┘
          │
   stdout: JSON / NDJSON 实时事件    stderr: 日志    exit: 0/1
```

## ① 工具直通

透传给 tke 同目录（`bin/<platform>/`）下的任意二进制，新增工具直接放文件，无需改代码：

```bash
tke adb shell pm list packages      # -d 自动转为 adb -s <id>
tke aapt dump badging app.apk
tke k6 run load.js
tke ffmpeg -i in.mp4 out.gif
tke opencv ...                      # 自动匹配 tke-opencv
```

## ② 原子方法（必带 -d）

```bash
# fetch: 采集页面 → workarea/current_screenshot.png + current_ui_tree.xml
tke fetch -d <dev> [--xml-only] [--elements] [--ocr [lang]] [--crop "x1,y1,x2,y2" --out 元素图.png]

# recognize: 定位元素（默认先重新采集；--cached 用已有页面状态）
tke recognize -d <dev> 登录按钮 [-s auto|xpath|id|text|desc|class|ocr|img] [--threshold 0.6] [--by-text] [--cached]
# → {"success":true,"element":"登录按钮","x":540,"y":1200,"bounds":{...}}

# control: 统一操作名（与 .tks 命令对应），坐标格式 x,y 或 x,y,毫秒
tke control -d <dev> click 540,1200
tke control -d <dev> press 100,200,800
tke control -d <dev> swipe 500,1500 500,300 -t 400
tke control -d <dev> swipe-dir 540,960 up 400
tke control -d <dev> input "hello" --at 300,500
tke control -d <dev> launch com.app .MainActivity
# 其他: drag / clear / hide-keyboard / back / home / close / key
```

回放循环（脚本正式执行时的内部逻辑）：`fetch → recognize → control → 循环至完成`

## ③ 工作流

```bash
tke run script <path.tks> -d <dev> [-p <project>]   # 逐行实时 NDJSON + 完整产物
tke run flow <flow.json> -d <dev>                   # 依次执行一组脚本
tke run ai <args...>                                # 透传 tester-ai（AI 探索生成 .tks）
tke run step "点击 [{100,200}]" -d <dev>            # 单行调试（编辑器用）
```

### run script 实时输出（NDJSON，每行一个事件）

```json
{"event":"run_start","script":"login.tks","total_steps":5,"run_dir":"runs/20260611-143000_login",...}
{"event":"step_start","index":0,"line":5,"command":"启动 [com.app, .Main]"}
{"event":"step_end","index":0,"line":5,"success":true,"duration_ms":2300,"screenshot":"steps/step_001.png","xml":"steps/step_001.xml"}
{"event":"step_end","index":1,"line":6,"success":false,"error":"元素未找到: 登录按钮 (所有定位策略均失败)",...}
{"event":"run_end","success":false,"successful_steps":1,"run_dir":"...","log":".../run.json"}
```

CI 实时监控和 App 编辑器逐行跟踪都消费这个事件流。

### 每次运行的完整产物

```
<project>/runs/<时间戳>_<脚本名>/
├── run.json              完整日志: 脚本path、起止时间戳、每步命令/成败/完整报错/耗时
└── steps/
    ├── step_001.png      每步截图（标注: 红框=目标元素框, 蓝点=点击坐标, 蓝线=滑动轨迹）
    └── step_001.xml      每步 UI 结构文件（定位问题用）

flow 运行: runs/<时间戳>_flow_<名>/ 下含 flow.json + 每个脚本一个子目录（结构同上）
```

flow 文件格式：`{"name": "冒烟测试", "scripts": ["a.tks", "b.tks"]}`（路径相对 flow 文件）

### run ai（AI 探索生成 .tks）

由同目录 `tester-ai` 二进制实现，tke 仅作为入口透传。循环：fetch 采集 → AI 决策元素+操作 → 元素入库 → 写 .tks → 真机执行 → 再 fetch，直到达成用例目标。产物（.tks + 元素库 + log + AI 对话 raw data）之后即可用 `run script` 无 AI 回放。

## .tks 脚本

```
步骤:
启动 [com.example.app, .MainActivity]
点击 [{登录按钮}]            # {元素名} 经 recognize 定位; {元素名}&xpath 指定策略
点击 [{100,200}]             # {x,y} 直接坐标
输入 [{用户名输入框}, hello]
等待 [2000]
断言 [{首页标题}, 存在]
关闭 [com.example.app]
```

命令：启动 关闭 点击 按压 滑动 定向滑动 输入 清理 隐藏键盘 返回 等待 断言

## legacy 兼容命令

Electron App handlers 仍在调用的旧命令全部保留：
`controller` / `fetcher` / `recognizer` / `file` / `app` / `device` / `ocr` / `run project`
（分别被 fetch/control、fetch --elements、recognize、run flow 替代，App 迁移后可删）

> 注意：`run script` 输出已从单个 JSON 改为 NDJSON 事件流，App 的
> `handlers/tke-integration/runner-handlers.js` 接入时需按行解析。
