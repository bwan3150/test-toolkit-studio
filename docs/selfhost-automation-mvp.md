# Self-Host Automation Framework - MVP 快速验证方案

## 目标
验证在 App 内运行 HTTP Server，提供比 adb 更快的截图和 UI 树获取能力。

---

## 技术栈选择

### 方案 A：纯 Kotlin (推荐快速验证)
- **HTTP Server**: NanoHTTPD (单文件，轻量级) 或 AndroidAsync
- **UI 操作**: UiAutomation API (需要在 instrumentation 下运行)
- **截图**: PixelCopy API (Android 8.0+)
- **UI 树**: View.getRootView() 递归遍历

### 方案 B：Kotlin + Rust (性能优化)
- **HTTP Server**: Kotlin 侧用 NanoHTTPD 或 Ktor
- **视频编码/传输**: Rust 通过 JNI 暴露接口
- **优点**: 视频流性能更好，TKE 可以直接复用 Rust 代码
- **缺点**: 开发周期稍长

**快速验证选方案 A，后续优化再加 Rust**

---

## MVP 核心功能 (3 个 HTTP 接口)

```
GET  /api/screenshot          → 返回 JPEG/PNG base64
GET  /api/ui-tree             → 返回当前界面 UI 结构 JSON
POST /api/action/tap          → 执行点击操作 {x, y}
```

---

## 实现步骤 (2 小时完成 MVP)

### 1. 创建 Android 项目
```bash
# 使用 Android Studio 创建空白项目
# 包名：com.yourcompany.testassistant
# Minimum SDK: API 26 (Android 8.0) - 为了使用 PixelCopy
```

### 2. 添加依赖 (build.gradle.kts)
```kotlin
dependencies {
    // HTTP Server
    implementation("org.nanohttpd:nanohttpd:2.3.1")

    // JSON 序列化
    implementation("com.google.code.gson:gson:2.10.1")

    // 协程（可选，异步处理）
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
}
```

### 3. 核心模块实现

#### 📡 HTTP Server (AutomationServer.kt)
```kotlin
class AutomationServer(private val port: Int = 8765) : NanoHTTPD(port) {

    override fun serve(session: IHTTPSession): Response {
        val uri = session.uri

        return when {
            uri == "/api/screenshot" -> handleScreenshot()
            uri == "/api/ui-tree" -> handleUITree()
            uri.startsWith("/api/action/tap") -> handleTap(session)
            else -> newFixedLengthResponse(Response.Status.NOT_FOUND, "text/plain", "Not Found")
        }
    }

    private fun handleScreenshot(): Response {
        val bitmap = ScreenshotModule.capture()
        val base64 = bitmapToBase64(bitmap)
        val json = """{"success": true, "data": "$base64"}"""
        return newFixedLengthResponse(Response.Status.OK, "application/json", json)
    }

    private fun handleUITree(): Response {
        val uiTree = UITreeModule.exportTree()
        val json = Gson().toJson(uiTree)
        return newFixedLengthResponse(Response.Status.OK, "application/json", json)
    }

    private fun handleTap(session: IHTTPSession): Response {
        val params = session.parameters
        val x = params["x"]?.firstOrNull()?.toIntOrNull() ?: return errorResponse("Missing x")
        val y = params["y"]?.firstOrNull()?.toIntOrNull() ?: return errorResponse("Missing y")

        val success = ActionModule.tap(x, y)
        val json = """{"success": $success}"""
        return newFixedLengthResponse(Response.Status.OK, "application/json", json)
    }
}
```

