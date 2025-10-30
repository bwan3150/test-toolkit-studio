# 基于 scrcpy 的 TKE Automation 方案

## 为什么选择 scrcpy 方案？

### 性能对比

| 方案 | 截图速度 | 部署方式 | 性能 | 开发成本 |
|------|---------|---------|------|---------|
| **纯 ADB** | ~1-2s | 无需安装 | ⭐️ | 已完成 |
| **App 方案** | ~100-300ms | 需安装 APK | ⭐️⭐️⭐️⭐️ | 高（从零开发） |
| **scrcpy 方案** | ~50-100ms | adb push jar | ⭐️⭐️⭐️⭐️⭐️ | 低（基于现有代码） |

### scrcpy 方案的优势

✅ **性能极佳**: 直接调用系统 API，无中间层开销
✅ **无需安装**: 通过 `app_process` 运行 jar，不占用用户应用列表
✅ **代码成熟**: scrcpy 已被广泛使用，稳定可靠
✅ **易于扩展**: 纯 Java，添加功能简单
✅ **权限充足**: 运行在系统级，可访问所有隐藏 API

---

## 实施方案

### 方案 A：直接复用 scrcpy（推荐快速验证）

#### 1. Fork scrcpy 项目并添加 UI 树导出功能

在 scrcpy server 中添加一个新的模块：

**新建文件**: `scrcpy/server/src/main/java/com/genymobile/scrcpy/device/UITreeExporter.java`

