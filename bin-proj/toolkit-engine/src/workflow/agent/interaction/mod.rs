// 【用户交互】ask_user 的答复通道
// v1：从 stdin 读一行（CLI 交互）。未来接 Electron 时可在此替换为 IPC/管道通道。

/// 读取用户一行输入（ask_user 用）；阻塞读放进 blocking 线程
pub async fn read_user_line(question: &str) -> String {
    println!("\n[AI 提问] {}\n> ", question);
    tokio::task::spawn_blocking(|| {
        let mut s = String::new();
        let _ = std::io::stdin().read_line(&mut s);
        s.trim().to_string()
    })
    .await
    .unwrap_or_default()
}
