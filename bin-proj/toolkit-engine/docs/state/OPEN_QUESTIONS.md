# 未决问题

只增删,不重写。格式:编号 / 提出时间 / 问题 / 阻塞什么。

## Q-1 (2026-07-13) 分诊层 2-5 真机质量未知
replace/wrong_page/path_changed/app_issue 只有 fake 测试,真实改版 App 上判断质量未验。
阻塞:对外宣称"改版不断档"的可信度。需要拿真实改版场景逼出。

## Q-2 (2026-06-25) 探索质量债(部分已修未复验)
web 小图标(eye/查看)落点识别、滚动查找关键词/方向策略、token 爆炸(横跳止损已做)、
工具/提示词按平台全面分类(只开了 hover 一个 gate)。见记忆 explore-doctor-quality-todo。

## Q-3 (2026-07-08) 真机 TUI 结果框/todo 行交错渲染错乱
疑 resize 或双 Summary 竞争,未复现未修。

## Q-4 (2026-07-06) wda/web 是否有 adb 同款"无限挂"风险
adb 全链路超时已加(P-03),wda/web 未审计。

## Q-5 (2026-07-13) 60s 步超时与对齐本地匹配准确率
真死页面要等 60s;本地匹配误判"不在起始页"会白烧一轮导航。待真机数据。

## Q-6 (2026-08-12) 两件套「拷走即跑」还差平台自包含
`.tks` 不记平台,`tke run foo.tks` 不带 `-d` 时按 Android 推断 → web 脚本报「adb 缺失」。
但 `foo.tklib` 的 meta.json **已经存了 platform**。是否让 run 在缺 `-d` 时从同名 tklib
读 platform 兜底(web→device="web";android→留 None 走默认设备;ios 的 UDID 不可照搬,仍需用户给)?
阻塞:INV-7「拷到别的机器直接能跑」目前还差这一口气。skill 实测撞出。
