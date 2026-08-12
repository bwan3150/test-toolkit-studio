# 交接单

**会话时间**: 2026-08-12（第三场，含用户 mac 侧实测回合）
**产出 commit**: `affaadbf` → `00a7ee6d` → 本次未提交（skill + 无头 + 实测）

## 做完了

- **web 无头支持**：`--headless=auto|on|off`；容器/root 自动 `--no-sandbox --disable-dev-shm-usage`。
  顺带修两个既有 bug：`find_chrome_binary` 只认 mac-arm64 硬编码、`env_clear` 清掉 `DISPLAY`（P-15/P-16）
- **`build-linux.sh`**（依赖预检 + `--no-ocr` CI 模式），两条路径实测通过
- **ADR-0010 生效（用户拍板）**：**skill 借调用方的 AI**——tke 退回成设备操作原语 + 证据产出器。
  `tke task`（ADR-0009）取消，该 ADR 标为已被取代（一行代码没写过）。harness 内置 AI 保留
- **skill 原型 `skill/ui-check/`**（SKILL.md + check-env.sh），可复制到使用者项目的 `.claude/skills/`
- **fix**：`element add --lib foo.tklib` 包不存在时建新包（P-17，skill 实测第一步撞出来的）
- **fix（用户 mac 实测撞出的两个）**：①会话跨命令复用致 `--headless` 静默失效——`SessionInfo`
  增记 `headless`，模式不符则销毁重建 + 明确报错（P-18）②`--platform web` 不连带定 device
  → 下游按 Android 推断报「adb 缺失」，现补成 `device="web"`，与交互式向导那条路拉齐
- **本机无头全链路实测通过**：Chrome for Testing + chromedriver 151.0.7922.138 → 无头启动/采集/操作
  → 落库建包 → 写 .tks → `tke run` **5/5 步、退出码 0**；标注截图/log.json/page xml 齐全，
  **无头下中文渲染正常**。测试 lib 36/36 + CLI 契约 11/11

- **`tke -d web control close` 省略包名 = 销毁会话**（浏览器+chromedriver+会话文件+孤儿收割）——
  用户反馈「不想每次记 `rm -f $TMPDIR/tke/web/*.json` + `pkill Chrome`」。web 分支本就忽略这个
  参数,只是 CLI 强制要填个没意义的值;移动端省略则明确报错。文档里的手工清理命令已全部替换

- **skill 定位纠正 + 重写**（用户指出我把 harness 的目标错塞进了 skill）:
  `skill/ui-test/` → `skill/ui-check/`,去掉 verify/explore、两件套、回放验证。
  **skill 只做「设备操控+查看能力交给调用方 AI + 留证据」**。
  证据落盘零改动就有:`tke steps '点击 [{x, y}]' --log <dir>`(用坐标,不需元素库)
- **跨设备**:flow per-script device ✅ / `tke run` 必填 `-d` ✅ / **重试断言** ✅
  `断言 [{提示}, 存在, 10s]`;步超时对自带时长的命令放宽(`等待 [30s]` 此前会被 20s 掐死)

- **ADR-0011 harness 侧全部实现**:5 个设备类工具加 `device` 参数 + `list_devices` 工具 +
  向导「由 AI 决定」+ 无设备不再拒绝启动 + 无设备调用给明确纠正指引 + 编排官提示词
  (不确定就问**绝不猜**、跨设备=多次 explore + flow.toml、别把多设备塞进一个 .tks)

- **skill 一键安装**:`skill/install.sh`(curl|bash 装齐 skill+tke+驱动+Chrome,
  `--profile web|android|ios|all`,幂等,自动体检且**不完整时非 0 退出**)+
  `skill/publish.sh`(打包成 S3 约定布局)。**配对好的 chromedriver+Chrome 放同一批**是
  自建分发源最实在的好处。本地 http server 模拟 S3 全流程实测通过,含用装出来的 tke 实跑一次
- **skill 补完备**:`reference/tke-commands.md` + `reference/tks-syntax.md`(AI 按需读),
  主文件补安卓 `app focus/list` 拿包名(此前完全没提,安卓场景会卡死)

## 没做完

- **ADR-0011 harness 侧的 AI 行为真机未验**(本机无 `[ai]` key):
  编排官会不会真的按语义选设备、拿不准时会不会问用户、跨设备会不会写 flow.toml——
  这些只有真跑才知道。代码与提示词都已就位
- **`tke harness` 的完整无头探索没验**——需要 `[ai]` key，这台机没有任何凭据。
  用户 mac 上 harness **有头跑通**（2 轮出两件套）,无头版待他重跑。
  harness 与 run/原子命令共用同一条 `WebDriver::start_new_session`,驱动层无头已验

## 埋的坑 / 需要后来人注意

- ✅ **坐标可移植性已验证**（用户 mac 实测 + 本机对照）:mac有头=mac无头=Linux无头=**1280x813**,
  元素 bounds `diff` 零差异。「本地录、CI 回放」成立。
  ⚠️ 做这类对照**必须先销毁会话**,否则复用旧会话会给出假阳性（P-18）
- **Q-6 平台自包含**：`.tks` 不记平台，`tke run foo.tks` 不带 `-d` 按 Android 推断 →
  web 脚本报「adb 缺失」。skill 里已写死必须带 `-d web`，但 tklib 的 meta.json 其实已存
  platform，「拷走即跑」还差这一口气
- **护栏退化是已知代价**（ADR-0010 写明）：asserter/supervisor/页面契约现在只是 SKILL.md 里的
  两条要求。若实测脚本质量不行，**出路是把护栏做成必须调用的子命令，不是把提示词写更长**
- 环境已就绪可复现：Chrome 在 `~/.local/share/tke/chrome-linux64/`，chromedriver 在
  `bin/linux-amd64/`（必须与 tke 同目录）
