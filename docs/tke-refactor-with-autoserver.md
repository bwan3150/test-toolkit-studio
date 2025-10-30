# 重构Toolkit Engine逻辑与手机端运行的Selfhost Automation Framework Server(tke-autoserver)配合



## 整体逻辑



新增一个叫作tke server的控制模块, 负责注入, 启动, 关闭 tke-autoserver到手机

通过 tke controller devices获取有哪些设备连接到本电脑并获取其id; 此后, 可以使用`tke server start`来启动手机上的tke-autoserver的视频流; 如果发现手机上还没有安装tke-autoserver或者 tke-autoserver的版本号落后于 当前的tke版本(也就是使用`tke --version`获得的结果里tke x.x.xx-beta里面的x.x.xx-beta这个, 因为不安装到手机无法使用`tke-autoserver --version`所以直接用tke的版本号就好, 一般来说都是同步的), 就将tke同文件夹下的tke-autoserver注入安装到手机上, 然后再开启视频流; 然后后面可以通过 `tke server stop`来停止视频流

然后修改已有的指令`tke controller capture`里面获取截图的逻辑改为使用autoserver新的那个screenshotServer来获取(如果此时还没开启视频流或甚至还没注入tke-autoserver到手机, 就先开启视频流再进行截图)然后放到项目下的 workarea/current_screenshot.png, 获取屏幕xml保存到workarea/current_ui_tree.xml的逻辑还是用原有的逻辑不改变; 然后新增两个方法: `tke controller capture screenshot` , 是(再注入autoserver并且开启视频流后)单独使用autoserver获取当前截图到 workarea/current_screenshot.png, `tke controller capture xml` , 是单独获取xml到workarea/current_ui_tree.xml

原有的adb获取截图的方法直接移除掉, 不需要兼容, 直接移除
