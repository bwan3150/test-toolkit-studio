// Fetch 命令处理器（② 原子方法）
// 采集当前页面：整张截图 + UI XML，可选 OCR / 元素列表 / 裁剪元素图

use tke::{Result, Fetch, FetchOptions, JsonOutput, TkeError};
use std::path::PathBuf;

/// Fetch 命令参数
#[derive(clap::Args)]
pub struct FetchArgs {
    /// 仅采集 UI XML（不截图）
    #[arg(long)]
    pub xml_only: bool,

    /// 同时提取元素列表
    #[arg(long)]
    pub elements: bool,

    /// 同时对截图做 OCR（可指定语言，默认 eng）
    #[arg(long, num_args = 0..=1, default_missing_value = "eng")]
    pub ocr: Option<String>,

    /// 按坐标裁剪元素图，格式: "x1,y1,x2,y2"
    #[arg(long)]
    pub crop: Option<String>,

    /// 裁剪元素图输出路径（默认 workarea/element_crop.png）
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// 解析 "x1,y1,x2,y2" 裁剪参数
fn parse_crop(s: &str) -> Result<(u32, u32, u32, u32)> {
    let nums: Vec<u32> = s
        .split(',')
        .map(|p| p.trim().parse::<u32>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| TkeError::InvalidArgument(format!("裁剪坐标格式无效: '{}' (期望 x1,y1,x2,y2)", s)))?;
    if nums.len() != 4 {
        return Err(TkeError::InvalidArgument(format!(
            "裁剪坐标格式无效: '{}' (期望 4 个数字)", s
        )));
    }
    Ok((nums[0], nums[1], nums[2], nums[3]))
}

/// 处理 Fetch 命令（必须指定 -d/--device）
pub async fn handle(
    args: FetchArgs,
    device_id: Option<String>,
    project_path: PathBuf,
) -> Result<()> {
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("fetch 必须指定设备: -d/--device <设备ID>"));

    let crop = match &args.crop {
        Some(s) => Some(parse_crop(s).unwrap_or_else(|e| JsonOutput::error(e.to_string()))),
        None => None,
    };

    let opts = FetchOptions {
        xml_only: args.xml_only,
        elements: args.elements,
        ocr_lang: args.ocr,
        crop,
        crop_out: args.out,
    };

    let fetch = Fetch::new(device, project_path)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    match fetch.run(opts).await {
        Ok(result) => {
            JsonOutput::print(&result);
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
