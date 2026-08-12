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
