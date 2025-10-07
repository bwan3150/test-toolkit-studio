# TKE CLI 使用示例

实际运行命令和输出示例。

---

## tke adb

将所有 `adb` 指令前面加上 `tke` 就可以通过内嵌的 adb 执行原生 adb 指令。

```bash
❯ tke adb --version

Android Debug Bridge version 1.0.41
Version 36.0.0-13206524
Installed as /var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb
Running on Darwin 24.6.0 (arm64)
```

```bash
❯ tke adb devices

List of devices attached
emulator-5554	device
R5CT20ABCDE	device
```

```bash
❯ tke adb shell "pm list packages | grep chrome"

package:com.android.chrome
```

---

## tke ocr

### 在线 OCR

HTTP 请求在线的 OCR 服务 API，返回纯 JSON。

```bash
❯ tke ocr --image /path/to/screenshot.png --online --url https://ocr.test-toolkit.app/ocr

{"texts":[{"text":"Settings","bbox":[[654.0,33.0],[1010.0,33.0],[1010.0,83.0],[654.0,83.0]],"confidence":0.95},{"text":"WiFi","bbox":[[210.0,559.0],[403.0,569.0],[400.0,634.0],[207.0,625.0]],"confidence":0.97},{"text":"Bluetooth","bbox":[[96.0,1053.0],[444.0,1053.0],[444.0,1103.0],[96.0,1103.0]],"confidence":0.91}]}
```

格式化后的 JSON：
```json
{
  "texts": [
    {
      "text": "Settings",
      "bbox": [[654.0, 33.0], [1010.0, 33.0], [1010.0, 83.0], [654.0, 83.0]],
      "confidence": 0.95
    },
    {
      "text": "WiFi",
      "bbox": [[210.0, 559.0], [403.0, 569.0], [400.0, 634.0], [207.0, 625.0]],
      "confidence": 0.97
    }
  ]
}
```

### 离线 OCR

使用本地内嵌的 tesseract（需要先安装 tessdata）。

```bash
❯ tke ocr --image /Users/eric_konec/Downloads/screenshot.png --lang eng

{"texts":[{"text":"Settings WiFi Bluetooth","bbox":[[0.0,0.0],[1080.0,0.0],[1080.0,1920.0],[0.0,1920.0]],"confidence":0.9}]}
```

**注意：** 离线模式目前需要先安装 tessdata：
```bash
brew install tesseract tesseract-lang
```

---

## tke controller

### 查看设备

```bash
❯ tke controller devices

2025-10-07T05:46:08.740363Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio"
2025-10-07T05:46:08.740832Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:46:08.753638Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
已连接的设备:
  - c9dc8614
```

### 截图和获取 UI 树

```bash
❯ tke controller capture

2025-10-07T05:47:42.517753Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:47:42.517794Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:47:42.533826Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
2025-10-07T05:47:45.331022Z  INFO tke::controller: UI状态已捕获并保存到workarea
UI状态已捕获并保存到workarea
```

生成文件：
- `workarea/current_screenshot.png`
- `workarea/current_ui_tree.xml`

### 点击和滑动

```bash
❯ tke controller tap 400 2000

2025-10-07T05:49:33.387028Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:49:33.387067Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:49:33.392784Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:49:33.772870Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
已点击坐标: (400, 2000)
```

```bash
❯ tke controller swipe 500 1500 500 500 --duration 300

2025-10-07T05:50:01.490219Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:50:01.490261Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:50:01.496548Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:50:01.737351Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
已滑动: (500, 1500) -> (500, 500) 持续300ms
```

### 应用控制

```bash
❯ tke controller launch com.android.settings .Settings

2025-10-07T05:50:16.612699Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:50:16.612740Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:50:16.619972Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:50:16.853855Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
2025-10-07T05:50:16.989574Z  INFO tke::controller: 启动应用: com.android.settings/.Settings
已启动应用: com.android.settings/.Settings
```

```bash
❯ tke controller stop com.android.settings

2025-10-07T05:50:40.921384Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:50:40.921420Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:50:40.932036Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:50:41.164778Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
2025-10-07T05:50:41.243136Z  INFO tke::controller: 停止应用: com.android.settings
已停止应用: com.android.settings
```

### 输入框输入

```bash
❯ tke controller input "Hello World"

2025-10-07T05:50:59.930393Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:50:59.930442Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:50:59.943884Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:51:00.182662Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
已输入文本: Hello World
```

### 系统级控制

```bash
❯ tke controller back

2025-10-07T05:51:26.704710Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:51:26.704758Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:51:26.710941Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:51:26.948042Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
已按返回键
```

