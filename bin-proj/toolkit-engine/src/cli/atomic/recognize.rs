// Recognize 命令处理器（② 原子方法）
// 在当前页面定位元素位置（默认先采集最新页面状态再定位）
// 元素库由 --element 指定（缺省按 ./element.json → ./locator/element.json 查找）

use tke::{Result, Recognize, JsonOutput, LocatorStrategy};

/// Recognize 命令参数
#[derive(clap::Args)]
pub struct RecognizeArgs {
    /// 元素名称（元素库 key）；--by-text 时为直接查找的文本
    pub name: String,

    /// 定位策略 (auto/xpath/resource-id/text/content-desc/class-name/ocr/img)
    #[arg(long, short, default_value = "auto")]
    pub strategy: String,

    /// 图像匹配置信度阈值 (0.0-1.0)
    #[arg(long, default_value = "0.60")]
    pub threshold: f32,

    /// 直接按文本在 UI 树中查找（不走元素库）
    #[arg(long)]
    pub by_text: bool,

    /// 使用工作区中已有的页面状态，跳过重新采集
    #[arg(long)]
    pub cached: bool,

    /// 元素库：`.tklib` 元素包（自动解包只读）或裸 element.json。--by-text 时可省略。
    /// 没有共享元素库——按元素名定位必须指明脚本的包。
    #[arg(long)]
    pub lib: Option<std::path::PathBuf>,
}

/// 解析策略字符串
fn parse_strategy(s: &str) -> LocatorStrategy {
    match s.to_lowercase().as_str() {
        "xpath" => LocatorStrategy::XPath,
        "resource-id" | "resourceid" | "id" => LocatorStrategy::ResourceId,
        "text" => LocatorStrategy::Text,
        "content-desc" | "contentdesc" | "desc" => LocatorStrategy::ContentDesc,
        "class-name" | "classname" | "class" => LocatorStrategy::ClassName,
        "ocr" => LocatorStrategy::Ocr,
        "img" | "image" => LocatorStrategy::Img,
        _ => LocatorStrategy::Auto,
    }
}

/// 处理 Recognize 命令（必须指定 -d/--device）
pub async fn handle(
    args: RecognizeArgs,
    params: std::sync::Arc<tke::Params>,
) -> Result<()> {
    let device_id = params.device();
    let device = device_id
        .unwrap_or_else(|| JsonOutput::error("recognize 必须指定设备: -d/--device <设备ID>"));
    // 元素库：--lib 给 .tklib 则解包只读；给裸 element.json 直接用；--by-text 可不带
    let element_path = args.lib.clone().map(|lib| {
        if lib.extension().and_then(|s| s.to_str()) == Some("tklib") {
            let stem = lib.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let dest = params
                .cache_root()
                .join("tklib-unpack")
                .join(format!("{}-recognize-{}", stem, std::process::id()));
            tke::utils::tklib::unpack(&lib, &dest)
                .unwrap_or_else(|e| JsonOutput::error(format!("解包元素包失败 {}: {}", lib.display(), e)))
        } else {
            lib
        }
    });
    if element_path.is_none() && !args.by_text {
        JsonOutput::error("recognize 按元素名定位必须指定 --lib <foo.tklib 或 element.json>（没有共享元素库）");
    }

    let mut recognize = Recognize::new(device, element_path.as_deref())
        .unwrap_or_else(|e| JsonOutput::error(e.to_string()));
    recognize.set_confidence_threshold(args.threshold);

    let result = if args.by_text {
        recognize.find_text(&args.name, args.cached).await
    } else {
        recognize
            .find(&args.name, parse_strategy(&args.strategy), args.cached)
            .await
    };

    match result {
        Ok((point, bounds)) => {
            JsonOutput::success(serde_json::json!({
                "success": true,
                "element": args.name,
                "x": point.x,
                "y": point.y,
                // 实时边界框：来自当前页面实际匹配到的元素
                "bounds": bounds,
            }));
            Ok(())
        }
        Err(e) => JsonOutput::error(e.to_string()),
    }
}
