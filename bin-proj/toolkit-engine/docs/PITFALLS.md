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

## P-35 (2026-08-18) 文字定位取「DOM 序第一个」→ 一直点在标题上,还报成功

**现象**（真事故,用户实跑一小时才发现）:登录页测了三组账号密码,每次
`点击 ["Sign in"]` 都**报成功**,但页面毫无反应。AI 的结论是——

> 「点 Sign in 后**均无任何反馈**……这个表单是个"死"表单」

写进了报告。实际上**表单是好的,是每次都点在标题上**:

```html
<h1>Sign In</h1>          ← DOM 里在前,点它当然"成功",也当然什么都不会发生
…
<button>Sign in</button>  ← 用户真正要点的
```

**原因**：`find_by_text` 是 `elements.iter().find(|e| e.matches_text(text))`——
**取 DOM 序第一个匹配**,不看它能不能点。

**注意 AI 没写错**:`fetch --interactive` 只输出 clickable/focusable 的元素,
标题根本不在 AI 看到的清单里。AI 写 `点击 ["Sign in"]` 完全合理,**是执行层选错了元素**。
所以这不能靠改提示词治(INV-8),得在定位层修。

**做法**：同名多候选时按「更像用户会点的那个」排序,依次比较——

1. **可点击的优先** ← 决定性的一条
2. 精确匹配优先(`Sign in` 该选按钮,不是 `Sign in with Google`)
3. 自身文字短的优先(包着按钮的容器文字更长)
4. DOM 序(前三项平手时保持稳定)

一个可点击候选都没有时**照旧返回第一个**——断言"页面上有这段文字"不需要可点击。

**这类 bug 的共性**：**点得中 + 报成功 + 什么也没发生**。三样凑齐,人和 AI 都会
转去怀疑被测对象,不会怀疑工具。跟 P-28(sr-only 幽灵元素)、P-27(白等 4.4 秒)是同一族。

**自查**：页面标题和按钮同名时,`点击 ["…"]` 点中的是不是按钮。

## P-36 (2026-08-18) iframe 里的东西一个都采不到 → 页面看起来是空的

**原因**：`DOM_WALK_JS` 从 `document.body` 往下走,而 **iframe 的内容是另一份 document**,
`children` 到 iframe 就到头了。支付、第三方登录、验证码、富文本编辑器都常住在 iframe 里——
AI 拿到的是一张「什么都没有」的页面,然后往被测对象身上找原因(「支付组件没渲染出来」)。

**做法**：
- 同源 iframe **递归进去采**。内部 rect 是相对**它自己的视口**的,要累加
  `iframe.getBoundingClientRect()` 的位置**再加边框宽度**(内容区从边框内侧开始),
  否则点击会整体偏移
- 视口裁剪也要换成 iframe 自己的尺寸(`clientWidth/clientHeight`),
  拿主窗口的 `innerHeight` 判断"在不在视口内"会把长 iframe 的下半截错误保留
- xpath 前缀成 `iframe[1]>>/*[@id='pay']`:内部 xpath 拿到主文档去找必然落空,
  带上前缀既能看出层级,也不会被误用
- **跨域 iframe 采不到,但必须留一条标记**(INV-9)。标记拼进这个 iframe 自己那条记录的
  aria 里,**不要另外 push 一条**——否则同一个 iframe 会在元素表里出现两次(实测撞到)

**实测**（本地两个端口造同源/跨域各一）：同源 iframe 内的按钮采到了、
`点击 ["Inner Pay"]` 真的点中(内部状态从 unpaid 变 PAID);跨域的出一条
`[跨域内容，采不到内部元素]`。

**自查**：页面里有 iframe 时,`fetch` 的结果里有没有它内部的元素。

## P-37 (2026-08-18) 原生对话框被 WebDriver 自动点掉,还是点的「取消」

**现象**（实测）:页面上点「删除」弹 `confirm('确定删除？')`——

```
点击 ["删除"]  → success: true
fetch          → 页面显示 CANCELLED
```

**全程没有一句提示**。AI 想测的是「确认删除」,实际每次都点了取消,然后看到数据还在,
得出「删除功能失效」——又一个 P-35 型假结论,而且更隐蔽:这次连"点错元素"都不是,
是**根本没人告诉它弹过窗**。

