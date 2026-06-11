// Refresh 命令处理器（② 原子方法）
// 刷新页面状态：采集截图 + UI XML 到设备缓存工作区，可选 OCR / 剪裁元素图

use tke::{Result, Refresh, RefreshOptions, JsonOutput, TkeError};
use std::path::PathBuf;

/// Refresh 命令参数
#[derive(clap::Args)]
pub struct RefreshArgs {
    /// 仅采集 UI XML（不截图）
    #[arg(long)]
    pub xml_only: bool,

    /// 同时对截图做 OCR（可指定语言，默认 eng）
    #[arg(long, num_args = 0..=1, default_missing_value = "eng")]
    pub ocr: Option<String>,

    /// 按坐标剪裁元素图，格式: "x1,y1,x2,y2"
    #[arg(long)]
    pub crop: Option<String>,

    /// 剪裁元素图输出路径（默认工作区 element_crop.png）
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// 解析 "x1,y1,x2,y2" 剪裁参数
fn parse_crop(s: &str) -> Result<(u32, u32, u32, u32)> {
    let nums: Vec<u32> = s
        .split(',')
        .map(|p| p.trim().parse::<u32>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| TkeError::InvalidArgument(format!("剪裁坐标格式无效: '{}' (期望 x1,y1,x2,y2)", s)))?;
    if nums.len() != 4 {
        return Err(TkeError::InvalidArgument(format!(
            "剪裁坐标格式无效: '{}' (期望 4 个数字)", s
        )));
    }
    Ok((nums[0], nums[1], nums[2], nums[3]))
}

/// 处理 Refresh 命令（必须指定 -d/--device）
pub async fn handle(args: RefreshArgs, device_id: Option<String>) -> Result<()> {
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("refresh 必须指定设备: -d/--device <设备ID>"));

    let crop = match &args.crop {
        Some(s) => Some(parse_crop(s).unwrap_or_else(|e| JsonOutput::error(e.to_string()))),
        None => None,
    };

    let opts = RefreshOptions {
        xml_only: args.xml_only,
        ocr_lang: args.ocr,
        crop,
        crop_out: args.out,
    };

    let refresh = Refresh::new(device)
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));

    match refresh.run(opts).await {
        Ok(result) => {
            JsonOutput::print(&result);
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
