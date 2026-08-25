---
name: tke-security-test
description: 亲手对一个产品（网站 / 后端 API）做探索式黑盒安全测试——用 tke 的 HTTP/侦察原语真实探测、顺藤摸瓜找泄露与配置缺陷，自己判定，最后出一份带证据的安全报告。**做完一个对外接口 / 上线前 / 用户问"这站安不安全""帮我扫一下"时用。** 只测你有授权的目标。
---

# 黑盒安全测试（tke）

**这是一种检查手段，你用自己的脑子做判断，tke 给你手、眼和证据。** 和 tke-ui-test 同一套哲学：
tke **不带 AI、不需要 API key**——怎么探、追哪条线索、什么算真问题，**你自己判断**。

tke 给你的是：
- **手**：`tke http`（发任意请求）、`tke recon <verb>`（七个 curated 检查）。
- **眼**：每个请求的状态码 / 响应头 / 响应体，都拿回来给你看。
- **证据**：每次探测自动落 `evidence/step_NNN_{req,resp}.txt`，去重、脱敏、连续编号。
- **报告**：你把判好的 findings 喂给 `tke report`，得到一份品牌 HTML 安全报告。

**你就是那个 analyst**：tke 探到什么，由你判断哪些是真问题、哪些是噪音。别把 recon 的原始命中当结论。

## ⛔ 红线（先读）

- **只测你有授权的目标。** 黑盒安全测试对无授权目标可能违法。目标必须是你自己的产品、
  或有明确授权的。顺藤时若线索指向第三方域名（CDN、第三方 API），**只观察不攻击**。
- **强度按档**（见下）。默认 `safe`：**只检测、不利用、不写入、不删改、不 DoS**。
  证明"数据可读"用一条最小只读请求即可，别把整库拉下来。升档要目标所有者同意。
- **凭据脱敏**：你记进 findings 的任何 token/密码/密钥，只留前缀（`AIzaSy••••`），绝不写全。
- **这不是在写测试资产**：一次性检查 + 留证据 + 出报告，不产 .tks、不回放（那是 tke harness/ui-test 的事）。

## 什么时候用（不必等用户开口）

- **做完一个对外接口 / 后端 / 网站**：上线前自己扫一遍泄露与配置面。
- **改了鉴权、CORS、对外数据接口**：确认没把不该给的给出去。
- **用户问**"这站安不安全""帮我看看有没有泄露""扫一下"时。

## 第 0 步：确认 tke 在

```bash
tke doctor      # 看 tke 与依赖齐不齐；缺了按提示 tke doctor --fix
```
`tke` 三平台命令一致；外围 shell 片段默认 bash，Windows 照 PowerShell 等价改。

## 强度档

| 档 | 做 | 不做 |
|---|---|---|
| `passive` | 只侦察：指纹/头/robots/bundle/TLS，不发主动/畸形请求 | 任何主动探测 |
| **`safe`（默认）** | 主动探测 + 注入**检测**（观察差异）+ 越权**探测** | 不利用、不落库、不删改、不 DoS |
| `aggressive` | 加**受控利用证明**：真拉一条数据证明可读 | 仍不破坏/不持久污染/不 DoS |
| `red-team` | 真实攻击者视角，破坏性向量 | 不可逆动作（删库/持续 DoS）必须目标方逐次点头 |

拿不准就停在 `safe`。**升档前问目标所有者。**

## 主流程

### 1. 起一个任务会话

```bash
tke task new --kind security --target https://target.example --mode safe --dir ./sec-run
# → 建目录 + 写 task.json 标记。后面所有命令都 --log ./sec-run，证据攒在一处
```

### 2. 侦察扫一圈（测绘攻击面）

七个 verb，都 `--log` 到任务目录：

```bash
for v in fingerprint headers tls cors graphql endpoints; do
  tke --log ./sec-run recon $v https://target.example
done
```
- `fingerprint` 认技术栈（框架/服务器/CMS）——给你挑下一步方向。
- `headers` 安全响应头（HSTS/CSP/点击劫持/nosniff/Server 版本）。
- `tls` 明文 HTTP 是否强制跳 HTTPS + HSTS。
- `cors` 带假 Origin 探跨域（反射任意 Origin+凭据=高危）。
- `graphql` introspection 是否对外开放。
- `endpoints` 探 `.env`/`.git`/actuator/server-status/robots 等（**已内置防 SPA 兜底假阳**）。
- `bundle` 见下（要指到真正的 JS）。

