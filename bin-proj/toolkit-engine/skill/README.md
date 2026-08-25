# tke-ui-test skill —— 安装说明（给人读的）

让 Claude Code 能亲手操作真实浏览器 / 安卓 / iOS，验证刚写完的功能，并留下带标注的截图证据。

分发物只有两样：**这个 skill 目录** + **tke 二进制及其同目录依赖**。不需要 tke 的源码。

---

## 一行安装（推荐）

```bash
# macOS / Linux
curl -fsSL https://<你的CDN>/tke/install.sh | bash
```

```powershell
# Windows（PowerShell）
irm https://<你的CDN>/tke/install.ps1 | iex

# 要带参数时先落地再跑：
iwr https://<你的CDN>/tke/install.ps1 -OutFile install.ps1; .\install.ps1 -Profile web
```

它会按平台自动下载 skill 文件、tke、对应驱动、Chrome for Testing，最后自动跑体检。
**环境不完整时会明确告诉你缺什么并以非 0 退出**，不会装完就说"好了"。

默认装 **tke-ui-test**（UI 实机检查）。装**安全测试** skill 用 `--skill`：

```bash
# 安全测试 skill（tke-security-test）——只装 tke，不要任何设备驱动（profile 自动=none）
curl -fsSL https://<你的CDN>/tke/install.sh | bash -s -- --skill tke-security-test
# Windows: iwr ... install.ps1 -OutFile install.ps1; .\install.ps1 -Skill tke-security-test
```

```bash
# 只装网页相关（不要安卓/iOS 工具，快很多）
curl -fsSL https://<你的CDN>/tke/install.sh | bash -s -- --profile web

# 装到项目级（跟着仓库走，团队 clone 即得）；默认是用户级 ~/.claude/skills
curl -fsSL https://<你的CDN>/tke/install.sh | bash -s -- --project
```

`--skill tke-ui-test|tke-security-test`（默认 ui-test）；`--profile web|android|ios|all|none`
（安全 skill 默认 none=只装 tke）；`TKE_HOME` 可改 tke 落点（默认 `~/.tke/bin`）。

**默认装用户级**（`~/.claude/skills/`）——装一次，所有项目都能用。

### 怎么把分发源建起来（维护者）

```bash
./publish.sh --with-chrome            # 打包到 dist/（不加 --with-chrome 则不含 600MB 的 Chrome）
aws s3 sync dist/ s3://<bucket>/tke/ --acl public-read
```

布局是约定好的，`install.sh` 按这个取：

```
<BASE_URL>/
├── install.sh
├── VERSION                      # 这批的 tke / chromedriver 版本，便于排查
├── skill/tke-ui-test.tar.gz
├── bin/<platform>/{tke,chromedriver,adb,aapt,go-ios}.gz
└── chrome/<chrome-mac-arm64|chrome-linux64|chrome-win64>.zip
```

**把配对好的 chromedriver 与 Chrome 放同一批**，使用者就不必再去查版本对应关系——
这是自建分发源相比"各自去 Google 下载"最实在的好处。
改 `install.sh` 顶部的 `DEFAULT_BASE_URL` 指向你的地址，使用者就不用带 `--base-url` 了。

---

## 手动安装

### 1. 装 skill 文件

```bash
# 用户级（推荐：装一次，所有项目都能用）
mkdir -p ~/.claude/skills
cp -r tke-ui-test ~/.claude/skills/

# 或项目级（跟着仓库走，团队 clone 即得）
mkdir -p <你的项目>/.claude/skills
cp -r tke-ui-test <你的项目>/.claude/skills/
```

装完目录长这样：

```
~/.claude/skills/tke-ui-test/
├── SKILL.md                    # 主文件，Claude Code 自动读
└── reference/                  # 以下 AI 按需读
    ├── pitfalls.md             # 踩坑册：会导致「假结论」的坑，新踩的往里加
    ├── tke-commands.md         # tke 命令速查
    └── steps-syntax.md         # 操作指令语法
```

**踩坑册是要长期养的那份**——主文件保持精干（怎么做），坑册收「为什么会得出错结论」，
每次实测踩到新坑就往里加一条，不要去撑大 SKILL.md。

### 2. 装 tke 二进制

**`tke` 必须在 PATH 里**，而且它的**同目录**要有对应的驱动——tke 只在自己所在目录找外部工具，
不搜 PATH（这是为了保证 chromedriver 与 Chrome 版本配对）。

