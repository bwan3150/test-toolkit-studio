# 交接单

**会话时间**: 2026-08-13 ~ 08-15（第四场：skill 上线 → 六平台分发 → CI → 用户实测反馈调优）
**产出 commit**: `b902e281` → … → `55e492cc`（全部已合进 **main**）

## 这场做完了什么

**分发与安装（已上线可用）**
- 分发源六平台齐备：`darwin-arm64/amd64`、`linux-amd64/arm64`、`windows-amd64/386`。
  依赖是**一次性手工补的**（不走 CI），tke 二进制由 CI 构建
- `install.sh` / `install.ps1` / `uninstall.sh` / `uninstall.ps1` 四件套，带 LOGO + 配色，
  一行装、一行卸（默认保留检查记录与 Chrome，`--logs` / `--chrome` / `--all` 才删）
- **`tke fix`**（ADR-0012）：唯一会联网下载的命令；普通命令缺依赖只报错指路。
  `--check` 同时是**三平台通用的体检**（替掉了 bash 版 check-env.sh）
- **CI 自动发版**：main 上动 `src/**` 或 `skill/**` 就自动构建六平台并发布；
  只改 skill 文档时跳过编译。详见 `docs/ci-publishing.md`

**能力**
- **两件套平台自包含**（Q-6 关闭）：`tke run` 缺 `-d` 时读 tklib 的 platform 兜底
- **HTML 检查报告**：单批 + 全流程（跨批次连续编号、按时间交错排跨设备批次）。
  每步显示 AI 写的意图（`.tks` 行内注释）+ 反查出的"点中了什么"
- **`选择` 指令**：原生 `<select>` 点不开（选项由浏览器绘制、DOM 不可见），只能走 DOM 设值
- **宿主机能力门禁**：iOS 只在 macOS 放行（门禁在 `Controller::new`，一处覆盖所有入口），
  留 `TKE_ALLOW_IOS=1` 逃生口

**用户实测反馈后的两刀（最重要）**
- **语义定位掉头**：SKILL.md 原本引导用坐标 → 每两三步就得重新 `fetch` 全量元素表 →
  token 爆炸。改为**首选 `点击 ["文字"]`**（能力早就有，只是没告诉 AI）。
  文字在**执行那一刻**才解析，所以能一次传五六步；且可读可复用
- **P-27 性能 bug**：`execute()` 已解包 value，`wait_ready` 与 `center_into_viewport`
  又多解一层 → **每次点击白等 4.4 秒**、视口尺寸一直用兜底值。
  修完 **4899ms → ~750ms（6.5×）**。同时给文字定位补了隐式等待

## 没做完 / 待验

- ⚠️ **用户还没用新版重跑那个跨端任务** —— 语义定位 + 提速的实际效果没有量过。
  预期：批次数从 20 降到个位数、总耗时 3.8 分钟 → 半分钟级。**这是下一步最该做的验证**
- ⚠️ **Windows 全链路真机没验**：install.ps1 的 Windows 专有部分（USERPROFILE/APPDATA、
  用户级 PATH、bsdtar、Expand-Archive）只在 Linux 的 pwsh 上验过语法与核心函数
- **ADR-0011 的 AI 行为真机未验**（本机无 `[ai]` key）：编排官会不会按语义选设备、
  拿不准会不会问用户、跨设备会不会写 flow.toml
- **`tke harness` 完整无头探索没验**（同样缺 key）

## 埋的坑 / 后来人注意

- **这场最大的教训**：用户报"token 太大/太慢"，根因都**不在 AI 侧**——
  一个是**文档把 AI 引到了最费 token 的路线**，一个是**驱动层静默白等 4.4 秒**。
  两个都是量了数据才逼出来的（数报告里的批次/步数、量单步耗时 vs 采集耗时）。
  **下次再听到"慢/贵"，先量再改**
- 坐标可移植性已验证（mac 有头 = mac 无头 = Linux 无头 = 1280×813）
- 护栏退化是 ADR-0010 的已知代价：asserter/supervisor 现在只是 SKILL.md 里的要求。
  若质量不行，**出路是把护栏做成必须调用的子命令，不是把提示词写更长**
- 环境可复现：Chrome 在 `~/.local/share/tke/chrome-linux64/`，chromedriver 与 tke 同目录
