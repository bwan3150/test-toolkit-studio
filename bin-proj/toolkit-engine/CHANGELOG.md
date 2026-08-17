# 变更记录（toolkit-engine）

只追加,不重写已有条目。每条带日期 + commit + 一句话;细节看 commit message（本仓库 commit 写得很全）。
更早历史直接看 `git log --oneline -- bin-proj/toolkit-engine`。

---

## [Unreleased]

### 2026-08-17 · `tke doctor`:把「本地是不是旧版」变成看得见的一行(ADR-0014,关闭 Q-11)
Q-11 的代价已经付过一次:用户装好的 skill **装完就不动**,没有任何东西告诉他有新版——
一整场会话改的四个修复,他重跑时拿到的仍是两天前的旧文档,**必然得出"没改善"的结论**。
- **fix(根因)** `fix --check` 其实一直在联网比版本,但只比 **tke 二进制的版本号**——
  而版本号**只在 bump 时才变**、SKILL.md 却天天改。**改成比 VERSION 里的 `build` 戳**
  (每次发布都变),`publish.sh` 把这份 VERSION **一起打进 skill 包**,装完就有据可查
- **feat(更名)** `tke fix` → **`tke doctor`**(体检,**不下载任何东西**);`doctor --fix` 才补依赖。
  `tke fix` / `tke fix --check` **保留为别名且语义不变**——已发布的 install.sh、用户脚本、
  以及**用户机器上那份旧 SKILL.md** 里全是老写法,它们不会因为我们改名就自己更新
  (正是本次要解决的问题,不能自己再犯一遍)
- **feat(挂在 steps 上)** 调用方 AI 每次操作设备都走 `steps`,**这是唯一保证被看见的位置**——
  指望人想起来跑体检正是踩坑的原因。三条克制:结果**缓存 4h**(每批都问、每 4h 才真联网一次,
  超时 5s)、打 **stderr**(stdout 是给 Electron 的 NDJSON)、`--json` 时闭嘴
- **决策(只提醒不代劳)** 发现不一致只打印一行 + 更新命令,**绝不自己覆盖二进制**——
  覆盖运行中的可执行文件在三平台各有各的坑(Windows 锁文件/Linux ETXTBSY/macOS 签名),
  install.sh 已经踩平并验证过
- **边界(宁可漏报不误报)** 老安装器装的 skill 没有版本文件 → 报"装了,但没有版本信息",
  **不当成过期**;本地自编的 tke(`unknown`)不参与比对。误报会让人学会忽略提醒,比不报更糟
- **不违反 ADR-0012** 只 `curl` 一个几十字节的 VERSION,不下载任何依赖;"唯一会下载的命令"
  仍然只有那一条。ADR-0012 已加指向
- **验证** 模拟 08-13 的旧 skill → doctor 与 steps 都如实报出并给更新命令;一致时安静;
  `tke fix --check`(install.sh 用的那条)行为不变。lib 70/70(+4) + CLI 25/25(+2)

### 2026-08-15 · 证据组织重做:一个任务一份报告(用户反馈"这种组织方式很乱")
用户看完上一份报告的评价:**"log 和 report 的组织方式很乱"**。确实——每调一次 `tke steps`
就建一个 `steps_<时间戳>/`,各带自己的 `screenshots/`/`page/`/`log.json`,外面再拼一份
全流程报告。调十次就是十个目录 + 十一份报告,人要审得先挑出哪份是总的。
- **feat(布局)** 新增 `Layout::Task`:**`--log` 指的就是任务目录本身**,反复调用**续写同一份证据**——
  `<任务>/{report.html, screenshots/, pages/, log.json}`,**步骤跨批次连续编号**(step_001…005),
  一个任务**始终只有一份 report.html**。`page/` 更名 `pages/`
- **范围** 只改 `steps`(一次性检查);`run`/`flow`/`harness` 保持 `<名>_<时间戳>/`——
  它们每次是**独立的一次回放**,分目录才对得起"跑第二遍和第一遍比一比"(用户拍板)
- **feat(log)** `TaskLog{batches:[…]}` 累积每批,**读-改-写**而不是覆盖;兼容读旧的单批格式
- **feat(跨设备不再分目录)** 每批自带 `device`,报告里标出来并**按时间排成一条线**——
  正好还原"平台侧做了什么 → 手机侧看到什么"。此前教人分 `web/`+`phone/`,反而把因果链切断、
  还要额外汇总一次。skill 文档同步改掉
- **feat(截图内嵌)** 任务报告**默认自包含**:单个 html 直接转发,对方不需要那个目录。
  内嵌走**缩放 + JPEG**(宽 960/质量 82)——报告容器只有 880px,内嵌更大的像素**一个字都不会更清楚**。
  5 步实测 **1.7MB(原图) → 598KB**。点报告里的截图可跳原图;`tke report --full-image` 出原图版
  - ⚠️ 实测教训:**光转 JPEG 几乎不省**(PNG 对大片纯色压得很好,JPEG 在文字锐边上还吃亏),
    真正省体积的是**缩放**。第一版用 1280 宽只从 1.1MB 降到…没降,量了才发现
- **feat(边界)** `next_step_index()` 按**文件**扫编号而不是数 log.json 的步数:中途 Ctrl+C 的批次
  可能没写完 log 但截图已经落了,漏算会**直接覆盖上一批的证据**(静默丢证据,加了回归测试挡)
- **验证** 三次独立 `steps` 调用 → 连续 5 步 / 3 批 / 一份 598KB 自包含报告,无死链。
  lib 65/65(+3) + CLI 23/23

### 2026-08-15 · 探索式使用不再把报告搞乱(用户追问"一步一步探索会不会分成两个任务")
不会——同一个 `--log` 就是同一个任务。但顺着这个问题实测了一遍"每次只走一步"的极端情况,
发现报告确实会被搞乱,以及两个真 bug:
- **fix(噪音)** 批次分隔行改为**只在有信息量时才插**:换设备、或中间停了 ≥60s。
  探索式会产生一长串"1 步"批次,每批插一行的话人看到的全是"AI 分几次调的"——
  那是工具的实现细节,不是这次检查发生了什么。实测 5 次单步调用:分隔行 **5 → 0**
- **fix(标题/目录指错)** Task 布局下 `dir` 就是任务目录,而渲染仍照旧取 `parent()` →
  **报告标题变成 "logs"、"打开检查目录"跳到 `~/.tke/logs`**。改为按 `prefix` 是否为空区分两种布局
- **feat(空档标注)** 两批间隔 ≥60s 时标一行「间隔 N 分钟」——那多半是**人在中间做了什么**
  (手动登录、去后台改配置),而这件事在证据里没有任何痕迹,只剩这个时间空档
- **docs** SKILL.md 讲清"`--log` 目录名 = 任务身份,一个字都不要改",并明确探索式怎么用;
  新增坑册 C-17(中途改名 → 一次检查散成几份报告,**且不会报错**)
- **验证** 单元测试锁住三种情况(同设备连续=0 行 / 换设备=1 行且写明设备 / 间隔 6 分钟=标出空档);
  真机换设备实测确实插行。lib 66/66(+1) + CLI 23/23