```bash
# 从本仓库构建
./bin-proj/toolkit-engine/build-mac.sh      # 或 build-linux.sh / build-win.bat
export PATH="<仓库>/bin/darwin-arm64:$PATH"  # 按平台改目录名；写进 ~/.zshrc 持久化
```

同目录需要的东西，按你要测什么准备：

| 要测 | 需要 |
|---|---|
| 浏览器 | `chromedriver`（与 tke 同目录）+ Chrome for Testing（见下） |
| 安卓 | `adb`（与 tke 同目录） |
| iOS | `go-ios`（与 tke 同目录）+ 设备上装好 WebDriverAgent |

**Chrome for Testing** 放用户数据目录，按官方 zip 原样结构解压（**版本必须与 chromedriver 一致**）：

| 平台 | 位置 |
|---|---|
| macOS | `~/Library/Application Support/tke/chrome-mac-arm64/` |
| Linux | `~/.local/share/tke/chrome-linux64/` |
| Windows | `%APPDATA%\tke\chrome-win64\` |

```bash
V=$(chromedriver --version | awk '{print $2}')   # 取你现有 chromedriver 的版本
cd ~/Library/Application\ Support/tke            # macOS 为例
curl -sSLO https://storage.googleapis.com/chrome-for-testing-public/$V/mac-arm64/chrome-mac-arm64.zip
unzip -q -o chrome-mac-arm64.zip && rm chrome-mac-arm64.zip
xattr -cr "chrome-mac-arm64/Google Chrome for Testing.app"   # macOS 必须清隔离属性
```

> macOS 三个坑：**必须用 curl 下载**（浏览器下载会打 quarantine 标记）；
> **不能放 `~/Documents`/`~/Desktop`/`~/Downloads`**（TCC 保护目录会让进程卡死且无报错）；
> 首次启动要等 30-60 秒（Gatekeeper 扫 600MB 包），之后秒开。

### 3. 验证

```bash
tke doctor           # 三平台通用（体检，不下载）
```

它会逐项告诉你 tke、chromedriver、Chrome、安卓设备的状态，以及当前会跑有头还是无头。
只要至少有一个可操作目标就算通过。

## 用起来

**两种触发方式：**

**① 直接提需求**（推荐）——Claude Code 读 SKILL.md 的 `description` 自动判断该不该用：

- "我刚改完设置页的保存按钮，帮我在浏览器上验一下真的能存"
- "验证一下这个功能在手机上能不能用"
- "在平台上建个智能场景，去手机 App 里看能不能正常查看和使用"

**② 斜杠命令显式调用**——输入 `/tke-ui-test`，后面可以直接跟任务：

```
/tke-ui-test 在 https://platform.example.com 建个场景「夜间回家」，去手机上确认能查看和使用
```

斜杠名 = 目录名 = frontmatter 里的 `name`（三者一致才认得出）。
默认装在 `~/.claude/skills/tke-ui-test/`（全局可用）；装到仓库的 `.claude/skills/` 下则
**跟着仓库走**，团队其他人 clone 下来就有。

它会自己走：体检 → 看页面 → 操作（带证据）→ 判断 → 报结论 + 证据目录。

证据落在 **`~/.tke/logs/<任务简称>/steps_<时间戳>/`**（截图序列 + 页面结构 + log.json）——
默认写在用户目录，**不污染你的仓库**，也就不必再加 `.gitignore`。
要让证据跟着 PR 走的话，可以让它改用 `--log .tke-ui-test/` 写进项目里（那时才需要 gitignore）。

用完想清理：`rm -rf ~/.tke/logs`。

## 常见问题

**报「adb 缺失」但我测的是网页** → 忘了带 `-d web`。tke 是无状态 CLI，不带 `-d` 默认按安卓处理。
（例外：`tke run foo.tks` 会先看同名 `foo.tklib` 里记的录制平台，web 用例可以不带 `-d`。）

**加了 `--headless=on` 但还是弹出了浏览器窗口** → 上一个会话是有头的、被复用了。
先 `tke -d web control close` 销毁会话再跑（新版会拦住并提示）。

**想在无头服务器 / CI 里跑** → 无桌面时自动走无头，不用传参。docker 镜像里还需要
Chrome 的系统库（libnss3/libatk/libgbm/libasound2 等）和**中文字体**（缺了整页豆腐块）。

**这个 skill 会不会生成测试脚本？** 不会，也不该会。它只做一次性检查 + 留证据。
要产出未来可复用的回放脚本，那是 `tke harness` 的事（自带 AI，需要配 `[ai]` 的 API key）。