#### 📸 截图模块 (ScreenshotModule.kt)
```kotlin
object ScreenshotModule {

    // 方法 1: 通过 PixelCopy (需要 View 和 Window 引用)
    fun captureFromView(view: View): Bitmap? {
        val bitmap = Bitmap.createBitmap(view.width, view.height, Bitmap.Config.ARGB_8888)
        val location = IntArray(2)
        view.getLocationInWindow(location)

        val window = (view.context as? Activity)?.window ?: return null

        PixelCopy.request(
            window,
            Rect(location[0], location[1], location[0] + view.width, location[1] + view.height),
            bitmap,
            { copyResult ->
                if (copyResult == PixelCopy.SUCCESS) {
                    // Bitmap ready
                }
            },
            Handler(Looper.getMainLooper())
        )

        return bitmap
    }

    // 方法 2: 通过 MediaProjection (推荐 - 可以截取整个屏幕)
    // 需要用户授权，但只需一次
    fun captureScreen(context: Context): Bitmap? {
        // 使用 MediaProjection API
        // 参考: https://developer.android.com/media/grow/media-projection
    }

    // 方法 3: 通过 UiAutomation (需要在 instrumentation 下运行)
    fun captureViaUiAutomation(): Bitmap? {
        val uiAutomation = InstrumentationRegistry.getInstrumentation().uiAutomation
        val screenshot = uiAutomation.takeScreenshot()
        return screenshot
    }
}
```

#### 🌲 UI 树导出模块 (UITreeModule.kt)
```kotlin
object UITreeModule {

    data class UINode(
        val className: String,
        val bounds: IntArray,  // [x, y, width, height]
        val text: String?,
        val contentDesc: String?,
        val resourceId: String?,
        val clickable: Boolean,
        val focusable: Boolean,
        val children: List<UINode>
    )

    fun exportTree(): List<UINode> {
        val roots = WindowManagerGlobal.getInstance().rootViews
        return roots.map { rootView ->
            traverseView(rootView)
        }
    }

    private fun traverseView(view: View): UINode {
        val children = if (view is ViewGroup) {
            (0 until view.childCount).map { i ->
                traverseView(view.getChildAt(i))
            }
        } else {
            emptyList()
        }

        val bounds = IntArray(2)
        view.getLocationOnScreen(bounds)

        return UINode(
            className = view.javaClass.name,
            bounds = intArrayOf(bounds[0], bounds[1], view.width, view.height),
            text = (view as? TextView)?.text?.toString(),
            contentDesc = view.contentDescription?.toString(),
            resourceId = try {
                view.resources.getResourceName(view.id)
            } catch (e: Exception) {
                null
            },
            clickable = view.isClickable,
            focusable = view.isFocusable,
            children = children
        )
    }
}
```

#### 🎯 操作模块 (ActionModule.kt)
```kotlin
object ActionModule {

    // 方法 1: 通过 UiAutomation (需要 instrumentation)
    fun tap(x: Int, y: Int): Boolean {
        val uiAutomation = InstrumentationRegistry.getInstrumentation().uiAutomation

        val downTime = SystemClock.uptimeMillis()
        val eventDown = MotionEvent.obtain(
            downTime, downTime, MotionEvent.ACTION_DOWN, x.toFloat(), y.toFloat(), 0
        )
        val eventUp = MotionEvent.obtain(
            downTime, downTime + 100, MotionEvent.ACTION_UP, x.toFloat(), y.toFloat(), 0
        )

        return uiAutomation.injectInputEvent(eventDown, true) &&
               uiAutomation.injectInputEvent(eventUp, true)
    }

    // 方法 2: 通过 Instrumentation
    fun tapViaInstrumentation(x: Int, y: Int): Boolean {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        instrumentation.sendPointerSync(
            MotionEvent.obtain(
                SystemClock.uptimeMillis(),
                SystemClock.uptimeMillis(),
                MotionEvent.ACTION_DOWN,
                x.toFloat(),
                y.toFloat(),
                0
            )
        )
        instrumentation.sendPointerSync(
            MotionEvent.obtain(
                SystemClock.uptimeMillis(),
                SystemClock.uptimeMillis(),
                MotionEvent.ACTION_UP,
                x.toFloat(),
                y.toFloat(),
                0
            )
        )
        return true
    }
}
```

### 4. 启动服务 (MainActivity.kt)
```kotlin
class MainActivity : AppCompatActivity() {

    private var server: AutomationServer? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 仅在 Debug 模式启动
        if (BuildConfig.DEBUG) {
            startAutomationServer()
        }
    }

    private fun startAutomationServer() {
        server = AutomationServer(8765)
        try {
            server?.start()
            Log.i("AutomationServer", "Server started on port 8765")
            Toast.makeText(this, "Automation Server started on :8765", Toast.LENGTH_LONG).show()
        } catch (e: IOException) {
            Log.e("AutomationServer", "Failed to start server", e)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        server?.stop()
    }
}
```

