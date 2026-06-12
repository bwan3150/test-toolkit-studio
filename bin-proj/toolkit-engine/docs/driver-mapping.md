# 驱动指令映射对照表

tke 原子指令在各端驱动下的底层实现对照，用于排查平台间行为差异。

- **Android**: `src/atomic/controller/adb.rs`（adb，串号寻址 `-d <serial>`）
- **iOS**: `src/atomic/controller/wda.rs`（WebDriverAgent HTTP 协议，`-d <UDID>` 或 `-d wda:<UDID>`，UDID 格式自动识别）
- **Web**: `src/atomic/controller/web.rs`（chromedriver + Chrome for Testing，W3C WebDriver HTTP 协议，`-d web`）

## 会话与基础设施

| | Android (adb) | iOS (wda) | Web (chromedriver) |
|---|---|---|---|
| 常驻服务 | adb server daemon（adb 自带） | go-ios 隧道 + runwda（拉起设备上的 WDA）+ 端口转发，**全部由 tke 自动拉起**并跨命令常驻 | chromedriver HTTP 服务，tke 首次使用自动拉起 |
| 会话持久化 | 无需（手机状态天然持续） | `$TMPDIR/tke/ios/<udid>.json` 存 {port, forward_pid, runwda_pid, session_id, scale}，跨 tke 进程复用 | `$TMPDIR/tke/web/<id>.json` 存 {port, session_id, pid}，跨 tke 进程复用，失效自动重建 |
| 浏览器/系统依赖 | 设备开 USB 调试 | 设备开发者模式 + WDA 已装（一次性 Xcode 安装）；go-ios 与 tke 同目录 | chromedriver 与 tke 同目录；Chrome for Testing 在 `~/Library/Application Support/tke/`（版本须与 driver 配对；**不可放 ~/Documents 等 TCC 目录**） |
| 坐标系 | 物理像素（=截图像素） | 对外统一截图像素；内部 ÷scale（视网膜倍率，/wda/screen 获取，如 iPhone 12=3）转逻辑点执行 | 对外统一截图像素；内部 ÷devicePixelRatio 转 CSS 坐标执行 |

## 原子指令

| tke 指令 | Android (adb) | iOS (wda) | Web (chromedriver) |
|---|---|---|---|
| `refresh` 截图 | `shell screencap -p /sdcard/...` + `pull` + `rm` | `GET /screenshot`（base64 PNG，全屏物理像素） | `GET /screenshot`（base64 PNG，视口范围） |
| `refresh` 结构 | `shell uiautomator dump /sdcard/...` + `pull`（uiautomator XML） | `GET /session/:id/source`（XCUI 元素树）→ **归一化为 uiautomator 风格 XML**（resource-id=name, content-desc=label, text=value\|label, class=XCUIElementType*, bounds=点×scale 像素；仅可见元素，跳过空容器） | `POST /execute/sync` 注入 JS 遍历可见元素 → **归一化为 uiautomator 风格 XML**（resource-id=DOM id, content-desc=aria-label, text=直接文本, bounds=像素） |
| `fetch` | 解析 uiautomator XML → UIElement 列表（含生成的 xpath） | 同一解析器解析归一化 XML（三端产物结构一致） | 同左 |
| `recognize` xml 通道 | 对结构文件按 xpath/resource-id/text/content-desc/class 匹配 | 同左（归一化后同一套匹配代码；元素库 ios 通道字段映射: name→resource-id, label→content-desc, value→text） | 同左 |
| `recognize` ocr/img 通道 | 对截图做 OCR / tke-opencv 模板匹配（**平台无关**） | 同左 | 同左 |

## control 操作