### 2026-08-15 · 语义定位这条路上的四个洞:实测走一遍才发现它一直是断的
上一场把 SKILL.md 从坐标掉头到语义定位(90d9dcad),但**没有人真的用新版走过一遍**(Q-9)。
这次自己当调用方 AI 实跑,一个普通的"搜索→进条目→点内链"链路就把路上的洞全撞出来了——
**四个洞环环相扣,单修任何一个这条路都还是断的**。
- **fix(P-28,感知层)** 读屏专用元素(`sr-only`/`screen-reader-text`)**人看不见却带着那行文字**,典型是 **1×1 像素**,却通过了 `width>0&&height>0` 的可见性过滤进了元素表,还**排在真输入框前面被先命中** → `输入 ["Search Wikipedia", …]` 点在那个 1×1 幽灵点上。同时真正的输入框**一个字都没有**(没直接文本、没 placeholder,可见名称来自 `<label for>`,而采集只认 `aria-label`/`placeholder`)。**两处一起修**:排除人点不到的(≤1px/`opacity:0`/`clip` 裁没的) + 补齐可及名称(`aria-labelledby`→`.labels`→`title`)
- **fix(INV-9,误导错误)** 上面那个点空,驱动报的是「当前没有聚焦的输入框(**请先点击输入框**)」——把人和 AI 都引向"那我先点一下",也就是**引回坐标路线**;真实原因是上一步点空了。改为回报焦点落在什么标签上 + 指出多半是定位命中了同名非输入元素
- **fix(P-29,平台白等)** `atomic/control.rs` 的 `Input` 点击后固定 `sleep(500ms)` **等软键盘**——那是移动端才有的东西,web 上纯白等(占该步耗时 ~38%)。下沉到 `Controller::has_soft_keyboard()`。**与 P-27 同族**:一个语境下正确的等待被搬到不需要它的语境,不报错、只是悄悄变慢
- **docs(P-30,文档坑)** 文字定位**只看得见视口内的元素**,目标在折叠下方时直接失败(还白等满 ~6s、整批中断)。破解它的 `滚动查找 ["文字", 方向]` **能力一直都在、且不需要元素库**,但 `steps-syntax.md` 把它标成"需要元素库",而这个 skill 明令不建元素库 → **调用方 AI 一次都没用过它**。与 90d9dcad **同型**(能力早就有,只是没告诉 AI / 告诉错了)
- **验证(本机无头,前后对照)** 同一步 `输入 ["Search Wikipedia", …]`:修前**失败**(点到 1×1 幽灵)→ 修后 **1315ms 通过**→ 去掉 500ms 白等后 **886ms**;整批 3 步**纯语义、零坐标、零 fetch**。`点击 ["Memory safety"]`:不滚 = 失败+白等 **9.1s**,先 `滚动查找` = **0.4s** 找到 + 点中
- **量到的 token 事实** 一次 `fetch --interactive` = **32KB(≈8K token)**,而 fetch 本身只要 237ms——**贵的从来不是时间,是每次重新 fetch 的那张表**。这就是坐标路线烧 token 的根因,与上一场的诊断对上了

### 2026-08-15 · `fetch --wait-text`:把"等文字出现"从提示词变成子命令(ADR-0013,关闭 Q-8)
skill 不建元素库 → 重试断言用不了 → 等异步下发只能让调用方 AI 手写 shell 轮询,
护栏全在措辞里。三个坑(忘超时/忘判命中/误加 `--interactive`)每个都产生**假结论**,
**第三个是写 SKILL.md 的我自己第一版就踩的**。
- **feat** `tke -d <设备> fetch --wait-text <文本> [--timeout <秒>]`:**出现即刻返回**(不是死等满)、正常输出元素表退出 0;超时**非零退出**,`||` 与 `$LASTEXITCODE` 天然接得住。查**全量**元素(不受 `--interactive` 影响),多候选 `"A|B"`,匹配口径与 `滚动查找` 共用 `utils::scroll`
- **docs** SKILL.md / pitfalls.md / steps-syntax.md / tke-commands.md 里的手写轮询范例**全部替换**;新增坑册 C-15(视口外要先滚动查找)、C-16(点到人看不见的同名元素)
- **依据** ADR-0010 早就写过"护栏退化的出路是做成必须调用的子命令,不是把提示词写更长";这次 `滚动查找` 被文档写错而无人使用,是**靠文档传递能力有多脆**的独立佐证
- **验证** 命中 0.45s 退出 0 / 超时 5.45s 退出 1;CLI 契约测试 +2。lib 62/62 + CLI 23/23

### 2026-08-15 · 审计:wda/web 没有 adb 同款"无限挂"(关闭 Q-4)、移动端没有 P-27 式白等(Q-10)
- **Q-4 关闭** `web`/`wda` 全部 **17 处 ureq 调用 1:1 都带 `.timeout()`**,所有等待循环都是有界的 `(0..N).any(…)`、无 `loop{}`。根本差异:adb 是 spawn 子进程(**没有任何自带超时**,故 P-03 要全链路兜底),web/wda 走 HTTP 由 ureq 兜底;外部进程(chromedriver/go-ios)是 spawn 后台化 + 有界轮询探活,不阻塞
- **Q-10 部分回答** 移动端**没有** P-27 那种"读不到值→等满"的静默退化——`adb`/`wda` 的 `tap` 后**根本不等**(web 才有 `wait_ready`)。反而查出**反向**同族问题(P-29:web 吃了移动端的软键盘等待,已修)
- **留给真机的** adb 每次采集要 **6 次进程往返**(`screencap`+`pull`+`rm`、`dump`+`pull`+`rm`),`input_text` 另有固定 500ms(输入法切换,adb 特有且有理由)。这是安卓侧"慢"的最大嫌疑,但**本机无设备量不了**——按 P-27 的教训**先量再改**,量法与结论记在 Q-10
用户反馈"等待太多、拖慢整体速度"。顺着量下去,发现**真凶不是 AI 写的等待,是 tke 自己在白等**。
- **fix(P-27,静默退化)** `WebDriver::execute()` 返回的是**已剥掉 `{"value":…}` 外壳**的结果,但两处调用方又多解了一层:①`wait_ready` 里 `document.readyState` **永远读不到 `complete`** → **每次点击都白等满 20×200ms + 400ms** ②`center_into_viewport` 里视口尺寸永远读不到 → 一直用硬编码兜底 1280×800,**坐标夹紧会算错**。两处都不报错、只是悄悄退化——**量了耗时才逼出来**(点击 4899ms vs 单次采集 110ms、原子点击 14ms)
- **效果** 每步 **4899ms → ~750ms(6.5×)**。按用户那次 47 步算:3.8 分钟 → 约 35 秒
- **feat** 文字定位补上**隐式等待**(12×500ms,与元素定位同一套):此前只采集一次、找不到就失败,调用方只能到处垫 `等待 [1s]` 兜底。现在**元素已在就立刻返回**、没渲染完才等,且能等够 6 秒(比死等 1 秒更可靠)
- **docs** SKILL.md 与坑册 C-14:**默认别写 `等待`**——定位自带隐式等待、点击也会等到页面就绪;只有"没有对应元素的过程(动画/toast)"和"后端异步下发(用轮询更准)"才需要。示例里那些 `等待 [1s]` 全删了(**是我在示例里带头写的**)
- **验证** 延迟 2.5s 才渲染的元素:不写等待照样点中;真实跨站跳转:确实等到新页面就绪(内容已是 iana.org)——**快的是不该等的地方,该等的一秒没少**

### 2026-08-14 · 治 token 爆炸的根因:从坐标路线掉头到语义定位(用户实测反馈)
用户跑一个跨端任务烧光了一整个 opus 会话。拉他的报告数出实据:**20 个批次 / 47 步,平均每批只有 2.35 步,其中 22 步(47%)是「等待」;坐标操作 23 步、语义操作 0 步。**
- **诊断** 根因**不是 tke 不能干,是 SKILL.md 把 AI 引到了最费 token 的那条路上**:用坐标就必须先 `fetch` 全量元素表,而坐标一变就失效 → 每两三步重新 fetch 一次 → 20 批 × 大 JSON。**语义定位的能力 tke 早就有**(`resolve_text` 在每步执行时实时刷新页面并按文字定位),实测 `点击 ["Learn more"]` 直接可用
- **docs(最要紧的一改)** SKILL.md 掉头:**首选 `点击 ["保存"]`,坐标降为兜底**。文字在**执行那一刻**才解析,所以能**一次传五六步**而不怕页面变——批次数掉下来,fetch 次数跟着掉。且 `点击 ["保存"]` 可读可复用,顺带解决用户担心的"坐标不利于后期发展"。**这不违反 ADR-0010**:文字定位不产 `.tklib` 资产
- **feat(真能力缺口)** 新增 **`选择`** 指令:原生 `<select>` 展开后选项由**浏览器绘制**、DOM 里不可见(`getBoundingClientRect` 为 0),点击路线**根本走不通**——用户只好绕道 python 读页面。现在直接走 DOM 设值 + 派发 input/change 事件(不派发的话 React/Vue 收不到)
- **fix** `<select>` 采集特判:此前只取**直接文本节点**,而文字全在子 `<option>` 里 → text 恒为空;option 自身又因不可见被过滤 → **AI 完全不知道有哪些选项**。现在带出当前值 + 全部可选项(`options` 字段),选错时报错也会把可选项列出来
- **fix** 全流程报告**跨批次连续编号**:此前每批各自从 1 开始,读起来像好几段互不相干的测试拼在一起(`01 02 | 01 02 | 01 02 03 04`)
- **docs** 坑册加 C-12(坐标烧 token 且不可复用)、C-13(原生 select 点不开);steps-syntax 首选写法改为文字
- **测试** lib 62/62 + CLI 21/21 + bin 3/3;`选择` 指令实测:按文字定位下拉框→选中→fetch 确认值真的变了;选不存在的项报错会列出可选项

