# 把一台机器接成测试节点

## 一、装 tke

```bash
curl -fsSL https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke/install.sh | bash
tke doctor          # 看这台机器能测什么
```

需要安卓模拟器（选装，约 1GB）：

```bash
tke doctor --fix --profile android-emu --emulators 2   # 建 tke / tke-2
```

Linux 上装完注意 KVM：没有它模拟器是纯软件模拟，慢到没法用。

```bash
sudo usermod -aG kvm $USER    # 然后重新登录
```

## 二、拿凭据

平台「云设备 → 接入节点」建一行，把弹窗里的命令复制走。
**两个口令只显示这一次**。

选「内网机器」还是「平台够得着」：

| | 什么时候选 | 命令上的区别 |
|---|---|---|
| 内网机器 | 机器在内网 / IP 会变 / 没有公网入口（**大多数情况**） | `--link` |
| 平台够得着 | 平台与节点同内网、或节点有固定地址 | `--advertise <地址>` |

## 三、常驻起来（systemd user service）

前台跑的话，SSH 一断进程就没了。用 systemd 管：

```bash
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/tke-node.service <<'EOF'
[Unit]
Description=tke 测试节点
After=network-online.target

[Service]
# 凭据放这儿，别写进 unit 文件本身 —— unit 是 644，环境文件可以 600
EnvironmentFile=%h/.config/tke-node.env
ExecStart=%h/.tke/bin/tke serve --port 8787 \
  --token ${TKE_TOKEN} \
  --platform ${TKE_PLATFORM} \
  --platform-token ${TKE_ENROLL} \
  --node-name ${TKE_NAME} \
  --link
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF

cat > ~/.config/tke-node.env <<'EOF'
TKE_TOKEN=bp_node_……
TKE_PLATFORM=https://your-platform
TKE_ENROLL=bp_node_……
TKE_NAME=测试间Arch开发机
EOF
chmod 600 ~/.config/tke-node.env

systemctl --user daemon-reload
systemctl --user enable --now tke-node
# 关掉 SSH 之后还要它继续跑（否则用户会话结束时被回收）
loginctl enable-linger $USER
```

## 四、管理

```bash
systemctl --user status tke-node        # 在不在跑
journalctl --user -u tke-node -f        # 看实时日志（连上/断开/重连都在这儿）
systemctl --user restart tke-node       # 重连
systemctl --user stop tke-node          # 下线（平台立刻标离线）
```

**下线就是注销**：走 `--link` 的节点连接一断，平台立刻标离线，不用等超时。
再起来就是重新注册 —— 凭据是长期的，不用重新申请。

## 五、几个会踩的点

- **`--node-name` 有空格要加引号**，否则 shell 拆成两个参数
- **`--web-slots` 默认 4**：一台机器同时能开 4 个无头浏览器。机器小就调小
  （`--web-slots 2`），它只是并发上限，不是"预先开了 4 个 Chrome"——
  没有任务时一个都不开
- **别用 `pkill -f "tke serve"` 找进程**：那个模式会匹配到你自己敲的这条命令，
  把发起命令的 shell 也杀掉（踩过两次）。用 systemd 或 pidfile
- 走 `--link` 时 `--advertise` 用不上，平台不需要够得着这台机器
