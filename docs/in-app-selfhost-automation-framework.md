
🚀 目标

构建一个正常可用的日常 App(用于直接用Insight页面的API获取和展示Bug信息之类的随手能看的工作)，但内部集成一个 Self-Hosted Automation Framework，能够与toolkit engine也就是tke合作, 进行更快速度的截图和xml获取(比如用 tke controller capture这种指令)
在“自动化模式”下能：
	•	执行点击/输入/滚动/清空输入框等操作；(最先解决的应该是基于adb的controller capture指令运行很慢的问题)
	•	输出截图类似minicap / 实时视频流类似scrcpy；
	•	导出当前界面结构（XML / JSON）；
	•	通过本地或远程接口控制。

⸻

🧩 整体架构

┌────────────────────────────────────────────┐
│                 YourApp                    │
│────────────────────────────────────────────│
│  普通功能（UI + 业务逻辑）                  │
│────────────────────────────────────────────│
│  🔒 Automation Framework (Internal)        │
│   • Command Server (HTTP/WebSocket/gRPC)   │
│   • UI Action Engine                       │
│   • Layout Inspector                       │
│   • Screenshot / Video Stream Module        │
│────────────────────────────────────────────│
│  🔌 Debug/Test Mode Switch                  │
└────────────────────────────────────────────┘


⸻

🧠 模块与任务清单

模块	职责	实现方式（建议）
1️⃣ Command Server	接收外部命令，返回执行结果	用 HTTP 或 WebSocket（Ktor / OkHttp / gRPC）
2️⃣ UI Action Engine	执行点击、输入、清空、滚动等	直接调用 View 方法或使用 Instrumentation API
3️⃣ Layout Inspector	输出当前 UI 树结构（XML）	递归遍历 View 层级，导出属性
4️⃣ Screenshot / Video Stream	截图或推流	PixelCopy / MediaProjection / MediaCodec（可配合 Rust 实现编码推流）
5️⃣ Auth & Switch	控制启用自动化模式	调试开关、命令参数或隐藏配置
6️⃣ (Optional) Rust Bridge	性能密集模块（视频编码、传输）	JNI 调 Rust 库，负责 H.264/JPEG 编码与网络 I/O


⸻

⚙️ 开发步骤简版
	1.	创建 Android App 工程（Kotlin/Java）
	2.	在 Application 或调试入口添加：
	•	启动 AutomationService（HTTP/WebSocket 监听）
	•	仅在测试/调试模式下启用
	3.	实现命令协议，例如：

POST /action/click?id=btn_login
POST /action/setText?id=input_name&text=Tom
GET  /screen/screenshot
GET  /ui/tree


	4.	截图模块：
	•	普通场景：用 PixelCopy → Bitmap → JPEG → Base64 / 流；
	•	需要流媒体：用 MediaProjection + MediaCodec → H.264 → WebSocket 推送。
	5.	布局导出：
	•	遍历根视图 (WindowManagerGlobal.getViewRootNames() + getRootView())，
	•	递归导出结构与坐标。
	6.	测试脚本侧（PC 工具）：
	•	通过 adb forward tcp:xxxx tcp:xxxx 把端口映射；
	•	脚本直接调用这些 HTTP 接口进行自动化控制与截图收集。

⸻

💡 技术要点与提示
	•	⚙️ 效率：所有模块常驻，长连接通信，避免重复启动 Activity；
	•	🔐 安全：自动化端口只在 Debug/测试构建中启用；
	•	⚡ 性能：截图/视频模块独立线程或 native 层（Rust）执行；
	•	🧩 可扩展：未来可添加录制脚本、断点调试、行为回放等功能。

⸻

✅ 最终效果
	•	普通用户使用时，App 表现正常；
	•	测试环境下，你的自动化脚本可直接连接 App：
	•	获取实时画面；
	•	拉取界面结构；
	•	执行高层指令（点击、输入、清空等）；
→ 速度快、无需 root、无需外部 agent、架构清晰。
