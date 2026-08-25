---
name: tke-security-test-remote
description: 跟 tke-security-test 完全一样地对网站 / API / App 做黑盒安全测试并出报告，**但探测从远端测试服务器发出**——你这台机器不用装 tke，架构不匹配也能用，跑在 CI 里也行。需要一个节点地址和凭据（`TKE_REMOTE` / `TKE_TOKEN`）。本机已经装了 tke 的话，用 tke-security-test。
---

# 远程安全测试（tke）

**这份和 `tke-security-test` 是同一套东西**——同样的命令、同样的判据、同样的报告。
唯一的差别是**探测从远端节点发出**，你这边只要一个客户端和一个凭据。

## 先连上

```bash
export TKE_REMOTE=https://<节点地址>
export TKE_TOKEN=<凭据>
tke remote status      # 连着谁、版本对不对得上
```

装客户端（**同一个二进制**）：

```bash
curl -fsSL https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke/install.sh | bash
export PATH="$HOME/.tke/bin:$PATH"
```

## 然后照正文那样敲

```bash
tke task new --kind security --target https://target.example --dir logs/scan
tke recon headers https://target.example --log logs/scan
tke http GET https://target.example/.env --log logs/scan
tke report logs/scan --log ./out      # 报告拉回本地 ./out
```

**安全轨不需要设备**，所以远程调用它**不会租设备、也不计设备时长**——
`tke http` / `tke recon` / `tke report` / `tke task` 默认开一个只有工作区的无设备会话。

`--log <目录>` 在远程的含义是「产物拉回本地这个目录」：`security-report.html`、
`findings.json`、每个确认漏洞的 `vuln-*.html`、以及所有落盘的请求/响应证据都会拉回来。

## 覆盖表（正文里遇到这几件事，以这里为准）

| 正文说 | 远程实际 |
|---|---|
| `curl install.sh` 装 tke | 只装客户端；探测由节点发出 |
| `tke security`（对话式 / `--json`） | **远程不开放**——它是 tke 自带 AI 的编排，属于任务层。命令层不跑服务端 AI，**你自己就是那个 AI**：照正文用 `tke http` / `tke recon` 一步步挖 |
| `--mode red-team` | **远程不开放**。`passive` / `safe`（默认）/ `aggressive` 可用——破坏性、不可逆的向量需要人就在那台机器前 |
| 证据落在本地 `--log` 目录 | 落在节点的会话目录，带 `--log` 才拉回本地 |

## 还有这些不一样

- **探测源 IP 是节点的**，不是你的。目标侧的 WAF / 风控看到的是节点——
  跟用户确认这一点，尤其是需要加白名单的时候。
- **授权仍然是前提**：远程不改变这条。你只该扫用户拥有或已获授权的目标；
  平台侧还会再校验一次目标归属。
- **用完 `tke remote close`**（安全轨没有设备要复位，但会话该还就还）。
- **出错先看** `tke remote status`：连得上吗？版本对得上吗？

---

**以下是与本地版完全相同的正文**（同一份源文件，没有第二处维护）：

