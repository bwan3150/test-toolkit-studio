# 发布 tke skill 到 Toolkit Cloud

> 把 `tke-ui-test` skill 的一键安装分发包（skill + tke 及驱动 + Chrome for Testing）
> 打包并上传到自建存储平台 Toolkit Cloud，供用户 `curl … | bash` 一行安装。
> 打包脚本见 [`../skill/publish.sh`](../skill/publish.sh)，安装脚本见 [`../skill/install.sh`](../skill/install.sh)。

## 分发布局（约定）

`install.sh` 按固定路径去存储平台取文件，所以上传后必须是这个结构：

```
<存储根>/tke/
├── install.sh                        # 用户入口脚本
├── VERSION                           # 版本留痕（tke / chromedriver 版本）
├── skill/tke-ui-test.tar.gz             # skill 本体，跟平台无关
├── bin/<platform>/                   # 平台名 = darwin-arm64 / darwin-amd64 / linux-arm64 / linux-amd64
│   └── {tke,chromedriver,adb,aapt,go-ios}.gz   # 每个二进制单独 gzip（不是 tar）
├── chrome/
│   └── <chrome-mac-arm64|chrome-mac-x64|chrome-linux64>.zip   # Chrome for Testing，官方目录原样 zip
└── wda/
    ├── WebDriverAgentRunner-Runner-sim.zip   # iOS 模拟器用的 WDA runner（21MB，arm64+x86_64 通用）
    └── WDA-VERSION                           # 版本留痕：编的是哪个 commit
```

要点：
- **二进制是单文件 gzip**（`tke.gz`），**Chrome 与 WDA 是整个目录 zip**，别搞混。
- **WDA 那份只有一个**：`.app` 是 arm64+x86_64 的 fat 包，**Intel 与 Apple Silicon 共用**，
  不按平台分目录。打包用 `scripts/package-wda-sim.sh`（版本锁在脚本里）。
- ⚠️ **`install.sh` / `VERSION` / `skill/tke-ui-test.tar.gz` 三个别漏传**——它们不在 `bin/`、
  `chrome/` 里，是另外三个顶层文件。漏了的话使用者根本装不上（见下面「SPA 兜底」）。
- bin 平台名用 `amd64`，Chrome 包名用 `x64`（`chrome-mac-x64`）——命名不一致，别统一。
- **chromedriver 与 chrome 必须同批、版本配对**，否则网页检查跑不起来。别单独更新其中一个。

## 下载地址（重要）

**公开下载路径是 `/sl/preview/<mount>/<key>`**，不是 `/<mount>/<key>`——后者是 SPA 页面，
返回一段 HTML。本项目的分发根：

```
https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke
```

`install.sh` 与 `tke doctor` 的默认地址都指向它。

## ⚠️ 坑二：Cloudflare 缓存 4 小时，且不认 no-cache

响应头是 `cache-control: max-age=14400` + `cf-cache-status: HIT`。
**传了新文件，使用者 4 小时内下到的还是旧的**——这正是"skill 永远停在旧版本"的根因。
实测 `Cache-Control: no-cache` / `Pragma: no-cache` 请求头**无效**，
唯一可靠的破缓存手段是**变化的查询参数**。

已解决（不用你操心，但要知道原理）：
- `publish.sh` 在 `VERSION` 里写 `build: <时间戳>`；
- `install.sh` 先带随机参数取 `VERSION`（保证新鲜），再用其中的 build 戳作为后续所有下载的
  `?b=` 键——**发过新版就自动破缓存，没发新版则照常命中 CDN**；
- `tke doctor` 的版本检查也带随机参数，否则永远看到旧版本号。

**所以每次发布都要重新上传 `VERSION`**，它是缓存键的来源。只传二进制不传 VERSION，
使用者不会拿到新文件。

> 另注：该平台**不支持 Range 请求**（返回 520），别用 `curl -r 0-1` 探测文件头。

## ⚠️ 坑一：不存在的路径也返回 200

存储平台是 SPA 兜底的：**任意不存在的路径都会返回 200 + 一段前端 HTML**，不是 404。
所以 `curl -f`（--fail）根本拦不住——它只对 4xx/5xx 生效。

后果很实在：漏传某个文件时，安装器会把那段 HTML 当成 `tke` 二进制存下来，
装完一跑才发现是垃圾。

`install.sh` 因此**逐个校验文件头**（gzip 的 `1f8b` / zip 的 `PK` / 版本号必须以 `tke ` 开头），
不合格一律当下载失败。`tke doctor` 的版本检查同理。

**自查有没有漏传**（看返回的是文件还是网页；注意平台不支持 Range，要整取后再截头）：

```bash
BASE=https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke
for f in VERSION install.sh skill/tke-ui-test.tar.gz bin/linux-amd64/tke.gz chrome/chrome-linux64.zip; do
  printf '%-34s %s\n' "$f" "$(curl -fsSL --max-time 60 "$BASE/$f?t=$$" | head -c2 | od -An -c | tr -s ' ')"
done
# 期望：VERSION → t k ；install.sh → # ! ；*.gz → 037 213 ；*.zip → P K
# 出现 < ! 就是拿到了 HTML，说明这个文件没传上去
```

## 第一步：打包（产出 dist）

