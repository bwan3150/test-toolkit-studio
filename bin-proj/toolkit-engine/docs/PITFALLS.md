# 踩坑记录（PITFALLS）

**只追加，不删除。** 每条：现象 → 根因 → 规则。重复踩过的坑升级进 `INVARIANTS.md`。
新会话动手前扫一遍标题即可，改到相关区域再细读。

---

## P-01 ratatui inline viewport 对 CJK 不可用

**现象**：TUI 中文全被打成「选 择 设 备」。
**根因**：`insert_before` 逐 cell 直喂 backend，宽字符续格跳过逻辑只在 `Buffer::diff` 里。
**规则**：TUI 历史行走手写 inline（print 进 scrollback，终端原生处理宽字符），ratatui 只借 text/style 类型。别再回去试（07231866）。

## P-02 macOS 覆盖二进制 → Killed: 9

**现象**：`build-mac.sh` 后新二进制一跑就 Killed: 9。
**根因**：`cp` 原地覆盖，macOS AMFI 按 inode 缓存签名。
**规则**：先 `rm -f` 再拷 + `codesign` ad-hoc 重签（4e401f8e 已进脚本）。这也是「不用 cargo build 产二进制」红线的一部分。

## P-03 adb uiautomator dump 无限挂 + 同步调用堵死 tokio

**现象**：真机整个 harness 卡死。
**根因**：`uiautomator dump` 等 idle 可以无限等；同步 std 调用又堵死 tokio worker。
**规则**：设备链路全环节必须带超时；async 环境不裸调同步阻塞 IO。wda/web 同类风险没查过（还欠着）。

## P-04 genai 丢思考块 → anthropic 400

**现象**：开 reasoning 后 anthropic 报 400。
**根因**：genai 不回传思考块，anthropic 旧模型要求思考块随历史回传。
**规则**：anthropic 必须 claude-sonnet-4-6+（adaptive thinking），4-5 会 400。

## P-05 提示词漏登记 → 静默发空串

