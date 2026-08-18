# ADR-0016 删掉 CLI 直通：设备操作一律经 tke 转译

- 日期：2026-08-18
- 状态：已落地（本机实测）
- 关联：ADR-0011（设备是工具的参数）、INV-9（失败必须可见）、`docs/platform-matrix.md`

## 背景

`tke <工具名> <原生参数>` 会把命令原样透传给同目录的二进制（`tke adb shell …`、
`tke ffmpeg …`）。当初的想法是"tke 作为所有测试工具的统一入口，新增工具零代码改动"。

问题出在**它同时也成了操作设备的第二条路**，而这条路绕开了 tke 的全部保障：

- **绕过证据留存**。`tke adb shell input tap 500 800` 点得中，但没有截图、没有 log、
  报告里一片空白。而 skill 从第一句起就在讲「用 `steps` 而不是 `control`，因为它留证据」——
  留着一条什么都不留的路，等于把这句话作废
- **绕过坐标换算**。web 的 `devicePixelRatio`、iOS 的 scale 都在驱动层处理；
  拿 adb / JS 自己算坐标必然偏
- **绕过唯一的动作映射**。`execute_action` 是「动作 → 设备」的单一来源，
  直通完全不经过它——同一个"点击"因此有了两种语义

一句话：直通让**"点得中但什么都没发生/没留下"**多了一条入口。这两天连修五个假结论
bug（P-35 ~ P-39）都是同一族，不该再自己留一个。

## 决策

**① 删掉 CLI 直通**（`external_subcommand` 的透传分支、`ToolManager::passthrough`、
`list_available`、`--help` 里的动态清单）。认不出的命令一律报「未知命令」并指路。

**② 保留 `ToolManager::resolve`**。它是**内部定位器**，adb / chromedriver / go-ios /
tke-opencv 都靠它找到自己在哪——这跟"让用户透传"是两回事。

**③ 保留 `tke <path.tks>` 便捷路由**。它挂在同一个 `external_subcommand` 上，
但它是**tke 自己的能力**，不是透传。

**④ 补上唯一真正的缺口：`tke app log`**（logcat）。删之前盘了一遍，
只有它无可替代——App 崩了、点了没反应，真因常常只在设备日志里。
按**包名的 PID** 过滤而不是 grep 包名：崩溃堆栈那几行**不含包名**，
grep 会把最有用的一段滤掉。默认 `*:W` 200 行——拉全量会把 AI 的上下文冲爆。

`screenrecord` 没补：逐步标注截图已经覆盖了"过程留证"，而录屏产物大、还得人去看。

## 代价（明说）

tke 没覆盖的场景变成死路。比如临时想 `adb shell settings put` 造个测试前置，
现在做不了——**得给 tke 加一条命令**。

这是有意为之：加命令要想清楚"另外两端怎么办"（见 `docs/platform-matrix.md` 的检查单），
而透传把这个问题绕过去了。慢一点，但每条路都留痕、都统一。

## 自查

- `tke adb devices` → 未知命令 + 指路，退出码 2
- `tke x.tks` → 照常跑
- `tke -d <设备> app log -p <包名>` → 出该包的日志
