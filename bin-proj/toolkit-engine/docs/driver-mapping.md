# 驱动指令映射对照表

tke 原子指令在各端驱动下的底层实现对照，用于排查平台间行为差异。

- **Android**: `src/atomic/controller/adb.rs`（adb，串号寻址 `-d <serial>`）
- **iOS**: WebDriverAgent（预留，未实现，`-d wda:<id>`）
- **Web**: `src/atomic/controller/web.rs`（chromedriver + Chrome for Testing，W3C WebDriver HTTP 协议，`-d web`）

## 会话与基础设施

| | Android (adb) | iOS (wda) | Web (chromedriver) |
|---|---|---|---|
| 常驻服务 | adb server daemon（adb 自带） | — | chromedriver HTTP 服务，tke 首次使用自动拉起 |
| 会话持久化 | 无需（手机状态天然持续） | — | `$TMPDIR/tke/web/<id>.json` 存 {port, session_id, pid}，跨 tke 进程复用，失效自动重建 |
| 浏览器/系统依赖 | 设备开 USB 调试 | — | chromedriver 与 tke 同目录；Chrome for Testing 在 `~/Library/Application Support/tke/`（版本须与 driver 配对；**不可放 ~/Documents 等 TCC 目录**） |
| 坐标系 | 物理像素（=截图像素） | — | 对外统一截图像素；内部 ÷devicePixelRatio 转 CSS 坐标执行 |

## 原子指令

| tke 指令 | Android (adb) | iOS (wda) | Web (chromedriver) |
|---|---|---|---|
| `refresh` 截图 | `shell screencap -p /sdcard/...` + `pull` + `rm` | — | `GET /screenshot`（base64 PNG，视口范围） |
| `refresh` 结构 | `shell uiautomator dump /sdcard/...` + `pull`（uiautomator XML） | — | `POST /execute/sync` 注入 JS 遍历可见元素 → **归一化为 uiautomator 风格 XML**（resource-id=DOM id, content-desc=aria-label, text=直接文本, bounds=像素） |
| `fetch` | 解析 uiautomator XML → UIElement 列表（含生成的 xpath） | — | 同一解析器解析归一化 XML（两端产物结构一致） |
| `recognize` xml 通道 | 对结构文件按 xpath/resource-id/text/content-desc/class 匹配 | — | 同左（归一化后同一套匹配代码） |
| `recognize` ocr/img 通道 | 对截图做 OCR / tke-opencv 模板匹配（**平台无关**） | 同左 | 同左 |

## control 操作

| tke 操作 | Android (adb) | iOS (wda) | Web (chromedriver) |
|---|---|---|---|
| `click x,y` | `shell input tap x y` | — | `POST /actions` pointer: move→down→up |
| `press x,y,ms` | `shell input swipe x y x y ms`（同点滑动模拟长按） | — | pointer: move→down→**pause(ms)**→up |
| `swipe x1,y1 x2,y2 [ms]` | `shell input swipe x1 y1 x2 y2 ms` | — | `POST /actions` **wheel 滚轮**：在起点滚动 delta=(起点-终点)。桌面浏览器拖拽不滚动页面，故映射为滚轮；duration 不生效 |
| `drag` | 同 swipe（默认 1500ms） | — | 同 swipe（滚轮） |
| `swipe-dir x,y 方向 距离` | 计算终点后 `input swipe` | — | 计算终点后滚轮 |
| `input "text"` | 切换 IME（中文检测→切对应输入法）→ `shell input text`（空格→%s 转义）→ 恢复原 IME | — | execute JS：原生 value setter + 派发 input/change 事件（兼容 React/Vue 受控组件）；目标=当前聚焦元素 |
| `clear` | `KEYCODE_DEL`×20 + `KEYCODE_FORWARD_DEL`×20 | — | execute JS：value='' + input 事件（聚焦元素） |
| `hide-keyboard` | `keyevent KEYCODE_BACK` | — | no-op（无软键盘） |
| `back` | `keyevent KEYCODE_BACK` | — | `POST /back`（history.back，**仅对真实导航有效**，下拉菜单/弹层无效） |
| `home` | `keyevent KEYCODE_HOME` | — | 导航到 `about:blank` |
| `launch` | `shell am start -n 包名/Activity`（双参数） | — | `POST /url` 导航（单参数 URL，无协议自动补 https://） |
| `close` | `shell am force-stop 包名` | — | `DELETE /session` + kill chromedriver（销毁整个会话） |
| `key CODE` | `keyevent <CODE>`（任意 Android keycode） | — | 仅映射 ENTER/TAB/ESC/BACKSPACE → WebDriver key actions；KEYCODE_BACK→back；其余忽略 |

## 已知行为差异（排查备忘）

| 差异点 | Android | Web |
|---|---|---|
| `启动` 参数 | 2 个（包名+Activity） | 1 个（URL） |
| `关闭` 后再 `启动` | App 冷启动，可能恢复上次页面 | 全新会话+空白历史 |
| `返回` 失效场景 | App 自行拦截 back 事件（如 AI Chat 页） | 点击未产生导航（下拉/弹层/锚点） |
| 历史/状态残留 | App 自身保存的状态 | 浏览器会话不销毁则 history/cookie 一直在，影响 `返回` |
| `swipe` 语义 | 触摸拖动（可拖元素/滑列表） | 滚轮滚动（不能拖元素，拖拽需后续单独实现 pointer-drag） |
| `等待 [{元素}]` 轮询开销 | 每秒 screencap+uiautomator dump（约 2-4s/次） | 每秒 screenshot+JS 提取（约 0.2-0.4s/次） |
| 单步典型耗时 | 3-4s（含采集） | 0.2-0.5s |
