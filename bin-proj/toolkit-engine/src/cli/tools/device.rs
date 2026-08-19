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
        return list(all, wants_json(&params));
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
                print_info(&info, wants_json(&params));
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
            print_info(&device_info, wants_json(&params));
        }
        DeviceCommands::List { .. } => unreachable!("函数开头已处理"),
        DeviceCommands::Prop { name } => {
            let value = device_manager.get_device_prop(&name)?;
            // ⚠️ **空值 = 没有这个属性**。adb getprop 对不存在的键回一个空行、
            // 退出码还是 0，于是原来会报 `{"success":true,"value":""}`——
            // 看起来像"查到了，值是空"（实测用户拿包名当属性名查，就撞上了）
            if value.trim().is_empty() {
                JsonOutput::error(format!(
                    "没有 `{}` 这个属性（`adb getprop` 对不存在的键回空值）。\n\u{3000}                     属性名长这样：ro.build.version.release / ro.product.model",
                    name
                ));
            }
            if !wants_json(&params) {
                // 终端里**只打值**：多半是要拿去接着用的，裹一层 JSON 只是碍事
                println!("{}", value.trim());
            } else {
                JsonOutput::print(serde_json::json!({
                    "success": true,
                    "property": name,
                    "value": value.trim()
                }));
            }
        }
    }

    Ok(())
}

/// `tke device list`
fn list(all: bool, as_json: bool) -> Result<()> {
    let d = tke::tools::discover::discover_with(all);

    // 管道 / `--json` 给 JSON（脚本/AI 好接），终端里给表（人好读）
    if as_json {
        JsonOutput::print(serde_json::to_value(&d).unwrap());
        return Ok(());
    }

    use tke::utils::text::{disp_width, pad_right};
    // 四列都**按显示宽度**对齐。`{:<w$}` 是按字符数填的，中文占两格——
    // 混着排必然错位（`CPH2305` 和 `iPhone 17 Pro` 那两行就是这么歪的）
    let w = |f: fn(&tke::tools::discover::Target) -> &str, head: &str| {
        d.targets.iter().map(|t| disp_width(f(t))).chain([disp_width(head)]).max().unwrap_or(2)
    };
    let (w_id, w_os, w_model) = (
        w(|t| &t.id, "ID"),
        w(|t| &t.os, "系统"),
        w(|t| &t.model, "型号"),
    );

    println!();
    println!(
        "  {}",
        dim(&format!(
            "{}  {}  {}  状态",
            pad_right("ID", w_id),
            pad_right("系统", w_os),
            pad_right("型号", w_model)
        ))
    );
    if d.targets.is_empty() {
        println!("  {}", dim("一个可测目标都没有"));
    }
    for t in &d.targets {
        let row = format!(
            "{}  {}  {}  {}",
            pad_right(&t.id, w_id),
            pad_right(&t.os, w_os),
            pad_right(&t.model, w_model),
            t.state
        );
        // 用不了的整行置灰（没启动的模拟器、离线的安卓）——**别让人去选一个选了必然失败的**；
        // 能用的按平台配色，扫一眼就能分出这是哪一类
        println!("  {}", if t.ready { paint(t.kind, &row) } else { dim(&row) });
    }

    // 没查成的单独一段。**混进上面那张表是不行的**：那会读成"这些是设备"，
    // 而它们恰恰是"这类根本没查"。
    // 措辞压到最短——CLI 不是教程，每行只放「事实 + 下一步」，不解释为什么
    if !d.skipped.is_empty() {
        println!();
        for s in &d.skipped {
            println!("  {}", dim(&s.why));
        }
    }
    Ok(())
}

/// 按平台上色：安卓绿、苹果蓝、网页黄。
/// 一眼分类比读那一列文字快——这也是为什么类别不再单独占一列
fn paint(kind: &str, s: &str) -> String {
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return s.to_string();
    }
    let color = match kind {
        "android" => "38;5;114",       // 安卓绿
        "ios" | "ios-sim" => "38;5;75", // 苹果蓝
        _ => "38;5;179",               // 网页黄
    };
    format!("\x1b[{}m{}\x1b[0m", color, s)
}

fn dim(s: &str) -> String {
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        format!("\x1b[38;5;245m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

/// 要不要输出 JSON。**两个条件任一成立**：显式 `--json`，或者输出被重定向/管道接走了
/// （那头多半是脚本或另一个程序，给它人类排版没有意义）
fn wants_json(params: &tke::Params) -> bool {
    params.json || !std::io::IsTerminal::is_terminal(&std::io::stdout())
}

/// 设备详情：终端给人看的排版，管道/`--json` 给 JSON（脚本/AI 好接）
fn print_info(info: &tke::DeviceInfo, as_json: bool) {
    if as_json {
        JsonOutput::print(serde_json::to_value(info).unwrap());
        return;
    }
    use tke::utils::text::pad_right;
    let kv = |k: &str, v: &str| {
        if !v.is_empty() && v != "—" {
            println!("  {} {}", dim(&pad_right(k, 10)), v);
        }
    };
    println!();
    kv("设备", &info.id);
    kv("型号", info.model.as_deref().unwrap_or(""));
    kv("厂商", info.manufacturer.as_deref().unwrap_or(""));
    kv("系统", info.android_version.as_deref().unwrap_or(""));
    if info.screen_width > 0 {
        kv("屏幕", &format!("{} × {}", info.screen_width, info.screen_height));
    }
    // 这些字段全是 Option：**查不到就整行不打**，别拿 0 或空字符串占位
    // （"0 核"、"电池 0%" 比不显示更误导）
    if let Some(h) = &info.hardware {
        if let (Some(c), Some(abi)) = (h.cpu_cores, h.cpu_abi.as_deref()) {
            kv("CPU", &format!("{} 核 · {}", c, abi));
        }
        if let (Some(a), Some(t)) = (h.available_memory_mb, h.total_memory_mb) {
            kv("内存", &format!("{} / {} MB 可用", a, t));
        }
        if let (Some(a), Some(t)) = (h.available_storage_gb, h.total_storage_gb) {
            kv("存储", &format!("{:.1} / {:.1} GB 可用", a, t));
        }
    }
    if let Some(b) = &info.battery {
        if let Some(l) = b.level {
            let temp = b.temperature.map(|t| format!(" · {:.1}°C", t)).unwrap_or_default();
            let st = b.status.as_deref().map(|s| format!(" · {}", s)).unwrap_or_default();
            kv("电池", &format!("{}%{}{}", l, temp, st));
        }
    }
    if let Some(n) = &info.network {
        if n.wifi_enabled == Some(true) {
            kv("网络", &format!("Wi-Fi {}", n.wifi_ssid.as_deref().unwrap_or("已开")));
        }
    }
    println!();
}
