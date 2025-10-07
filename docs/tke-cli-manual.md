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

{"devices":["c9dc8614"]}
```

### 截图和获取 UI 树

```bash
❯ tke controller capture

{"screenshot":"/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_5/workarea/current_screenshot.png","success":true,"xml":"/Users/eric_konec/Documents/GitHub/test-toolkit-studio/test-projects/test_project_5/workarea/current_ui_tree.xml"}
```

覆盖生成文件：
- `workarea/current_screenshot.png`
- `workarea/current_ui_tree.xml`

### 点击和滑动

```bash
❯ tke controller tap 400 2000

{"success":true,"x":400,"y":2000}
```

```bash
❯ tke controller swipe 500 1500 500 500 --duration 300

{"duration":300,"from":{"x":500,"y":1500},"success":true,"to":{"x":500,"y":500}}
```

### 应用控制

```bash
❯ tke controller launch com.android.settings .Settings

{"activity":".Settings","package":"com.android.settings","success":true}
```

```bash
❯ tke controller stop com.android.settings

{"package":"com.android.settings","success":true}
```

### 输入框输入

```bash
❯ tke controller input "Hello World"

{"success":true,"text":"Hello World"}
```

### 系统级控制

```bash
❯ tke controller back

{"success":true}
```

```bash
❯ tke controller home

{"success":true}
```

---

## tke fetcher

返回 JSON 格式，可用 `jq` 处理。

### 提取 UI 元素

```bash
❯ tke fetcher extract-ui-elements < workarea/current_ui_tree.xml