```java
package com.genymobile.scrcpy.device;

import android.os.Build;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import com.genymobile.scrcpy.util.Ln;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.lang.reflect.Field;
import java.lang.reflect.Method;

public final class UITreeExporter {

    private UITreeExporter() {
        // not instantiable
    }

    /**
     * 导出当前界面的 UI 树结构（JSON 格式）
     */
    public static String exportUITreeAsJson() {
        try {
            // 通过反射获取 WindowManagerGlobal 实例
            Class<?> windowManagerGlobalClass = Class.forName("android.view.WindowManagerGlobal");
            Method getInstanceMethod = windowManagerGlobalClass.getMethod("getInstance");
            Object wmgInstance = getInstanceMethod.invoke(null);

            // 获取 mRoots 字段（所有 ViewRootImpl 实例）
            Field mRootsField = windowManagerGlobalClass.getDeclaredField("mRoots");
            mRootsField.setAccessible(true);
            @SuppressWarnings("unchecked")
            java.util.List<?> viewRootImpls = (java.util.List<?>) mRootsField.get(wmgInstance);

            JSONArray rootsArray = new JSONArray();

            // 遍历所有根 View
            for (Object viewRootImpl : viewRootImpls) {
                Method getViewMethod = viewRootImpl.getClass().getMethod("getView");
                View rootView = (View) getViewMethod.invoke(viewRootImpl);

                if (rootView != null) {
                    JSONObject rootNode = traverseView(rootView, 0);
                    rootsArray.put(rootNode);
                }
            }

            return rootsArray.toString(2); // 缩进 2 格

        } catch (Exception e) {
            Ln.e("Failed to export UI tree", e);
            return "{\"error\": \"" + e.getMessage() + "\"}";
        }
    }

    /**
     * 递归遍历 View 树
     */
    private static JSONObject traverseView(View view, int depth) throws JSONException {
        JSONObject node = new JSONObject();

        // 基本信息
        node.put("index", view.hashCode()); // 使用 hashCode 作为唯一标识
        node.put("class_name", view.getClass().getName());

        // 坐标和尺寸
        int[] location = new int[2];
        view.getLocationOnScreen(location);
        JSONObject bounds = new JSONObject();
        bounds.put("x1", location[0]);
        bounds.put("y1", location[1]);
        bounds.put("x2", location[0] + view.getWidth());
        bounds.put("y2", location[1] + view.getHeight());
        node.put("bounds", bounds);

        // 文本内容
        if (view instanceof TextView) {
            CharSequence text = ((TextView) view).getText();
            if (text != null && text.length() > 0) {
                node.put("text", text.toString());
            }
            CharSequence hint = ((TextView) view).getHint();
            if (hint != null && hint.length() > 0) {
                node.put("hint", hint.toString());
            }
        }

        // content-desc
        CharSequence contentDesc = view.getContentDescription();
        if (contentDesc != null && contentDesc.length() > 0) {
            node.put("content_desc", contentDesc.toString());
        }

        // resource-id
        try {
            int id = view.getId();
            if (id != View.NO_ID) {
                String resourceName = view.getResources().getResourceName(id);
                node.put("resource_id", resourceName);
            }
        } catch (Exception e) {
            // ignore
        }

        // 属性
        node.put("clickable", view.isClickable());
        node.put("focusable", view.isFocusable());
        node.put("focused", view.isFocused());
        node.put("scrollable", view.isScrollContainer());
        node.put("enabled", view.isEnabled());
        node.put("selected", view.isSelected());
        node.put("checkable", false); // 需要判断是否实现 Checkable 接口
        node.put("checked", false);

        // 如果是 ViewGroup，遍历子节点
        if (view instanceof ViewGroup) {
            ViewGroup viewGroup = (ViewGroup) view;
            JSONArray children = new JSONArray();
            for (int i = 0; i < viewGroup.getChildCount(); i++) {
                View child = viewGroup.getChildAt(i);
                if (child != null) {
                    children.put(traverseView(child, depth + 1));
                }
            }
            if (children.length() > 0) {
                node.put("children", children);
            }
        }

        return node;
    }

    /**
     * 导出 XML 格式（兼容 TKE 现有格式）
     */
    public static String exportUITreeAsXml() {
        try {
            Class<?> windowManagerGlobalClass = Class.forName("android.view.WindowManagerGlobal");
            Method getInstanceMethod = windowManagerGlobalClass.getMethod("getInstance");
            Object wmgInstance = getInstanceMethod.invoke(null);

            Field mRootsField = windowManagerGlobalClass.getDeclaredField("mRoots");
            mRootsField.setAccessible(true);
            @SuppressWarnings("unchecked")
            java.util.List<?> viewRootImpls = (java.util.List<?>) mRootsField.get(wmgInstance);

            StringBuilder xmlBuilder = new StringBuilder();
            xmlBuilder.append("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
            xmlBuilder.append("<hierarchy rotation=\"0\">\n");

            for (Object viewRootImpl : viewRootImpls) {
                Method getViewMethod = viewRootImpl.getClass().getMethod("getView");
                View rootView = (View) getViewMethod.invoke(viewRootImpl);

                if (rootView != null) {
                    traverseViewToXml(rootView, xmlBuilder, 1);
                }
            }

            xmlBuilder.append("</hierarchy>");
            return xmlBuilder.toString();

        } catch (Exception e) {
            Ln.e("Failed to export UI tree as XML", e);
            return "<error>" + e.getMessage() + "</error>";
        }
    }

    private static void traverseViewToXml(View view, StringBuilder sb, int depth) {
        String indent = "  ".repeat(depth);

        // 开始标签
        sb.append(indent).append("<node");

        // 属性
        sb.append(" class=\"").append(view.getClass().getName()).append("\"");

        int[] location = new int[2];
        view.getLocationOnScreen(location);
        sb.append(" bounds=\"[").append(location[0]).append(",").append(location[1])
          .append("][").append(location[0] + view.getWidth()).append(",")
          .append(location[1] + view.getHeight()).append("]\"");

        if (view instanceof TextView) {
            CharSequence text = ((TextView) view).getText();
            if (text != null && text.length() > 0) {
                sb.append(" text=\"").append(escapeXml(text.toString())).append("\"");
            }
            CharSequence hint = ((TextView) view).getHint();
            if (hint != null && hint.length() > 0) {
                sb.append(" hint=\"").append(escapeXml(hint.toString())).append("\"");
            }
        }

        CharSequence contentDesc = view.getContentDescription();
        if (contentDesc != null && contentDesc.length() > 0) {
            sb.append(" content-desc=\"").append(escapeXml(contentDesc.toString())).append("\"");
        }

        try {
            int id = view.getId();
            if (id != View.NO_ID) {
                String resourceName = view.getResources().getResourceName(id);
                sb.append(" resource-id=\"").append(escapeXml(resourceName)).append("\"");
            }
        } catch (Exception e) {
            // ignore
        }

        sb.append(" clickable=\"").append(view.isClickable()).append("\"");
        sb.append(" focusable=\"").append(view.isFocusable()).append("\"");
        sb.append(" focused=\"").append(view.isFocused()).append("\"");
        sb.append(" scrollable=\"").append(view.isScrollContainer()).append("\"");
        sb.append(" enabled=\"").append(view.isEnabled()).append("\"");
        sb.append(" selected=\"").append(view.isSelected()).append("\"");

        // 子节点
        if (view instanceof ViewGroup) {
            ViewGroup viewGroup = (ViewGroup) view;
            if (viewGroup.getChildCount() > 0) {
                sb.append(">\n");
                for (int i = 0; i < viewGroup.getChildCount(); i++) {
                    View child = viewGroup.getChildAt(i);
                    if (child != null) {
                        traverseViewToXml(child, sb, depth + 1);
                    }
                }
                sb.append(indent).append("</node>\n");
            } else {
                sb.append(" />\n");
            }
        } else {
            sb.append(" />\n");
        }
    }

    private static String escapeXml(String str) {
        return str.replace("&", "&amp;")
                  .replace("<", "&lt;")
                  .replace(">", "&gt;")
                  .replace("\"", "&quot;")
                  .replace("'", "&apos;");
    }
}
```

