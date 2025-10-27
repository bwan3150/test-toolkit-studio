
device-manager/ - 设备屏幕管理

  1. screen-state.js - 全局状态管理(XML状态、UI元素列表等)
  2. screen-capture.js - 调用TKE获取设备截图
  3. ui-extractor.js - 提取UI元素并更新
  4. xml-overlay.js - 控制XML overlay的开启/关闭
  5. overlay-renderer.js - 渲染UI框、处理ResizeObserver、元素选择
  6. device-screen-manager.js - 总控制器,整合所有功能

mode-manager/ - 四种模式管理

  1. coordinate-converter.js - 坐标转换(屏幕⇄图片⇄设备)
  2. mode-slider.js - 滑块UI交互
  3. mode-switcher.js - 模式切换核心逻辑(normal/xml/screenshot/coordinate)
  4. screenshot-selector.js - 截图框选并保存为定位器
  5. coordinate-mode.js - 点击获取坐标
  6. screen-mode-manager.js - 总控制器,协调各子模块