[{"index":0,"class_name":"android.widget.FrameLayout","bounds":{"x1":0,"y1":0,"x2":1080,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[0]"},{"index":1,"class_name":"android.view.View","bounds":{"x1":0,"y1":0,"x2":1080,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[1]"},{"index":2,"class_name":"android.widget.ImageView","bounds":{"x1":425,"y1":270,"x2":1032,"y2":450},"text":null,"content_desc":"Add Device","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[2]"},{"index":3,"class_name":"android.widget.ImageView","bounds":{"x1":425,"y1":453,"x2":1032,"y2":633},"text":null,"content_desc":"Add Tap-to-Run","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[3]"},{"index":4,"class_name":"android.widget.ImageView","bounds":{"x1":425,"y1":636,"x2":1032,"y2":816},"text":null,"content_desc":"Add Automation","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[4]"},{"index":5,"class_name":"android.widget.ImageView","bounds":{"x1":425,"y1":834,"x2":1032,"y2":1014},"text":null,"content_desc":"Find Your Device","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[5]"},{"index":6,"class_name":"android.widget.HorizontalScrollView","bounds":{"x1":36,"y1":475,"x2":1032,"y2":625},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":true,"selected":false,"enabled":true,"xpath":"//node[6]"},{"index":7,"class_name":"android.view.View","bounds":{"x1":36,"y1":475,"x2":208,"y2":625},"text":null,"content_desc":"All Devices\nTab 1 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[7]"},{"index":8,"class_name":"android.view.View","bounds":{"x1":208,"y1":475,"x2":411,"y2":625},"text":null,"content_desc":"Network\nTab 2 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[8]"},{"index":9,"class_name":"android.view.View","bounds":{"x1":411,"y1":475,"x2":578,"y2":625},"text":null,"content_desc":"Switch\nTab 3 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[9]"},{"index":10,"class_name":"android.view.View","bounds":{"x1":578,"y1":475,"x2":746,"y2":625},"text":null,"content_desc":"Sensor\nTab 4 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":true,"enabled":true,"xpath":"//node[10]"},{"index":11,"class_name":"android.view.View","bounds":{"x1":746,"y1":475,"x2":878,"y2":625},"text":null,"content_desc":"Lock\nTab 5 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[11]"},{"index":12,"class_name":"android.view.View","bounds":{"x1":878,"y1":475,"x2":1032,"y2":625},"text":null,"content_desc":"Other\nTab 6 of 6","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[12]"},{"index":13,"class_name":"android.widget.ScrollView","bounds":{"x1":0,"y1":96,"x2":1080,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":true,"selected":false,"enabled":true,"xpath":"//node[13]"},{"index":14,"class_name":"android.widget.ImageView","bounds":{"x1":48,"y1":143,"x2":740,"y2":230},"text":null,"content_desc":"Test New User第十八次","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[14]"},{"index":15,"class_name":"android.widget.ImageView","bounds":{"x1":924,"y1":132,"x2":1032,"y2":240},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[15]"},{"index":16,"class_name":"android.view.View","bounds":{"x1":0,"y1":306,"x2":1080,"y2":439},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":false,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":true,"selected":false,"enabled":true,"xpath":"//node[16]"},{"index":17,"class_name":"android.widget.ImageView","bounds":{"x1":48,"y1":306,"x2":564,"y2":439},"text":null,"content_desc":"Turn Off All Lights","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[17]"},{"index":18,"class_name":"android.widget.ImageView","bounds":{"x1":564,"y1":306,"x2":1080,"y2":439},"text":null,"content_desc":"Turn Off All Lights","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[18]"},{"index":19,"class_name":"android.view.View","bounds":{"x1":36,"y1":625,"x2":525,"y2":983},"text":null,"content_desc":"Offline\nSensor Outdoor Siren Alarm ","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[19]"},{"index":20,"class_name":"android.view.View","bounds":{"x1":555,"y1":625,"x2":1044,"y2":983},"text":null,"content_desc":"Presence\nKonec Human Presence Sensor ","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[20]"},{"index":21,"class_name":"android.widget.ImageView","bounds":{"x1":36,"y1":1013,"x2":525,"y2":1370},"text":null,"content_desc":"Sensor Contact Sensor 8","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[21]"},{"index":22,"class_name":"android.widget.ImageView","bounds":{"x1":555,"y1":1013,"x2":1044,"y2":1370},"text":null,"content_desc":"Sensor Indoor Siren Alarm 1","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[22]"},{"index":23,"class_name":"android.widget.ImageView","bounds":{"x1":36,"y1":1400,"x2":525,"y2":1757},"text":null,"content_desc":"Sensor TH Sensor ","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[23]"},{"index":24,"class_name":"android.widget.ImageView","bounds":{"x1":555,"y1":1400,"x2":1044,"y2":1757},"text":null,"content_desc":"Sensor Human Sensor 2nd Gen ","resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[24]"},{"index":25,"class_name":"android.widget.ImageView","bounds":{"x1":55,"y1":1973,"x2":244,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[25]"},{"index":26,"class_name":"android.widget.ImageView","bounds":{"x1":316,"y1":1973,"x2":504,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[26]"},{"index":27,"class_name":"android.widget.ImageView","bounds":{"x1":576,"y1":1973,"x2":764,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[27]"},{"index":28,"class_name":"android.widget.ImageView","bounds":{"x1":836,"y1":1973,"x2":1025,"y2":2208},"text":null,"content_desc":null,"resource_id":null,"hint":null,"clickable":true,"checkable":false,"checked":false,"focusable":true,"focused":false,"scrollable":false,"selected":false,"enabled":true,"xpath":"//node[28]"}]
```

### 推断屏幕尺寸

```bash
❯ tke fetcher infer-screen-size < workarea/current_ui_tree.xml

{"width": 1080, "height": 2208}
```

### 生成 UI 树

```bash
❯ tke fetcher generate-tree-string < workarea/current_ui_tree.xml

UI Tree Structure:
=================

[0] FrameLayout [focusable]
[1] View [clickable, focusable]
    [2] ImageView (Add Device) [clickable, focusable]
    [3] ImageView (Add Tap-to-Run) [clickable, focusable]
    [4] ImageView (Add Automation) [clickable, focusable]
    [5] ImageView (Find Your Device) [clickable, focusable]
    [6] HorizontalScrollView [focusable]
    [7] View (All Devices
Tab 1 of 6) [clickable, focusable]
    [8] View (Network
Tab 2 of 6) [clickable, focusable]
    [9] View (Switch
Tab 3 of 6) [clickable, focusable]
    [10] View (Sensor
Tab 4 of 6) [clickable, focusable]
    [11] View (Lock
Tab 5 of 6) [clickable, focusable]
    [12] View (Other
Tab 6 of 6) [clickable, focusable]
[13] ScrollView [focusable]
    [14] ImageView (Test New User第十八次) [clickable, focusable]
    [15] ImageView [clickable, focusable]
    [16] View [focusable]
    [17] ImageView (Turn Off All Lights) [clickable, focusable]
    [18] ImageView (Turn Off All Lights) [clickable, focusable]                                                             [19] View (Offline                                                                                                  Sensor Outdoor Siren Alarm ) [clickable, focusable]                                                                         [20] View (Presence                                                                                                 Konec Human Presence Sensor ) [clickable, focusable]                                                                        [21] ImageView (Sensor Contact Sensor 8) [clickable, focusable]                                                         [22] ImageView (Sensor Indoor Siren Alarm 1) [clickable, focusable]                                                     [23] ImageView (Sensor TH Sensor ) [clickable, focusable]                                                               [24] ImageView (Sensor Human Sensor 2nd Gen ) [clickable, focusable]                                                    [25] ImageView [clickable, focusable]                                                                                   [26] ImageView [clickable, focusable]                                                                                   [27] ImageView [clickable, focusable]                                                                                   [28] ImageView [clickable, focusable]
```

---

## tke recognizer(基于项目内的./locator/element.json里记录的元素来实现)

### 查找元素

```bash
❯ tke recognizer find-text "Test"

{"success":true,"text":"Test","x":394,"y":186}


///// 失败则是
❯ tke recognizer find-text "xxx"

Error: ElementNotFound("未找到包含文本 'xxx' 的元素")
```

```bash
(这里需要elements.json里已经保存了一个叫plus的xml元素)
❯ tke recognizer find-xml plus

{"locator":"plus","success":true,"x":728,"y":360}

///// 失败则是:
❯ tke recognizer find-xml settings_button

Error: ElementNotFound("Locator 'settings_button' 未定义")
```

```bash
❯ tke recognizer find-image findUrDevice

2025-10-07T07:17:12.173046Z  INFO tke::recognizer::fast_matcher: 截图尺寸: 1080x2340, 模板尺寸: 504x108
2025-10-07T07:17:12.208964Z  INFO tke::recognizer::fast_matcher: 精搜索找到最佳位置: Point { x: 495, y: 757 }, 相似度: 0.626
Error: ElementNotFound("图像匹配置信度不足: 0.626 < 0.750")
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

{"case_id":": 测试用例","script_name":": test_script","steps_count":4,"valid":true,"warnings":[]}
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

## JSON 输出命令（可直接用 jq查看）

```bash
❯ tke fetcher infer-screen-size < workarea/current_ui_tree.xml | jq
{
  "width": 1080,
  "height": 2208
}
```