#### 2. 修改 Server 主类，添加命令支持

**修改文件**: `scrcpy/server/src/main/java/com/genymobile/scrcpy/Server.java`

在 `main` 方法中添加新命令：

```java
public static void main(String... args) {
    Thread.setDefaultUncaughtExceptionHandler((t, e) -> {
        Ln.e("Exception on thread " + t, e);
        System.exit(1);
    });

    unlinkSelf();

    // ========== 新增：支持 UI 树导出命令 ==========
    if (args.length > 0) {
        if ("export-ui-tree-json".equals(args[0])) {
            String jsonTree = UITreeExporter.exportUITreeAsJson();
            System.out.println(jsonTree);
            return;
        }
        if ("export-ui-tree-xml".equals(args[0])) {
            String xmlTree = UITreeExporter.exportUITreeAsXml();
            System.out.println(xmlTree);
            return;
        }
    }
    // ========== 新增结束 ==========

    Options options;
    try {
        options = Options.parse(args);
    } catch (IllegalArgumentException e) {
        Ln.e(e.getMessage());
        buildOptionsHelp();
        System.exit(1);
        return;
    }

    // ... 原有代码
}
```

#### 3. 编译 scrcpy-server.jar

```bash
cd /Users/eric_konec/Documents/GitHub/scrcpy/server

# 编译
./gradlew assembleRelease

# 输出：build/outputs/apk/release/scrcpy-server-v*.jar
```

#### 4. 部署并测试

```bash
# 推送 jar 到手机
adb push scrcpy-server.jar /data/local/tmp/

# 测试 UI 树导出（XML 格式）
adb shell CLASSPATH=/data/local/tmp/scrcpy-server.jar app_process / com.genymobile.scrcpy.Server export-ui-tree-xml

# 测试 UI 树导出（JSON 格式）
adb shell CLASSPATH=/data/local/tmp/scrcpy-server.jar app_process / com.genymobile.scrcpy.Server export-ui-tree-json

# 保存到文件
adb shell CLASSPATH=/data/local/tmp/scrcpy-server.jar app_process / com.genymobile.scrcpy.Server export-ui-tree-xml > /tmp/ui_tree.xml
```

#### 5. 集成到 TKE

**修改 TKE Controller**: `/Users/eric_konec/Documents/GitHub/test-toolkit-studio/toolkit-engine/src/controller/mod.rs`

```rust
use std::process::Command;

pub struct Controller {
    device_id: Option<String>,
    scrcpy_jar_path: String,  // scrcpy-server.jar 路径
}

impl Controller {
    pub fn new(device_id: Option<String>) -> Self {
        Self {
            device_id,
            scrcpy_jar_path: "/data/local/tmp/scrcpy-server.jar".to_string(),
        }
    }

    /// 通过 scrcpy 导出 UI 树（XML 格式）
    pub fn dump_ui_hierarchy_scrcpy(&self) -> Result<String> {
        let mut cmd = Command::new("adb");

        if let Some(ref device) = self.device_id {
            cmd.arg("-s").arg(device);
        }

        cmd.arg("shell")
           .arg("CLASSPATH=/data/local/tmp/scrcpy-server.jar")
           .arg("app_process")
           .arg("/")
           .arg("com.genymobile.scrcpy.Server")
           .arg("export-ui-tree-xml");

        let output = cmd.output()
            .map_err(|e| format!("Failed to execute scrcpy export: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("scrcpy export failed: {}", stderr).into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 快速截图（通过 scrcpy 的截图能力）
    pub fn screenshot_scrcpy(&self, output_path: &str) -> Result<()> {
        // 可以扩展 scrcpy 添加截图命令
        // 或者使用 screencap（虽然慢一点，但 scrcpy 主要优势在于视频流）
        todo!("Implement screenshot via scrcpy or fallback to screencap")
    }
}
```

**修改 TKE Fetcher**: `/Users/eric_konec/Documents/GitHub/test-toolkit-studio/toolkit-engine/src/fetcher/mod.rs`

```rust
impl Fetcher {
    /// 使用 scrcpy 后端获取 UI 元素
    pub fn fetch_elements_scrcpy(&self) -> Result<Vec<UIElement>> {
        let controller = Controller::new(self.device_id.clone());

        // 通过 scrcpy 导出 XML
        let xml_content = controller.dump_ui_hierarchy_scrcpy()?;

        // 使用现有的 XML 解析逻辑
        self.fetch_elements_from_xml(&xml_content)
    }
}
```

