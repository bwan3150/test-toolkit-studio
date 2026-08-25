# 服务暴露 playbook —— 往后端深挖的测试方向

通用 recon 只能查表面（缺头、暴露路径、bundle 关键词）。**真正的泄露多在后端服务的误配里**——
而后端是什么，线索就在前端 bundle / 请求里。这份 playbook 是「认出某个服务 → 它的已知误配 → 精确的零凭据探测」。

用法：`recon fingerprint` / `recon detect` / `http GET 首页` 拿到线索后，对照下面这张表，用 `http` 构造探测。
**每条都是零凭据只读**（safe 档就能跑）。命中即高危级数据泄露——但**要真复现出返回了数据才算 confirmed**。

---

## Sanity（Headless CMS）

- **认出**：bundle/请求里有 `projectId`、`dataset`，或 `<projectId>.api.sanity.io`、`apicdn.sanity.io`。
  常见于 Next/Nuxt/Framer 站的 JS chunk：`"projectId":"ab12cd34"`、`"dataset":"production"`。
- **已知误配**：dataset 分 public/private，**public dataset 凭 projectId+名即可零凭据全量读**（Free 套餐只给 public；
  试用期结束 private 会**自动回落 public** 且无告警）。
- **探测**（零凭据）：
  ```bash
  # 先数一下有多少文档（有返回=public 可读）
  tke --log <dir> http GET "https://<projectId>.api.sanity.io/v2021-06-07/data/query/<dataset>?query=count(*)"
  # 再抽一类看是不是敏感（价格/订单/用户/内部 ID）
  tke --log <dir> http GET "https://<projectId>.api.sanity.io/v2021-06-07/data/query/<dataset>?query=*%5B0...5%5D"
  ```
- **坐实**：返回 `{"result": …}` 且含**敏感业务数据**（价目/库存/订单/PII/ERP 内部 ID）= confirmed 高危/严重。
  只返回公开营销内容（标题/图片）= 设计如此，不算漏洞（标 info 或不报）。
- **注意**：管理面（`api.sanity.io/v1/projects/<pid>`）返回 401 是正常的；漏洞在**数据面**。

## Firebase（Google BaaS）

- **认出**：`*.firebaseio.com`、`firebaseConfig = {apiKey, authDomain, projectId, databaseURL}`、`firebasestorage`。
  （`apiKey: "AIza…"` 是**设计上可公开**的客户端标识，本身不是泄露——见文末防误报。）
- **RTDB 已知误配**：规则 `read: true` → 整库零凭据可读。
  ```bash
  tke --log <dir> http GET "https://<project>.firebaseio.com/.json?shallow=true"   # 非 null / 非 "Permission denied" = 开放
  ```
- **Firestore 已知误配**：`match /{document=**} { allow read: if true; }`。
  ```bash
  tke --log <dir> http GET "https://firestore.googleapis.com/v1/projects/<projectId>/databases/(default)/documents/"
  ```
- **坐实**：RTDB 返回数据结构（非 `null`/`Permission denied`）、Firestore 返回 `documents` = confirmed。
  **safe 档只读**：别 PUT 测写（那是利用/污染）。

## Supabase（Postgres BaaS）

- **认出**：`<ref>.supabase.co`、`supabaseUrl`、`supabaseKey`/anon key（一个 `eyJ…` JWT，role=anon）。
- **已知误配**：表默认 **RLS 关闭**，anon key 又是公开的 → 任何人凭 URL+anon key 读全表（2025 年 Lovable 170+ 站中招）。
- **探测**（零凭据，带公开的 anon key）：
  ```bash
  # 枚举常见表名
  for t in users profiles orders messages customers payments; do
    tke --log <dir> http GET "https://<ref>.supabase.co/rest/v1/$t?select=*&limit=1" -H "apikey: <anon-jwt>"
  done
  ```
- **坐实**：返回**行数据**（非 `[]`、非 `{"message":"...RLS..."}`）= 缺 RLS，confirmed 高危。返回 `[]` 或权限拒绝 = 正常。

## S3 / 云存储桶

- **认出**：`<bucket>.s3.amazonaws.com`、`s3.<region>.amazonaws.com/<bucket>`、CloudFront 回源。
- **已知误配**：桶可公开列举 / 文件公开读。
  ```bash
  tke --log <dir> http GET "https://<bucket>.s3.amazonaws.com/"
  ```
- **坐实**：返回 XML `<ListBucketResult>` 且列出**非公开资产**（配置/备份/私有文件）= confirmed。

## Algolia（搜索）

- **认出**：`applicationId`/`appId` + `apiKey`（`<appId>-dsn.algolia.net`）。
- **已知误配**：前端**只该**放 search-only key；放了 **admin key** 就能改索引/读全量。
- **探测**：用发现的 key 打 `https://<appId>-dsn.algolia.net/1/indexes/*/queries`；能列索引/超出 search 权限 = admin key 泄露（高危）。search-only 是设计如此。

## Hasura / 公开 GraphQL

- **认出**：`/v1/graphql`、`/graphql`、`x-hasura-*`。
- **已知误配**：`UNAUTHORIZED_ROLE=public` + anonymous role 开了 SELECT → 无 auth 头就能 introspection + 查表。
- **探测**：`recon graphql <endpoint>`（introspection）；再无 auth 头 query 某张业务表看能不能读。匿名能读敏感表 = 高危。

---

## 防误报：哪些"key"是设计上可公开的

前端 bundle 里出现这些**不是泄露**（别报成漏洞）：
- Stripe **publishable** key `pk_…`、Firebase 客户端 `apiKey: AIza…`、Supabase **anon** key、Google Maps `AIza…`。
  它们本就发给浏览器。（Supabase/Firebase 的**真问题是后端没设 RLS/规则**，不是 key 本身。）

这些一旦出现在前端**就是真泄露**（高危/严重）：
- `sk_…`（Stripe secret）、`sk-…`（OpenAI）、`AKIA…`（AWS 长期凭据）、`service_role`（Supabase 服务端 key）、
  任何名字带 `secret`/`private`、私钥块 `-----BEGIN … PRIVATE KEY-----`。

判 key 时先分清这两类：`bundle` verb 命中 `AIza` 会报 High——但如果确认是 Google Maps/Firebase 客户端 key，
**降级或标注**，别当严重漏洞（除非能证明它还能访问私有 API，如 Gemini）。

---

## 通用深挖顺序

1. `recon fingerprint` + `recon detect` + `http GET 首页` → 认后端。
2. 从 bundle（真正的 JS chunk，不是首页 HTML）扒出后端标识：`projectId`/`dataset`/`supabaseUrl`/`firebaseConfig`/`appId`。
3. 对照上表构造**零凭据只读**探测。
4. 返回了敏感数据才 `record_finding(confirmed:true)` 并附复现命令（脱敏）；只是"看着可能"标 `confirmed:false`。
5. 没有后端服务、或都返回空/拒绝 = 如实报"未发现后端泄露"。