```bash
❯ tke controller home

2025-10-07T05:51:38.795980Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"
2025-10-07T05:51:38.796020Z  INFO tke::adb_manager: 使用内置 ADB: adb
2025-10-07T05:51:38.802224Z  INFO tke::adb_manager: ✓ ADB 已提取到: "/var/folders/pq/l00t670n1fs_74g7bt9m460r0000gn/T/tke_adb/adb" (15212 KB)
2025-10-07T05:51:39.034565Z  INFO tke::adb_manager: ✓ ADB 验证成功: Android Debug Bridge version 1.0.41
已按主页键
```

### 获取 UI XML

```bash
❯ tke controller get-xml --output my_ui.xml

XML已保存到: my_ui.xml
```

---

## tke fetcher

返回 JSON 格式，可用 `jq` 处理。

### 提取 UI 元素

```bash
❯ tke fetcher extract-ui-elements < workarea/current_ui_tree.xml

从XML中提取了 35 个UI元素
[{"index":0,"class_name":"android.widget.FrameLayout","bounds":{"x1":0,"y1":0,"x2":1080,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[0]"},{"index":1,"class_name":"android.widget.HorizontalScrollView","bounds":{"x1":36,"y1":475,"x2":1032,"y2":625},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":true,"selected":false,"enabled":true,"xpath":"//node[1]"},{"index":2,"class_name":"android.view.View","bounds":{"x1":36,"y1":475,"x2":269,"y2":625},"text":null,"content_desc":"All Devices\nTab 1 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":true,"enabled":true,"xpath":"//node[2]"},{"index":3,"class_name":"android.view.View","bounds":{"x1":269,"y1":475,"x2":472,"y2":625},"text":null,"content_desc":"Network\nTab 2 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[3]"},{"index":4,"class_name":"android.view.View","bounds":{"x1":472,"y1":475,"x2":639,"y2":625},"text":null,"content_desc":"Switch\nTab 3 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[4]"},{"index":5,"class_name":"android.view.View","bounds":{"x1":639,"y1":475,"x2":810,"y2":625},"text":null,"content_desc":"Sensor\nTab 4 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[5]"},{"index":6,"class_name":"android.view.View","bounds":{"x1":810,"y1":475,"x2":943,"y2":625},"text":null,"content_desc":"Lock\nTab 5 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[6]"},{"index":7,"class_name":"android.view.View","bounds":{"x1":943,"y1":475,"x2":1032,"y2":625},"text":null,"content_desc":"Other\nTab 6 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[7]"},{"index":8,"class_name":"android.widget.ScrollView","bounds":{"x1":0,"y1":96,"x2":1080,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":true,"selected":false,"enabled":true,"xpath":"//node[8]"},{"index":9,"class_name":"android.widget.ImageView","bounds":{"x1":48,"y1":143,"x2":740,"y2":230},"text":null,"content_desc":"Test New User第十八次","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[9]"},{"index":10,"class_name":"android.widget.ImageView","bounds":{"x1":924,"y1":132,"x2":1032,"y2":240},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[10]"},{"index":11,"class_name":"android.view.View","bounds":{"x1":0,"y1":306,"x2":1080,"y2":439},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":true,"selected":false,"enabled":true,"xpath":"//node[11]"},{"index":12,"class_name":"android.widget.ImageView","bounds":{"x1":48,"y1":306,"x2":564,"y2":439},"text":null,"content_desc":"Turn Off All Lights","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[12]"},{"index":13,"class_name":"android.widget.ImageView","bounds":{"x1":564,"y1":306,"x2":1080,"y2":439},"text":null,"content_desc":"Turn Off All Lights","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[13]"},{"index":14,"class_name":"android.view.View","bounds":{"x1":0,"y1":625,"x2":1080,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[14]"},{"index":15,"class_name":"android.view.View","bounds":{"x1":36,"y1":625,"x2":525,"y2":983},"text":null,"content_desc":"Other Aircon 3","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[15]"},{"index":16,"class_name":"android.widget.ImageView","bounds":{"x1":351,"y1":659,"x2":525,"y2":809},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[16]"},{"index":17,"class_name":"android.view.View","bounds":{"x1":555,"y1":625,"x2":1044,"y2":983},"text":null,"content_desc":"Other Aircon 5","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[17]"},{"index":18,"class_name":"android.widget.ImageView","bounds":{"x1":870,"y1":659,"x2":1044,"y2":809},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[18]"},{"index":19,"class_name":"android.view.View","bounds":{"x1":36,"y1":1013,"x2":525,"y2":1370},"text":null,"content_desc":"Other Aircon 1","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[19]"},{"index":20,"class_name":"android.widget.ImageView","bounds":{"x1":351,"y1":1046,"x2":525,"y2":1196},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[20]"},{"index":21,"class_name":"android.view.View","bounds":{"x1":555,"y1":1013,"x2":1044,"y2":1370},"text":null,"content_desc":"Other Aircon 2","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[21]"},{"index":22,"class_name":"android.widget.ImageView","bounds":{"x1":870,"y1":1046,"x2":1044,"y2":1196},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[22]"},{"index":23,"class_name":"android.widget.ImageView","bounds":{"x1":36,"y1":1400,"x2":525,"y2":1757},"text":null,"content_desc":"Motion Sensor","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[23]"},{"index":24,"class_name":"android.widget.ImageView","bounds":{"x1":555,"y1":1400,"x2":1044,"y2":1757},"text":null,"content_desc":"Network Multimode Gateway 12","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[24]"},{"index":25,"class_name":"android.view.View","bounds":{"x1":36,"y1":1787,"x2":525,"y2":2144},"text":null,"content_desc":"Other Aircon 4","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[25]"},{"index":26,"class_name":"android.widget.ImageView","bounds":{"x1":351,"y1":1820,"x2":525,"y2":1970},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[26]"},{"index":27,"class_name":"android.view.View","bounds":{"x1":555,"y1":1787,"x2":1044,"y2":2144},"text":null,"content_desc":"Offline\nSensor Outdoor Siren Alarm ","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[27]"},{"index":28,"class_name":"android.widget.ImageView","bounds":{"x1":36,"y1":2174,"x2":525,"y2":2208},"text":null,"content_desc":"Lock K9","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[28]"},{"index":29,"class_name":"android.view.View","bounds":{"x1":555,"y1":2174,"x2":1044,"y2":2208},"text":null,"content_desc":"开关3","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[29]"},{"index":30,"class_name":"android.widget.ImageView","bounds":{"x1":870,"y1":2207,"x2":1044,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[30]"},{"index":31,"class_name":"android.widget.ImageView","bounds":{"x1":55,"y1":1973,"x2":244,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[31]"},{"index":32,"class_name":"android.widget.ImageView","bounds":{"x1":316,"y1":1973,"x2":504,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[32]"},{"index":33,"class_name":"android.widget.ImageView","bounds":{"x1":576,"y1":1973,"x2":764,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[33]"},{"index":34,"class_name":"android.widget.ImageView","bounds":{"x1":836,"y1":1973,"x2":1025,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[34]"}]
```