**现象**：某角色行为怪异极难排查。
**根因**：builtin/*.md 建了但 defaults.rs 没登记 include_str，运行时解析为空、静默发给 LLM。
**规则**：运行时守卫 `guard_nonempty` 已兜底（debug 直接断言炸）；提交期由 `scripts/check-prompts.sh` 查登记一致性。

## P-06 fake::node 全是 Button，改版页同 class 会"意外命中"

**现象**：自愈测试全绿但 heal 根本没触发。
**根因**：库条目结构通道含 class_name；fake 改版页节点同为 Button，recognizer 的 class_name 结构容错直接命中——逼不出失败路径。
**规则**：fake 测试里"改版页"的节点 class 必须 ≠ 库条目（手写 TextView 等）。更早版 heal 测试踩过同一坑（测试注释里有警告，要看）。

## P-07 增强层把可选前提当硬前提 → 静默失效

**现象**：用户不带 -d 跑，起始对齐/自愈全程没生效，且零线索（"为啥没先导航就报错了"）。
**根因**：宿主命令容忍 device=None（adb 默认设备），增强层却 `let Some(device) = ... else return Skip`。
**规则**：= INV-9。增强层前提容忍度 ≤ 宿主；被环境跳过必须 emit Warn。

## P-08 步硬超时掐死步内 LLM 调用

**现象**：自愈还没等到 LLM 回复，步就被 20s 超时判死。
**根因**：heal 在 `interpret_step` 的超时作用域内。
**规则**：healer 在场时非滚动查找步超时 20s→60s。同类:滚动查找是 30 次循环，20s 必然误杀（已 75s）——**给"内部含循环/网络调用"的步定超时前，先算它的真实预算**。

## P-09 收尾关闭销毁校验证据

**现象**：探索明明达成，终点判定却失败。
**根因**：用例结尾"关闭浏览器/退出 App"把校验要看的页面销毁了。
**规则**：= INV-11。终点校验在收尾前完成。

## P-10 同秒并行回放共享工作区互相覆盖

**现象**：测试 flaky，截图/XML 张冠李戴。
**根因**：`Workarea::temp_for_run` 目录名 = 秒级时间戳+pid，同进程同秒并行撞名。
**规则**：唯一性要进程内自增序号兜底，别信时间戳（1f6d49b1）。

## P-11 按 text 撞同名 → 回放点错元素、断言假阳性

**现象**：多个「Sign In」点歪到不导航的那个；断言在错误页假通过。
**根因**：text 多命中取第一个。
**规则**：Locator 带 anchor（仅 tiebreak 绝不作主定位坐标）；断言挑"该页专属"标志（ae7429d7）。web 还需唯一 DOM 路径（512400b3）。

## P-12 守卫脚本可能"假绿"

**现象**：（splat-bot 移植教训）守卫从不报警，其实是自己的输出把自己的违规过滤掉了。
**规则**：**新写/改守卫脚本必须先造一个故意违规的现场，验证它真的会报错**。全绿的守卫可能只是没在工作。

## P-13 Controller::new 吃 Option\<String\>

**现象**：`Controller::new(device.clone())` 编译错。
**规则**：设备参数在 drivers 层全链是 `Option<String>`（None=默认设备），别包 Some 之前先看签名。

## P-14 探索终态 ≠ 回放起点

**现象**：刚探索完就回放,从终态"闭眼开跑"越修越卡。
**根因**：探索结束设备停在完成页,脚本却假设从起始页开始。
**规则**：回放前对齐起始态（现由 `tke run` 起始态对齐 / 编排官 navigate 承担）；轻模式脚本也要落标志（5ea793cd 修过 gate bug）。

## P-15 env_clear 清掉 DISPLAY → Linux 有头模式 Chrome 起不来

`web/infra.rs` 拉起 chromedriver 时 `env_clear()` 只保留 PATH/HOME/USER/LOGNAME/TMPDIR/LANG
（为治终端模拟器注入环境导致 Chrome 崩溃——Ghostty 下的 `Mach rendezvous failed` / `BUS_ADRALN`，
见 setup-notes）。mac/win 不看这些变量所以一直没暴露，但 **Linux 有头模式下
Chrome 靠 DISPLAY/WAYLAND_DISPLAY/XAUTHORITY 连图形栈**，被清掉就直接起不来。
已把这三个加进保留列表（无头模式下有没有都不影响）。

**教训**:环境变量白名单是平台相关的——在一个平台上"够用"的白名单，换平台就是缺失。

## P-16 `num_args = 0..=1` 的可选值参数会吞掉后面的子命令

`--copilot` 踩过一次,`--headless` 又踩一次:clap 里 `num_args = 0..=1` 的参数,
`tke --headless run x.tks` 会把 **`run` 当成 `--headless` 的值**吃掉,子命令就没了。
解法:加 `require_equals = true`(强制 `--headless=on` 形式)+ `value_parser` 白名单。
裸 `--headless` 仍可用(走 `default_missing_value`)。

**这类坑只有黑盒 CLI 契约测试逮得到**(单测测不到 clap 装配层)——
`tests/cli.rs` 已有回归用例 `headless_bare_flag_does_not_swallow_subcommand`。

## P-17 能力只在一条路径上完整——`element add --lib foo.tklib` 建不了新包

`.tklib` 此前**只有 harness finalize 会创建**。改用原子命令直接攒两件套时（skill 模式，
ADR-0010），第一次 `tke element add --lib foo.tklib` 必然失败:「打不开元素包…No such file」——
因为包还不存在,而落库路径只会解包、不会建包。裸 `element.json` 反倒一直支持首次自建。

已修:包不存在 → 跳过解包,直接落库再打包（走与裸 json 首次落库同一条路）。

**教训**:一个能力"能用"往往只是**在主路径上能用**。新增调用路径（本例:原子命令替代
AI 编排）会立刻暴露那些藏在编排层里的隐含前置。**这类缺口只有真正跑一遍才会现形**——
本条就是 skill 原型实测第一步撞出来的。

## P-18 会话跨命令复用 → `--headless` 静默不生效

web 会话存 `$TMPDIR/tke/web/<设备>.json` 跨命令复用,而 `--headless` **只在建会话时**起作用。
于是 `tke -d web refresh`(有头建会话)之后再 `tke --headless=on -d web refresh`,
**沿用的还是那个有头会话**——参数看似接受、实则毫无效果,且零提示。

用户实测对照时正是这样撞上的:两次截图尺寸/XML 完全一致,差点被当成"两种模式渲染一致"的
证据(**假阳性**)。真正的对照必须先销毁会话:`tke -d web control close`（web 省略包名即销毁会话）。

已修:`SessionInfo` 记 `headless` 字段,复用前比对——模式不符则销毁旧会话 + 明确报错要求
重新 launch(不静默沿用,INV-9)。

**教训**:**带持久状态的开关,必须把"状态是用什么开关建的"一起持久化**,否则开关与状态会
悄悄失配。这类 bug 不会报错,只会让你得到一个看似合理的错误结论。

## P-19 分发源两个坑：SPA 兜底 200 + Cloudflare 缓存不认 no-cache

自建存储平台（Toolkit Cloud）发分发包时连撞两个：

**① 不存在的路径返回 200 + 前端 HTML**（SPA 兜底），不是 404。
`curl -f` 完全拦不住——它只对 4xx/5xx 生效。漏传一个文件，安装器会把那段 HTML
当成 `tke` 二进制存下来，装完才发现是垃圾。
→ 解法:`install.sh` **逐个校验文件头**（gzip 的 `1f8b` / zip 的 `PK` / 版本号以 `tke ` 开头），
不合格一律当下载失败。**别只信状态码**。

**② Cloudflare 缓存 4 小时，且不认 `Cache-Control: no-cache` 请求头。**
传了新文件，使用者下到的还是旧的——正是"skill 永远停在旧版本"的根因之一。
实测唯一可靠的破缓存手段是**变化的查询参数**。
→ 解法:`VERSION` 里放 `build: <时间戳>`；install.sh 先带随机参数取 VERSION（它必须新鲜），
再用其中的 build 戳作为后续所有下载的 `?b=` 键——发新版自动破缓存，同版本仍命中 CDN。

**另外**:该平台**不支持 Range 请求**（返回 520），所以用 `curl -r 0-1` 探测文件头的办法
在这里不可用（我就是这么被 ① 骗过一次——只看状态码 206 就以为文件在）。

真正的下载路径是 `/sl/preview/<mount>/<key>`，不是 `/<mount>/<key>`（后者是 SPA 页面）。

## P-20 shell 变量名紧跟中文 → macOS bash 3.2 上 unbound variable

`echo "... 不在 $SRC，跳过"` 在 Linux/新版 bash 下正常，但 **macOS 自带 bash 3.2**
会把后面的中文字节当成变量名的一部分，去找一个叫 `SRC，` 的变量 →
`unbound variable`（脚本又开了 `set -u`，直接崩）。

用户在 mac 上跑 `publish.sh` 撞到，我在 Linux 上 **`LC_ALL=C` 都复现不出来**——
这是 bash 版本差异，不是 locale。

**规则：写给多平台用的 shell 脚本，变量后面只要跟着非 ASCII 字符，一律写 `${VAR}`。**
自查：`grep -rnP '\$[A-Za-z_][A-Za-z0-9_]*[^\x00-\x7F{]' --include='*.sh' .`

同类历史：用户此前修过 `$pkg（600MB+，慢）` → `${pkg}`（commit 1d4d5e92）——
**同一个坑在同一个脚本里犯了两次**，所以这次全项目扫了一遍。

## P-21 (2026-08-13) 上传脚本的源写 `.` → 目标路径混进 `./` → 502

**现象**：`curl … upload.sh | bash -s -- . mount:tke/` 全部文件 502 失败，报错里能看到
目标 key 是 `tke/./bin/…`。

**原因**：`.` 作为源时，相对路径原样拼进 key，平台不接受含 `./` 的 key。

**做法**：源用**带尾斜杠的目录名**（`dist/` = 只传其内容），别用 `.`：
```bash
curl -fsSL .../upload.sh | bash -s -- dist/ tookit-engine-resource:tke/     # ✅
curl -fsSL .../upload.sh | bash -s -- .     tookit-engine-resource:tke/     # ❌ 502
```
同族提醒：目录**不带**尾斜杠会把目录名本身也带上（`dist` → `tke/dist/…`），
这在只想更新二进制时反而有用（`dist/bin` → `tke/bin/…`）。


## P-22 (2026-08-14) CI 上传步骤对"空目录"和 `[ ] && cmd` 双重踩坑

**现象**：只改了文档/workflow 的那次 CI，publish 步骤报 `✗ 没有匹配到任何要上传的文件`，
退出码 1。构建其实是**按设计跳过的**（没动 `src/`），失败的是上传。

**两个原因叠在一起**：

1. `mkdir -p dist/skill dist/bin` **无条件创建**了 `dist/bin`，于是 `[ -d dist/bin ]` 恒为真，
   对着一个**空目录**调上传工具 → 工具报"没有匹配到任何文件"并退出 1。
   → 判据要用**里面有没有文件**：`[ -n "$(ls -A dir 2>/dev/null)" ]`
2. `[ -d dist/bin ] && up dist/bin` 这种写法在 `set -euo pipefail` 下，**条件为假时整行返回 1，
   直接终止整个 step**。即使修好判空，写法本身仍是雷。
   → 用 `if ... fi`，别用 `&&` 短路当条件语句

**顺带**：改了 workflow 自己（换 runner / target / 构建参数）却不触发重新构建，等于改了没验。
`paths` 过滤器要包含 workflow 文件本身，"要不要编译"的判断里也要算上它。

## P-23 (2026-08-14) `curl <大文件> | head` 必炸：EPIPE → 退出码 23

**现象**：CI 的复验步骤报 `curl: (23) Failure writing output to destination`，整步失败。
本地对同一个 URL 复现，一模一样。

**原因**：`head -c2` 读够两字节就退出并关掉管道，curl 还在往里写 10MB，拿到 EPIPE 就以
**退出码 23** 结束。小文件侥幸不炸（内容全进了管道缓冲区，curl 早写完了），所以这坑
只在文件变大后才现形。

**做法**：**下完再验**，别用管道截断：
```bash
tmp="$(mktemp)"
curl -fsSL --max-time 300 "$url" -o "$tmp"
head -c2 "$tmp" | od -An -tx1 | tr -d ' \n'
```
不能用 `curl -r 0-1` 只取头两字节绕过去——**这个分发平台不支持 Range，会回 520**（P-19）。

**照抄提醒**：`curl … | head -1` 取 VERSION（一百多字节）是安全的，但那是因为文件小。
换成任何大文件都会踩这个坑。

## P-24 (2026-08-14) PowerShell 的两个标识符坑（与 P-20 同族）

写 install.ps1 / uninstall.ps1 时接连踩到，**都是"标识符边界"问题，与 bash 的 P-20 同一类**：

**① 变量名不区分大小写。**
```powershell
function Section($t) { Write-Host "$B$T▸ $t$R" }   # $T(颜色) 被参数 $t 覆盖 → 标题打两遍
param([switch]$Logs); $logs = "路径"                # 给 switch 赋字符串 → 调用时报类型转换失败
```
→ 颜色/工具变量用两字母以上（`$Cy` `$Dm` `$Rs`），且**别与参数同名**。

**② 变量名可以包含中文。**
```powershell
"$Ye试运行：..."     # `$Ye试运行` 整个被当成变量名 → 那三个字直接消失
"${Ye}试运行：..."   # 正确
```
→ 变量后面紧跟中文一律加花括号。**与 P-20（macOS bash 3.2 把中文吞进变量名）如出一辙。**

**③ 函数名别长得像内置 cmdlet 的扩展。** `Remove-Item-Reported` 会让参数绑定错乱
（实测报 SwitchParameter 转换失败）。改成 `RemovePath` 这种不带内置动词前缀的。

**自查**：
```bash
grep -nE '\$[A-Za-z_][A-Za-z0-9_]*[一-鿿]' *.ps1     # ② 会吞字的写法，应为 0
```

## P-25 (2026-08-14) `Invoke-WebRequest .Content` 可能是 byte[] 而不是字符串

**现象**：install.ps1 里版本号显示成 `116` —— 那是 `'t'` 的 ASCII 码。

**原因**：`.Content` 的类型取决于响应头与 PowerShell 版本；拿到 byte[] 时当字符串用，
就会得到一串数字。**更坏的是** `build:` 戳也解析不出来，破 CDN 缓存的键悄悄失效（P-19），
而表面上一切正常。

**做法**：
```powershell
$raw = (Invoke-WebRequest ... ).Content
$text = if ($raw -is [byte[]]) { [System.Text.Encoding]::UTF8.GetString($raw) } else { [string]$raw }
```

## P-26 (2026-08-14) 测试拷大文件进 /tmp 却不清理 → 跑几轮撑爆磁盘

**现象**：`No space left on device`，构建和 push 全挂。查下来 `/tmp/tke-cli-test-*-fix-check`
堆了一串，**每个 260MB**。

**原因**：CLI 契约测试 `fix_check_reports_without_downloading` 要把 tke 二进制拷进临时目录
才能验"空目录里跑 fix"，但**结尾没删**。而且断言失败会 panic，`remove_dir_all` 写在结尾
也执行不到。

**做法**：**先清理，再断言**——
```rust
let o = Command::new(&mine).args([...]).output().unwrap();
let s = format!("{}{}", stdout(&o), stderr(&o));
let _ = std::fs::remove_dir_all(&d);   // ← 放在断言前
assert!(s.contains("adb"), ...);
```
（更稳的做法是用带 Drop 的临时目录守卫，但对单个测试来说这样够了。）

**自查**：`du -sh /tmp/tke-* | sort -rh | head` —— 跑完测试不该有几十 MB 以上的残留。

## P-27 (2026-08-15) `execute()` 已解包 value，再写 `["value"]` 会静默退化

**现象**：用户反馈"整体太慢"。量下来**每次点击 4.9 秒**，而单次页面采集只要 110ms、
原子点击只要 14ms —— 4.7 秒不知去向。

**原因**：`WebDriver::execute()` 返回的是**已经剥掉 `{"value": …}` 外壳**的结果，
但两处调用方又写了一层 `["value"]`：

| 位置 | 后果 |
|---|---|
| `wait_ready` | `document.readyState` 永远读不到 `complete` → **每次点击白等满 20×200ms + 400ms** |
| `center_into_viewport` | 视口尺寸永远读不到 → 一直用硬编码兜底 1280×800，**坐标夹紧会算错** |

**两处都不会报错**，只是悄悄退化成"等满"和"用兜底值"——这类 bug 光看代码很难发现，
是量了耗时（4899ms vs 110ms 采集）才逼出来的。修完每步 4899ms → **~750ms，快 6.5 倍**。

**做法**：`execute()` 的返回值直接 `.as_str()` / `.as_i64()`。
`get()` / `post()` 返回的才是原始响应，那里才需要 `["value"]`。函数注释已写明这个区别。

**自查**：
```bash
grep -n 'execute(' -A2 src/drivers/web/mod.rs | grep '\["value"\]'   # 应为空
```

## P-28 (2026-08-15) 读屏专用元素（sr-only）会把文字定位骗走

**现象**：`输入 ["Search Wikipedia", "关键词"]` 失败，报「当前没有聚焦的输入框」。
可页面上那行字明明就在，`点击` 同样的文字也"成功"了却什么都没发生。

**原因**：无障碍标签（`sr-only` / `screen-reader-text`）**人看不见，却带着那行文字**，
典型实现是 **1×1 像素** + `clip` 裁掉。它通过了 `width>0 && height>0` 的可见性过滤，
进了元素表，还**排在真正的输入框前面**被 `find()` 先命中——于是点在那个 1×1 点上。

真正的输入框反而**一个字都没有**：没有直接文本、没有 placeholder，
它的可见名称来自 `<label for>`，而采集层当时只认 `aria-label` 和 `placeholder`。

**两处一起修**（缺一条这条路就还是断的）：
- 采集**排除**人点不到的：宽或高 ≤1px、`opacity:0`、`clip`/`clip-path` 裁没的
- 采集**补上**可及名称：`aria-labelledby` → `.labels`（`<label for>` 与包裹两种写法）→ `title`，
  并进对应控件的 text

**教训**：给 AI 的元素表**必须与人眼所见一致**——多一个人看不见的，就多一个静默点空。

**自查**：`tke -d web fetch | python3 -c "…"` 过一遍，不该出现 1×1 的元素。

## P-29 (2026-08-15) 一个平台的必需品被无条件套到所有平台

**现象**：web 上每次 `输入` 都比预期慢半秒。

**原因**：`atomic/control.rs` 的 `Input` 在点击后固定 `sleep(500ms)` **等软键盘弹出**——
这是**移动端**才有的东西。web 没有软键盘，而且 `tap` 本身已经等到页面就绪了。
实测这 500ms 占了「输入」这一步耗时的 ~38%（1315ms → 886ms）。

与 **P-27 同族**：都不是"写错了"，而是**一个语境下正确的等待，被搬到了不需要它的语境**。
两个都不报错，只是悄悄变慢。

**做法**：这类平台相关的取舍下沉到 `Controller`（`has_soft_keyboard()`），
别在上层按设备 id 猜。加新的固定 `sleep` 前先问一句：**哪个平台需要它？其他平台呢？**

## P-30 (2026-08-15) 文字定位只看得见视口内的元素

**现象**：`点击 ["Memory safety"]` 报「未找到包含文本 …」，但往下滚一屏就看得见。

**原因**：DOM 采集有 `r.top < innerHeight && r.bottom > 0` 的视口过滤（**有意为之**，
否则长页面的元素表会爆炸）。视口外的东西根本不在表里——定位的隐式等待再等 6 秒也没用，
它等的是"还没渲染完"，不是"在别处"。代价：白等 ~9 秒 + 这一批后续步骤全中断。

**做法**：先 `滚动查找 ["文字", 上]`（纯文字、**不需要元素库**）把目标带进视口，再点。
实测：直接点 = 失败 + 9.1s；先滚动查找 = 0.4s 找到 + 点中。

**注意这个坑的真正形态是文档坑**：`滚动查找` 的能力一直都在，
但 skill 的 `steps-syntax.md` 把它标成"需要元素库"，而那个 skill 明令不建元素库——
**于是调用方 AI 一次都没用过它**。与 90d9dcad（语义定位能力早就有、只是没告诉 AI）同型。

## P-31 (2026-08-17) CI 只比 `HEAD^..HEAD` → 一次 push 里的代码改动被漏编

**现象**：用户装完最新版，`tke doctor` 报「doctor 可执行文件缺失或不完整」——
那是 **passthrough 层**的报错：clap 不认识这个子命令，于是当成外部工具去找。
说明发出去的二进制**根本没有这个命令**，可 CI 全绿、skill 包也照发了。

**原因**：`changes` job 用 `git diff --name-only HEAD^ HEAD` 判断要不要编译，
**只比最后一个提交**。而那次 push 推了两个：

```
8f9eb45d  feat(tke): tke doctor …        ← 动了 src/
337fb835  docs(tke): STATE 对齐 HEAD      ← 只动 docs/  ← CI 只看到这个
```

于是判定"只动了文档" → **跳过六平台编译** → 新功能的二进制从来没被构建过。
而本项目的收尾惯例正是**最后补一个 docs 提交**（STATE/CHANGELOG），
所以这个坑会**稳定复现在每一次带收尾提交的功能发布上**。

**做法**：比**整个 push 范围** `${{ github.event.before }}..${{ github.sha }}`，
并把 `fetch-depth` 从 2 改成 0。取不到 before（新分支/force push）时**默认编译**——
多编一次十几分钟，漏编一次是"改了却没发出去"。

**这类 bug 的共性**：每一步都成功、没有任何红色，只是**不起作用**。
判断"要不要做某事"的条件写错时，错的那一侧永远是静默的——
所以条件本身要能被看见（改完后 CI 会打印本次 push 的全部改动文件）。

**自查**：发布后确认新命令真的在里面
```bash
curl -fsSL "$BASE/bin/<platform>/tke.gz?t=$RANDOM" | gunzip > /tmp/tke && chmod +x /tmp/tke
/tmp/tke --help | grep doctor
```

## P-32 (2026-08-17) 管道里的 `tr` 会把实时进度攒成一坨

**现象**：安装 Chrome 时进度条"出来很慢"——盯着空白等半天，最后才一次性跳到 100% 变对钩。

**原因**：这条流水线里 `tr` 是**块缓冲**的：

```bash
curl -#  … 2>&1 >/dev/null | tee log | tr '\r' '\n' | while read -r frame; do …
```

curl 的 `-#` 进度靠 `\r` 原地刷新、**一行到底不换行**，而 `tr` 输出到管道（不是终端）时
按 4KB 块缓冲——要攒够 4KB 进度帧才吐给下游。10MB 的文件下完也攒不满几块，
于是**整个下载期间一帧都不显示**，最后才全吐出来。

**实测**：第一帧出现时间 **9.25s → 0.28s**（限速 1200k、约 9 秒下完的文件）。

**做法**：管道里**别放会缓冲的外部命令**。按 `\r` 切帧用 bash 内建就够：

```bash
| while IFS= read -r -d $'\r' frame; do …
```

`stdbuf -oL` 能治 GNU 的 tr，但 **macOS 没有 stdbuf**，而安装脚本要三平台通用。

**另一半是心理上的慢**：建连接/TLS 握手那几秒 curl 一个字节都不输出。
先把 `· <名字> ` 打出来占位，人就知道"开始了"，而不是盯着空白猜。

**自查**：给下载限速跑一遍，看第一帧多久出现——应当是零点几秒，不是下完才出现。

## P-33 (2026-08-18) 用「当前进程的 PATH」判断装没装 → rc 一个字都没写

**现象**：装完那个 tab 里 `which tke` 有；**开新 tab 就 `tke not found`**。
更糟的是 `tke doctor` 在这台机器上一路绿灯，最后还写着「✓ 全局已就绪」。

**原因**：install.sh 的 PATH 段是这么判的——

```bash
if command -v tke >/dev/null && [ "$(command -v tke)" = "$TKE_HOME/tke" ]; then
    echo "PATH 已就绪"          # ← 直接跳过，一个 rc 文件都没写
```

`command -v` 看的是**当前 bash 进程的 PATH**，而它可能只是刚才临时 export 的
（上一轮我们恰恰教 AI 在装完后敲 `export PATH="$HOME/.tke/bin:$PATH"` 好让当前会话能用）。
于是"临时可用"被当成了"已经装好"，`~/.zshrc` 里始终没有那行——窗口一关就没了。

**根子**：把**易变的运行时状态**当成了**持久配置**的证据。判断"装没装"只能看落盘的东西。

**做法**：只看 rc 文件的内容，且**宁可多写一个也不能漏**（学 fnm）：

| shell | 写哪些 |
|---|---|
| zsh | `.zshrc` |
| bash | `.bashrc` **+ `.bash_profile`** |
| 其它 | `.profile` |

bash 那两个都要：**macOS 的终端开的是登录 shell，只读 `.bash_profile` 不读 `.bashrc`**。
rc 文件不存在就创建——新机器上常常压根没有 `.bash_profile`，跳过等于没装。

**doctor 也得能看出来**（INV-9）：新增「新终端」一项，同样只查 rc 内容，
不持久时结论从「全局已就绪」降级成「当前窗口可用 · 新终端里还找不到 tke」。
体检报的是**这台机器**行不行，不是**这个窗口**行不行。

**自查**：`export PATH="$HOME/.tke/bin:$PATH"` 之后再跑一遍安装，看 rc 里有没有那行。

## P-34 (2026-08-18) 「优雅停止」在等用户输入时 = 按 Ctrl+C 没反应

**现象**：`tke uninstall` 停在 `继续？[y/N]`,按 Ctrl+C——

```
继续？[y/N] ^C
⏹ 收到中断（Ctrl+C）——将在当前步骤结束后安全停止…
^C^C^C^C^C^C^C^C^C^C
已取消          ← 敲了回车才出现
```

按十次都不动,**还得再敲一次回车才真的退出**。

**原因**：全局 Ctrl+C 监听只做一件事——置 `ABORTED` 标志,由各处循环在检查点查到后
"走完当前步骤再停"。这对**正在跑步骤**是对的（不硬杀,产物写完整）。
但此刻主线程阻塞在 `read_line`,**没有任何循环会去查那个标志**;要等用户敲回车、
读到输入、返回上层,才轮得到检查点。取消键需要用户再按一次确认键,这不叫中断。

**两种情况必须立刻退出**：

1. **等用户输入时**（没有"当前步骤"需要收尾,收尾语义本身就不成立）
2. **按第二次**（第一次没停下来,说明当前步骤很长或卡住了;通用惯例也是二次硬退）

**做法**：
- 监听改成 `loop`,`ABORTED.swap(true)` 拿到旧值——第一次 `false`(优雅停)、
  第二次 `true`(`exit(130)`,128+SIGINT 的 shell 约定)
- 提示语补一句「再按一次立即退出」,别让人猜
- 阻塞读 stdin 的那几行用 `interrupt::prompting()` 包住,期间 Ctrl+C 直接退出。
  用 **Drop guard** 而不是手工配对的 `end()`：中间那段全是 `return`,漏一次就变成
  "后面的 Ctrl+C 全成硬退",那是另一种难查的怪事

**自查**：在每个交互提示处按一次 Ctrl+C——必须**不用回车**就退出,且退出码 130。