### 5. 权限配置 (AndroidManifest.xml)
```xml
<manifest>
    <!-- 网络权限 -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />

    <!-- 如果使用 MediaProjection 截图 -->
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />

    <application
        android:usesCleartextTraffic="true"
        ...>
        <activity android:name=".MainActivity">
            ...
        </activity>
    </application>
</manifest>
```

---

## 测试验证

### 1. 安装 App 并启动
```bash
./gradlew installDebug
adb shell am start -n com.yourcompany.testassistant/.MainActivity
```

### 2. 端口转发
```bash
adb forward tcp:8765 tcp:8765
```

### 3. 测试接口
```bash
# 测试截图
curl http://localhost:8765/api/screenshot | jq

# 测试 UI 树
curl http://localhost:8765/api/ui-tree | jq

# 测试点击
curl -X POST "http://localhost:8765/api/action/tap?x=100&y=200"
```

### 4. 集成到 TKE
修改 TKE 的 controller 模块，添加一个新的 backend：

```rust
// toolkit-engine/src/controller/mod.rs

pub enum ControllerBackend {
    Adb,           // 现有的 adb 方式
    SelfHost,      // 新的 self-host 方式
}

impl Controller {
    pub fn capture_screen(&self) -> Result<Vec<u8>> {
        match self.backend {
            ControllerBackend::Adb => self.capture_via_adb(),
            ControllerBackend::SelfHost => self.capture_via_http(),
        }
    }

    fn capture_via_http(&self) -> Result<Vec<u8>> {
        let response = reqwest::blocking::get("http://localhost:8765/api/screenshot")?;
        let json: serde_json::Value = response.json()?;
        let base64_data = json["data"].as_str().unwrap();
        let image_bytes = base64::decode(base64_data)?;
        Ok(image_bytes)
    }
}
```

---

## 性能对比预期

| 操作 | ADB 方式 | Self-Host 方式 | 提升 |
|------|----------|----------------|------|
| 截图 | ~1-2s | ~100-300ms | **5-10x** |
| UI 树获取 | ~500ms | ~50-100ms | **5x** |
| 点击操作 | ~200ms | ~50ms | **4x** |

---

## 后续优化方向

### 阶段 2️⃣：性能优化
1. **视频流**: 使用 MediaCodec 编码 H.264，通过 WebSocket 推送
2. **Rust 加速**: 将编码和网络传输交给 Rust（通过 JNI）
3. **长连接**: WebSocket 代替 HTTP，减少连接开销

### 阶段 3️⃣：功能增强
1. **输入法支持**: 输入文本
2. **手势支持**: swipe, pinch, rotate
3. **录制回放**: 录制操作序列，生成 .tks 脚本

---

## 关键技术难点

### 难点 1: 如何在非 instrumentation 下注入输入？
**解决方案**:
- 方案 A: 通过 `adb shell input tap` (仍依赖 adb)
- 方案 B: 使用 Accessibility Service 模拟点击
- 方案 C: 使用 `app_process` 运行，获取 INJECT_EVENTS 权限（类似 scrcpy）

### 难点 2: 如何获取所有 Window 的 View？
**解决方案**:
```kotlin
val windowManagerGlobal = WindowManagerGlobal::class.java
val getRootViewsMethod = windowManagerGlobal.getDeclaredMethod("getRootViews")
getRootViewsMethod.isAccessible = true
val rootViews = getRootViewsMethod.invoke(null) as List<View>
```

### 难点 3: MediaProjection 需要用户授权
**解决方案**:
- 首次启动时弹窗请求授权
- 授权后保存 token，后续无需再授权
- 或者使用前台服务常驻

---

## 总结

这个方案**完全可行**，建议：

1. **先用纯 Kotlin 验证 MVP**（2 小时）
2. **测试性能是否满足需求**（截图 < 300ms）
3. **如果满足，再逐步加 Rust 优化视频流**
4. **最终形态**: 一个用户日常使用的 App + 隐藏的自动化能力

核心优势：
- ✅ 比 adb 快 5-10 倍
- ✅ 可以做成独立 App，用户友好
- ✅ 与 TKE 完美配合
- ✅ 后续可扩展性强（AI Tester 也可以用）
