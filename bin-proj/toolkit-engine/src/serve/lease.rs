// 【租约】INV-17 的执行点：一个 session = 一份设备独占租约 + 一个隔离目录。
//
// 为什么要"独占"：这是"租赁"这件事的技术实体——平台按时长计费的那台设备，
// 同一时刻只能有一个人在用。为什么要"隔离目录"：P-10（同秒共享工作区互相覆盖）
// 在本地靠调用方自觉，远程不能靠自觉，所以 `--log`/`--cache`/`--current-dir`
// 一律由服务端注入到会话目录，调用方连指定的机会都没有（见 `allowlist.rs` 的禁用旗标）。
//
// 目录布局（`logs` 放在 workspace 里是有意的）：
//   <root>/sessions/<sid>/
//   ├── workspace/        进程 cwd；上传落这里；产物下载也从这里读
//   │   └── logs/         `--log` 指这里 —— 于是调用方可以直接 `tke report logs`
//   └── cache/            `--cache`：运行中间产物，不给下载
// 证据要能被下一条命令按**相对路径**引用，所以它必须在 cwd 树内——
// 否则调用方只能写绝对路径，而绝对路径正是我们禁掉的东西。
//
// 复位（INV-17）：本地从不需要这条（机器是你自己的），租赁模式下不复位
// = 下一个租户接手一台登录着的浏览器。复位**只出计划不执行**（`ResetPlan`），
// 执行在 routes 那层——这样这里保持纯逻辑，可以无设备单测。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 池里的一台设备
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolDevice {
    /// `-d` 直接填这个
    pub id: String,
    /// android / android-avd / ios / ios-sim / web / fake
    pub kind: String,
    /// 给人看的名字
    pub label: String,
    /// 机型 / 系统 —— 分开留着而不是拼进 label：
    /// 平台的设备表要按机型筛、按系统排，拼成一个串就切不开了
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub os: String,
}

impl PoolDevice {
    /// 归一到 `capabilities.platform` 那四个值
    /// 这是**一台实物**吗。
    ///
    /// 平台据此决定"这次没报上来"意味着什么：
    ///   实物（插着的手机、连着的 iPhone）→ 可能只是拔了线，标离线留着，
    ///     人看得出这台机器上平时有什么
    ///   非实物（web 槽、AVD、模拟器）→ **就是没有了**：调小 --web-slots、
    ///     删掉一个 AVD、关掉模拟器，那东西不存在，显示成"离线"是误导
    ///
    /// **由节点判定，不让平台猜**：booted 的安卓模拟器经 adb 报上来时
    /// kind 就是 "android"（跟真机一样），只有序列号 `emulator-` 前缀能区分 ——
    /// 让平台去认这个前缀，等于把 adb 的实现细节搬到另一个仓库里
    pub fn physical(&self) -> bool {
        match self.kind.as_str() {
            "web" | "android-avd" | "ios-sim" | "none" | "fake" => false,
            // adb 把 booted 的模拟器和真机都报成 android，序列号前缀是唯一的分界
            "android" => !self.id.starts_with("emulator-"),
            "ios" => true,
            _ => false,
        }
    }

    pub fn platform(&self) -> &str {
        match self.kind.as_str() {
            "android" | "android-avd" => "android",
            "ios" | "ios-sim" => "ios",
            "web" => "web",
            // 无设备会话：安全轨的 http/recon 只打 URL，不碰设备。
            // 让它也去租一台手机 = 让用户为没用到的设备付租金（计费模型见 ADR-0022 D3）
            "none" => "none",
            _ => "fake",
        }
    }
}

/// 会话目录三件套
#[derive(Debug, Clone)]
pub struct SessionDirs {
    pub root: PathBuf,
    pub workspace: PathBuf,
    pub logs: PathBuf,
    pub cache: PathBuf,
}

impl SessionDirs {
    fn under(root: &Path, sid: &str) -> Self {
        let root = root.join("sessions").join(sid);
        let workspace = root.join("workspace");
        Self {
            logs: workspace.join("logs"),
            cache: root.join("cache"),
            workspace,
            root,
        }
    }

    fn create(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.logs)?;
        std::fs::create_dir_all(&self.cache)?;
        Ok(())
    }
}