### 2026-08-14 · 安装/卸载输出精简(用户逐条反馈)
- **change** 分节标题统一成**英文大写**:`SKILL` / `DEPENDENCY` / `DOCTOR` / `REMOVED`(不再用中文标题)
- **change** 文案砍到最短:头部三项挤成一行(`tke 0.7.4-beta · darwin-arm64 · all`);Chrome 那句"已在 …（换版本先删这个目录）"删掉;PATH 两行并一行;结尾一句话 + 一行卸载命令
- **change** `tke fix` 的「环境/状况」**两段并一段**——分两段会与安装器的分节套在一起、还把平台报了两遍;安装器也不再单开「体检」节
- **change** 卸载**只报实际发生的事**:不存在的默默跳过(不再列"没有检查记录""没有安装 Chrome"),保留了什么压缩成结尾一句 `保留 logs(-Logs) · chrome(-Chrome)`
- **change** 卸载器用回 **ENGINE** 的 LOGO(品牌只有一个,不另做 UNINSTALL 字样)
- **feat** Chrome 下载**显示进度条**(几百 MB,静默会让人以为卡死);其余小文件仍静默。PowerShell 侧临时开 `$ProgressPreference`,只对大文件开——管道执行时它会刷屏
- **fix** PowerShell 又扫出 4 处 `$变量中文`(P-24 那个坑),已全部加花括号;自查命令已在 PITFALLS

### 2026-08-14 · 安装/卸载体验:LOGO + 配色 + 一行卸载
- **feat** 安装器加 TOOLKIT ENGINE 的 ASCII LOGO,输出改成**符号 + 颜色**(`▸` 分节 / `✓` `!` `·`),**不用 emoji**——等宽终端里对不齐、SSH/CI 日志里常变方块。`tke fix` 的输出同步到同一套(用户反馈"CLI 输出也不好看")
- **feat** **一行卸载**:`uninstall.sh` / `uninstall.ps1`。默认删 skill + tke/驱动 + PATH 行,**默认保留**检查记录(跑过的证据)与 Chrome(几百 MB);`--logs` / `--chrome` / `--all` 显式加。带 `--dry-run` 先看会删什么;改 rc 文件前先备份
- **fix(用户发现)** macOS 上不该找 `libc++.so`——那是 **Linux 版 aapt** 的运行时依赖(RUNPATH 含 `$ORIGIN`)。无条件装会在 mac/Windows 上请求一个不存在的文件、拿到 404
- **fix(PowerShell 两个标识符坑,P-24)** ①变量名**不区分大小写**:`$T`(颜色)被参数 `$t` 覆盖→标题打两遍;局部 `$logs` 覆盖 switch 参数 `$Logs`→赋值直接报类型错 ②变量名**可以含中文**:`$Ye试运行` 整个被当变量名、那三个字消失——**与 bash 的 P-20 如出一辙**。③函数名 `Remove-Item-Reported` 撞内置 `Remove-Item` 致参数绑定错乱
- **fix(P-25)** `Invoke-WebRequest .Content` 可能是 **byte[]**:版本号显示成 `116`(那是 `'t'` 的 ASCII),更坏的是 `build:` 戳解析不出来、**破 CDN 缓存的键悄悄失效**而表面正常
- **验证** 装了 pwsh 7.6.4 在本机真跑:install/uninstall 两个 ps1 语法通过 + 模拟 Windows 环境跑通(落地名正确补 `.exe`、DLL 保持原样);bash 版走完整安装→卸载闭环,试运行确认一个字节没删、logs 默认保留、rc 改前有备份

### 2026-08-14 · 宿主机能力门禁:做不了的组合直接拦下并说清
- **feat** 新增 `utils::capability`:**iOS 只在 macOS 放行**,Windows/Linux 上碰 iOS 设备直接拦下,报错说清**为什么**(设备上的 WDA 要用 Xcode 装一次,Xcode 只有 mac 有)、**这台机器能做什么**(web/安卓)、以及**逃生口**
- **落点** 门禁放在 `Controller::new` —— 所有设备操作的**唯一必经之路**,`control`/`run`/`steps`/`harness` 一处覆盖,不会漏
- **feat** 源头也不摆做不到的选项:`list_devices`(给编排官的)与交互式向导在非 mac 上**不列 iOS**——摆出来只会让人/AI 选一次、撞一次门禁、再回来重选
- **feat** `tke fix` 在非 mac 上不报"缺 go-ios"(补上也用不了),并说明原因
- **fix(误导措辞)** `tke fix --check --profile ios` 在 Linux 上原本显示"✅ ios 需要的依赖都在"——**这台机器压根做不了 iOS**,说"依赖都在"是骗人的。说明被 early return 跳过了,已提到列缺失之前,措辞改成"没有可补的依赖——这台机器做不了 iOS"
- **留了逃生口 `TKE_ALLOW_IOS=1`**,因为**这条界线是产品决策不是技术极限**:go-ios 本身跨平台、运行期也不需要 Xcode(经 testmanagerd 拉起 WDA),真正卡住的是那次一次性安装。"WDA 已装好的设备接到 Linux CI"技术上是通的,不该被堵死
- **docs** SKILL.md 的设备表加"哪些机器能做"一列;`tke fix --check` 会告诉你这台机器能做什么
- **测试** 4 条新单测(web/android 恒放行、iOS 按宿主机分且报错要说清原因与替代、逃生口、可选平台列表);`profile_scopes_what_is_checked` 随行为变更改成按宿主机分支断言。lib 61/61 + CLI 21/21 + bin 3/3

