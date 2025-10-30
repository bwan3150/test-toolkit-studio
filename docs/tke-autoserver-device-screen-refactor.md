# Electron Renderer层Testcase页面Device Screen模块重构



## 重构逻辑

因为新增了一个用于在手机端运行的一个Selfhost Automation Framework Server(也就是tke-autoserver)程序, 可以向电脑端提供手机的屏幕投屏视频流, 并且可以通过screenshotServer获取手机截图到到电脑; 并且项目的核心toolkit-engine(tke)中也新增了管控这个tke-autoserver, 并且更新了原有的截图获取也就是`tke controller capture`方法, 以及新增了小方法 `tke controller capture screenshot`用于快速获取屏幕信息, 这些获取截图的方法都要求必须在 `tke server start` 后, 让 `tke controller status`查看时发现能保持 running字段为true也就是tke-autoserver正在手机上运行的时候才能成功运行; 所以为了可用性和效率需要对这个很依赖于以上方法的部分 ./renderer/js/testcase/screen/ 进行重构



## 重构内容

### 滑块



### Normal模式

这部分的重构可能需要新增一个handler层的代码来处理视频流

Normal模式从原来的纯截图模式, 彻底改为使用实时视频流投影的模块

需要在`tke server start`后, 能够获取手机的实时视频流, 并展示在DEVICE SCREEN位置, 要求可以根据边缘的拖动动态改变手机视频流的显示大小, 不能是独立窗口, 需要在./renderer/html/pages/testcase.html中嵌入显示



### XML Overlay模式



### Screenshot模式(截取图片元素模式)



### Coordinate模式(从图片中点击以获取坐标的模式)