/// 一份活着的租约
#[derive(Debug, Clone)]
pub struct Lease {
    pub id: String,
    pub device: PoolDevice,
    pub dirs: SessionDirs,
    pub created_at: u64,
    pub expires_at: u64,
    /// 调用方带来的归账标签，原样回传（设备租赁计费靠它归到 App/用户名下）
    pub meta: Option<serde_json::Value>,
    /// 这次会话里启动过的 App 包名——释放时要挨个停掉（复位的依据来自事实，不靠猜）
    pub launched_apps: Vec<String>,
}

impl Lease {
    /// 复位计划：**只出计划不执行**，执行在 routes 层（保持本模块可无设备单测）
    pub fn reset_plan(&self) -> ResetPlan {
        let mut actions: Vec<Vec<String>> = Vec::new();
        match self.device.platform() {
            // 浏览器：关掉会话，否则下一个租户接手的是上一个人登录着的页面
            "web" => actions.push(vec!["control".into(), "shutdown".into()]),
            "android" | "ios" => {
                for pkg in &self.launched_apps {
                    actions.push(vec!["app".into(), "stop".into(), pkg.clone()]);
                }
            }
            _ => {}
        }
        ResetPlan { actions }
    }
}

/// 复位计划（一组 argv，按顺序执行；失败不阻断后续——尽力而为，但结果要如实回报）
#[derive(Debug, PartialEq)]
pub struct ResetPlan {
    pub actions: Vec<Vec<String>>,
}

#[derive(Debug, PartialEq)]
pub enum AcquireError {
    /// 池里就没有这一类设备
    NoSuchDevice(String),
    /// 有，但都被别人租着
    AllBusy(String),
}

impl std::fmt::Display for AcquireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // "没有"和"忙"必须分开说：一个该换节点，一个该等一等（INV-9 的精神：查不了要说出来）
            Self::NoSuchDevice(p) => write!(f, "本节点没有 {p} 设备可租。"),
            Self::AllBusy(p) => write!(f, "{p} 设备都在租用中，稍后再试或换一个节点。"),
        }
    }
}

/// 租约表。`pool` 由外面装配（真实设备来自 `tools::discover`，测试直接塞假的）
pub struct LeaseTable {
    root: PathBuf,
    pool: Mutex<Vec<PoolDevice>>,
    leases: Mutex<HashMap<String, Lease>>,
    default_ttl: Duration,
    seq: AtomicU64,
}

impl LeaseTable {
    pub fn new(root: PathBuf, pool: Vec<PoolDevice>, default_ttl: Duration) -> Self {
        Self {
            root,
            pool: Mutex::new(pool),
            leases: Mutex::new(HashMap::new()),
            default_ttl,
            seq: AtomicU64::new(0),
        }
    }

    pub fn pool(&self) -> Vec<PoolDevice> {
        self.pool.lock().expect("pool 锁中毒").clone()
    }

    /// 换一批设备（真机插拔后刷新）
    pub fn set_pool(&self, pool: Vec<PoolDevice>) {
        *self.pool.lock().expect("pool 锁中毒") = pool;
    }

    /// 这台设备现在被谁租着
    pub fn holder_of(&self, device_id: &str) -> Option<String> {
        self.leases
            .lock()
            .expect("leases 锁中毒")
            .values()
            .find(|l| l.device.id == device_id)
            .map(|l| l.id.clone())
    }

    /// 租一台：`platform` 二选一地给，或直接点名 `device_id`
    pub fn acquire(
        &self,
        platform: Option<&str>,
        device_id: Option<&str>,
        ttl: Option<Duration>,
    ) -> Result<Lease, AcquireError> {
        let want = platform.unwrap_or("any");
        // 无设备会话：不占池、不互斥、不复位——它只要一个隔离的工作区
        if want == "none" && device_id.is_none() {
            return Ok(self.insert_lease(
                PoolDevice { id: String::new(), kind: "none".into(), label: "无设备（只用工作区）".into(), model: String::new(), os: String::new() },
                ttl,
            ));
        }
        let pool = self.pool.lock().expect("pool 锁中毒").clone();
        let candidates: Vec<PoolDevice> = pool
            .into_iter()
            .filter(|d| match device_id {
                Some(id) => d.id == id,
                None => platform.is_none() || d.platform() == want,
            })
            .collect();
        if candidates.is_empty() {
            return Err(AcquireError::NoSuchDevice(device_id.unwrap_or(want).to_string()));
        }

        let leases = self.leases.lock().expect("leases 锁中毒");
        // **过期租约照样占着设备**，直到 sweep 把它复位掉——这是 INV-17 的要求：
        // 在这里顺手 retain 掉它们，设备就绕过复位直接给了下一个租户
        // （单测「sweep交出过期的那批」当初就是这么逼出来的）。
        // 代价是 TTL 到期后最多再等一轮清扫（15s）设备才回池，值得
        let free = candidates
            .into_iter()
            .find(|d| !leases.values().any(|l| l.device.id == d.id));
        let device = match free {
            Some(d) => d,
            None => return Err(AcquireError::AllBusy(device_id.unwrap_or(want).to_string())),
        };

        drop(leases);
        Ok(self.insert_lease(device, ttl))
    }

