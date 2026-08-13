# tke 命令速查（tke-ui-test 用得上的部分）

全局参数：`-d <设备>` 必带；`--log <dir>` 留证据；`--cache <dir>` 并发隔离；
`--headless=on|off` 强制无头/有头（**必须带等号**）。

## 看：页面上有什么

```bash
tke -d web fetch --interactive        # 可交互元素 JSON（text/bounds/resource_id/xpath）← 主要输入
tke -d web fetch                      # 全部元素（含纯文本节点，判断结果时用）
tke -d web fetch --cached             # 复用上次采集，不重新抓（省一次往返）
tke -d web fetch --ocr offline        # 图标没有文字时，用 OCR 补可读文字
tke -d web refresh                    # 采集截图 + 页面结构，路径在输出里
tke -d web refresh --crop 100,200,400,500 --out /tmp/x.png   # 剪裁某块区域
```

元素中心点 = `((x1+x2)/2, (y1+y2)/2)`，就是要点的坐标。

## 做：操作设备

**优先用 `steps --log`**（会留证据），`control` 只在不需要留痕时用：

```bash
tke -d web steps '点击 [{640, 380}]' --log .tke-ui-test/
tke -d web control click 640,380      # 等价操作，但什么都不留
```

`control` 子命令：`click` `press`(长按) `swipe` `drag` `swipe-dir` `input` `clear`
`hide-keyboard` `back` `home` `launch` `close` `key` `switch` `hover`(web)

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
adb devices                           # 有哪些安卓设备（tke 没有列设备的子命令）
tke -d <序列号> device info           # 硬件/电池/网络等完整信息
tke -d <序列号> device prop <属性名>   # 单个 prop
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

## 直通原生工具

tke 同目录下的二进制可以直接透传：

```bash
tke -d <序列号> adb shell dumpsys battery     # 注意 -d 要放在 adb 前面
tke ffmpeg -i in.mp4 out.gif
```

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
