# tke 命令速查（tke-ui-test 用得上的部分）

**没装 tke？**（`tke: command not found`）——skill 只是文档，二进制要单独装：

```bash
curl -fsSL https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke/install.sh | bash
export PATH="$HOME/.tke/bin:$PATH"    # 当前会话立即可用（安装器只写了 rc 文件）
```

装好后 `tke update` 升级、`tke uninstall` 卸载，不用再记这串 URL。

全局参数：`-d <设备>` 必带（**iOS 只有 macOS 能做**，其余平台 tke 会拦下）；`--log <dir>` 留证据；`--cache <dir>` 并发隔离；
`--headless=on|off` 强制无头/有头（**必须带等号**）。
**浏览器默认无头**（不抢鼠标）；要开窗口让人手动登录/扫码时用 `--headless=off`，
开一次即可，后续命令会沿用那个会话。

## 看：页面上有什么

```bash
tke -d web fetch --interactive        # 可交互元素 JSON（text/bounds/resource_id/xpath）← 主要输入
tke -d web fetch                      # 全部元素（含纯文本节点，判断结果时用）
tke -d web fetch --cached             # 复用上次采集，不重新抓（省一次往返）
tke -d web fetch --ocr offline        # 图标没有文字时，用 OCR 补可读文字
tke -d web fetch --wait-text "已保存" --timeout 30
                                      # ★ 等这段文字出现（异步下发/跨设备同步用）
                                      #   出现即刻返回；超时非零退出，`||` 直接接住
                                      #   多候选写 "A|B"；查全量，纯文本标签也找得到
tke -d web refresh                    # 采集截图 + 页面结构，路径在输出里
tke -d web refresh --crop 100,200,400,500 --out /tmp/x.png   # 剪裁某块区域
```

元素中心点 = `((x1+x2)/2, (y1+y2)/2)`，就是要点的坐标。

**`fetch` / `refresh` 四种设备是同一套**（安卓 / iOS 真机 / iOS 模拟器 / 网页）——
把 `-d web` 换成序列号或 `sim:<UDID>` 就行，输出格式、坐标口径（**一律截图像素**）、
`--interactive` / `--ocr` / `--wait-text` 全都一样。底层差别（uiautomator / XCUI / AX 树 / DOM）
tke 已经归一化掉了，你不用关心。

**iframe 里的内容也会采到**（同源的递归进去采，坐标已换算好，直接点就行；xpath 带
`iframe[1]>>` 前缀标明层级）。**跨域** iframe 拿不到里面——那是浏览器的安全边界，
会留一条 `[跨域内容，采不到内部元素]` 的记录，看到它就知道**不是"页面空了"**（C-24）。

**原生对话框（alert/confirm/prompt）`fetch` 采不到**——它是浏览器画的、不在 DOM 里。
弹出时看每步结果的 `dialog` 字段，用 `确认对话框` / `取消对话框` 处理（C-23）。

## 做：操作设备

**优先用 `steps --log`**（会留证据），`control` 只在不需要留痕时用：

```bash
tke -d web steps '点击 [{640, 380}]' --log ~/.tke/logs/<任务简称>/
tke -d web control click 640,380      # 等价操作，但什么都不留
```

`control` 子命令：`click` `press`(长按) `swipe` `drag` `swipe-dir` `input` `clear`
`hide-keyboard` `back` `home` `launch` `close` `key` `switch` `hover`(web)

## 浏览器专属（`-d web`）

```bash
tke -d web control browser-reset             # 回到「首次访问」：清 cookie/localStorage/sessionStorage/IndexedDB/缓存
tke -d web control browser-eval "localStorage.getItem('token')"   # 在页面里跑一段 JS
tke -d web control browser-viewport 390x844  # 改视口测响应式（iPhone 竖屏）
tke -d web control browser-download --dir ~/dl            # 指定下载目录（无头 Chrome 默认不落盘）
tke -d web control browser-download --dir ~/dl --wait 15  # 并等下载完成，打印文件路径
tke -d web control browser-dialog accept     # 处理原生对话框（steps 里写 `确认对话框`）
```

**`reset` 什么时候必须用**：浏览器会话跨命令复用，登录态会一直带着。测登录、首访引导、
权限弹窗前不清，你以为在测新用户，其实看到的是老用户视角——这类假结论最难发现。

**`eval` 的边界**：用来**观察和造前置状态**（读 storage、看 window 上的状态、mock 时间）。
**别拿它代替用户操作**——直接调函数改状态，测的就不是真链路了，那正是这个 skill 存在的意义。

**`--wait` 靠"目录里有下完的文件"判定**（`.crdownload` 还在就继续等）。
它**分不出新旧**——每条 CLI 命令都是独立进程，记不住基线。要区分就用个空目录。

**页面报错会自动收**：每一步执行后，tke 会把这步里的 console.error、未捕获异常、
加载失败的请求写进结果（`errors` 字段，终端也会打）。**「点了没反应」最常见的真因就在这儿**，
而它在页面结构和截图里都看不见——先看这行，再怀疑功能本身。

## 安卓：包名和 Activity 从哪来