**原因**：W3C 的 `unhandledPromptBehavior` 默认是 **dismiss and notify**,
WebDriver 自作主张把对话框**取消**掉。而 alert/confirm/prompt 是**浏览器画的**,
不在 DOM 里——`fetch` 一个字都采不到,截图也拍不到(它在页面之外)。
不主动探测,它就等于不存在。

**做法**（三件事缺一不可）:
1. 建会话时 `unhandledPromptBehavior: "ignore"` —— 别让 WebDriver 替人做决定
2. **每步执行后探测一次** `GET /alert/text`,有就写进 StepResult / StepEnd / 报告 /
   终端。注意要**在补采页面之前**探:对话框挂着时任何页面命令都回
   `unexpected alert open`,补采只会白报一串错
3. **下一步执行前拦截**:有对话框挂着而当前指令不是对话框指令 → 直接报
   「先用 `确认对话框` / `取消对话框`」。否则冒出来的是 `unexpected alert open`,
   那串错跟"元素找不到"长得差不多,AI 多半会去改定位、重试、绕路

指令:`确认对话框` / `取消对话框` / `对话框输入 ["文本"]`(填完自动确定——
填了不确定等于没填)。

**顺带炸出来的真 bug**：`session_alive()` 探活用 `GET /url`,而对话框挂着时它同样回
`unexpected alert open` → tke 判定**会话已死**。于是撞上对话框的下一条命令直接报
「无活动浏览器会话」,**AI 连把它点掉的机会都没有**。改成:Status 错误里含
`unexpected alert` 的照样算活着(真死了的会话回的是 `invalid session id`)。

**自查**：`confirm` 弹出后,①终端有没有那行⚠ ②下一步的报错讲不讲人话
③跨批次还能不能把它点掉。

## P-38 (2026-08-18) 「点了没反应」的真因常在 console 里,而 AI 完全看不见

页面报了个 JS 错、某个请求 404 —— 结果就是"按钮点了没反应"。而 `fetch` 拿到的
页面结构里**一切正常**（元素都在、状态没变）,截图也看不出名堂。AI 只能得出
「这个功能失效」,没法说出为什么。

**做法**：`POST /log {"type":"browser"}`（chromedriver 扩展端点,**不用**额外
capability）一把拿到 console.error + 未捕获异常 + 加载失败的请求,还带 source 分类。
语义是**取走**（读一次清空）,所以每步收一次,天然落到"是哪一步触发的"。

噪音控制(用户明确抱怨过 WARN 刷屏)：只留 SEVERE、滤掉 favicon.ico、每步最多 3 条、
单条截到 300 字。

**自查**：造一个 `console.error` + 404 + `null.x=1` 的页面,看这三条是不是都落在
触发它的那一步上。

## P-39 (2026-08-18) 视口尺寸设成了窗口尺寸 → 差一截,断点测错

`POST /window/rect` 改的是**窗口**,里面还有标签栏/地址栏/边框。实测设 390x844,
量到的 `innerHeight` 是 **757**。测响应式看的就是断点,差几十像素可能整个跨过断点,
测的根本不是那个布局。

**做法**：走 CDP `Emulation.setDeviceMetricsOverride`。`deviceScaleFactor` 传 **0**
（= 沿用设备默认）——顺手改了 dpr 的话,截图坐标换算全跟着偏,整套点击都会打歪。

**顺带**：下载等待**不能用「有没有新增文件」当判据**。CLI 每条命令都是独立进程,
记不住上一次的基线。实测踩过:文件明明下好了,却因为"跟进来时一样"报超时。
判据改成「有文件 **且** 没有 `.crdownload` 半成品」;要区分新旧就换个空目录——
那是调用方的事,不该由一个记不住状态的进程假装能办。

## P-40 (2026-08-18) 认不出的按键 `_ => Ok(())` —— 三端里两端都这么写

写平台矩阵时逐格核对实现,发现 iOS 和 Web 的 `key_event` 都是同一个写法：

```rust
_ => Ok(())        // 认不出的键：什么都不做,报成功
```

于是 `按键 ["TAB"]` 在 iOS 上、`按键 ["KEYCODE_VOLUME_UP"]` 在网页上,**都报成功但什么
都没发生**。人会以为焦点已经移走了,接着往下写,错在后面几步才暴露——排查时根本不会
回头怀疑那个"成功"的按键(INV-9)。

