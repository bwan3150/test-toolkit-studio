// 视频流服务器模块
// 从 autoserver 接收 H264 流，转换为 MJPEG 并通过 HTTP 推流

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::process::{Command, Stdio, ChildStdout};
use std::path::PathBuf;
use std::fs;
use std::io::Write as IoWrite;
use crate::{Result, TkeError};

/// MJPEG 边界标记
const BOUNDARY: &str = "tkevideoboundary";

// 引入构建时生成的 ffmpeg 二进制数据
include!(concat!(env!("OUT_DIR"), "/embedded_ffmpeg.rs"));

/// 获取 ffmpeg 可执行文件路径
/// 如果有内置 ffmpeg，则解压到临时目录；否则使用系统 ffmpeg
fn get_ffmpeg_path() -> Result<PathBuf> {
    if HAS_BUNDLED_FFMPEG {
        // 使用内置 ffmpeg
        let temp_dir = std::env::temp_dir().join("tke_ffmpeg");

        // 确保临时目录存在
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .map_err(|e| TkeError::IoError(e))?;
        }

        let ffmpeg_path = temp_dir.join(FFMPEG_BINARY_NAME);

        // 检查是否需要重新提取 ffmpeg
        let should_extract = if ffmpeg_path.exists() {
            // 检查文件大小是否匹配
            match fs::metadata(&ffmpeg_path) {
                Ok(metadata) => metadata.len() != EMBEDDED_FFMPEG_BINARY.len() as u64,
                Err(_) => true,
            }
        } else {
            true
        };

        if should_extract {
            // 写入 ffmpeg 二进制文件
            let mut file = fs::File::create(&ffmpeg_path)
                .map_err(|e| TkeError::IoError(e))?;

            file.write_all(EMBEDDED_FFMPEG_BINARY)
                .map_err(|e| TkeError::IoError(e))?;

            // 设置执行权限（Unix 系统）
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = file.metadata()
                    .map_err(|e| TkeError::IoError(e))?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&ffmpeg_path, perms)
                    .map_err(|e| TkeError::IoError(e))?;
            }
        }

        Ok(ffmpeg_path)
    } else {
        // 使用系统 ffmpeg
        which::which("ffmpeg")
            .map_err(|_| TkeError::InvalidArgument(
                "未找到 ffmpeg，请安装 ffmpeg 或使用内置版本".to_string()
            ))
    }
}

/// 视频流服务器
pub struct VideoStreamServer {
    /// HTTP 服务器监听的端口
    http_port: u16,
    /// 连接到 scrcpy 视频流的端口
    video_port: u16,
    /// 是否正在运行
    running: Arc<Mutex<bool>>,
    /// 当前帧数据
    current_frame: Arc<Mutex<Option<Vec<u8>>>>,
}

