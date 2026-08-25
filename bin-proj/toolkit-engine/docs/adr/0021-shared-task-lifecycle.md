# ADR-0021: 共享任务生命周期（task / steps / report 领域无关，领域即数据）

- **状态**: 生效（用户 2026-08-25 拍板，取代 ADR-0020 的命名拆分）
- **日期**: 2026-08-25
- **关联**: 取代 ADR-0020 / 承 ADR-0010（skill 借调用方 AI）+ ADR-0019（security 领域）

## 背景

ADR-0020 把报告命令按轨拆成 `tke ui report` / `tke security report`。用户指出更干净的模型：
**task / steps / report 是领域无关的「测试生命周期」层**，一个任务是 UI 还是安全测试是它的**属性**
（数据），不该体现在命令名上——两条轨本来就都建任务目录、记日志、攒证据、出报告。

## 决策

三层架构（用户提出）：

1. **基石 primitive**：设备 `control`/`refresh`/`fetch`/`recognize`；安全 `http`/`recon`。确定性、落证据。
2. **独立 agent**：`tke harness`（UI）/ `tke security`（安全）。tke 自带 AI 编排。
3. **共享生命周期层**：`task` / `steps` / `report`，**领域无关**，靠任务目录里的标记分派。

### 领域即数据：`task.json` 标记

任务目录根放 `task.json`（`{kind: "ui"|"security", target?, mode?}`）。谁起测试谁写：
- `tke task new --kind <ui|security> [--target] [--mode]`：建目录 + 写标记（skill/脚本的干净起点）。
- `tke security`（交互/无头）起始自动写 `kind=security`。

### `tke report <dir>` 统一分派

**一条命令，两轨通用**：读 `task.json`（没标记则看有没有 `findings.json` 兜底）→
- security → 读 `<dir>/findings.json` 出安全报告（品牌 HTML + 每确认漏洞 vuln-*.html）。
- 否则 → 原设备/UI 报告（截图+步骤汇总 + verdict）。

调用方（两条 skill）都只用 `tke report <dir>`，不用记 `ui report`/`security report`。
撤销 ADR-0020：不再有 `tke ui report`（回到 `tke report`）与 `tke security report` 子命令。

### `tke steps` 暂不统一（有意）

`steps` 目前只跑设备 .tks 指令。把 http/recon 也塞进来语义勉强（设备 vs URL 两种目标模型）、
收益低（安全多步已由 agent/skill 顺序调覆盖）。**暂缓**，等 tke-security-test skill 证明「每步一进程太慢」再议。

## 理由与代价

- 为什么优于 ADR-0020：一条 `tke report` 更少命令、对 skill 统一、"领域是数据"是更准的抽象；
  ADR-0020 担心的"通用名含糊"被标记文件消解（目录自己知道自己是什么）。
- 代价：`report` 内部按 kind 分派两种输入（内部细节）；多一个 `task.json` 约定。
- 重审触发：出现第三条轨且其报告输入差异大到无法在一个 `report` 里干净分派时，重议分派机制。