**做法**：认不出就报错,并**列出这个平台支持哪些**(光说"不支持"等于让人猜)。
web 顺带支持了单个字符——`按键 ["a"]` 是有意义的请求,没道理不给。

**这条的普遍性**：`_ => Ok(())` 是"通配分支返回成功"的典型。**通配分支返回 Ok 之前
先问一句：这里真的什么都不用做吗?** 大多数时候答案是"这是没实现的情况",
那就该报错。同类还有 P-27(execute 已解包却再写 `["value"]`,永远拿到 null 又静默退化)。

**自查**：给每个 match 的 `_` 分支念一遍"这种情况下什么都不做是对的吗"。

## P-41 (2026-08-18) 更新了 skill 却没重读 → 更新等于白做

**现象**：`tke doctor` 报「skill 可用更新」,AI 老老实实停下来问用户"要我跑 tke update 吗"
——多一轮往返不说,**跑完之后它手上那份文档还是旧的**。

**原因**：两件事叠在一起。

1. **它在问一件不用问的事**。`tke update` 幂等、十几秒、已装的依赖会跳过。
   除非本机缺 Chrome 那种大件(doctor 的「依赖」那行会说),否则没什么好征求同意的
2. **更新换的是磁盘文件,不是 AI 的上下文**。SKILL.md 是**会话开始时加载**进去的,
   `tke update` 之后那份内容一个字都不会变——不显式重读一遍,新写法它一条都不知道

第 2 条是真正的坑:更新"成功"了,行为却完全没变,而且**没有任何迹象**。

**做法**：
- SKILL.md 写明:看到过期就直接更新,**更新完 `cat` 一遍自己**
- **同一句话也要由 tke 自己说出来**——`hint()` 在 skill 过期时缀「更新后重读 SKILL.md」。
  只写在 SKILL.md 里是没用的:**手上文档旧的那个 AI,恰恰就是看不到新指示的那个**。
  这是 ADR-0010 的老道理:护栏进工具,别只写在提示词里

**顺带**：那行提醒有个"必须短"的单测(它缀在别人正干着的活后面,不该抢戏)。
加了这句会撑破 40 字符的阈值——**不是把断言删掉**,是分两支:
skill 过期那支放宽到 60 并断言必须含"重读 SKILL.md",只有 tke 旧的那支仍然 <40
且断言**不许**提 SKILL.md。阈值可以谈,"这行要短"这件事不能忘。

**自查**：`tke update` 之后,AI 有没有重新读一遍 SKILL.md。

## P-42 (2026-08-19) 构建成功了,但你敲的 `tke` 还是旧的

**现象**（实测，浪费了一整轮）：`build-mac.sh` 打印 "Build successfully"，紧接着——

```
$ tke device list
error: unrecognized subcommand 'list'          ← 明明刚加的

$ tke -d sim:<UDID> steps '点击 ["..."]'
ADB错误: adb: unknown host service '92AA7443-...:features'   ← sim: 被当成安卓序列号
```

**原因**：两个不同的文件。

```
build-*.sh 拷到  →  <repo>/bin/<platform>/tke
日常敲的 tke     →  ~/.tke/bin/tke      ← 安装器装的那个，还是旧版
```

编译**确实**是新代码（警告里都能看到新文件名），只是跑的不是它。
这是「装好的东西不自更新」那一族的又一个变种（同 P-33 的 PATH、Q-11 的 skill 过期）：
**每一步都报成功，合起来什么也没生效**。

**做法**（两条，缺一不可）：

1. **验证脚本一律用构建产物的绝对路径**，不用 `tke` 这个名字。
   `scripts/verify-*.sh` 自己算出 `<repo>/bin/<platform>/tke` 再调——
   验的必须是"刚改的这份代码"，不能是碰巧在 PATH 里的那个
2. **构建脚本只提示，不覆盖**。`command -v tke` 指到的那个是**用户日常在用的**
   （可能还给别的项目用着），构建脚本没有资格替他换掉。所以只打一行：
   ```
   注意: 你敲 tke 用的是 /Users/x/.tke/bin/tke（不是刚构建的这个）
         要用新的: /path/to/repo/bin/darwin-arm64/tke
   ```

