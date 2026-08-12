# 交接单

**会话时间**: 2026-08-12（第二场）
**产出 commit**: 未提交（工作树内,纯文档）

## 做完了

- 通读项目并向用户汇报现状（结构/开发流程/提交历史）
- **skill 集成设计稿**:`docs/skill-integration.md`——tke 作为 skill 融入 coding agent
  (Claude Code)工作流。verify/explore 两动作分离、intent 意图契约、report 硬软证据分级、
  skill 布局与安装、四阶段路线。**首版范围 Web+Android**（用户拍板,iOS 缓）
- **ADR-0009 提案**:headless 一次性任务模式 `tke task`,五态出口 + 决策点结构化回传
- ROADMAP/CHANGELOG/STATE/docs README 同步;顺手修正 STATE 的 Last-Commit
  （上一场写 STATE 时那两个 chore commit 还没提交,字段停在 7c4138c9,已改 aedd2201）

- **`build-linux.sh`**（用户追加要求:开发机和 CI 都是 Linux）。依赖预检 + `--no-ocr`(CI)
  + `--quiet`;无 codesign,但保留先删后拷(Linux 理由是 ETXTBSY,不是 P-02 的签名)。
  **实测 Linux/amd64 `--no-ocr` 通过**:9m33s / 28M / 版本号注入正确 / 退出码语义正确

## 没做完

- **除构建脚本外一行代码都没写**（用户明确:先只写设计文档/ADR）

- **ADR-0009 已拍板生效**（用户,2026-08-12）,headless 命名定 **`tke task`**（顶层命令）。
  同时给 INV-3 补了延伸条款 + 失效红线。**契约已定,实现未开始**

## 埋的坑 / 需要后来人注意

- **`tke task` 可以开工了,但先跟用户确认从阶段 0 还是阶段 1 起步**（ROADMAP 有三阶段）。
  实现时守住 ADR-0009 四条契约,尤其:决策点必须回传不得自行决定（**违反即违反 INV-3**）、
  硬软证据分字段
- 实现阶段 2（intent 契约）时最容易踩 INV-5:**别把 intent 示例内容写进
  `prompt/builtin/*.md`**。intent 是运行时输入合法,提示词写死即泄题
- 设计稿指出两个既有事实,实现前请复核:①`PlainFrontend::supports_prompts()` 用默认 false
  但 `await_answer` 仍走 `read_user_line` 阻塞读 stdin（非交互下未定义行为）
  ②Linux 构建脚本缺失（只有 mac/win）