启动 App 需要包名 + Activity，**不知道就查，别猜**：

```bash
tke -d <序列号> app focus             # 当前前台应用的包名和 Activity ← 最快的办法
tke -d <序列号> app list              # 设备上所有第三方应用及版本
tke -d <序列号> app launch <包名> <Activity>
tke -d <序列号> app stop <包名>
```

拿到之后脚本里写：`启动 ["com.example.app", ".MainActivity"]`

## 设备信息

```bash
tke device list                       # ★ 有哪些可测目标（安卓/iOS真机/iOS模拟器/浏览器）
                                      #   第一列就是 -d 要填的值；查不了的那类会说明原因
tke -d <ID> device info               # 某台的详情：型号/屏幕尺寸/系统版本
                                      #   四端都能用；安卓另有硬件/电池/网络
tke -d <序列号> device prop <属性名>   # 安卓系统属性（adb getprop，仅安卓）
```

## 安卓文件系统（需要看日志/配置文件时）

```bash
tke -d <序列号> file ls /sdcard/
tke -d <序列号> file cat /sdcard/x.log
tke -d <序列号> file find /sdcard/ "*.apk"
tke -d <序列号> file tree /sdcard/Download
tke -d <序列号> file write <设备路径> <内容>
tke -d <序列号> file rm|cp|mv ...
```

## iOS 模拟器

```bash
tke device list                                  # 找 sim:<UDID>
tke -d sim:<UDID> steps '启动 ["com.example.app"]' --log ~/.tke/logs/<任务简称>/
tke -d sim:<UDID> fetch --interactive            # 之后跟别的设备一模一样
```

**第一次用先 `启动 [BundleID]`，别直接 `fetch`。** tke 要把 WebDriverAgent 拉进模拟器
才能操作，而拉起它会**把当前前台 App 挤到后台**（`simctl launch` 必然带到前台，没有
后台启动选项）。`启动` 这条会一次性做完两件事：拉起 WDA + 把目标 App 带到前台。

直接 `fetch` 的话，采到的多半是**桌面那一屏图标**——tke 会认出来并报错，不会让你
拿着一屏 Fitness/通讯录 去找按钮。只有第一次会这样，WDA 起来后就一直跑着。

包名不知道就查：`xcrun simctl listapps <UDID> | grep -i <你的App名>`
（`tke app` 那套是安卓专属的，iOS 用不了）。

缺 WebDriverAgent 的话 `tke doctor` 会说，装它一条命令：`tke doctor --fix --profile ios`。

## 安卓：看设备日志（App 崩了 / 点了没反应）

```bash
tke -d <序列号> app log -p com.example.app          # 只看这个 App 的日志（按 PID 过滤）
tke -d <序列号> app log -p com.example.app -n 500   # 多取几行
tke -d <序列号> app log -l E                        # 只看 Error
```

**「点了没反应」先看这里再下结论**——崩溃堆栈、后台异常都在设备日志里，
页面结构和截图里一个字都看不到。网页侧对应的东西 tke 会自动收（见上面「页面报错会自动收」）。

> **tke 不透传原生工具**：`tke adb shell …` 这类用法已经删掉了。设备操作一律走 tke 指令
> ——直通绕过证据留存和坐标换算，点得中、什么都没留下、报告里一片空白。
> 缺什么能力就提，别绕路。

## 收尾

```bash
tke -d web control close              # web 省略包名 = 销毁会话（浏览器+driver+会话文件）
tke -d <序列号> app stop <包名>        # 安卓关掉启动过的 App
```

## 排查

- web 起不来：chromedriver 日志在 `$TMPDIR/tke/web/chromedriver-<端口>.log`
- 会话信息：`$TMPDIR/tke/web/<设备>.json`
- 页面采集缓存：`$TMPDIR/tke/workarea/<设备>/`
- **切换有头/无头时**必须先 `control close`，否则会沿用旧模式的会话（tke 会拦住并提示）

## 收尾：写结论 + 打开报告

```bash
tke report ~/.tke/logs/<任务简称>/ \
  --task "用户让我验的那件事" \
  --verdict pass|fail|blocked \
  --summary "一句话结果" \
  --open                      # 用系统浏览器打开（无图形界面时自动跳过）
tke report <目录> --full-image  # 原图版，逐像素复核用
```

`--verdict`：`pass`=功能可用 / `fail`=**被测对象有问题** / `blocked`=没验成。
**某一步没点中不算 `fail`**——那是过程里的无效尝试，见 SKILL.md。

## 证据目录里有什么

```
~/.tke/logs/<任务简称>/
├── report.html   全程报告（自带截图，可直接转发）
├── screenshots/  每步标注截图
├── pages/        每步**元素表 JSON**（"这一页的元素库"）← 想回看页面读这个，别重新 fetch
├── raw_pages/    每步**原始页面**（浏览器 DOM / uiautomator 原文，没被 tke 动过）
└── log.json      每批命令/成败/耗时 + 任务与结论
```

`pages/` vs `raw_pages/`：实测同一页 **原始 1151 个标签 → 元素表 74 个**。
定位不到某个元素时对比这两份，就知道是被筛掉了还是页面上根本没有。