| tke 操作 | Android (adb) | iOS (wda) | Web (chromedriver) |
|---|---|---|---|
| `click x,y` | `shell input tap x y` | `POST /wda/tap` {x,y}（÷scale） | `POST /actions` pointer: move→down→up |
| `press x,y,ms` | `shell input swipe x y x y ms`（同点滑动模拟长按） | `POST /wda/touchAndHold` {x,y,duration 秒} | pointer: move→down→**pause(ms)**→up |
| `swipe x1,y1 x2,y2 [ms]` | `shell input swipe x1 y1 x2 y2 ms` | `POST /wda/dragfromtoforduration`（真实触摸拖动） | `POST /actions` **wheel 滚轮**：在起点滚动 delta=(起点-终点)。桌面浏览器拖拽不滚动页面，故映射为滚轮；duration 不生效 |
| `drag` | 同 swipe（默认 1500ms） | 同 swipe | 同 swipe（滚轮） |
| `swipe-dir x,y 方向 距离` | 计算终点后 `input swipe` | 计算终点后 dragfromto | 计算终点后滚轮 |
| `input "text"` | 切换 IME（中文检测→切对应输入法）→ `shell input text`（空格→%s 转义）→ 恢复原 IME | `POST /wda/keys`（输入到当前聚焦元素，需先点击输入框） | execute JS：原生 value setter + 派发 input/change 事件（兼容 React/Vue 受控组件）；目标=当前聚焦元素 |
| `clear` | `KEYCODE_DEL`×20 + `KEYCODE_FORWARD_DEL`×20 | `/wda/keys` 退格×50（WDA 无全局 clear） | execute JS：value='' + input 事件（聚焦元素） |
| `hide-keyboard` | `keyevent KEYCODE_BACK` | `POST /wda/keyboard/dismiss`（键盘未弹出时忽略错误） | no-op（无软键盘） |
| `back` | `keyevent KEYCODE_BACK` | **左缘右滑手势**（iOS 无系统返回键；App 可拦截手势） | `POST /back`（history.back，**仅对真实导航有效**，下拉菜单/弹层无效） |
| `home` | `keyevent KEYCODE_HOME` | `POST /wda/homescreen` | 导航到 `about:blank` |
| `launch` | `shell am start -n 包名/Activity`（双参数） | 单参数 BundleID；无会话→创建会话随之拉起（唯一会话创建入口），有会话→`/wda/apps/launch` | `POST /url` 导航（单参数 URL，无协议自动补 https://） |
| `close` | `shell am force-stop 包名` | `POST /wda/apps/terminate` {bundleId}；空参 = `DELETE /session` 销毁会话（保留转发/隧道/WDA 进程） | `DELETE /session` + kill chromedriver（销毁整个会话） |
| `key CODE` | `keyevent <CODE>`（任意 Android keycode） | 仅 ENTER（→输入 \n）/KEYCODE_BACK（→返回手势）；其余忽略 | 仅映射 ENTER/TAB/ESC/BACKSPACE → WebDriver key actions；KEYCODE_BACK→back；其余忽略 |

## 已知行为差异（排查备忘）

| 差异点 | Android | iOS | Web |
|---|---|---|---|
| `启动` 参数 | 2 个（包名+Activity） | 1 个（BundleID） | 1 个（URL） |
| `关闭` 后再 `启动` | App 冷启动，可能恢复上次页面 | App 结束；WDA 会话仍在（除非 control close 空参销毁） | 全新会话+空白历史 |
| `返回` 失效场景 | App 自行拦截 back 事件（如 AI Chat 页） | App 禁用边缘滑动手势的页面 | 点击未产生导航（下拉/弹层/锚点） |
| `swipe` 语义 | 触摸拖动（可拖元素/滑列表） | 触摸拖动（同 Android） | 滚轮滚动（不能拖元素，拖拽需后续单独实现 pointer-drag） |
| 会话生命周期 | 无会话概念 | 单脚本/原子指令保留会话；flow 结束关 App+销毁会话 | 单脚本/原子指令保留会话；flow 结束销毁会话 |
| 单步典型耗时 | 3-4s（含采集） | 1-1.5s（含采集） | 0.2-0.5s |
| 前置条件 | adb 可见即可 | 设备装过 WDA 即可（tke 经 go-ios 自动启动，冷启动约 10s） | 无（chromedriver 自动拉起） |