### 推断屏幕尺寸

```bash
❯ tke fetcher infer-screen-size < workarea/current_ui_tree.xml

{"width": 1080, "height": 2208}
```

### 生成 UI 树

```bash
❯ tke fetcher generate-tree-string < workarea/current_ui_tree.xml

从XML中提取了 35 个UI元素
UI Tree Structure:
=================

[0] FrameLayout [focusable]
    [1] HorizontalScrollView [focusable]
    [2] View (All Devices
Tab 1 of 6) [clickable, focusable]
    [3] View (Network
Tab 2 of 6) [clickable, focusable]
    [4] View (Switch
Tab 3 of 6) [clickable, focusable]
    [5] View (Sensor
Tab 4 of 6) [clickable, focusable]
    [6] View (Lock
Tab 5 of 6) [clickable, focusable]
    [7] View (Other
Tab 6 of 6) [clickable, focusable]
[8] ScrollView [focusable]
    [9] ImageView (Test New User第十八次) [clickable, focusable]
    [10] ImageView [clickable, focusable]
    [11] View [focusable]
    [12] ImageView (Turn Off All Lights) [clickable, focusable]
    [13] ImageView (Turn Off All Lights) [clickable, focusable]                                                         [14] View [focusable]                                                                                                       [15] View (Other Aircon 3) [clickable, focusable]                                                                       [16] ImageView [clickable, focusable]                                                                                   [17] View (Other Aircon 5) [clickable, focusable]                                                                       [18] ImageView [clickable, focusable]                                                                                   [19] View (Other Aircon 1) [clickable, focusable]                                                                       [20] ImageView [clickable, focusable]                                                                                   [21] View (Other Aircon 2) [clickable, focusable]                                                                       [22] ImageView [clickable, focusable]                                                                                   [23] ImageView (Motion Sensor) [clickable, focusable]                                                                   [24] ImageView (Network Multimode Gateway 12) [clickable, focusable]                                                    [25] View (Other Aircon 4) [clickable, focusable]                                                                       [26] ImageView [clickable, focusable]                                                                                   [27] View (Offline                                                                                                  Sensor Outdoor Siren Alarm ) [clickable, focusable]                                                                         [28] ImageView (Lock K9) [clickable, focusable]                                                                         [29] View (开关3) [clickable, focusable]                                                                                [30] ImageView [clickable, focusable]                                                                                   [31] ImageView [clickable, focusable]                                                                                   [32] ImageView [clickable, focusable]                                                                                   [33] ImageView [clickable, focusable]                                                                                   [34] ImageView [clickable, focusable]
```

