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
