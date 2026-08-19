# 平台能力矩阵

**一句话**：哪些原子动作三端都有、哪些是某个平台独有、哪些"名字一样但干的事不一样"。

写给两拨人看：
- **开发**：加动作前先看这张表——`ControlAction` 每加一个变体，就要回答"另外两端怎么办"
- **调用方 AI**（skill / harness）：跨端脚本别写上不支持的动作；同名不同义的地方别想当然

> **来源**：`src/drivers/mod.rs` 的 `Controller` 分发 + 各驱动实现。改了实现要回来改这张表。
>
> **想知道"底层落成什么 adb / HTTP 调用"**，看 [`driver-mapping.md`](driver-mapping.md)。
> 那份面向排查，这份面向调用和加新动作。**两份说法不一致本身就是坑**，改代码时一起改。

## 图例

| 记号 | 含义 |
|---|---|
| ✅ | 真做，语义与其它平台一致 |
| ≈ | 做得到，但**实现路径或语义不同**（见下方「同名不同义」） |
| ❌ | 不支持，**明确报错**（不是静默跳过） |
| — | 这个平台没有这个概念 |

## 通用动作（三端都有）

| 动作 | tks 指令 | Android | iOS 真机 | iOS 模拟器 | Web | 说明 |
|---|---|---|---|---|---|---|
| 点击 | `点击` | ✅ | ✅ | ✅ | ✅ | 坐标一律是**截图像素**；web 内部按 dpr 换算成 CSS 坐标 |
| 长按 | `按压` | ≈ | ✅ | ✅ | ✅ | Android 用「同点 swipe」模拟，没有真正的 longpress |
| 滑动 | `滑动` / `定向滑动` | ✅ | ✅ | ✅ | ✅ | web 是滚动容器，不是触摸拖拽 |
| 输入 | `输入` | ≈ | ✅ | ✅ | ≈ | 见下方「同名不同义」 |
| 清空输入 | `清理` | ✅ | ≈ | ≈ | ✅ | iOS 两边都是连发 50 个退格（没有"全选删除"这种整体操作） |
| 按键 | `按键` | ✅ | ≈ | ≈ | ≈ | Android 认全部 keycode；**iOS 两边都只认 ENTER/BACK**（同一套 WDA）；web 认 ENTER/TAB/ESC/BACKSPACE/BACK + 单个字符。**认不出一律报错**，不静默跳过（P-40） |
| 返回 | `返回` | ✅ | ≈ | ≈ | ≈ | Android=BACK 键 / **iOS 两边都是左边缘右滑手势**（App 能拦截它，所以未必生效）/ web=浏览器后退 |
| 主页 | （无） | ✅ | ✅ | ✅ | ≈ | iOS 两边都走 WDA `/wda/homescreen`；web 的"主页"= `about:blank` |
| 启动 | `启动` | ≈ | ≈ | ≈ | ≈ | 参数完全不同，见下 |
| 关闭 | `关闭` | ≈ | ≈ | ≈ | ≈ | 安卓 force-stop / iOS 真机 WDA terminate（空参=销毁会话）/ 模拟器 `simctl terminate`（模拟器没有"会话"）/ web 销毁整个会话 |
| 切换 | `切换` | ≈ | ≈ | ≈ | ≈ | 移动端切 App 到前台 / web 切标签页或开新标签 |
| 隐藏键盘 | `隐藏键盘` | ≈ | ✅ | ✅ | — | Android 是按 BACK；web 无软键盘，**空操作** |
| 采集页面 | `fetch` / `refresh` | ✅ | ✅ | ✅ | ✅ | 统一成 uiautomator 风格 XML（web 由 DOM 归一化而来） |

## 平台独有

| 动作 | 谁有 | 说明 |
|---|---|---|
| 悬停 `悬停` | **Web** | 触摸屏没有 hover 这回事；Android / iOS 真机 / iOS 模拟器调用一律**明确报错** |
| 下拉选择 `选择` | **Web** | 原生 `<select>` 展开后由浏览器绘制，DOM 里看不见，点不到，只能 DOM 设值 + 派事件 |
| `control browser-reset` | **Web** | 清 cookie / localStorage / sessionStorage / IndexedDB / 缓存 |
| `control browser-eval` | **Web** | 在页面里跑 JS |
| `control browser-viewport` | **Web** | 改视口测响应式断点 |
| `control browser-download` | **Web** | 设下载目录 / 等下载完成 |
| `control browser-dialog`<br>`确认对话框`/`取消对话框`/`对话框输入` | **Web** | alert/confirm/prompt 是**浏览器画的**，不在 DOM 里，采不到也点不到 |
| 标签页列表 | **Web** | 移动端返回空列表（不报错——"没有标签页"是事实，不是错误） |
| 页面报错采集 | **Web** | 每步自动收 console.error / 未捕获异常 / 加载失败请求 |
| 软键盘等待 | Android / iOS | `has_soft_keyboard()`；web 和 fake 不为此白等 |
| 应用管理 `tke app` | Android / iOS | 列表 / 卸载 / 当前前台应用 |
| 设备文件系统 `tke file` | **Android** | 走 adb；iOS 沙盒限制拿不到 |
| 设备信息 `tke device` | Android / iOS | web 返回浏览器与窗口信息 |

