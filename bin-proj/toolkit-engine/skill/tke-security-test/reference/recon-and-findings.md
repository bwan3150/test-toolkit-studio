# 参考：recon verb 判据 + findings 字段 + 顺藤套路

主文件够日常用；这里是细节，按需读。

## `tke http` —— 原始请求

```bash
tke --log <dir> http <METHOD> <URL> [-H 'K: V']... [--data <body>]
```
- 4xx/5xx **照常返回**（探测就是要看状态码，不当错误）。默认**不跟随重定向**（3xx 是观察对象）。
- 响应体限 2 MiB。请求/响应自动落 `evidence/step_NNN_{req,resp}.txt`。
- 输出 JSON：`status` / `elapsed_ms` / `header_count` / `body_bytes` / `evidence`。
- **注意**：请求体用 `--data`（长名）——短名 `-d` 被全局 `--device` 占用。

## 七个 recon verb

| verb | 探什么 | 命中判据 |
|---|---|---|
| `fingerprint` | 技术栈 | 从 Server / X-Powered-By / Set-Cookie / 页面特征（`__NEXT_DATA__`、`wp-content`、generator…）认框架。info 级 |
| `detect` | **后端服务标识** | 从 bundle 扒 Sanity `projectId`+`dataset` / Supabase `<ref>.supabase.co` / Firebase `firebaseio.com` / Algolia / GraphQL / S3，**并在 detail 里给出零凭据探测式**。往后端深挖的桥，见 `service-playbook.md` |
| `headers` | 安全响应头 | 缺 HSTS / CSP / X-Frame-Options（或 frame-ancestors）/ nosniff；Server 带版本号 |
| `cors` | 跨域 | 带 `Origin: https://evil.example` 探——反射任意 Origin **且** `Allow-Credentials:true`=**高危**；`*`=info |
| `graphql` | introspection | POST 最小 introspection 查询，返回 `__schema` 且非报错=开着（低危，降低后续门槛） |
| `bundle` | JS 里的密钥 | 正则扫 AWS/Google/Slack/JWT/私钥/通用 `api_key=`。命中=高危，**自动脱敏**（只留前 6 字符） |
| `endpoints` | 敏感路径 | 探 `.env`/`.git/HEAD`/`.git/config`/actuator/server-status/`.DS_Store`/security.txt/robots。**命中要 200 + 非 HTML + 内容签名对得上**（防 SPA 兜底假阳） |
| `tls` | 传输（轻量） | 明文 http:// 是否强制跳 https；https 有没有 HSTS。深度证书检查暂未做 |

每个 verb 返回 `{verb, url, evidence_steps, findings:[...]}`。**findings 是线索，你来判定进不进报告。**

## 顺藤套路（多试几种角度）

- **SPA/CMS → bundle → 后端**：`fingerprint` 认出 Framer/Next/Nuxt/WordPress → 抓首页扒 `<script src>` →
  拉真正的 JS chunk（不是 `framer.*.mjs`/`react.*.mjs` 这类库，是站点自己的 chunk）→ `recon bundle` 扫，
  或自己 grep `projectId`/`dataset`/`apiKey`/`https://.*\.api\.` → 顺着后端标识发**零凭据**请求看能不能读。
- **robots/sitemap → 隐藏路径**：`endpoints` 报了 robots → 取回看 Disallow，那些路径常是后台/内测入口。
- **鉴权面**：注册接口报错区不区分"用户存在/密码错"（用户枚举）；登录有没有速率限制（连打看有没有 429）；
  低权限 token 能不能访问高权限资源 / 改 id 读到别人的（越权，`safe` 只探测差异）。
- **注入检测（safe，不利用）**：可疑参数塞布尔/时延特征，只看响应差异**判断存不存在**，不拉数据、不破坏。

## findings.json 完整字段

```jsonc
{
  "target": "https://target.example",     // 必填
  "mode": "safe",                          // 可选，默认 safe
  "summary": "一句话总评",                  // 可选（不给则按数量自动生成）
  "findings": [                            // 必填（可空数组=干净）
    {
      "id": "open-dataset",               // 必填，短横线短名（进 vuln-<id>.html 文件名）
      "severity": "critical",             // 必填 critical|high|medium|low|info
      "category": "data-exposure",        // 必填 auth|data-exposure|injection|transport|config
      "title": "一句话标题",               // 必填
      "detail": "怎么回事、影响谁",         // 必填
      "confirmed": true,                  // 可选默认 false；true=已复现硬证据（出独立报告），false=疑似（只进清单）
      "repro": "curl ...  # 脱敏",         // 可选，强烈建议：可直接跑的复现命令
      "evidence": [                       // 可选，关联证据（让报告可复核）
        {"seq": 12, "request": "evidence/step_012_req.txt", "response": "evidence/step_012_resp.txt"}
      ]
    }
  ]
}
```

评级由 reporter 按最高严重度自动算（critical→F… info/none→A）。`confirmed:false` 不出独立漏洞报告。

## 常见坑

- **别把 recon 命中当结论**：`endpoints` 报 robots（info）不是漏洞；`headers` 缺 CSP 是低危不是高危。你来定级。
- **别打第三方**：顺藤指向 CDN/第三方 API，只观察（GET 读）不攻击。
- **凭据脱敏**：写进 findings 的密钥只留前缀。`bundle` 命中已自动脱敏，你手写的也要。
- **别反复抓同一 URL**：追到死胡同就换线索。
- **空结果如实报**：没问题就 findings 空 + summary 说清测了什么，别编。