### 2026-08-14 · Windows 这条路补通（同事主力平台）
- **feat** **`install.ps1`**:Windows 一键安装器,与 install.sh 一一对应。此前 Windows 同事**根本装不上**——`install.sh` 是 bash,而那句"请用 install.ps1"指向的文件压根不存在
- **feat** **体检并进 `tke fix --check`**:除了列缺失依赖,还报安卓设备/版本比对/证据落点/有头还是无头。**一份 Rust 实现三平台通用**——`check-env.sh` 是 bash,Windows 用户跑不了,而 Windows 恰恰是"同事跑完 Claude Code 要验一遍"的主力。SKILL.md 第 0 步已统一成这条
- **docs** SKILL.md 与坑册的 shell 片段**补 PowerShell 版本**(轮询、Select-String、`$env:USERPROFILE\.tke\logs\`):Windows 上 Claude Code 用 PowerShell,`grep -q` / `for i in $(seq)` 直接跑不了
- **fix** CI 与 `publish.sh` 都只发 `install.sh`——`install.ps1` 不带上等于没做,已补
- **验证** 本机装了 pwsh 7.6.4 专门验这个(没跑过的脚本等于没写,今天已吃过一次亏):语法解析通过 + 抽出核心函数真跑——文件头校验(真 gz 通过 / HTML 被拦下)、gzip 解压内容正确、build 戳解析、落地名补 `.exe` 的规则;`$Profile` 作为参数名(PowerShell 自动变量)实测在脚本作用域内可用。云上那份取回来再验一次语法
- **change(措辞)** 版本比对不再摆 ⬆️ 箭头:本地可能是刚编的、比分发源还新,箭头会让人以为该更新。改成如实报"不一致"

### 2026-08-14 · 平台补到六个 + 摸清上游的三条边界
- **feat** CI matrix 加 **linux-arm64**(`ubuntu-24.04-arm` runner)与 **windows-386**(`i686-pc-windows-msvc` 交叉编译),构建步骤支持 `--target`
- **deps** 补齐 **win32 全套**(chromedriver + Chrome 152 + adb/aapt + 两个 DLL,都是 i386)与 **linux-arm64 的 go-ios**(ELF aarch64)
- **fix(差点传错)** win32 的 go-ios 我一开始是从 amd64 直接拷的——**上游的 go-ios Windows 包只有 64 位**(PE32+ x86-64),32 位跑不了。逐个 `file` 验架构时抓出来,已移除
- **事实(实测,非推断)** ①Chrome for Testing **只出 5 个平台**(linux64/mac-arm64/mac-x64/win32/win64),`linux-arm64` 与 `win-arm64` 直连 **404** ②Google 的 platform-tools **不出 arm64 Linux 版**(三种命名全 404) ③go-ios 的 Windows 包只有 64 位
- **feat** `tke fix` 知道这些边界:arm64 Linux 上直说"上游没有官方驱动,请 `apt install chromium-driver adb` 再软链到 tke 同目录",而不是让人对着下载失败反复试;32 位 Windows 不再报"缺 go-ios"(报了也补不上)
- **决定** **windows-arm64 有意不做**:Windows on ARM 自带 x64 模拟,windows-amd64 那套直接能跑;而 Chrome for Testing 也没有 arm64 Windows 版,单出一份只多一套要维护的东西

### 2026-08-14 · Windows 的 adb 还缺两个 DLL（用户提醒）
- **fix(Windows 上 adb 直接起不来)** `adb.exe` **直接依赖 `AdbWinApi.dll`**,USB 还要 `AdbWinUsbApi.dll`(由前者**运行时加载,不在导入表里**)。我第一版只传了 adb.exe,Windows 上根本跑不起来——**跟 Linux 版 aapt 缺 libc++.so 是同一类问题**,是用户想起来问才发现的
- **verify** 用 `objdump -p` 把四个 Windows 二进制的导入表都查了一遍:`aapt.exe` / `chromedriver.exe` / `ios.exe` **都自包含**(只用系统 UCRT 与系统 DLL),只有 adb 需要补。两个 DLL 已上传
- **feat** `tke fix` 的**伴生文件按平台分**:Linux 带 `aapt`+`libc++.so`,Windows 带 `aapt`+两个 DLL,mac 带 `aapt`。另外「adb.exe 在但 DLL 不在」这种半装状态(从别处拷 adb 过来最容易出现)现在也会被检出并补齐

### 2026-08-14 · 补齐四平台依赖 + 修 Windows 落地名
- **fix(Windows 上必炸)** `tke fix` 下载的二进制**落地时没补 `.exe`**——分发源上统一叫 `adb.gz`,Windows 落成一个没有扩展名的 `adb`,**根本执行不了**。现按平台补回扩展名(`libc++.so` 这类本身带点的不动)
- **deps** 手工补齐 **darwin-amd64 / windows-amd64** 两个空白平台:chromedriver + Chrome for Testing(Stable **152.0.7977.42**,driver 与 Chrome 同版本配对)+ adb + aapt + go-ios。逐个验过解压出来的架构:mac 是 Mach-O(chromedriver x86_64、其余 universal),win 是 PE32/PE32+(adb/aapt 是官方原样的 32 位)
- **注意** 现有 darwin-arm64(149) / linux-amd64(151) **有意不动**——`install.sh` 对已存在的 Chrome 目录是跳过的,升 driver 不升 Chrome 会版本不配对起不来。各平台内部配对即可,跨平台不必一致
- **docs** `install.sh` 里指向了一个**不存在的 `install.ps1`**,改成实话:Windows 手工放 tke.exe + `tke fix` 补依赖
- **change** CI 定位按用户要求收窄:`tke-deps.yml` 降级为"要整体升 Chrome 版本时才跑",**CI 的日常职责只剩「tke/skill 改了能发新版」**

### 2026-08-14 · GitHub Actions 发布流水线
- **feat** `tke-publish.yml`(常用):四平台构建 tke(darwin-arm64/darwin-amd64/linux-amd64/windows-amd64)+ 打包 skill + 刷新 VERSION,一键发到分发源。开关:`targets` 选平台、`ocr`(online 默认/full 含离线 tesseract/none)、**`skill_only`** 只改文档时一分钟发完、`dry_run` 验流程
- **feat** `tke-deps.yml`(低频):抓 Chrome for Testing + chromedriver + adb + aapt/libc++.so + go-ios。**driver 与 Chrome 从同一份官方清单的同一版本取**——版本必然配对,这是自建分发源最实在的价值
- **重要** **上传顺序:VERSION 最后传**。它的 build 戳是破 CDN 缓存的键,先传的话使用者拿新键去取还没传完的文件(P-19)。传完还会**从分发源真取一遍复验是 gzip 而不是 HTML**
- **fix(不跑就发现不了)** go-ios 的 zip **三个平台三种结构**:linux 里是 `ios-amd64`+`ios-arm64` **两个架构**,mac 是单个 `ios`,win 是 `ios.exe`。原写法 `find -o | head -1` 取目录遍历顺序,**在双架构包上会选错架构**;已改成按架构名优先级逐个找
- **验证** 三段下载逻辑**全部从 YAML 里抽出来本地实跑**:android(adb/aapt/libc++.so 三件到位)、ios(三平台各拿对架构,linux 那个确认是 x86-64)、chrome+driver(Stable 152.0.7977.42,driver 解压后版本一致、chrome zip 解压即 `chrome-linux64/` 结构)。**CI 脚本不本地跑一遍等于没写**
- **docs** `docs/ci-publishing.md`:两个 workflow 怎么用、Secret 怎么配、各家下载源的实测结构

### 2026-08-13 · 全流程报告:一次检查一份,不再是一堆碎报告
- **fix(设计缺陷)** AI 做一次检查要调很多次 `tke steps`(看页面→操作→再看→再操作),每次留下一个 `steps_<时间戳>/` 和一份独立 report.html。**人要审核时面对十几份碎报告,根本没法读**(用户提)
- **feat** `steps` 每批跑完**自动重建**父目录的 `report.html`:所有批次按时间接成一条时间线,每批带批次头(序号/设备/时刻/步数/跳回单批链接)。**AI 什么都不用做**
- **feat** `tke report <目录> [--embed]` 显式汇总:跨设备时证据分在 `web/` 与 `phone/` 子目录,自动重建只管到各自那层,收尾跑一次就把两台设备的批次**按时间交错**排成一条线——正好还原"平台侧做了什么 → 手机侧看到什么"的因果
- **取舍** 全流程报告默认**相对链接**引用截图(3 批 12K,重建极快);`--embed` 内嵌成单文件(420K)供贴工单/发群。单批报告仍然内嵌,它本来就要能单独发
- **refactor** 提出 `Ctx{run_dir,prefix,img}` 与共用 `BASE_CSS`:两份报告长得不一样会让人以为是两个工具出的
- **测试** 新增 5 条,钉的是**会读错因果**的地方:跨子目录必须**按时间**排(不是按目录名)、步数跨批累计、空目录要报错而不是产出骗人的空报告
- **样例** https://cloud.test-toolkit.app/sl/preview/guest/test/AI_Reference/tke-session-report-sample.html
- **测试** lib 57/57 + CLI 契约 21/21

### 2026-08-13 · 读图策略:该看的时候必须看(坑册 C-11)
- **docs** SKILL.md 此前只强调「每步都读图会让 token 爆掉」,**引导过头**——AI 可能一张图不看就下结论。现在给出**必读判据**:下最终结论前、结果与预期不符时、操作后页面没如预期变化时、要判断布局/颜色/选中态/图表/图片时
- **docs** 新增坑册 **C-11「从不读图 → 拿'元素存在'冒充'功能可用'」**,与 C-9(每步读图爆 token)互为反面、双向交叉引用。文本能证明"节点在控件树里",**证明不了"用户看到的这一屏是对的"**——渲染失败/被遮挡/颜色错/图没加载,元素表里全都一样
- **docs** 算清成本再讲取舍:一张图约上千 token,二十步几万确实爆;但**一次检查读两三张关键的可以忽略**。省 token 省到不敢看结果是本末倒置
- **docs** 指明读**标注截图**(`steps --log` 已存好,路径在输出的 `screenshot` 字段)——带操作横幅/元素框/点击点,比重新 `refresh` 一张信息量大

### 2026-08-13 · `tke fix`:一条命令补齐运行依赖(ADR-0012)
- **feat** `tke fix` 从分发源补齐 chromedriver / Chrome for Testing / adb(+aapt+libc++.so) / go-ios。`--profile web|android|ios|all`、`--check` 只看不下(缺东西时**退出码非 0**,CI 可判)、`-y` 免确认、`--base-url` 换源
- **decision** **下载只在这条命令里发生**,普通命令缺依赖只报错指路。一条 CLI 命令突然静默拖 600MB,在内网/离线/CI/按流量计费的机器上都是灾难,企业还有合规问题——**要不要下是使用者的决定**(用户拍板)
- **fix(误导报错)** 缺 chromedriver 时先跑 `fetch` 会报「无活动浏览器会话,请先执行 启动」——**指错方向**,让人撞第二堵墙才看到真原因。现在先分清"还没启动"和"根本装不了"
- **fix(误导报错)** 缺 Chrome 时只报 `session not created (日志: …)`,完全看不出缺的是浏览器本体。现在检测到没有 Chrome for Testing 就补一句说明 + `tke fix --profile web`
- **fix(自己犯的假成功)** 第一版 Chrome 解压失败(zip 关了 deflate 特性),但**半个解压出来的目录留在那儿**,复验只看目录存在就报「✅ 补齐了」+ 退出码 0。判据已改成**可执行文件在不在**,且解压失败会清掉半成品。**一路在防的假成功,自己犯了**
- **fix** `zip` crate 补 `deflate` 特性——Chrome 官方包是 deflate 压的,只留 stored 会报 "Compression method not supported"
- **refactor** 新增 `utils::deps`:Chrome 路径(`CHROME_REL`)与工具探测**驱动层和 fix 共用一份**。各写一套会出这种怪事:fix 说装好了、驱动却找不到
- **choice** 下载走 `curl` 子进程而非 Rust HTTP 客户端:reqwest 是 `ocr-online` 的可选依赖,CI 的 `--no-default-features` 构建里没有,而 fix 必须在任何构建下都能用;tke 本来就是"调外部工具"的架构
- **fix** 手写的 `cli/help.rs` 又漏了新命令(P-16 同款),测试抓住
- **实测** 空目录只放一个 tke → `tke fix -y --profile web` → chromedriver 20MB + Chrome 600MB 装齐 → **用装出来的环境真跑通一次网页检查**;android 那套也验了(含顺带的 aapt/libc++.so)、幂等、非交互不确认不下载
- **测试** lib 54/54 + CLI 契约 19/19(新增 3 条)

### 2026-08-13 · 报告三连:点了什么 · AI 写的评语 · 相关文件按钮
- **feat** **「点了什么」**:脚本里写的是 `点击 [{299, 242}]`,光看这行没人知道点的是啥。报告从**执行时的页面结构**反查该坐标命中的元素(取**最内层**那个),展开可看 class/text/resource-id/xpath/范围/可点击性,来源带平台前缀(web/android/ios)
- **fix(会悄悄标错)** 反查必须用**上一步**的 xml——每步存的是**动作执行后**的页面(点完早跳走了),拿本步的查会把"点了什么"标成"点完到了哪"。专门加测试钉住
- **feat** **点空必须说出来**:坐标没命中任何元素 → 红标「点了个空处,这一步多半没起作用」。tke 本身仍报 success(驱动层不校验),这是眼下唯一能拦住这种**假成功**的地方
- **feat** **`.tks` 支持行内注释** → 成为报告里的「这一步在干什么」。`点击 [{927,112}] # 点保存,验证会落库` 原样进报告。**写指令的 AI 是当时唯一知道意图的人**,不写下来这信息就永远丢了
- **fix(会切坏指令)** `#` 只在**引号外**才算注释:URL 锚点 `"https://x/#/list"`、文本 `"话题#标签"` 都是数据;且要求 `#` 前面是空白(`KEYCODE#1` 不算)。5 个测试钉这几种
- **change(自省)** 一度写了套**规则生成的评语**("点击了链接「X」"),被用户否掉——定型文不够灵活,更坏的是**让人以为读懂了其实没有**。改成只显示 AI 真写的那句,没写就不占位置
- **feat** 顶部「相关文件」从一行链接改成**一排按钮**(查看原始日志/截图序列/页面 XML/打开执行目录),文案说「点了会看到什么」而不是裸文件名;删掉页脚两句废话
- **feat** 顶部补:设备、脚本路径、起止时刻、run_dir;chips 改成通过/失败/AI找回/总步数/耗时(失败与 AI 找回只在非零时出现)
- **feat** `ExecutionResult` 加 `device`、`StepResult` 加 `note`(都是可选字段,App 的 NDJSON 消费不受影响)
- **测试** lib 52/52 + CLI 契约 16/16;样例 https://cloud.test-toolkit.app/sl/preview/guest/test/AI_Reference/tke-report-sample.html

### 2026-08-13 · `--log` 时自动生成人看的 `report.html`
- **feat** 一次运行的 log.json + 截图序列缝成**一个自包含 HTML**:顶部结论(通过/失败·N/N 步·耗时)+ 每步命令/成败/耗时/报错/标注截图。`steps` 与 `run` 共用这条路径,**不用加任何参数**
- **取舍** 截图 **base64 内嵌**(单文件发给同事/贴工单也能看图,人最需要的);页面结构 xml **不内嵌**(动辄几百 KB、只有 AI 排障才看),留相对链接、原目录打开可用
- **取舍** CSS 全内联不引 CDN——离线/内网/断网 CI 里打开都一样;支持 `prefers-color-scheme` 深色
- **fix(自省)** 报告生成失败只 `warn` 不中断:证据本体(log.json/截图)已经落好了,不该因为报告生不出来把整次运行判失败
- **测试** 4 个单测挑的都是**会悄悄坏掉**的地方:HTML 转义(一个带 `<` 的报错就能把报告打歪)、失败信息必须出现(INV-9)、截图读不到时报告照出、汇总数字正确
- **样例** https://cloud.test-toolkit.app/sl/preview/guest/test/AI_Reference/tke-report-sample.html

### 2026-08-13 · skill 默认装用户级 + 证据默认落 `~/.tke/logs`
- **change** `install.sh` 默认 `--user`(`~/.claude/skills`,装一次所有项目通用),`--project` 才装进当前仓库。此前反过来——每换一个项目就得重装一次(用户提)
- **change** 证据默认落 **`~/.tke/logs/<任务简称>/steps_<时间戳>/`**,不再往被检查的项目里写。它是一次性检查的过程产物,**不该混进人家仓库、也不该逼人加一条 `.gitignore`**。时间戳子目录 tke 自动建,AI 只给任务简称
- **docs** 同时给 AI 留了改写口子:证据要跟 PR 走 / 工具链只能读项目内文件时,改用 `--log .tke-ui-test/`,**那时才提醒用户加 `.gitignore`**
- **feat** `check-env.sh` 新增「证据落点」一段,直接报 `~/.tke/logs` 及已有几次记录——人找证据不用问 AI
- **实测** `~` 展开正常、目录自动创建、体检计数正确;两个脚本 `bash -n` 过

### 2026-08-13 · skill 更名 ui-check → **tke-ui-test**（用户定名）
- **breaking(分发)** 目录、frontmatter `name`、斜杠命令、分发包名全部改:`skill/tke-ui-test/`、`/tke-ui-test`、`skill/tke-ui-test.tar.gz`。**三者必须一致**才认得出
- **fix** `install.sh` 装完自动清除旧的 `ui-check` 目录——不清的话两个 skill 同时在册、description 几乎一样,AI 会乱挑、用户也看不出该用哪个
- **注意** 云上 `skill/ui-check.tar.gz` 是旧路径,下次 publish 会传新名;**老包不会自动消失**,需要时手动删
- **实测** 三个脚本 `bash -n` 过;本机重装 + 体检全绿(tke 0.7.3-beta / chromedriver 151 / Chrome 就位)

### 2026-08-13 · skill 拆出踩坑册 + 澄清「不产 .tks」
- **docs** 新增 `reference/pitfalls.md`(C-1~C-10):**专收"会得出假结论"的坑**——不是跑不起来,是跑起来了结论是错的。主文件只留"怎么做",坑册收"为什么会错",**新踩的坑往坑册加、不再撑大 SKILL.md**(用户提)。SKILL.md 214 → 173 行
- **docs** `reference/tks-syntax.md` → **`steps-syntax.md`**:旧名暗示"要写 .tks",而这个 skill **只把指令当 `steps` 的命令行参数用**,不产脚本资产、不建元素库——产可回放脚本是 `tke harness` 的活,**两个东西**(用户强调)
- **fix(误导)** steps-syntax 里原写着"等异步结果**必用**重试断言"——但断言的目标必须是元素、**需要元素库**,skill 里根本用不了。改成指向 shell 轮询 + 坑册 C-1/C-3
- **docs** README 补**斜杠调用**:`/ui-check <任务>`;斜杠名 = 目录名 = frontmatter `name`,三者一致才认得出。装进 `~/.claude/skills/` 后**当场生效,不用重启会话**(本机实测:拷进去后 skill 立刻出现在可用列表里)

### 2026-08-13 · skill 补跨设备检查（在 A 上做，去 B 上验）
- **docs** `SKILL.md` 新增「跨设备检查」一节,针对"平台建场景 → 手机 App 验收"这类真实需求。此前只有两句话「按语义分别指定、不确定就问」,不够用
- **fix(重要)** **轮询找内容必须用全量 `fetch`,不能加 `--interactive`**——要验收的名字往往是**不可点击的文本标签**(标题/列表项文字),`--interactive` 只输出可点击元素会漏掉,于是等到超时、报假失败。我第一版片段就是这么写错的,实测才发现(example.com 的 `Example Domain` 正是只在全量里)
- **docs** 另外四条都是"别骗自己"类的:①**先验起点**(动手前确认 B 上还没有,否则看到的可能是旧数据=假成功)②轮询要有超时且**退出后必须判断有没有命中**(否则"循环跑完"被当成"通过")③手机侧**先下拉刷新**(App 只在进页面时拉一次,别把没刷新当成没下发)④**验"能用"不只是"能看到"**(点进详情、执行一次,只验列表有这行是漏检)
- **docs** 平台侧登录:tke 的 web 会话是独立 Chrome 实例、**不共享用户日常浏览器的登录态**,又不许代登 → 停下来让用户在那个有头窗口里自己登,**中途别 `control close`**(会连登录态一起销毁)
- **背景** `.tks` 的重试断言(`断言 [{元素}, 存在, 15s]`)**在 skill 里用不了**——它需要元素库,而 skill 明令不建元素库。所以跨设备等下发只能用 shell 轮询。是否给 tke 加个不依赖元素库的 `fetch --wait-text` 待定(Q-8)
- **实测** 本机 Linux + web 验了轮询片段的正反例:命中即刻退出、未命中如实报未出现

### 2026-08-13 · 两件套平台自包含（Q-6 关闭）
- **feat** `tke run foo.tks` 不带 `-d` 时,从同名 `foo.tklib` 的 `meta.json` 读**录制平台**兜底:web → `device="web"`(零参数即可回放)/ android → 放行走默认 adb 设备 / ios → 仍要求显式给,但报错附上录制时的 UDID 便于对照。平台认不出或没有元素包,照旧报缺设备
- **feat** `tklib::read_meta()`:zip 随机存取只读 meta.json,不解整包;全程 `Option`——读元信息失败绝不把回放拦下来
- **背景** meta.json 里的 platform 此前**只写不读**(注释写着"给后续留钩子")。INV-7 承诺「两件套拷到别的机器直接能跑」,差的就是这一口气:脚本不记平台,而元素包早就记了
- **实测** 本机 Linux 无头:造 web 两件套 → `tke run case.tks`(不带 `-d`)→ 提示「按元素包记录的平台回放：web」→ 浏览器实跑 **2/2 步、退出码 0**。lib 39/39 + CLI 契约 16/16(新增 2 条)

### 2026-08-13 · publish.sh 默认只打 tke
- **feat** 日常发布只打 `tke + skill + install.sh + VERSION` 四个文件;驱动/Chrome 改为显式 `--with-drivers` / `--with-chrome` / `--full`。**驱动几乎不变,云上已有的不会因为没重传而消失**,每次都传纯属浪费(用户提)
- **注意** `VERSION` 仍每次必传——它是破 Cloudflare 缓存的键(P-19),不传则使用者拿不到新 tke

### 2026-08-13 · 修 shell 变量名吞中文（macOS bash 3.2 崩溃）
- **fix** `publish.sh` 在 mac 上跑到一半崩 `line 67: SRC: unbound variable`——`$SRC` 后面紧跟中文逗号,**macOS 自带 bash 3.2 会把中文字节当成变量名的一部分**。全项目扫了一遍,`publish.sh`/`install.sh`/`check-env.sh`/`build-linux.sh` 共 6 处一并改成 `${VAR}`(P-20)
- **注意** 这不是 locale 问题:我在 Linux 上 `LC_ALL=C` 都复现不出来,是 bash 版本差异。**同一个坑在同一个脚本里犯过两次**(用户此前修过 `${pkg}`,commit 1d4d5e92),所以这次加了自查命令进 PITFALLS

### 2026-08-13 · 分发上线 Toolkit Cloud + 自动更新检查
- **feat** skill 体检加**版本检查**:跟分发源比对 `VERSION` 第一行,落后就提示更新命令;3s 超时、失败静默(离线/内网照常用)。解决"skill 一直用着旧 tke"
- **fix(重要)** 安装器**逐个校验文件头**(gz 的 `1f8b` / zip 的 `PK` / 版本号以 `tke ` 开头):分发平台对**不存在的路径返回 200 + 前端 HTML**(SPA 兜底),`curl -f` 只对 4xx/5xx 生效、完全拦不住——漏传一个文件就会把网页当二进制装进去(P-19)
- **fix(重要)** **Cloudflare 缓存 4h 且不认 `no-cache` 请求头**,传了新文件使用者仍下到旧的。现在 `VERSION` 里带 `build: <时间戳>`,install.sh 先破缓存取 VERSION、再用 build 戳作为所有下载的 `?b=` 键——发新版自动破缓存,同版本仍命中 CDN(P-19)
- **feat** Linux 依赖收齐并上传:adb(platform-tools 37.0.1)、aapt + **libc++.so**、go-ios v1.3.2、chromedriver/Chrome 151.0.7922.138。**Linux 版 aapt 单独拿出来跑不了**(缺 libc++.so),但其 RUNPATH 含 `$ORIGIN`,放 tke 同目录即可——两个脚本都已带上这个依赖
- **docs** 下载路径是 `/sl/preview/<mount>/<key>`(不是 `/<mount>/<key>`,后者是 SPA 页面);平台**不支持 Range 请求**(520),别用 `curl -r` 探文件头
- **实测** 端到端全通:从云端一行安装(含 170M Chrome)→ 体检全绿 → **用装出来的 tke 实跑检查 3/3 步通过、证据齐全**

### 2026-08-12 · skill 一键安装器
- **feat** `skill/install.sh`:`curl -fsSL <BASE_URL>/install.sh | bash` 一行装齐——按平台自动取 skill 文件 + tke + 对应驱动 + Chrome for Testing,写 PATH,自动跑体检。`--profile web|android|ios|all` 按需装(只测网页就不必拖安卓/iOS 工具);`--user`/`--project` 选装到哪;幂等,重复跑只覆盖不装重
- **feat** `skill/publish.sh`:把 skill 与二进制打包成约定布局到 `dist/`,`aws s3 sync` 上去即可。**把配对好的 chromedriver 与 Chrome 放同一批**——使用者不必再去查版本对应关系,这是自建分发源最实在的好处
- **fix(自省)** 安装器最初"体检失败也照样说装好了"——已改成如实反映:环境不完整时明确列出缺什么并**非 0 退出**(INV-9 的精神,自己写的脚本也该守)
- **实测** 本地 http server 模拟 S3 + 临时 HOME 全流程验证:缺 Chrome → 警告 + 退出码 1;Chrome 就位 → 体检全绿 + 退出码 0;**用装出来的 tke 实跑一次检查流程,3/3 步通过、截图序列与 log.json 齐全**
- **docs** `skill/README.md` 补一行安装 + 分发源布局说明(维护者视角)

### 2026-08-12 · skill 补完备性(用户质疑内容太薄,属实)
- **skill** 补 `reference/tke-commands.md`(元素采集/OCR、**安卓 app focus/list 拿包名+Activity**、file 文件系统、device 信息、原生直通、排查日志位置)与 `reference/tks-syntax.md`(全部指令+参数写法+重试断言)。主文件保持精简(AI 必读),细节按需读——分发物只有 skill 目录 + tke 二进制,没有源码可查,所以必须自包含
- **skill** 主文件补关键缺口:**安卓不知道包名就查 `app focus`/`app list`,别猜**(此前完全没提,安卓场景会卡死);图标无文字用 `fetch --ocr`;体检脚本路径写明确
- **skill** 新增 `skill/README.md`(给人读的安装说明):skill 目录两种装法、tke 及**同目录依赖**(chromedriver 必须与 tke 同目录,不搜 PATH)、Chrome for Testing 按平台落点与 macOS 三个坑、验证、常见问题

### 2026-08-12 · ADR-0011 harness 侧落地：设备成为工具参数
- **feat** `explore`/`navigate`/`replay_tks`/`resume_explore`/`optimize_tks` 五个设备类工具各加 `device` 参数——**编排官按任务语义决定每一步跑在哪台上**;不传则沿用默认(`-d`/向导),单设备场景照旧
- **feat** 新增 `list_devices` 工具:枚举 Android 设备 + web + iOS 说明 + 当前默认设备。**"按语义选设备"的前提是先知道有什么**
- **feat** 交互向导加「由 AI 决定」选项;**无默认设备不再拒绝启动**(此前报「需要指定操作目标」直接退出)——编排官会 list_devices/问用户
- **feat** 无设备时调设备类工具 → 明确报「先 list_devices…拿不准就 ask_user 问用户」。此前设备落成空串、被当 Android,只得到一句莫名其妙的「adb 缺失」(INV-9)
- **prompt** 编排官提示词加「设备怎么选」一节:不确定就问**绝对不要猜**(打错设备有真实副作用)、跨设备=多次 explore 各指定 device + `save_file` 写 flow.toml 串起来、**别把多台设备塞进一个 .tks**(脚本没有设备维度、回放不了)、等异步用重试断言
- **实现要点** `AgentRunOptions::with_device()` 造设备覆盖副本(平台按新设备重新推断,否则会拿上一台的平台去操作)

### 2026-08-12 · skill 定位纠正 + 重试断言 + run 设备必填
- **skill 定位纠正（用户）**:`skill/ui-test/` → **`skill/ui-check/`**。此前把 harness 的目标(可复用资产)错塞进了 skill——去掉「先 verify 后 explore / 产两件套 / 回放验证」那一套。**skill 只做:把设备操控+查看能力交给调用方 AI,并留下可复核的证据**,是改完代码后的一次性检查手段(类比单测/API 测试)。用坐标操作,不建元素库
- **发现** 证据落盘**零改动就有**:`tke steps '点击 [{640, 380}]' --log <dir>` 即产标注截图 + 页面 xml + log.json,用坐标不需要元素库。SKILL.md 据此改用 `steps` 而非 `control`(control 什么都不留)
- **feat 重试断言** `断言 [{提示}, 存在, 10s]`——第三参数=最长等待,在这段时间内反复采页重判,一成立就通过。用于等异步结果(后台下发/跨设备同步/请求返回);不给该参数则行为不变(采一次判一次)
- **fix** 步超时对**自带时长的命令**放宽:`断言`/`等待` 的步预算 = 自身时长 + 20s。此前 `等待 [30s]` 会被 20s 步超时掐死——代码注释早写了"等待命令也放宽"但**实现里根本没有**(P-08 同类)
- **feat** `tke run <.tks>` **必须显式 -d**(用户拍板):.tks 不记平台,不给会被当 Android、web 用例只得到一句「adb 缺失」。校验放在脚本/元素包检查**之后**——文件不存在、缺两件套是更基础的问题,先报那个(测试逮住过一次顺序退化)
- **feat** flow 校验:无全局 `-d` 时,逐项检查是否自带 device,缺的直接点名报错
- **实测** 重试断言对照(页面加载 15s 后才出现提示):不带等待 → **失败**「元素不存在」3.2s;带 25s 重试 → **通过** 10.9s。同时验证步超时放宽生效(25s 断言没被 20s 掐死)
- **test** lib +2(flow TOML 解析)、CLI 契约 13→14(run 设备必填);serialize 往返样例加重试断言

### 2026-08-12 · flow 支持跨设备（per-script device）
- **feat** flow 的 `scripts` 每项可指定设备:`{ path = "a.tks", device = "phoneA" }`,不指定则沿用全局 `-d`;纯字符串列表**完全向后兼容**。跨设备测试的表达方式定为「一个 .tks = 一个设备上的一段流程(两件套自包含,INV-7),跨设备 = 串成 flow」——用户场景:A 手机改设置 → B 手机验收 / web 后台下发 → 手机端看
- **fix** flow 收尾清场改为**按设备分组**:此前只按全局 `-d` 清一台,跨设备时其余设备会留下孤儿浏览器/App
- **adr** `ADR-0011`(**提案,待拍板**):设备从「会话级全局」降为「工具级参数」,harness 启动不再强制选设备、AI 按任务语义选、不确定问用户。关键取舍:**一次 explore 仍只跑一个设备**(explorer 内部零改动),跨设备靠编排官多次调用 + flow 串——因为**回放层没有设备维度**,多设备混合脚本回放不了
- **test** flow TOML 两种写法解析 + 老格式兼容(单测 2 条)

### 2026-08-12 · `control close` 可省包名（web）
- **feat** `tke -d web control close` **省略包名即销毁会话**（浏览器 + chromedriver + 会话文件 + 孤儿收割）——替掉此前要人手敲的 `rm -f $TMPDIR/tke/web/*.json` + `pkill Chrome`。web 分支本就忽略这个参数（`Controller::stop_app` → `close_session`），只是 CLI 强制要求填一个没意义的值
- **feat** 移动端省略包名 → 明确报错（不拿空串去 force-stop）
- **test** CLI 契约 +2（11→13）;文档里的手工清理命令全部替换

### 2026-08-12 · 无头坐标可移植性**已验证** + 两个真机撞出的修复
- **✅ 关键结论(用户 mac 实测 + 本机 Linux 对照)**:`mac 有头 = mac 无头 = Linux 无头 = 1280x813`,
  元素 bounds `diff` 零差异。**像素坐标跨模式、跨平台可移植——「本地录、CI 回放」成立**。
  这是 skill/CI 路线最大的未知,现已结掉
- **fix** 会话跨命令复用导致 `--headless` **静默不生效**(P-18):`SessionInfo` 增记 `headless`,
  复用前比对,模式不符则销毁旧会话 + 明确报错要求重新 launch。
  用户对照实验正是被这个坑出**假阳性**(两次结果一致其实是同一个浏览器)
- **fix** `--platform web` 不连带定 device → 下游按 Android 推断报「adb 缺失」(用户实测发现)。
  web 是唯一「设备 id 就是平台名」的端,现补成 `device="web"`,与交互式向导那条路径拉齐
- **验证** `tke harness` 在 mac 上跑通(有头,2 轮出两件套);无头 harness 待用户重跑

### 2026-08-12 · skill 模式落地 + 无头实测通过
- **adr** `ADR-0010` 生效(用户拍板):**skill 借调用方的 AI**——Claude Code 直接调 tke 原子命令,tke 退回成设备操作原语 + 证据产出器。**`tke task`(ADR-0009)取消**,该 ADR 标为已被取代(一行代码没写过)。`tke harness` 内置 AI 保留(App/纯 CLI 用户),两条路并存
- **skill** 新增可分发原型 `skill/ui-test/`(SKILL.md + check-env.sh):先 verify 后 explore、主循环用 `fetch --interactive` 文本元素表省 token、结束条件=`tke run` 回放通过(硬证据)
- **fix** `element add --lib foo.tklib` **包不存在时建新包**——此前 .tklib 只有 harness finalize 会造,用原子命令攒两件套第一步必失败(P-17)
- **实测(本机 Linux/amd64 无头)** 全链路通过:装 Chrome for Testing + chromedriver 151.0.7922.138 → 无头启动/采集/操作 → 落库建包 → 写 .tks → **`tke run` 5/5 步通过、退出码 0**;标注截图(横幅+红框+蓝点)、log.json、page/*.xml 齐全,**无头下中文渲染正常**
- **实测数据** 无头截图 **1280x813**(window-size 1280x900 减去 87px 浏览器 UI 高度,说明 headless=new 在模拟真实窗口)。**有头对照本机做不了**(无 DISPLAY、无 xvfb)——待 mac 上跑同样命令比对
- **未验** `tke harness` 的完整无头探索(需 `[ai]` key,本机无)。但 harness 与 run/原子命令**共用同一条 `WebDriver::start_new_session`**,驱动层无头已验
- **发现** 记 Q-6:`.tks` 不记平台,`tke run foo.tks` 不带 `-d` 按 Android 推断 → web 脚本报「adb 缺失」;而 tklib 的 meta.json 已存 platform,「拷走即跑」还差这一口气

### 2026-08-12 · web 无头支持（为无头服务器 / docker CI 铺路，**真机未验**）
- **feat** `--headless=auto|on|off`(全局参数 + config `headless`)。**auto 默认**:mac/win 恒有桌面;Linux 看 `DISPLAY`/`WAYLAND_DISPLAY`,都没有 → 无头。无头用 `--headless=new`(完整渲染路径,与有头一致;老实现的精简渲染截图对不上)
- **feat** 容器/root 自动加 `--no-sandbox --disable-dev-shm-usage`(探测 `/.dockerenv`、`/run/.containerenv`、uid==0);普通桌面保留沙箱
- **fix** `find_chrome_binary` 此前**只认 mac-arm64 硬编码路径**,Linux/Windows 上永远找不到 Chrome(只能回退系统 Chrome、版本可能不配对)。改为跨平台:搜索根=tke 同目录 + `<data_dir>/tke`,相对路径按 Chrome for Testing 官方 zip 原样结构(解压即用,便于自建 S3 镜像)
- **fix** `env_clear` 保留列表补 `DISPLAY`/`WAYLAND_DISPLAY`/`XAUTHORITY`——Linux **有头**模式下 Chrome 靠它们连图形栈,清掉直接起不来(P-15;mac/win 不看这些所以一直没暴露)
- **fix** `--headless` 裸旗标会吞掉后面的子命令(`tke --headless run x.tks` 里 `run` 被当成值)——加 `require_equals` + `value_parser` 白名单。`--copilot` 踩过同类坑,这次由**黑盒 CLI 契约测试当场逮住**(P-16)
- **test** 单测 4 条(HeadlessMode 解析/定案)+ CLI 契约 4 条(帮助登记/裸旗标不吞子命令/无效值明确报错/off 可接受);lib 32→36
- **注意** 手写帮助(`cli/help.rs`)不会自动收录新参数,靠 `help_lists_headless` 契约测试兜住
- **未验** ①有头录/无头回放的**像素坐标是否一致**(决定"本地录、CI 回放"成不成立) ②docker 系统库与中文字体(下载器解决不了,得靠 Dockerfile)

### 2026-08-12 · ADR-0009 拍板生效
- **adr** ADR-0009 提案 → **生效**（用户拍板）:headless 一次性模式命名定为 **`tke task`**(顶层命令,非 `harness --headless` 旗标——两者出口语义与 `ask_user` 行为不同,做成旗标会让"会不会阻塞问人"取决于一个 flag)
- **不变量** INV-3 补延伸条款:「对话层」不限定必须是 tke 自己的 REPL,外部 agent 调用时调用方即对话层;**headless 一旦自行决策即违反 INV-3**。这是本 ADR 的失效红线,写进不变量当锚点
- **状态** 契约已定、**实现未开始**;下一步做阶段 0(零改动包 `tke run`)还是直接阶段 1(`tke task`),待用户定

### 2026-08-12 · Linux 构建脚本
- **build** 新增 `build-linux.sh`:依赖预检(cc/cmake/pkg-config,缺了直接给 apt 命令)+ `--no-ocr`(走 `--no-default-features`,CI 用,跳过 tesseract 源码编译)+ `--quiet`;去掉 mac 专属 codesign,但保留「先删后拷」——Linux 上的理由是覆盖运行中二进制会 ETXTBSY(与 P-02 同做法不同因);产物 `--version` 跑不起来就 exit 1
- **实测** Linux/amd64 两条路径都通过:`--no-ocr` 9m33s / 28M;完整(含 tesseract) 3m17s / 34M(**注意:这是 tesseract-rs 已在 cargo 缓存里的增量耗时,冷机首次会久得多**);两者版本号注入均正确、落点 `bin/linux-amd64/tke`
- **实测** OCR 门控对照:`--no-ocr` 产物调 `tke ocr` 报 `ocr-offline feature not enabled`,完整产物报图像解码错——证明 feature 确实生效;两者都是明确报错 + 退出码 1(不静默,INV-9)
- **订正** 此前"没有 Linux 构建"的说法不准:`build-mac.sh` 的 case 本就有 Linux 分支,真正缺的是命名可发现性、依赖预检、CI 跳 OCR 开关

### 2026-08-12 · skill 集成设计（只有文档,无代码）
- **docs** 新增 `docs/skill-integration.md`:tke 作为 skill 融入 coding agent(Claude Code)工作流的设计稿——verify/explore 两动作分离、intent 意图契约、report 硬软证据分级 schema、skill 布局与安装、四阶段路线;首版范围 Web+Android
- **adr** 新增 `ADR-0009`(**提案,待拍板**):headless 一次性任务模式 `tke task`——五态出口+退出码,决策点不静默降级而结构化回传给调用方(调用方 agent 即 INV-3 所说的"对话层")。背景:Plain 前端 `supports_prompts()=false` 但 `await_answer` 仍阻塞读 stdin,非交互下属未定义行为
- **待办** 记入 ROADMAP:Linux 构建脚本缺口(现只有 mac/win),skill 若要落到 Linux 开发机/CI 需先补
- **test** 新增 `tests/cli.rs` 黑盒 CLI 契约测试(7 条:--copilot 裸旗标回归/两件套缺包/JSON error 契约等,spawn 真二进制,秒级)+ `tests/e2e/smoke.sh` 真机冒烟(需设备手动跑);测试三层放置定稿 ADR-0008;pre-push 纳入 CLI 契约测试
- **docs** 整理:`tke-flow.md` 更新到当前架构(去医生/repair_tks,补 resume_explore/navigate/页面契约/run 辅助驾驶);`codebase-map/refactor-plan/tke-overview` 归档进 `docs/archive/`;新增 `docs/README.md` 导航;引用同步

### 2026-08-12
- **治理** 落地项目治理体系:INVARIANTS/PITFALLS/ADR(0001-0007 补录)/state 三件套/ROADMAP/CHANGELOG/守卫脚本+hook;AGENTS.md 改造为路由+协议入口

### 2026-07-13
- **feat** `7c4138c9` 起始态对齐输出瘦身（compact 前端/顶格/分段空行）
- **fix** `db33162c` 辅助驾驶设备缺省不再静默失效（INV-9 的由来之一）
- **feat** `b7e30a2d` tke run 起始态对齐——开跑前导航回起始页,失败拒跑
- **feat** `75842dda` tke run AI 辅助驾驶——定位自愈两段分诊,不改脚本资产（ADR-0006）

### 2026-07-06 → 07-12（修复重建线,详见 ADR-0001/0004/0005）
- **refactor** `57ed54e7` 删除医生 agent,修复重建为断点续探
- **feat** `1f6d49b1` 定位级自愈 + workarea 并发竞态修复
- **feat** `1e58b91a` 页面契约:「断言页面」指令统一起始/终点校验
- **refactor** `87ef690a` 单次 agent 全部迁到 oneshot 强制工具调用
- **refactor** `181246bc` 删一键黑盒 repair,修复决策交编排官
- **feat** `a8fe8c07` navigate 导航原语;`e27eb6e8` replay 失败报告带逐步轨迹
- **feat** `8ac12699` explorer 提问经参谋中转 + 卡住升级梯度
- **fix** TUI 手写 inline 定稿系列（`07231866`→`46475c53`,ADR-0007）