> 一开始我写成了「构建完自动同步过去」，被用户当场拦下——**开发产物不该覆盖人家在用的东西**。
> 提示 + 验证脚本自己找产物，两边都不越界。

**自查**：改完编译完，别只看 `--version`——敲一条**这次新加的子命令**。它认得，才算真生效。

### 顺带踩的两个（都在"验证工具本身"里）

**① `$BASH_SOURCE` 是相对路径，`cd` 走之后就指不到了**

```bash
cd "$(dirname "${BASH_SOURCE[0]}")/.."          # ← 先 cd 了
TKE="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)/..."   # ← 这里已经找不到
# cd: bin-proj/toolkit-engine/scripts/../../..: No such file or directory
```

用 `bash path/to/x.sh` 调用时 `$BASH_SOURCE` 就是那串相对路径。**先解析成绝对路径再 cd**：

```bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$SCRIPT_DIR/.."
```

**② `"$VAR（中文）"` 在 macOS 自带的 bash 3.2 下会吃字节**

```
注意: 你敲 tke 用的是 ��不是刚构建的这个）        ← 路径没了，全角括号也烂了
```

变量名后面紧跟多字节字符时，老 bash 的边界判断不可靠（Linux 的 bash 5 不复现，
**恰恰因此最容易漏**——本机测着好好的，用户机器上是乱码）。
中文文案里一律写 `${VAR}`，别省那对花括号。

> **记完文档还是又犯了一次**：隔了半小时写打包脚本，`step "① 取源码（锁定 $WDA_REF）"`
> ——`set -u` 下直接 `WDA_REF�: unbound variable` 崩掉，脚本第一步就跑不动。
> 所以加了守卫 `scripts/check-shell-vars.sh`，进 pre-commit **拦死**。
> **这类"本机测不出来"的坑，光写进坑册没用，得让工具替人记**（同 ADR-0010）。

## P-43 (2026-08-19) 产物收集认扩展名白名单 → 新驱动的原文静静地没了

**现象**：iOS 模拟器链路全部验通（元素采到、点也点中了），只有一项——
`raw_pages/` 是空的。不报错、不提示，只有人对着报告数目录才发现。

**原因**：收集那段写的是

```rust
for ext in ["html", "xml"] {          // web=.html，安卓与 iOS 真机=.xml
    let src = workarea.raw_page_path(ext);
    ...
}
```

而模拟器的 AX 原文存成了 `.json`——**不在名单里，于是当它不存在**。

**做法**：**按前缀扫，别列扩展名**。

```rust
// current_raw_page.* —— 驱动想用什么后缀是它的事，收集方不该维护一份清单
std::fs::read_dir(dir)?.flatten().map(|e| e.path())
    .find(|p| p.file_name()...starts_with("current_raw_page."))
```

**更普遍的那条**：凡是「加了新东西就要记得同步改另一处」的地方，都是在等着漏。
能靠约定（前缀/命名）自动发现的，就别写清单。清单会漏，而且**漏了是静默的**。
同族：P-31（CI 只比 `HEAD^..HEAD`，一次 push 的代码改动被漏编）。

**自查**：换一个没见过的扩展名，看产物还收不收得到（已用单测钉住，
夹具里那个后缀就叫「以后某个新驱动的后缀」）。

## P-44 (2026-08-20) 子命令的开关跟全局参数撞名 → 运行时 panic

**现象**：给 `control boot` 加了个 `--headless`，然后**任何** control 命令都崩：

```
thread 'main' panicked at src/main.rs:77:
Mismatch between definition and access of `headless`.
Could not downcast to alloc::string::String, need to downcast to bool
```

**原因**：全局已经有一个 `--headless=<auto|on|off>`（String），子命令又定义了同名的
`bool`。clap 按**名字**存取参数，两个定义撞在一起——而且**不是编译期报错，是运行时 panic**。

同一个坑第二次：`browser reset --cache` 撞全局的 `--cache <目录>`（那次也是 panic）。

**做法**：加子命令开关前**先扫一眼全局参数**。撞了就换名，或者干脆别加——
`--headless` 那次的正解是**删掉它**：无头本来就是默认，全局那个已经够用了。

**自查**：新加的开关名，在 `main.rs` 的全局 `Params` 里 grep 一遍。