每个 verb 返回它命中的 findings（JSON）。**这些是线索，不是最终结论**——你来判断。

### 3. 顺藤摸瓜（这是这活的核心）

看侦察结果，顺着线索追。典型链条：

**A. 前端 bundle → 后端标识 → 零凭据探数据**
```bash
# 指纹认出 Framer/Next/某 SPA → 抓首页，从 <script src> 找真正的 JS
tke --log ./sec-run http GET https://target.example
grep -oE 'src="https://[^"]+\.(js|mjs)"' ./sec-run/evidence/*_resp.txt | sort -u
# 拉那些 JS，扫密钥 / 找 projectId、apiBase、dataset 这类可继续追的标识
tke --log ./sec-run recon bundle https://cdn.example/app.abc123.js
# 若挖到后端标识（如某 CMS 的 projectId），构造对它的最小只读请求，看零凭据能不能读
tke --log ./sec-run http GET "https://backend.example/api/query?..."
```
> 首页 HTML 往往不是真 bundle；SPA 的数据/配置在 JS chunk 里。**看一两个够判断就行，别刷。**

**B. 敏感端点命中 → 取回坐实**
```bash
# endpoints 报了 /.env 或 /.git → 用 http 取回确认内容是真的（不是 SPA 兜底的 HTML）
tke --log ./sec-run http GET https://target.example/.env
```

**C. GraphQL 开着 introspection → 据 schema 继续探越权。**

**D. 鉴权/越权**：拿一个低权限 token，去请求高权限资源 / 改 id 看能不能读到别人的（`safe` 档只**探测**差异，不批量拉取）。

追一条线索走到死胡同（404 / 401 / 只是公开营销内容）就**放下**，别反复抓同一个。

### 4. 判定（你是 analyst）

对每条你追出来的可疑点，**据证据**判断：
- **确认（confirmed）**：从 evidence 里的 req/resp 就能复现、就能看到问题本身。
- **疑似（suspected）**：值得记但证据只到"看着像"，需进一步验证。
- **不是**：证据不支持 / 只是噪音 → 不记。

宁可保守：证据不足以坐实的标疑似，别报成事实。**假阳比漏报更伤报告可信度。**

### 5. 写 findings.json（进任务目录）

```jsonc
// ./sec-run/findings.json
{
  "target": "https://target.example",
  "mode": "safe",
  "summary": "一句话总评：整体什么情况、最值得关注的点。",
  "findings": [
    {
      "id": "open-dataset",                 // 短横线短名
      "severity": "critical",               // critical|high|medium|low|info
      "category": "data-exposure",          // auth|data-exposure|injection|transport|config
      "title": "后端数据集零凭据可读",
      "detail": "怎么回事、影响谁。",
      "confirmed": true,                     // 硬证据=true；疑似=false（不出独立漏洞报告）
      "repro": "curl 'https://backend.example/api/query?query=count(*)'   # → 3603 条",
      "evidence": [{"seq": 12, "request": "evidence/step_012_req.txt", "response": "evidence/step_012_resp.txt"}]
    }
  ]
}
```
- 只有 `target` 和 `findings` 必填，其余给默认。
- `evidence` 的 `seq` 就是探测的步号（看命令输出的 `evidence/step_NNN`）。**关联证据让报告可复核。**
- **凭据脱敏**：`repro`/`detail` 里的密钥只留前缀。

### 6. 出报告

```bash
tke report ./sec-run
# → 读 task.json(kind=security) + findings.json，出：
#   security-report.html（全局，含评级/环形/清单）
#   vuln-<id>.html（每个 confirmed 一份；疑似只进全局清单）
#   findings.json（机器可读）
```
`confirmed:false` 的疑似项只出现在全局清单，不单独出漏洞报告——**没复现就不当实锤**。

## 空结果是合格结论

侦察 + 几条线索追下来确实没问题（后端已修好、返回 401/403、无泄露），就如实报"未发现可确认问题"，
在 summary 里说清测了什么、为什么没发现。**别为了凑而编 finding。**

## 交付给用户

把 `security-report.html` 给用户看，一句话说结论（评级 + 最该修的一两项）。
需要给非本机的人看，按你们的约定传云端再给链接。

## 需要更多时

- `reference/recon-and-findings.md`：七个 verb 的判据细节 + findings.json 完整字段 + 常见顺藤套路。
