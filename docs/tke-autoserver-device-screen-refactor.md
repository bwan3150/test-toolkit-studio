# Electron Renderer层Testcase页面Device Screen模块重构



## 重构逻辑

因为新增了一个用于在手机端运行的一个Selfhost Automation Framework Server(也就是tke-autoserver)程序, 可以向电脑端提供手机的屏幕投屏视频流, 并且可以通过screenshotServer获取手机截图到到电脑; 并且项目的核心toolkit-engine(tke)中也新增了管控这个tke-autoserver, 并且更新了原有的截图获取也就是`tke controller capture`方法, 以及新增了小方法 `tke controller capture screenshot`用于快速获取屏幕信息, 这些获取截图的方法都要求必须在 `tke server start` 后, 让 `tke controller status`查看时发现能保持 running字段为true也就是tke-autoserver正在手机上运行的时候才能成功运行; 所以为了可用性和效率需要对这个很依赖于以上方法的部分 ./renderer/js/testcase/screen/ 进行重构



## 重构内容

### 滑块

当没有选择或连接设备时, 还是显示那个"请连接并选择设备"的按钮

然后, 比如等 `tke server status` running状态为true的时候才解锁, 否则需要一直归于normal模式, 并且显示那个"点击获取屏幕信息"的按钮

点击 "点击获取屏幕信息" 的按钮后, 应该`tke server start`, 并且开始normal模式的视频流投影, 同时给滑块解锁, 允许切换到其他模式

如果与设备上的这个AutoServer的视频流服务器断开了, 就要立刻回到normal模式然后锁定滑块并显示"点击获取屏幕信息" 的按钮

其他比如模式切换时用什么获取信息, 详见下面几个模式中的描述



### Normal模式

这部分的重构可能需要新增一个handler层的代码来处理视频流

Normal模式从原来的纯截图模式, 彻底改为使用实时视频流投影的模块

需要在`tke server start`后, 能够获取手机的实时视频流, 并展示在DEVICE SCREEN位置, 要求可以根据边缘的拖动动态改变手机视频流的显示大小, 不能是独立窗口, 需要在./renderer/html/pages/testcase.html中嵌入显示

刷新按钮只是用来跑一下  `tke server status`确认状态, 不需要进行啥刷新

### XML Overlay模式

如果从normal模式进入这个模式后, 下面不再展示normal视频流, 而是调用 `tke controller capture` , 进行一翻加载后, 还是类似原来的那个, current_screeshot.png显示截图, 上面覆盖经过分析和可视化的xml, 和原来一样

如果从screenshot模式或coodinate模式进入此模式, 则直接展示当前的current_screeshot.png截图然后用`tke controller capture xml`获取xml并可视化渲染

后续刷新则使用`tke controller capture`更新current_screenshot.png和current_ui_tree.xml



### Screenshot模式(截取图片元素模式)

如果从xml或coodinate进入此模式, 则直接展示当前的current_screeshot.png截图就好, 如果是从Normal模式进入, 则使用`tke controller capture screenshot`进行截图, 然后显示current_screenshot.png; 裁切并保存到元素库的逻辑和原来一样

后续刷新则使用`tke controller capture screenshot`更新current_screenshot.png就好



### Coordinate模式(从图片中点击以获取坐标的模式)

如果从xml或coodinate进入此模式, 则直接展示当前的current_screeshot.png截图就好, 如果是从Normal模式进入, 则使用`tke controller capture screenshot`进行截图, 然后显示current_screenshot.png; 点击获取坐标到剪切板的逻辑和原来一样

后续刷新则使用`tke controller capture screenshot`更新current_screenshot.png就好



### 刷新按钮

已经在上面各个模式提及了
