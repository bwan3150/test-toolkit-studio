# 交接单

**会话时间**: 2026-08-12（第三场）
**产出 commit**: `affaadbf` → `00a7ee6d` → 本次未提交（skill + 无头 + 实测）

## 做完了

- **web 无头支持**：`--headless=auto|on|off`；容器/root 自动 `--no-sandbox --disable-dev-shm-usage`。
  顺带修两个既有 bug：`find_chrome_binary` 只认 mac-arm64 硬编码、`env_clear` 清掉 `DISPLAY`（P-15/P-16）
- **`build-linux.sh`**（依赖预检 + `--no-ocr` CI 模式），两条路径实测通过
- **ADR-0010 生效（用户拍板）**：**skill 借调用方的 AI**——tke 退回成设备操作原语 + 证据产出器。
  `tke task`（ADR-0009）取消，该 ADR 标为已被取代（一行代码没写过）。harness 内置 AI 保留
- **skill 原型 `skill/ui-test/`**（SKILL.md + check-env.sh），可复制到使用者项目的 `.claude/skills/`
- **fix**：`element add --lib foo.tklib` 包不存在时建新包（P-17，skill 实测第一步撞出来的）
- **本机无头全链路实测通过**：Chrome for Testing + chromedriver 151.0.7922.138 → 无头启动/采集/操作
  → 落库建包 → 写 .tks → `tke run` **5/5 步、退出码 0**；标注截图/log.json/page xml 齐全，
  **无头下中文渲染正常**。测试 lib 36/36 + CLI 契约 11/11

## 没做完

- **`tke harness` 的完整无头探索没验**——需要 `[ai]` key，这台机没有任何凭据。
  但 harness 与 run/原子命令**共用同一条 `WebDriver::start_new_session`**，驱动层无头已验证

## 埋的坑 / 需要后来人注意

- **有头/无头的像素坐标对照还没做**。本机无 DISPLAY、无 xvfb，做不了对照。
  实测无头截图 **1280x813**（window-size 1280x900 减 87px 浏览器 UI）。
  **请在 mac 上跑同样脚本比对截图尺寸**——这决定"本地录、CI 回放"成不成立
- **Q-6 平台自包含**：`.tks` 不记平台，`tke run foo.tks` 不带 `-d` 按 Android 推断 →
  web 脚本报「adb 缺失」。skill 里已写死必须带 `-d web`，但 tklib 的 meta.json 其实已存
  platform，「拷走即跑」还差这一口气
- **护栏退化是已知代价**（ADR-0010 写明）：asserter/supervisor/页面契约现在只是 SKILL.md 里的
  两条要求。若实测脚本质量不行，**出路是把护栏做成必须调用的子命令，不是把提示词写更长**
- 环境已就绪可复现：Chrome 在 `~/.local/share/tke/chrome-linux64/`，chromedriver 在
  `bin/linux-amd64/`（必须与 tke 同目录）
