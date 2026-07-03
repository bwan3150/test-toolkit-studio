// Element 命令处理器 - 元素库管理

use tke::{Result, JsonOutput};

/// Element 命令枚举
#[derive(clap::Subcommand)]
pub enum ElementCommands {
    /// 按坐标从当前页面取元素并落库（自动填当前平台通道 + crop 模板图 + ocr 文本）
    Add {
        /// 元素名（元素库 key，脚本中 {元素名} 引用）
        name: String,
        /// 目标库：`.tklib` 元素包（解包→落库→回包）或裸 element.json（直接写）。
        /// 没有共享元素库——必须指明要写进哪个脚本的包。
        #[arg(long)]
        lib: std::path::PathBuf,
        /// 取元素的屏幕坐标（截图像素），格式 x,y
        #[arg(long)]
        at: String,
        /// 元素描述
        #[arg(long)]
        desc: Option<String>,
        /// 使用工作区中已有的页面状态，跳过重新采集
        #[arg(long)]
        cached: bool,
        /// 强制覆盖已有的 img/ocr 通道
        #[arg(long)]
        force: bool,
    },
}

/// 解析 "x,y" 坐标
fn parse_at(s: &str) -> Result<(i32, i32)> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    if parts.len() == 2 {
        if let (Ok(x), Ok(y)) = (parts[0].parse(), parts[1].parse()) {
            return Ok((x, y));
        }
    }
    Err(tke::TkeError::InvalidArgument(format!(
        "坐标格式错误: {} (应为 x,y 如 363,231)", s
    )))
}

/// 处理 Element 相关命令（必须指定 -d/--device；目标库经 --lib 显式给出）。
/// 没有共享元素库：--lib 是 `.tklib` 时走「解包→落库→回包」，是裸 element.json 则直接写。
pub async fn handle(
    action: ElementCommands,
    params: std::sync::Arc<tke::Params>,
) -> Result<()> {
    let device_id = params.device();
    match action {
        ElementCommands::Add { name, lib, at, desc, cached, force } => {
            let device = device_id
                .unwrap_or_else(|| JsonOutput::error("element add 必须指定设备: -d/--device <设备ID>"));
            let (x, y) = parse_at(&at)?;
            let is_pack = lib.extension().and_then(|s| s.to_str()) == Some("tklib");
            let (lib_json, repack_to) = if is_pack {
                let stem = lib.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let dest = params
                    .cache_root()
                    .join("tklib-unpack")
                    .join(format!("{}-elemadd-{}", stem, std::process::id()));
                let json = tke::utils::tklib::unpack(&lib, &dest)
                    .unwrap_or_else(|e| JsonOutput::error(format!("解包元素包失败 {}: {}", lib.display(), e)));
                (json, Some(lib.clone()))
            } else {
                (lib.clone(), None)
            };
            let result = tke::tools::element::add_element(
                device.clone(), &lib_json, &name, desc, x, y, cached, force,
            )
            .await?;
            if let Some(target) = repack_to {
                let platform = tke::Platform::from_device(Some(&device));
                let meta = tke::utils::tklib::TklibMeta::new(platform.name(), &device);
                tke::utils::tklib::pack(&lib_json, &target, &meta)?;
            }
            JsonOutput::print(result);
        }
    }
    Ok(())
}