    fn insert_lease(&self, device: PoolDevice, ttl: Option<Duration>) -> Lease {
        let sid = self.next_id();
        let dirs = SessionDirs::under(&self.root, &sid);
        // 建目录失败当成"没有设备可租"处理不合适——直接 panic 也不合适，
        // 交给上层：这里只在内存里建租约，目录建不出来 routes 会回 500
        let _ = dirs.create();
        let ttl = ttl.unwrap_or(self.default_ttl);
        let now = now_secs();
        let lease = Lease {
            id: sid.clone(),
            device,
            dirs,
            created_at: now,
            expires_at: now + ttl.as_secs(),
            launched_apps: Vec::new(),
            meta: None,
        };
        self.leases.lock().expect("leases 锁中毒").insert(sid, lease.clone());
        lease
    }

    pub fn get(&self, sid: &str) -> Option<Lease> {
        let leases = self.leases.lock().expect("leases 锁中毒");
        leases.get(sid).filter(|l| l.expires_at > now_secs()).cloned()
    }

    /// 续租；返回新的到期时刻
    pub fn heartbeat(&self, sid: &str, ttl: Option<Duration>) -> Option<u64> {
        let mut leases = self.leases.lock().expect("leases 锁中毒");
        let ttl = ttl.unwrap_or(self.default_ttl);
        let now = now_secs();
        let l = leases.get_mut(sid)?;
        if l.expires_at <= now {
            return None;
        }
        l.expires_at = now + ttl.as_secs();
        Some(l.expires_at)
    }

    /// 挂上归账标签（建租约时调用方给的）
    pub fn set_meta(&self, sid: &str, meta: Option<serde_json::Value>) {
        if meta.is_none() {
            return;
        }
        if let Some(l) = self.leases.lock().expect("leases 锁中毒").get_mut(sid) {
            l.meta = meta;
        }
    }

    /// 记下这次会话启动过的 App（复位时要停掉）。从 exec 的 argv 里认，不靠调用方申报
    pub fn note_launch(&self, sid: &str, argv: &[String]) {
        let Some(pkg) = launched_package(argv) else { return };
        let mut leases = self.leases.lock().expect("leases 锁中毒");
        if let Some(l) = leases.get_mut(sid) {
            if !l.launched_apps.contains(&pkg) {
                l.launched_apps.push(pkg);
            }
        }
    }

    /// 摘下租约（复位由调用方按 `reset_plan()` 执行）
    pub fn take(&self, sid: &str) -> Option<Lease> {
        self.leases.lock().expect("leases 锁中毒").remove(sid)
    }

    /// 清掉过期租约，返回它们（调用方负责复位）
    pub fn sweep(&self) -> Vec<Lease> {
        let now = now_secs();
        let mut leases = self.leases.lock().expect("leases 锁中毒");
        let expired: Vec<String> = leases
            .iter()
            .filter(|(_, l)| l.expires_at <= now)
            .map(|(k, _)| k.clone())
            .collect();
        expired.iter().filter_map(|k| leases.remove(k)).collect()
    }

    pub fn active(&self) -> Vec<Lease> {
        let now = now_secs();
        self.leases
            .lock()
            .expect("leases 锁中毒")
            .values()
            .filter(|l| l.expires_at > now)
            .cloned()
            .collect()
    }