## P-45 (2026-08-20) iOS 的密码框从来没被识别过 → 真密码明文进了报告

**现象**（用户实跑的双端报告里）：

```
04 ✓ 输入 ["Password", "TempTest001"]      填密码(tke 会自动打码)
```

**它没打码。** 那份报告已经传到云端了。

**原因**：打码的判据是「点中的元素在页面结构里标着 `password="true"`」——

| 平台 | password 属性 | |
|---|---|---|
| 安卓 | uiautomator **原生就有** | ✅ |
| web | DOM 归一化时对齐了（`type=password` → `password="true"`） | ✅ |
| **iOS** | XCUI 归一化**根本没输出这个属性** | ❌ |

而 `target_resolver` 里那句注释写的是「**三个平台同一条路**（安卓原生有，web 侧已对齐）」
——**它把 iOS 漏了，注释还让人以为覆盖了**。于是 iOS 上密码一路明文写进
log.json、报告、**截图顶部横幅**，全是要发给别人看的东西。

**做法**：`XCUIElementTypeSecureTextField` → 输出 `password="true"`（两个单测钉住）。

**这类漏法的共性**：**能力按平台一个个加，而注释按"应该都有"写**。
三端里做了两端最危险——剩下那端不但没功能，还因为注释显得已经做过了。
同族：P-40（`_ => Ok(())` 在 iOS 和 web 两边都有，先只修了一边）。

**自查**：涉及三端的能力，逐端各写一个测试；注释里写"三平台同一条路"之前，
先去另外两端的代码里确认一遍。

## P-46 吞掉下层的错误，等于把最有用的那句话删了

`ensure_existing` 里原来是 `if let Ok(conn) = self.attach_foreground(..)`，附着失败就
换成一句「无活动 WDA 会话，请先执行 启动 [BundleID]」。而 `attach_foreground` 失败时
说的是**「现在前台是桌面（主屏幕），不是你要测的 App——用 `启动 [你的BundleID]` 把它拉回来」**
——恰恰是能让人立刻动手的那句。

用户跑双模拟器时撞上：一台有截图、一台只有报告，报的却是那句泛泛的话，看不出到底是
没建成会话、还是附到了旁观者身上。

**改法**：下层的错误原样透出去；只有在"连 /session 都没建起来"这一种情况下才补一句
下一步该敲什么。**用 `if let Ok(..)` 处理 Result，就是在丢错误**——写的时候要问一句：
我丢掉的那句，是不是比我换上的这句更有用（同 INV-9）。

## P-47 `cargo test --lib` 不覆盖 bin crate，那批测试烂了很久没人知道

`src/cli/` 属于 **bin crate**（`main.rs` 里 `mod cli`），而 AGENTS.md 的必过清单一直写的是
`cargo test --no-default-features --lib`——**`--lib` 只测 lib crate**。于是 `cli/fix.rs`
里那几个测试在 `detect_missing` 改名成 `detect_deps` 之后就编不过了，连着好几次
"全绿"的提交都没发现，直到有人手敲 `--bin tke` 才炸出来。

**改法**：清单改成 `cargo test --no-default-features`（不带 `--lib`，跑全部 target）。

**更一般的教训**：绿灯的范围要跟你以为的范围对得上。一条只覆盖一半代码的命令，
比没有命令更危险——它会给你"都验过了"的错觉。

## P-48 安卓模拟器无头下 `swiftshader_indirect` 截图是纯色

Linux amd64 实测：`-gpu swiftshader_indirect` 起得来、元素采得到、坐标也点得中，
**唯独截图是一张纯色图**（63KB 的 PNG，而正常的同一屏是 1.7MB）。
emulator 自己的 `adb emu screenrecord screenshot` 拿到的也一样——说明不是 `screencap`
的问题，是合成器只出了背景层、App 的内容层根本没合上去。

**改法**：`-gpu swiftshader`（**不带 `_indirect`**）。换完同一屏壁纸、图标、状态栏全在。

**为什么值得单列一条**：这是"每一步都报成功"的又一个变种（同 P-35 那一族）——
起成功、采到元素、点也点中、页面确实变了，只有**留给人看的那张证据**是空的。
而 tke 的立身之本就是留证据（ADR-0010）。查的时候容易把注意力放在"是不是没渲染"，
其实该问的是"哪一层没合上去"。

