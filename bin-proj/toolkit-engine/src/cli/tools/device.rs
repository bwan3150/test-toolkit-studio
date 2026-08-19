// Device 命令处理器

use tke::{Result, JsonOutput, DeviceManager};

/// Device 命令枚举
#[derive(clap::Subcommand)]
pub enum DeviceCommands {
    /// 列出这台机器现在能测什么：安卓 / iOS 真机 / iOS 模拟器 / 浏览器
    ///
    /// 输出里的 ID 就是 `-d` 该填的值。**查不了的那类会说明原因**——
    /// "没装 adb" 和 "没连手机" 在结果上长得一样，不说清楚人只会去插拔数据线
    List {
        /// 连**没启动的模拟器**一起列（要挑一台来启动时用）。
        /// 默认不列：装了 Xcode 的 mac 上动辄二三十台，全摆出来那份清单就没法看了
        #[arg(long)]
        all: bool,
    },
    /// 某台设备的详情：型号 / 屏幕尺寸 / 系统版本（安卓另有硬件、电池、网络）
    ///
    /// 四种设备都能用；要 `-d` 指定哪一台
    Info,
    /// 读安卓的系统属性（`adb getprop`）——**仅安卓**
    Prop {
        /// 属性名称 (如 ro.build.version.release)
        name: String,
    },
}

/// 处理 Device 相关命令
pub fn handle(action: DeviceCommands, params: std::sync::Arc<tke::Params>) -> Result<()> {
    // list 是**唯一不需要 -d 的**：它回答的正是"该填什么"
    if let DeviceCommands::List { all } = action {
        return list(all);
    }
    let device_id = params.device();
    // 只有**真安卓**才走 DeviceManager（它是 adb 专属）。
    // ⚠️ `fake:` 前缀在 Platform 上算 Android（没有 Fake 这个平台），
    // 直接按平台判会让它去连 adb，然后报「缺少 adb」——测试设备要能离线跑才有意义
    let via_driver = tke::Platform::from_device(device_id.as_deref()) != tke::Platform::Android
        || device_id.as_deref().is_some_and(|d| d.starts_with("fake:"));
    if via_driver {
        match action {
            DeviceCommands::Info => {
                let controller = tke::Controller::new(device_id)?;
                let info = controller.get_device_info()?;
                JsonOutput::print(serde_json::to_value(info).unwrap());
                return Ok(());
            }
            DeviceCommands::Prop { .. } => {
                return Err(tke::TkeError::InvalidArgument(
                    "device prop 仅支持 Android 设备 (adb getprop)".to_string(),
                ));
            }
            DeviceCommands::List { .. } => unreachable!("函数开头已处理"),
        }
    }

    let device_manager = DeviceManager::new(device_id)?;

    match action {
        DeviceCommands::Info => {
            let device_info = device_manager.get_full_device_info()?;
            JsonOutput::print(serde_json::to_value(device_info).unwrap());
        }
        DeviceCommands::List { .. } => unreachable!("函数开头已处理"),
        DeviceCommands::Prop { name } => {
            let value = device_manager.get_device_prop(&name)?;
            JsonOutput::print(serde_json::json!({
                "success": true,
                "property": name,
                "value": value
            }));
        }
    }

    Ok(())
}

/// `tke device list`
fn list(all: bool) -> Result<()> {
    let d = tke::tools::discover::discover_with(all);

    // 管道里给 JSON（脚本/AI 好接），终端里给对齐的表（人好读）
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        JsonOutput::print(serde_json::to_value(&d).unwrap());
        return Ok(());
    }

    // 只对**ASCII 的前两列**做对齐。中文一律靠后、不参与列宽——
    // `{:<w$}` 按**字符数**填充，而中文在终端里占两格，混进来必然错位（TUI 那次的老账）
    let w_id = d.targets.iter().map(|t| t.id.len()).max().unwrap_or(3).max(3);
    // 中文类别按**显示宽度**算（一个汉字占两格），不能按字符数——
    // `{:<w$}` 是按字符数填充的，混着算必然错位
    let w_kind = d.targets.iter().map(|t| t.kind_label.chars().count() * 2).max().unwrap_or(8);

    println!();
    if d.targets.is_empty() {
        println!("  {}", dim("一个可测目标都没有"));
    }
    for t in &d.targets {
        // 类别列自己补空格（中文占两格，交给 `{:<w$}` 会歪）
        let pad = " ".repeat(w_kind.saturating_sub(t.kind_label.chars().count() * 2));
        println!(
            "  {:<w_id$}  {}{}  {}  {}",
            t.id, t.kind_label, pad, t.name, dim(&t.state),
        );
    }

    // 没查成的单独一段。**混进上面那张表是不行的**：那会读成"这些是设备"，
    // 而它们恰恰是"这类根本没查"
    if !d.skipped.is_empty() {
        println!();
        for s in &d.skipped {
            println!("  {} {}", dim(&format!("未检测 {}", s.kind)), dim(&s.why));
        }
    }
    println!("\n  {}", dim("第一列就是 -d 要填的值"));
    Ok(())
}

fn dim(s: &str) -> String {
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        format!("\x1b[38;5;245m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}
