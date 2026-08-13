# 变更记录（toolkit-engine）

只追加,不重写已有条目。每条带日期 + commit + 一句话;细节看 commit message（本仓库 commit 写得很全）。
更早历史直接看 `git log --oneline -- bin-proj/toolkit-engine`。

---

## [Unreleased]

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