顺带：`aosp_atd` 镜像**默认关掉硬件渲染**，截图恒为纯色，Google 让你改用
AndroidX Test Screenshot API（instrumentation 进程内，外部拿不到）。它小 100MB，
但对 tke 不可用——已在 `cli/android_sdk.rs` 里写死用 `default` 镜像。

## P-49 `am start` 失败时退出码是 0，错误走的是设备那边的 stderr

两层遮蔽叠在一起：

1. `am start` 找不到 Activity 时**退出码仍是 0**（错误只写文本）
2. 那段文本走的是**设备上的 stderr**，`adb shell` 不会把它并进 stdout，
   而 tke 这层只收 stdout

于是包根本没装、组件名拼错（`pkg/.Act/` 末尾多一个斜杠，`result code=-92`），
`tke steps "启动 [...]"` 照样报成功——App 从没起来过，后面每一步都在对着桌面操作。

**改法**：`adb shell "am start -n <组件> 2>&1"`，再查输出里有没有 `Error:` / `Error type`。
**更一般的**：凡是靠退出码判断成败的外部命令，都要先确认"它失败时退出码真的会变"。

## P-50 写死的错误文案会把查的人带偏（我自己被带偏了三轮）

`boot` 的就绪判据有三项（系统起完 / 屏幕亮着 / 有焦点窗口），而超时时的错误一律写着
**「adb 已看到 emulator-5554，但 sys.boot_completed 一直不是 1」**。

真正卡住的是第二项：`screen_on` 只认 `Display Power: state=ON`，而 Android 15 的
`dumpsys power` 里那一行是 `Display Power: com.android.server.power.…$1@6fe9d67`
——一个对象引用，永远匹配不上。

于是查的人（当时是我）盯着 `getprop sys.boot_completed` 反复确认「明明是 1 啊」，
一路怀疑到 adb 路径、userdata 损坏、强杀残留，绕了三轮才想起去看另外两项。

**改法**：报错时**逐项验一遍，只说真正没过的那些**。
**更一般的**：一条覆盖多个判据的失败路径，错误文案不能挑其中一个写死——
那等于在替读者做一个你并没有验证过的判断。

## P-51 模拟器的三代 API 判据

同一件事在不同 Android 版本上字段名完全不同，写判据时要认全（实测踩过前两条）：

| 要判断 | 现在（API 30+） | 旧格式 |
|---|---|---|
| 屏幕亮着 | `dumpsys power` 里 `mWakefulness=Awake` | `Display Power: state=ON` / `mScreenOn=true` |
| App 的 launchd label（iOS） | `UIKitApplication:<bundle>[0x…][rb-legacy]` | 拿 bundle id 精确查是查不到的 |
| `am start` 失败 | 退出码仍是 0，错误走**设备那边的 stderr** | — |

**共同点**：判据不匹配时**看起来都像"东西没就绪"**，而不是"我判错了"。
写这类判据时，先在目标版本上把原始输出打出来看一眼，别照着记忆写。

## P-52 TUI 里 AI 对用户说的多行话要用 Assistant 事件，不是 Notice

security orchestrator 真机 TUI 里，agent 的长回复（带 Markdown 项目符号）渲染成**逐行右移的阶梯**。
根因：用了 `UiEvent::Notice` 发对话消息，而 Notice 的 TUI 渲染走**带缩进的包裹逻辑**，多行时每行累加缩进。
harness 本就为「主 AI 说话」设计了 `UiEvent::Assistant`——它按 `text.lines()` **逐行从第 0 列**渲染，多行正确。
**规矩**：agent 对用户说的话（Text 回复 + 调工具的前导说明）一律 `Assistant`；短状态行（→ http/findings）才用 `Notice`。
（token 用量取 `session.last_usage()` 填 `Tokens::new(pt,ct)`。）commit 3c453a17。

## P-53 (2026-08-26) 顺手清掉过期的条目 = 跳过了它的收尾流程

写租约表时，`acquire()` 里加了一句"先 retain 掉过期租约，否则设备会被死租约永久占住"——
看起来是常识性的卫生动作，实际是**让设备绕过复位直接给了下一个租户**：
复位（关浏览器会话/停 App，INV-17）挂在 `sweep()` 那条路上，被 retain 掉的租约根本不会走到那里。
下一个租户接手的就是上一个人登录着的浏览器。