    fn next_id(&self) -> String {
        // 不是密码（鉴权靠 Bearer token），只要够唯一：时间 + 自增
        let n = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("s{:x}{:03x}", now_secs(), n & 0xfff)
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// 从 argv 认出"启动了哪个 App"：`app launch <pkg>` / `control launch <pkg>`
fn launched_package(argv: &[String]) -> Option<String> {
    let a: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
    match a.as_slice() {
        ["app", "launch", pkg, ..] | ["control", "launch", pkg, ..] => {
            (!pkg.starts_with('-')).then(|| pkg.to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, kind: &str) -> PoolDevice {
        PoolDevice { id: id.into(), kind: kind.into(), label: id.into(), model: String::new(), os: String::new() }
    }

    fn table(pool: Vec<PoolDevice>) -> (LeaseTable, tempdir::Tmp) {
        let tmp = tempdir::Tmp::new("lease");
        (LeaseTable::new(tmp.path().to_path_buf(), pool, Duration::from_secs(600)), tmp)
    }

    #[test]
    fn 一台设备同时只能有一份租约() {
        let (t, _tmp) = table(vec![dev("web:1", "web")]);
        let a = t.acquire(Some("web"), None, None).unwrap();
        // 第二个人来租同一类，池里没有空的了 → 是"忙"，不是"没有"
        assert_eq!(t.acquire(Some("web"), None, None).unwrap_err(), AcquireError::AllBusy("web".into()));
        // 还回去之后立刻能再租
        t.take(&a.id).unwrap();
        assert!(t.acquire(Some("web"), None, None).is_ok());
    }

    #[test]
    fn 没有那类设备与都在忙要分开说() {
        let (t, _tmp) = table(vec![dev("web:1", "web")]);
        // 这条区别很重要：一个该换节点，一个该等一等
        assert_eq!(t.acquire(Some("ios"), None, None).unwrap_err(), AcquireError::NoSuchDevice("ios".into()));
    }

    #[test]
    fn 过期租约取不出来但设备要等复位后才回池() {
        let (t, _tmp) = table(vec![dev("web:1", "web")]);
        let l = t.acquire(Some("web"), None, Some(Duration::from_secs(0))).unwrap();
        assert!(t.get(&l.id).is_none(), "过期了就不该还能取到");
        assert!(t.heartbeat(&l.id, None).is_none(), "过期的续不了——否则等于永不过期");
        // **关键**：过期不等于立刻回池。设备要先被 sweep 复位（INV-17），
        // 否则下一个租户接手的是上一个人登录着的浏览器
        assert_eq!(t.acquire(Some("web"), None, None).unwrap_err(), AcquireError::AllBusy("web".into()));
        assert_eq!(t.sweep().len(), 1);
        assert!(t.acquire(Some("web"), None, None).is_ok(), "复位过后才该回池");
    }

    #[test]
    fn 心跳能续命() {
        let (t, _tmp) = table(vec![dev("web:1", "web")]);
        let l = t.acquire(Some("web"), None, Some(Duration::from_secs(1))).unwrap();
        let new_exp = t.heartbeat(&l.id, Some(Duration::from_secs(600))).unwrap();
        assert!(new_exp > l.expires_at);
        assert!(t.get(&l.id).is_some());
    }

    #[test]
    fn 点名租某一台() {
        let (t, _tmp) = table(vec![dev("web:1", "web"), dev("web:2", "web")]);
        let l = t.acquire(None, Some("web:2"), None).unwrap();
        assert_eq!(l.device.id, "web:2");
        assert_eq!(t.holder_of("web:2"), Some(l.id.clone()));
        assert_eq!(t.holder_of("web:1"), None);
    }

    #[test]
    fn 会话目录建在会话内且logs在工作区里() {
        let (t, tmp) = table(vec![dev("web:1", "web")]);
        let l = t.acquire(Some("web"), None, None).unwrap();
        assert!(l.dirs.logs.starts_with(&l.dirs.workspace), "logs 必须在 cwd 树内，否则只能用绝对路径引用它");
        assert!(l.dirs.workspace.exists() && l.dirs.cache.exists());
        assert!(l.dirs.root.starts_with(tmp.path()));
    }

    #[test]
    fn 无设备会话不占池不互斥() {
        let (t, _tmp) = table(vec![dev("web:1", "web")]);
        // 安全轨的 http/recon 只打 URL。让它去租一台手机 = 让用户为没用到的设备付租金
        let a = t.acquire(Some("none"), None, None).unwrap();
        let b = t.acquire(Some("none"), None, None).unwrap();
        assert_ne!(a.id, b.id, "无设备会话之间不互斥");
        assert!(a.device.id.is_empty() && a.device.platform() == "none");
        assert!(a.reset_plan().actions.is_empty(), "没设备就没什么好复位的");
        // 真设备一台没少
        assert!(t.acquire(Some("web"), None, None).is_ok());
        assert!(a.dirs.workspace.exists(), "工作区照样要有——证据得有地方落");
    }

    #[test]
    fn 复位计划按平台出() {
        let (t, _tmp) = table(vec![dev("web:1", "web"), dev("emu-5554", "android")]);
        let web = t.acquire(Some("web"), None, None).unwrap();
        assert_eq!(web.reset_plan().actions, vec![vec!["control".to_string(), "shutdown".into()]]);

        let mut a = t.acquire(Some("android"), None, None).unwrap();
        // 没启动过 App 就没什么好停的——不要为了"有个动作"而乱敲设备
        assert!(a.reset_plan().actions.is_empty());
        a.launched_apps = vec!["com.demo".into()];
        assert_eq!(a.reset_plan().actions, vec![vec!["app".to_string(), "stop".into(), "com.demo".into()]]);
    }

    #[test]
    fn 启动过的app从argv里认出来() {
        let (t, _tmp) = table(vec![dev("emu-5554", "android")]);
        let l = t.acquire(Some("android"), None, None).unwrap();
        t.note_launch(&l.id, &["app".into(), "launch".into(), "com.demo".into()]);
        t.note_launch(&l.id, &["app".into(), "launch".into(), "com.demo".into()]); // 重复不叠加
        t.note_launch(&l.id, &["control".into(), "launch".into(), "com.other".into()]);
        t.note_launch(&l.id, &["fetch".into()]);
        assert_eq!(t.get(&l.id).unwrap().launched_apps, vec!["com.demo", "com.other"]);
    }

    #[test]
    fn sweep交出过期的那批() {
        let (t, _tmp) = table(vec![dev("web:1", "web"), dev("web:2", "web")]);
        let dead = t.acquire(None, Some("web:1"), Some(Duration::from_secs(0))).unwrap();
        let alive = t.acquire(None, Some("web:2"), None).unwrap();
        let swept = t.sweep();
        assert_eq!(swept.len(), 1);
        assert_eq!(swept[0].id, dead.id);
        assert_eq!(t.active().len(), 1);
        assert_eq!(t.active()[0].id, alive.id);
    }

    /// 极简临时目录（不引第三方 crate：只为测试造几个目录，用完删）
    pub mod tempdir {
        use std::path::{Path, PathBuf};
        pub struct Tmp(PathBuf);
        impl Tmp {
            pub fn new(tag: &str) -> Self {
                let p = std::env::temp_dir().join(format!(
                    "tke-test-{tag}-{}-{:?}",
                    std::process::id(),
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                ));
                std::fs::create_dir_all(&p).unwrap();
                Self(p)
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
        }
        impl Drop for Tmp {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
}

#[cfg(test)]
mod physical_tests {
    use super::PoolDevice;

    fn dev(id: &str, kind: &str) -> PoolDevice {
        PoolDevice { id: id.into(), kind: kind.into(), label: String::new(),
                     model: String::new(), os: String::new() }
    }

    /// 实物：拔了线只是"离线"，平台要留着记录
    #[test]
    fn 真机是实物() {
        assert!(dev("f64b3b4d", "android").physical());
        assert!(dev("00008030-001", "ios").physical());
    }

    /// 非实物：不报上来就是**没有了**，平台该删掉而不是显示成离线。
    /// 调小 --web-slots 之后 web:3 显示成"离线"是误导（用户实测反馈）
    #[test]
    fn 槽位与模拟器不是实物() {
        assert!(!dev("web:3", "web").physical());
        assert!(!dev("avd:tke", "android-avd").physical());
        assert!(!dev("sim:ABC", "ios-sim").physical());
    }

    /// **booted 的安卓模拟器经 adb 报上来时 kind 就是 android**，跟真机一样，
    /// 只有 `emulator-` 前缀能区分。这个判断放在节点这边，
    /// 别让平台去认 adb 的命名习惯
    #[test]
    fn 起来的模拟器不算实物() {
        assert!(!dev("emulator-5554", "android").physical());
        assert!(dev("emulator5554", "android").physical(), "少了连字符就不是 adb 的模拟器命名");
    }
}