`publish.sh` 只打**当前机器所在平台**。缺哪个平台，就去那台机器各跑一次再合并上传。

```bash
# 前提：<studio>/bin/<platform>/ 下已有 build-*.sh 构建好的二进制
./skill/publish.sh --with-chrome --out <输出目录>
```

- 不加 `--with-chrome` 则不打那 ~470MB 的 Chrome zip（只测 Android/iOS 时可省）。
- `--out` 省略时默认输出到 `skill/dist/`。
- 结束会打印 `VERSION`（当前 tke / chromedriver 版本），核对是否是想发的那批。

> 打包脚本会自动剔除 `.DS_Store`。若你是手工整理目录再上传，记得先
> `find <目录> -name .DS_Store -delete`，否则会把它们一起传上去（你会在上传列表里看到）。

## 第二步：上传到 Toolkit Cloud

目标 = `mount:key前缀`。以本项目为例，存储平台上落点是
`https://cloud.test-toolkit.app/tookit-engine-resource/tke/`，对应：

| URL 片段 | 含义 |
|---|---|
| `https://cloud.test-toolkit.app` | 服务地址（`TKC_HOST`，curl 时由 Host 自动带上） |
| `tookit-engine-resource` | **mount** |
| `tke/` | **key 前缀** |

→ 目标参数就是 `tookit-engine-resource:tke/`

### 认证

先在 Toolkit Cloud 的「权限 (ACL)」页面创建一个 API Key（PAT），授予目标桶 **WRITE** 权限。
令牌只在创建时完整显示一次。放进环境变量，别写进脚本、别进 shell history：

```bash
export TKC_TOKEN=tkc_xxxxxxxx_xxxxxxxxxxxxxxxxxx
```

### 推荐：curl 一次性上传（不装任何东西）

`upload.sh` 逻辑跟本地 `tkc` 完全一致，只是管道执行、用完即走，适合「打完就推」：

```bash
export TKC_TOKEN=tkc_xxxxxxxx_xxxxxxxxxxxxxxxxxx

curl -fsSL https://cloud.test-toolkit.app/script/upload.sh \
  | bash -s -- <dist目录>/ tookit-engine-resource:tke/
```

- **源路径末尾的 `/` 很关键**：目录尾斜杠 = 只传目录内容、不带目录名本身（rsync 语义），
  这样 dist 里的东西正好平铺到 `tke/` 下，层级原样保留。
- ~470MB 的 Chrome zip 会走**分片上传**，中断可续传（`TKC_MAX_RETRY` 轮自愈，默认 3）。

### 想控制顺序 / 单独重传大块

Chrome 太大想分开传（其余先上、Chrome 单独失败可续）：

```bash
# 先传轻的
# ⚠️ **别漏 <dist>/skills**（没有 s 的 skill/ 是目录，带 s 的 skills 是 manifest 文件）——
# install.sh 读它决定装哪些 skill；漏了它分发源上 404，安装器会走兜底「只装 tke-ui-test」，
# 别的 skill 打好包传上去了也没人装得到（P-55，真发生过）
curl -fsSL https://cloud.test-toolkit.app/script/upload.sh | bash -s -- \
  <dist>/skill/ <dist>/skills <dist>/bin/ <dist>/install.sh <dist>/VERSION  tookit-engine-resource:tke/

# 大块单独传
curl -fsSL https://cloud.test-toolkit.app/script/upload.sh | bash -s -- \
  <dist>/chrome/  tookit-engine-resource:tke/chrome/
```

### 可用环境变量

| 变量 | 说明 |
|---|---|
| `TKC_TOKEN` | PAT 凭证（必填） |
| `TKC_HOST` | 覆盖服务地址（默认 `https://cloud.test-toolkit.app`） |
| `TKC_MAX_RETRY` | 分片上传自愈重跑轮数（默认 3） |

## 第三步：让用户能装

上传只是放文件。用户 `curl | bash` 时，`install.sh` 里的 `BASE_URL` 得指向这个存储地址。
两种做法：

- **临时**：用户侧覆盖环境变量
  ```bash
  curl -fsSL https://cloud.test-toolkit.app/tookit-engine-resource/tke/install.sh \
    | TKE_BASE_URL=https://cloud.test-toolkit.app/tookit-engine-resource/tke bash -s -- --profile web
  ```
- **一劳永逸**：打包前把 `install.sh` 顶部的 `DEFAULT_BASE_URL` 直接改成这个地址，
  用户就能纯裸 `curl … | bash`。

## 坑

- **只上传单平台**：`publish.sh` 只打当前平台。别以为一次上传就覆盖了所有用户——
  Linux / Intel Mac 用户得在对应机器上各打一次、把各自的 `bin/<平台>/` 和 `chrome/` 合并上去。
- **URL 里是 `tookit-engine-resource`**（少个 `l`）——若是笔误，存储平台和这里要一起改。
- **`publish.sh --with-chrome` 曾在非 UTF-8 locale 下崩**（`$pkg（` 把中文括号吃进变量名报
  `unbound variable`）。已修为 `${pkg}`；若复现，先 `export LANG=en_US.UTF-8` 再跑。
- **Windows 不支持**：`install.sh` 只管 macOS / Linux，Windows 需另配 `install.ps1`（当前 skill 未含）。