**是单测逼出来的**：「sweep 交出过期的那批」写完就红——因为 acquire 早就把它们吃掉了。

**规矩**：**过期不等于可以立刻重用**。凡是"释放时要做点什么"的资源，过期条目只能由那条
带收尾动作的路径回收；别的地方一律**只判断、不删除**。这里的代价是 TTL 到期后最多再等
一轮清扫（15s）设备才回池——比"静默跳过复位"便宜得多。

**同类嫌疑**：任何 `retain`/`remove_if`/缓存淘汰旁边，问一句"这个条目死的时候本来该做什么"。

## P-54 (2026-08-26) URL 里的 `../..` 会被 HTTP 客户端在本地就吃掉

给上传接口写"跳出工作区必须被拒"的测试，请求 `PUT /v1/sessions/{sid}/workspace/../../evil.txt`，
断言 400/404 —— 结果拿到的是别的状态码，测试红了。根因不在服务端：**HTTP 客户端（和多数代理）
会在发出前按 RFC 3986 把 `..` 归一化掉**，服务端收到的根本不是带 `..` 的路径。
也就是说那条测试测的是客户端的归一化，一个字节都没碰到服务端的沙箱。

**规矩**：测路径穿越必须用**百分号编码**（`%2e%2e%2f`）——只有编码过的才真的把 `..` 送到服务端手里。
顺带：真实攻击者也正是这么发的，所以这不是"为了让测试通过而绕路"，而是本来就该测的那一种输入。

**反过来也成立**：如果你的服务端只在"看见字面量 `..`"时才拒绝，那它挡不住 `%2e%2e`——
沙箱要在**解码之后**的路径上判断（axum 的 Path 提取器已经解码，我们的检查在它之后，所以是对的）。

## P-55 (2026-08-26) 传了 `skill/` 目录，漏了 `skills` 这个文件 —— 安装器静默少装

用户 mac 上 `tke doctor` 只报了一个 skill：`tke-security-test` 明明打了包也传上去了
（分发源上 `skill/tke-security-test.tar.gz` 9848 字节，取得回来），却谁也装不到。

根因：`install.sh` 读 `<分发源>/skills` 这个 **manifest 文件**（一行一个 skill 名）决定装哪些，
而 CI 的上传那行是

```
up dist/skill dist/install.sh …          # 只有目录 skill/，没有文件 skills
```

`dist/skills` 生成了、从没上传过 → 分发源 404 → 安装器走兜底 `SKILL_LIST="tke-ui-test"`。
**兜底不报错**，于是三方都显得正常：CI 绿、安装成功、doctor 说"已是最新"。

两个放大它的因素：

1. **名字差一个 s**：`skill/`（目录，装的东西）与 `skills`（文件，装哪些）平级又同名，
   写上传清单时眼睛会自动把它们归成同一样东西。
2. **复验只遍历目录**：`for f in $(cd dist && find bin skill -type f)` —— `skills` 在
   `bin/` 与 `skill/` **之外**，够不着。于是"复验分发源"这一步给了虚假的安心。

修法（三处）：上传清单加 `dist/skills`；复验**逐个名字比对 manifest 内容**（平台对不存在
的路径回落 200 + HTML，P-19，"取回来了"什么都不证明）；兜底那行**往 stderr 说一句**。

**共性**：兜底逻辑（`|| SKILL_LIST="tke-ui-test"`）把"少了一个文件"表现成"功能少了一半"
而不是"报错"。凡是写 `|| 默认值` 的地方，问一句：**默认值生效时，有没有任何人看得出来？**
这里的答案是没有。

## P-56 (2026-08-26) 没有 `步骤:` 的 .tks 一步不跑，却报 success + 退出码 0

平台全流程实跑，第一版脚本我自己写成这样：

```
# example.com 冒烟
启动 ["https://example.com"]
断言 [{首页标题}, 存在]
```

`tke run` 的输出：`{"success":true,"total_steps":0,"successful_steps":0}`，退出码 0。

根因：解析器要先遇到 `步骤:` 这一行才开始收指令（`in_steps`）。少了它，所有行都落在
标记之前被整段跳过 → 零步 → "全部成功"。**认不出的指令是会报错的**（INV-9 早就治过），
但"一条也没认"这条路没人守。