---

## tke recognizer(基于项目内的./locator/element.json里记录的元素来实现)

### 查找元素

```bash
❯ tke recognizer find-text "Test"

2025-10-07T05:54:43.013889Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"                                                                                                          2025-10-07T05:54:43.014560Z  INFO tke::recognizer: 加载了 0 个locator定义                                               从XML中提取了 35 个UI元素                                                                                               找到文本 'Test' 的位置: (394, 186)


///// 失败则是
❯ tke recognizer find-text "xxx"

2025-10-07T05:56:30.311774Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"                                                                                                          2025-10-07T05:56:30.311891Z  INFO tke::recognizer: 加载了 0 个locator定义                                               从XML中提取了 35 个UI元素
Error: ElementNotFound("未找到包含文本 'xxx' 的元素")
```

```bash
(这里需要elements.json里已经保存了一个叫plus的xml元素)
❯ tke recognizer find-xml plus
2025-10-07T05:58:59.045492Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_5"
2025-10-07T05:58:59.046817Z  INFO tke::recognizer: 加载了 3 个locator定义
从XML中提取了 85 个UI元素
找到XML元素 'plus' 的位置: (187, 107)

///// 失败则是:
❯ tke recognizer find-xml settings_button

2025-10-07T05:55:22.537889Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_6"                                                                                                          2025-10-07T05:55:22.538323Z  INFO tke::recognizer: 加载了 0 个locator定义                                               从XML中提取了 35 个UI元素                                                                                               Error: ElementNotFound("Locator 'settings_button' 未定义")
```

```bash
❯ tke recognizer find-image findUrDevice

2025-10-07T06:01:04.626413Z  INFO tke: 项目路径: "/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_5"
2025-10-07T06:01:04.628397Z  INFO tke::recognizer: 加载了 3 个locator定义
2025-10-07T06:01:04.649587Z  INFO tke::recognizer::fast_matcher: 截图尺寸: 1080x2340, 模板尺寸: 504x108
2025-10-07T06:01:04.688789Z  INFO tke::recognizer::fast_matcher: 精搜索找到最佳位置: Point { x: 512, y: 758 }, 相似度: 0.653
Error: ElementNotFound("图像匹配置信度不足: 0.653 < 0.750")
```

---

## tke parser

返回 JSON 格式。

```bash
❯ tke parser parse cases/case_005/script/script_001.tks

{"case_id":": 测试用例","details":{"appActivity":".MainActivity","appPackage":"com.example.app"},"script_name":": test_script","steps":[{"command":"断言 [{示例元素}, 存在]","command_type":"Assert","line_number":7,"params":[{"XmlElement":"示例元素"},{"Boolean":true}]},{"command":"点击 [{示例按钮}]","command_type":"Click","line_number":8,"params":[{"XmlElement":"示例按钮"}]},{"command":"输入 [@{示例111}]","command_type":"Input","line_number":9,"params":[{"ImageElement":"示例111"}]},{"command":"按压 [1000]","command_type":"Press","line_number":10,"params":[{"Number":1000}]}],"success":true}
```

```bash
❯ tke parser validate cases/case_005/script/script_001.tks

脚本验证通过 ✓
  用例ID: 测试用例
  脚本名: test_script
  步骤数: 4
```

---

## tke run

### 执行脚本

```bash
❯ tke run script tests/login.tks

开始执行脚本: "tests/login.tks"

执行结果:
  状态: 成功 ✓
  用例ID: TC001
  脚本名: 登录测试
  开始时间: 2025-10-07 14:30:00
  结束时间: 2025-10-07 14:30:15
  总步骤数: 10
  成功步骤: 10
```

### 执行项目所有脚本

```bash
❯ tke run project

开始执行项目中的所有脚本...

项目执行结果:
  总脚本数: 5
  成功脚本: 4
  失败脚本: 1

  ✓ 登录测试 (TC001)
  ✓ 设置测试 (TC002)
  ✗ 支付测试 (TC003)
      错误: 元素未找到: pay_button
  ✓ 退出测试 (TC004)
  ✓ 搜索测试 (TC005)
```

## 全局选项

```bash
# 指定设备
tke -d emulator-5554 controller tap 500 1000

# 指定项目路径
tke -p /path/to/project run script test.tks

# 详细输出
tke -v controller capture
```

---

## JSON 输出命令（可直接用 jq）

- `tke ocr`
- `tke fetcher infer-screen-size`
- `tke fetcher extract-ui-elements`
- `tke parser parse`
- `tke run content`

示例：
```bash
tke ocr --image test.png --online --url http://localhost:8000/ocr | jq '.texts[0].text'
```
