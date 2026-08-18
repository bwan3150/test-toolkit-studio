// 总览 help 渲染 - 手工排版（按四大块分组，直通清单动态扫描同目录二进制）
// 纯展示逻辑，与 main 的路由分离。

/// 生成 `tke --help` 的总览文本（终端带颜色，重定向时纯文本）
pub fn build_help() -> String {
    use std::io::IsTerminal;

    let tty = std::io::stdout().is_terminal();
    let (b, c, g, d, r) = if tty {
        ("\x1b[1m", "\x1b[1;36m", "\x1b[32m", "\x1b[2m", "\x1b[0m")
    } else {
        ("", "", "", "", "")
    };


    format!(
        "\
{b}Toolkit Engine{r}  {d}v{}{r}

{c}原子指令{r}
  {g}refresh{r}      刷新页面状态: 采集截图 + UI XML 到工作区 {d}(+OCR / 剪裁元素图){r}
  {g}fetch{r}        提取当前页面元素列表 {d}(含 xpath, 输出 JSON 数组){r}
  {g}recognize{r}    定位元素: xml / ocr / 图像匹配, 返回坐标
  {g}control{r}      执行操作: click / press / swipe / drag / swipe-dir / input
               clear / hide-keyboard / back / home / launch / close / key

{c}工作流{r}
  {g}run{r}          执行 .tks 单脚本 或 .toml flow {d}(多脚本顺序执行){r}
  {g}steps{r}        不落文件执行一串指令  {d}例: tke steps \"点击 [{{登录按钮}}]\" \"等待 [2s]\"{r}
  {g}report{r}       重建一次检查的报告 {d}例: tke report ~/.tke/logs/登录检查/（steps 每批跑完会自动重建；--full-image 出原图版）{r}
  {g}harness{r}      AI 探索测试并生成脚本  {d}例: tke harness 用例.md --scripts 目录/（文件名 AI 起·简写 harn）{r}

{c}环境{r}
  {g}doctor{r}       环境体检 {d}(依赖齐不齐·设备连没连·版本与 skill 跟不跟得上；不下载任何东西){r}
  {g}update{r}       升级到最新版 {d}(tke 与 skill 一起；跑官方安装脚本){r}
  {g}uninstall{r}    卸载 {d}(默认保留日志与 Chrome，--all 一并删){r}
  {g}doctor --fix{r} 顺带补齐缺的依赖 {d}(chromedriver/Chrome/adb/go-ios；唯一会联网下载的命令){r}
  {d}fix 是 doctor --fix 的别名，老写法照常能用{r}

{c}自有工具{r}
  {g}ocr{r}          图片文字识别 {d}(离线 / 在线){r}
  {g}file{r}         设备文件系统管理
  {g}app{r}          设备应用管理 {d}(app log 看 logcat: App 崩了/点了没反应先看它){r}
  {g}device{r}       设备详细信息
  {g}element{r}      元素库管理 {d}(element add <名称> --at x,y 取元素落库, 自动crop模板图+ocr){r}

{c}全局参数{r}
  {g}-d, --device{r} <ID>      目标设备 {d}(Android序列号 / web / iOS UDID){r}
      {g}--element{r} <path>   元素库 element.json {d}(缺省 ./element.json → ./locator/element.json){r}
      {g}--log{r} <dir>        产物输出目录 {d}(不传则 run/steps 不保存产物){r}
  {g}-c, --config{r} <toml>    配置文件 {d}(缺省读 tke 同目录 config.toml; CLI 参数优先){r}
      {g}--json{r}             强制 NDJSON 输出 {d}(终端默认友好格式, 管道自动 NDJSON){r}
      {g}--copilot{r} <bool>   AI 辅助驾驶 {d}(默认开; 回放定位失败让 AI 按当前页面找回, 不改脚本/元素包){r}
      {g}--headless{r}[=模式]   web 无头 {d}(auto 默认按有无桌面自动判断; =on 强制无头; =off 强制有头){r}
  {g}-v, --verbose{r}          DEBUG 日志    {g}-h{r} 帮助    {g}-V{r} 版本
",
        env!("BUILD_VERSION")
    )
}
