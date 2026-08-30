// 总览 help 的排版。纯展示逻辑，与 main 的路由分离。
//
// **形照 clap/zellij 的惯例**：USAGE 在最前，命令与参数各成一段，
// 每条一行、动词开头。人来这儿是找"下一条该敲什么"，不是读说明书。
//
// 所以这里写不写一句话的判据是 INV-18：**它会不会改变他下一步做什么**。
// "唯一会联网下载的命令"、"老写法照常能用"这类背景交代一概不写 ——
// 那些属于 `docs/`，或者属于子命令自己的 `--help`。

/// 生成 `tke --help` 的总览文本（终端带颜色，重定向时纯文本）
pub fn build_help() -> String {
    use std::io::IsTerminal;

    let tty = std::io::stdout().is_terminal();
    // 只用三种：段标题、命令名、次要说明。颜色多了就不是区分而是装饰
    let (head, cmd, dim, r) = if tty {
        ("\x1b[1;33m", "\x1b[1;36m", "\x1b[2m", "\x1b[0m")
    } else {
        ("", "", "", "")
    };

    format!(
        "\
tke {ver}

{head}USAGE:{r}
    tke [OPTIONS] <COMMAND>
    tke <path.tks>                {dim}回放一个脚本（= tke run <path>）{r}

{head}原子指令{r}  {dim}要 -d 指定设备{r}
    {cmd}refresh{r}      采集当前页面：截图 + UI 结构到工作区
    {cmd}fetch{r}        输出当前页面的元素列表（JSON）
    {cmd}recognize{r}    找一个元素，返回坐标
    {cmd}control{r}      操作设备：click / input / swipe / launch / key …

{head}工作流{r}
    {cmd}run{r}          执行 .tks 脚本或 .toml 流程
    {cmd}steps{r}        直接执行几条指令，不落文件
    {cmd}harness{r}      AI 探索并生成脚本
    {cmd}task{r}         新建一次测试会话
    {cmd}report{r}       出报告

{head}安全{r}
    {cmd}security{r}     安全测试（对话式）
    {cmd}http{r}         发一个请求，落证据
    {cmd}recon{r}        侦察：响应头、指纹、TLS …

{head}环境{r}
    {cmd}doctor{r}       体检：依赖、设备、版本
    {cmd}doctor --fix{r} 补齐缺的依赖
    {cmd}update{r}       升级 tke 与 skill
    {cmd}uninstall{r}    卸载

{head}工具{r}
    {cmd}app{r}          设备应用：安装、启动、看日志
    {cmd}device{r}       设备信息
    {cmd}file{r}         设备文件
    {cmd}element{r}      元素库
    {cmd}ocr{r}          图片文字识别
    {cmd}sandbox{r}      源码沙盒：这次改动碰了哪些界面

{head}服务{r}
    {cmd}serve{r}        把本机能力开成 HTTP 接口
    {cmd}remote{r}       远程会话管理

{head}OPTIONS:{r}
    {cmd}-d, --device{r} <ID>       目标设备：Android 序列号 / web / iOS UDID
        {cmd}--element{r} <PATH>    元素库路径
        {cmd}--log{r} <DIR>         产物目录（不传则不留产物）
    {cmd}-c, --config{r} <TOML>     配置文件
        {cmd}--json{r}              强制 NDJSON 输出
        {cmd}--copilot{r} <BOOL>    回放定位失败时让 AI 找回（默认开）
        {cmd}--headless{r}[=MODE]   web 无头：auto / on / off
    {cmd}-v, --verbose{r}           DEBUG 日志
    {cmd}-h, --help{r}              帮助
    {cmd}-V, --version{r}           版本

{dim}看某条命令怎么用：tke <command> --help{r}
",
        ver = env!("BUILD_VERSION"),
    )
}