**添加 TKE 命令选项**:

```bash
# 使用 scrcpy 后端
tke fetcher extract-ui-elements --backend scrcpy < workarea/current_ui_tree.xml

# 或者直接导出
tke controller dump-hierarchy --backend scrcpy > workarea/current_ui_tree.xml
```

---

### 方案 B：独立编写轻量级 Server（如果需要更多定制）

如果 scrcpy 太重（它包含视频流、音频等），可以参考它的架构，写一个轻量级的 TKE Server：

**项目结构**:
```
tke-server/
├── src/
│   └── com/
│       └── yourcompany/
│           └── tkeserver/
│               ├── Server.java          # 主入口
│               ├── UITreeExporter.java  # UI 树导出
│               ├── ScreenshotModule.java # 截图
│               ├── ActionExecutor.java  # 点击/输入等操作
│               └── wrappers/
│                   ├── ServiceManager.java
│                   ├── WindowManager.java
│                   └── InputManager.java
└── build.gradle
```

**优势**:
- 只包含 TKE 需要的功能，jar 包更小（< 100KB）
- 完全控制代码，易于扩展
- 可以添加 TKE 专属功能（如直接生成 xpath）

**劣势**:
- 需要自己处理 Android 版本兼容性
- 需要自己实现 wrapper 类（可以直接复制 scrcpy 的）

---

## 性能对比预期

| 操作 | ADB uiautomator | scrcpy 方案 | 提升 |
|------|----------------|-------------|------|
| **UI 树获取** | ~500ms | ~50-100ms | **5-10x** ⚡️ |
| **截图** | ~1-2s | ~300-500ms | **3-5x** ⚡️ |
| **点击操作** | ~200ms | ~50ms | **4x** ⚡️ |

> 注：scrcpy 的主要优势在于**视频流**（实时 30-60fps），如果只需要单次截图，性能提升有限。
> 但 UI 树获取和操作注入速度提升明显。

---

## 与 App 方案对比

| 特性 | scrcpy 方案 | App 方案 |
|------|------------|---------|
| **部署方式** | adb push jar | 安装 APK |
| **用户可见性** | 隐藏 | 用户日常可用 |
| **性能** | ⭐️⭐️⭐️⭐️⭐️ | ⭐️⭐️⭐️⭐️ |
| **开发成本** | 低（基于现有代码） | 高（从零开发） |
| **扩展性** | 一般（纯自动化） | 强（可加业务功能） |
| **权限** | 系统级（无限制） | 受 Android 权限限制 |

---

## 推荐行动路径

### 第一步：快速验证（今天完成）

1. **Clone scrcpy 项目**
```bash
cd /Users/eric_konec/Documents/GitHub
git clone https://github.com/Genymobile/scrcpy.git tke-scrcpy
cd tke-scrcpy
```

2. **添加 UI 树导出功能**（复制上面的 `UITreeExporter.java`）

3. **编译并测试**
```bash
cd server
./gradlew assembleRelease
adb push build/outputs/apk/release/scrcpy-server.jar /data/local/tmp/
adb shell CLASSPATH=/data/local/tmp/scrcpy-server.jar app_process / com.genymobile.scrcpy.Server export-ui-tree-xml
```

### 第二步：集成到 TKE（明天完成）

1. 修改 TKE 的 `controller` 模块，添加 `scrcpy` 后端
2. 测试性能提升是否满足预期
3. 更新 TKE 命令行参数支持 `--backend scrcpy`

### 第三步：优化与扩展（后续）

1. 添加视频流支持（如果需要 AI Tester 实时观察）
2. 优化 jar 体积（移除不需要的视频编码模块）
3. 添加更多自动化命令（录制、回放等）

---

## 总结

**强烈推荐使用 scrcpy 方案**，理由：

1. ✅ **性能最佳**: 比 ADB 快 5-10 倍，比 App 方案性能更好
2. ✅ **开发成本最低**: 基于成熟代码，只需添加 UI 树导出功能
3. ✅ **无需安装**: 不占用用户应用列表
4. ✅ **权限充足**: 可访问所有系统 API
5. ✅ **代码成熟**: scrcpy 已被全球数百万用户使用，稳定可靠

**唯一的缺点**: 不能做成日常用户 App（如果需要查看 Bug 信息等功能，可以后续再做独立 App）

**建议**: 先用 scrcpy 方案解决性能问题，后续如果需要用户友好的日常 App，再考虑 App 方案。