放大它的是判据本身：`successful_steps == total_steps` 在 0/0 时成立。空集上的全称命题
恒真——**"没有失败"被当成了"成功"**。

修法：解析出 0 步直接报错，并且分辨得出是哪种空：有内容但都在标记之前 → 明说
「指令必须写在 `步骤:` 下面」；文件本身空 → 说文件是空的。

**共性**：凡是拿"失败数为 0"当成功判据的地方，先问一句：**样本数是 0 吗？**

## P-57 (2026-08-26) 平台字段没传下来，报的却是「本节点没有 android 设备可租」

平台下发 `platform: "web"`，三个任务全部在 0.3 秒内 `error`，理由是这句。人第一反应是
去查那台机器的设备池——而池子里明明有 4 个 web 槽，一个 android 都没有，也不该有。

根因两半，**各自看都像对的**：

1. 平台侧 `TaskSpec` 压根没有 platform / device_id 字段 —— 下发时选的设备类别只用于
   平台自己挑 slot，到了起任务那一步没往下带。
2. tke 侧 `spawn()` 里写死 `let platform = "android"`。

于是错误信息**如实描述了它做的事**（它确实在找 android），但那件事本身就是错的。
这类报错最难查：它不撒谎，只是没提"我为什么找 android"。

修法：两边都加字段；tke 侧抽成 `wanted_platform()` 并加回归（不给才回落 android，
只填空格等于没填）；平台侧排期器**点名派给它已经领到的那台 slot**（`spec.DeviceID`），
而不是只说个类别让节点自己再挑一遍——两边各挑各的，占用记录和实际跑的设备会对不上。

**共性**：跨进程传参时，接收方的默认值会把"你没告诉我"伪装成"你要的就是这个"。
默认值只该出现在**一处**，另一处宁可留空。

## P-58 (2026-08-26) 缺 AI 配置被判成「用例没通过」

节点没配 `[ai]`，编排官当场崩，任务回的是 `failed`，detail 里只有
`{"script": null, "conversation": ""}` —— 一个字的理由都没有。

平台按五态映射把 `failed` 记成"这条用例不通过"，于是人去查产品代码。**查的是错的方向**：
一步都没测成，产品有没有问题这次根本没验。

根因：`outcome_from_event` 只看 `done.success`，false 一律 `Failed`；而 stdout 里那条
`{"type":"notice","level":"err","text":"编排官出错：AI模型错误…"}` 被丢掉了。

修法：泵事件时记住最后一条 err 播报；`done success=false` 时——编排官层面崩了判
`Outcome::Error`，其余仍是 `Failed`，但**两种都必须带上 why**。

**共性**：ADR-0022 D6 那条"执行失败 ≠ 测试失败"，落地时容易只落在**有明确错误码**的路径上
（超时、连不上）。真正危险的是**有终态事件、只是内容说失败**的那种——它长得跟正常失败一样。

## P-59 (2026-08-26) 写进去了，读回来没有 —— 上传报成功、下发说没有

`.tklib` 上传接口回 `replayable: true`；紧接着回放下发说"这个 run 里没有一条用例挂着
可回放的两件套"。两句话都对，就是拼不到一起。DB 里 `tklib_file_id` 确实非空。

根因：`scriptSelectAll` 这个 SELECT 列表里没有 `tklib_file_id` / `tklib_hash`——
迁移加了列、写入路径加了 UPDATE，**读取路径没人动**。于是 `Replayable()` 永远看到 nil。

第二层：上传接口报的 `replayable` 是拿**上传前那份内存对象**自己算的
（`ScriptType=="tks" && Content!=""`），压根没经过刚写的那一列。所以它不是在报告事实，
是在重复我的意图。

修法：SELECT 补两列；上传接口**回读一次再报**；加回归——直接断言 SELECT 文本里有这两列
（这种"列没跟上"的漏，用真库测反而要先造一堆前置数据，断言 SQL 文本更直接）。

**共性**：加一列要走完**三段**：迁移、写、**读**。前两段做完程序能编译、能跑、日志也正常，
只有跨接口对拼时才露馅。凡是新增字段，问一句：**有没有一条路径把它读回来验过？**