## 同名不同义（最容易想当然的地方）

**`启动`**
```
启动 ["com.example.app", ".MainActivity"]   # Android：包名 + Activity（用 tke app focus 查，别猜）
启动 ["com.example.BundleID"]               # iOS：BundleID；同时也是唯一的会话创建入口
启动 ["https://example.com"]                # Web：导航到 URL（会话已存在就复用）
```

**`关闭`**
```
关闭 ["com.example.app"]   # 移动端：force-stop 这个 App，设备还在
关闭                       # Web：销毁整个会话 —— 浏览器进程、driver、会话文件一起没
```
web 这条**会连登录态一起销毁**。要人工登录的流程中途别关（踩过：C-6）。

**`切换`**
```
切换 ["2"]                    # Web：切到第 2 个标签页
切换 ["https://..."]          # Web：用新标签打开
切换 ["com.example.app"]      # 移动端：把这个 App 切到前台（内部走 launch）
```

**`输入`**
- **Android**：中文要切输入法（tke 会自动切换再切回），所以比另外两端慢
- **iOS**：直接发 keys
- **Web**：往 `document.activeElement` 里设值 + 派发 input/change 事件（React 那套受控组件必须这样才认）
- 三端都要求**先聚焦**（点一下目标输入框）

**`返回`**
- Android：`KEYCODE_BACK`
- iOS：从屏幕左边缘往右拖 —— 是**手势模拟**，某些 App 里不生效
- Web：浏览器后退，等于 `history.back()`

**坐标口径**：所有平台的元素 `bounds` 和点击坐标都是**截图像素**。
web 内部按 `devicePixelRatio` 换算成 CSS 坐标，调用方不用管——但**别自己拿 adb / JS 去算**，
那会绕过换算（这也是不建议直通操作设备的原因之一）。

## iOS：真机与模拟器是两条接入路

| | 真机（`-d <UDID>`） | 模拟器（`-d sim:<UDID>`） |
|---|---|---|
| 靠什么 | **WebDriverAgent**（HTTP + JSON） | **同一个 WebDriverAgent** |
| 怎么连上 | go-ios 建隧道 + USB 端口转发 → 随机端口 | 与主机共享网络，直连 **8100** |
| 设备上的 runner 谁装 | 一次性 Xcode 安装，**必须签名**（Apple 硬要求） | tke 自己：`simctl install` 一个预编译 `.app`，**免签名** |
| runner 谁拉起 | tke（go-ios `runwda`，免 Xcode） | tke（`simctl launch`，**连 xcodebuild 都不用**） |
| 装起来 | go-ios 随 `tke doctor --fix` 下载；WDA 要 Xcode 装一次 | **`tke doctor --fix --profile ios` 全包**（下 21MB 的 .app 到 `~/.tke/wda/`） |
| 协议之后的一切 | **完全同一套代码**——元素树、坐标换算、点击、截图 | 同左 |

**为什么两边都用 WDA**：协议是 HTTP+JSON（客户端代码早就在跑），归一化现成，
分发物只有 21MB。曾经短暂走过 idb（`brew install` 免签名），但一旦决定
**自己分发预编译产物、锁版本**，WDA 就全面胜出了——详见 ADR-0017 的「修订」。

**tke 不需要内置 WDA 源码**：它只说 WDA 的 HTTP 协议，分发的是**编译好的 runner**。

**模拟器端口写死 8100**：模拟器与主机共享网络，**多台模拟器同时跑会撞端口**。
单台够用；要并行得给每台传 `USE_PORT`。

**`Platform` 仍然只有三个**（Android/iOS/Web）：模拟器跑的是同一套 iOS App，
元素库的 ios 通道、定位策略、归一化目标格式全都复用。多出来的只是**驱动**
（`Driver::IosSim`），跟 `Driver::Fake` 一样——有驱动，没平台。

## 加新动作时的检查单

1. 三端都有吗？没有的话，**没有的那端要明确报错**，不能 `Ok(())` 静默跳过
   （踩过：iOS 的 `按键 ["TAB"]` 原先返回成功却什么都不做，人以为焦点已经移走了）
2. 语义一致吗？不一致就写进上面「同名不同义」，别让人靠猜
3. 进 `ControlAction` 了吗？`execute_action` 是**唯一的动作 → 设备映射**，
   绕过它的话 `tke control` / tks 解释器 / AI agent 会各走各的
4. 平台独有的能力，命名上要看得出来（`browser-` 前缀）
5. 改了实现，回来改这张表