impl VideoStreamServer {
    /// 创建新的视频流服务器
    pub fn new(http_port: u16, video_port: u16) -> Self {
        Self {
            http_port,
            video_port,
            running: Arc::new(Mutex::new(false)),
            current_frame: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动视频流服务器
    pub fn start(&self) -> Result<()> {
        *self.running.lock().unwrap() = true;

        // 启动视频接收线程
        self.start_video_receiver()?;

        // 启动 HTTP 服务器
        self.start_http_server()?;

        Ok(())
    }

    /// 停止视频流服务器
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }

    /// 启动视频接收线程
    fn start_video_receiver(&self) -> Result<()> {
        let video_port = self.video_port;
        let running = Arc::clone(&self.running);
        let current_frame = Arc::clone(&self.current_frame);

        thread::spawn(move || {
            if let Err(e) = Self::receive_video_loop(video_port, running, current_frame) {
                eprintln!("视频接收线程错误: {}", e);
            }
        });

        Ok(())
    }

    /// 视频接收循环
    fn receive_video_loop(
        video_port: u16,
        running: Arc<Mutex<bool>>,
        current_frame: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        // 创建 TCP 监听器，等待 autoserver 连接
        let listener = TcpListener::bind(format!("127.0.0.1:{}", video_port))
            .map_err(|e| TkeError::IoError(e))?;

        println!("✅ 等待 autoserver 连接到端口 {} ...", video_port);

        // 设置非阻塞模式，以便可以检查 running 状态
        listener.set_nonblocking(true)
            .map_err(|e| TkeError::IoError(e))?;

        // 等待连接
        let mut stream = loop {
            if !*running.lock().unwrap() {
                return Ok(()); // 如果已停止，退出
            }

            match listener.accept() {
                Ok((stream, addr)) => {
                    println!("✅ autoserver 已连接: {}", addr);
                    break stream;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 非阻塞模式，没有连接时会返回 WouldBlock
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    return Err(TkeError::IoError(e));
                }
            }
        };

        // 连接建立后，设置为阻塞模式
        stream.set_nonblocking(false)
            .map_err(|e| TkeError::IoError(e))?;

        // 获取 ffmpeg 路径
        let ffmpeg_path = get_ffmpeg_path()?;
        println!("使用 ffmpeg: {:?}", ffmpeg_path);

        // 启动 ffmpeg 进程
        // 命令：ffmpeg -f h264 -i pipe:0 -f image2pipe -vcodec mjpeg -q:v 3 -r 30 pipe:1
        let mut ffmpeg = Command::new(&ffmpeg_path)
            .args(&[
                "-f", "h264",           // 输入格式：H264
                "-i", "pipe:0",         // 从 stdin 读取
                "-f", "image2pipe",     // 输出格式：图像管道
                "-vcodec", "mjpeg",     // 视频编码器：MJPEG
                "-q:v", "3",            // 质量（1-31，越小越好）
                "-r", "30",             // 帧率 30fps
                "pipe:1"                // 输出到 stdout
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())  // 忽略 stderr 输出
            .spawn()
            .map_err(|e| TkeError::IoError(e))?;

        let mut ffmpeg_stdin = ffmpeg.stdin.take()
            .ok_or_else(|| TkeError::InvalidArgument("无法获取 ffmpeg stdin".to_string()))?;
        let ffmpeg_stdout = ffmpeg.stdout.take()
            .ok_or_else(|| TkeError::InvalidArgument("无法获取 ffmpeg stdout".to_string()))?;

        // 创建线程从 ffmpeg stdout 读取 JPEG 帧
        let frame_reader_running = Arc::clone(&running);
        let frame_reader_current = Arc::clone(&current_frame);
        let frame_reader = thread::spawn(move || {
            Self::read_jpeg_frames(ffmpeg_stdout, frame_reader_running, frame_reader_current)
        });

        // 主线程：从 scrcpy 读取 H264 -> 写入 ffmpeg stdin
        let mut buffer = vec![0u8; 65536]; // 64KB buffer
        while *running.lock().unwrap() {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    println!("视频流连接已关闭");
                    break;
                }
                Ok(n) => {
                    // 将 H264 数据写入 ffmpeg stdin
                    if let Err(e) = ffmpeg_stdin.write_all(&buffer[..n]) {
                        eprintln!("写入 ffmpeg 失败: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("读取视频流错误: {}", e);
                    break;
                }
            }
        }

        // 等待帧读取线程结束
        drop(ffmpeg_stdin); // 关闭 stdin，让 ffmpeg 退出
        let _ = frame_reader.join();
        let _ = ffmpeg.kill(); // 确保 ffmpeg 进程被杀死

        Ok(())
    }

    /// 从 ffmpeg stdout 读取 JPEG 帧
    fn read_jpeg_frames(
        mut stdout: ChildStdout,
        running: Arc<Mutex<bool>>,
        current_frame: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        let mut buffer = Vec::new();
        let mut read_buf = vec![0u8; 4096];

        while *running.lock().unwrap() {
            match stdout.read(&mut read_buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    buffer.extend_from_slice(&read_buf[..n]);

                    // 检查是否有完整的 JPEG（以 FFD8 开始，FFD9 结束）
                    if let Some(jpeg) = Self::extract_jpeg(&mut buffer) {
                        *current_frame.lock().unwrap() = Some(jpeg);
                    }
                }
                Err(e) => {
                    eprintln!("读取 ffmpeg 输出错误: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// 从缓冲区提取完整的 JPEG 图像
    fn extract_jpeg(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
        // JPEG 文件格式：以 0xFF 0xD8 开始，以 0xFF 0xD9 结束
        const JPEG_START: &[u8] = &[0xFF, 0xD8];
        const JPEG_END: &[u8] = &[0xFF, 0xD9];

        // 查找 JPEG 开始标记
        if let Some(start_pos) = buffer.windows(2).position(|w| w == JPEG_START) {
            // 从开始位置之后查找结束标记
            if let Some(end_pos) = buffer[start_pos..].windows(2).position(|w| w == JPEG_END) {
                let jpeg_end = start_pos + end_pos + 2; // +2 包含结束标记本身
                let jpeg = buffer[start_pos..jpeg_end].to_vec();
                buffer.drain(0..jpeg_end); // 移除已提取的 JPEG
                return Some(jpeg);
            }
        }

        None
    }

    /// 启动 HTTP 服务器
    fn start_http_server(&self) -> Result<()> {
        let http_port = self.http_port;
        let running = Arc::clone(&self.running);
        let current_frame = Arc::clone(&self.current_frame);

        thread::spawn(move || {
            if let Err(e) = Self::http_server_loop(http_port, running, current_frame) {
                eprintln!("HTTP 服务器错误: {}", e);
            }
        });

        Ok(())
    }

    /// HTTP 服务器循环
    fn http_server_loop(
        http_port: u16,
        running: Arc<Mutex<bool>>,
        current_frame: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", http_port))
            .map_err(|e| TkeError::IoError(e))?;

        println!("✅ HTTP 视频流服务器已启动在端口 {}", http_port);
        println!("   访问 http://127.0.0.1:{}/stream 查看视频流", http_port);

        for stream in listener.incoming() {
            if !*running.lock().unwrap() {
                break;
            }

            match stream {
                Ok(stream) => {
                    let current_frame = Arc::clone(&current_frame);
                    thread::spawn(move || {
                        if let Err(e) = Self::handle_client(stream, current_frame) {
                            eprintln!("处理客户端错误: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("接受连接错误: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 处理客户端请求
    fn handle_client(
        mut stream: TcpStream,
        current_frame: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        // 读取 HTTP 请求
        let mut buffer = [0u8; 1024];
        stream.read(&mut buffer).map_err(|e| TkeError::IoError(e))?;

        // 简单解析请求行
        let request = String::from_utf8_lossy(&buffer);

        if request.starts_with("GET /stream") {
            // 返回 MJPEG 流
            Self::stream_mjpeg(stream, current_frame)?;
        } else {
            // 返回 404
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            stream.write_all(response.as_bytes()).map_err(|e| TkeError::IoError(e))?;
        }

        Ok(())
    }

    /// 推送 MJPEG 流
    fn stream_mjpeg(
        mut stream: TcpStream,
        current_frame: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<()> {
        // 发送 MJPEG 响应头
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: multipart/x-mixed-replace; boundary={}\r\n\
             Cache-Control: no-cache\r\n\
             Connection: close\r\n\r\n",
            BOUNDARY
        );
        stream.write_all(header.as_bytes()).map_err(|e| TkeError::IoError(e))?;

        // 持续推送帧
        loop {
            // 获取当前帧
            let frame = {
                let guard = current_frame.lock().unwrap();
                guard.clone()
            };

            if let Some(frame_data) = frame {
                // 发送 MJPEG 帧
                let frame_header = format!(
                    "--{}\r\n\
                     Content-Type: image/jpeg\r\n\
                     Content-Length: {}\r\n\r\n",
                    BOUNDARY,
                    frame_data.len()
                );

                if stream.write_all(frame_header.as_bytes()).is_err() {
                    break; // 客户端断开
                }

                if stream.write_all(&frame_data).is_err() {
                    break; // 客户端断开
                }

                if stream.write_all(b"\r\n").is_err() {
                    break; // 客户端断开
                }

                stream.flush().map_err(|e| TkeError::IoError(e))?;
            }

            // 控制帧率 (约 30 FPS)
            std::thread::sleep(std::time::Duration::from_millis(33));
        }

        Ok(())
    }
}

/// 启动视频流服务器（阻塞）
pub fn start_video_stream_server(http_port: u16, video_port: u16) -> Result<()> {
    println!("启动视频流服务器...");
    println!("  - HTTP 端口: {}", http_port);
    println!("  - 视频流端口: {}", video_port);

    let server = VideoStreamServer::new(http_port, video_port);
    server.start()?;

    println!("✅ 视频流服务器已启动");
    println!("   在浏览器中访问: http://127.0.0.1:{}/stream", http_port);

    // 保持运行
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
