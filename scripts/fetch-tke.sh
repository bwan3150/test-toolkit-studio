#!/usr/bin/env bash
# 取 tke 二进制。**不再从源码构建** —— toolkit-engine 已经拆成独立仓库
# （TOOLKIT/test-system/tke，github.com/bwan3150/Test-Toolkit-Engine），
# 它自己的 CI 会把六个平台的二进制发到分发源，这里直接取现成的。
#
# 落点是**运行时真正会找的地方**：handlers 里一律
# `path.join('bin', process.platform, 'tke')`，也就是 bin/darwin|win32|linux/。
# 注意分发源那边用的是另一套名字（darwin-arm64 / linux-amd64 / …），中间要映射。
#
# 用法：
#   ./scripts/fetch-tke.sh                 # 按本机 os/arch 取
#   ./scripts/fetch-tke.sh --force         # 已存在也重新取
#   ./scripts/fetch-tke.sh --remote darwin-amd64   # 指定分发源平台（交叉打包用）
#   ./scripts/fetch-tke.sh --out resources/darwin  # 指定落点（CI 里打包用）
set -euo pipefail

BASE="${TKE_DIST_BASE:-https://cloud.test-toolkit.app/sl/preview/tookit-engine-resource/tke}"
FORCE=0
REMOTE=""
OUT=""

while [ $# -gt 0 ]; do
    case "$1" in
        --force)  FORCE=1; shift ;;
        --remote) REMOTE="${2:?--remote 要给一个平台名}"; shift 2 ;;
        --out)    OUT="${2:?--out 要给一个目录}"; shift 2 ;;
        -h|--help) sed -n '1,20p' "$0"; exit 0 ;;
        *) echo "不认识的参数：$1" >&2; exit 2 ;;
    esac
done

# —— 本机 → 分发源平台名 ——
if [ -z "${REMOTE}" ]; then
    case "$(uname -s)" in
        Darwin) OS=darwin ;;
        Linux)  OS=linux ;;
        MINGW*|MSYS*|CYGWIN*) OS=windows ;;
        *) echo "不认识的系统：$(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        arm64|aarch64) ARCH=arm64 ;;
        x86_64|amd64)  ARCH=amd64 ;;
        *) echo "不认识的架构：$(uname -m)" >&2; exit 1 ;;
    esac
    REMOTE="${OS}-${ARCH}"
fi

# —— 落点：Node 的 process.platform，不是上面那套名字 ——
if [ -z "${OUT}" ]; then
    case "${REMOTE}" in
        darwin-*)  NODE_PLATFORM=darwin ;;
        linux-*)   NODE_PLATFORM=linux ;;
        windows-*) NODE_PLATFORM=win32 ;;
        *) echo "不认识的平台：${REMOTE}" >&2; exit 1 ;;
    esac
    OUT="bin/${NODE_PLATFORM}"
fi
BIN_NAME=tke
case "${REMOTE}" in windows-*) BIN_NAME=tke.exe ;; esac
TARGET="${OUT}/${BIN_NAME}"

# —— 版本：先取回来，它同时是破 CDN 缓存的键（Cloudflare 缓存 4h 且不认 no-cache）——
VERSION_TXT="$(curl -fsSL --max-time 30 "${BASE}/VERSION?t=$$$RANDOM" || true)"
case "${VERSION_TXT}" in
    tke\ *) : ;;
    # 这个平台对不存在的路径会回 200 + HTML（SPA 兜底），所以验内容不验状态码
    *) echo "取不到 VERSION，分发源不可用：${BASE}" >&2; exit 1 ;;
esac
VER="$(printf '%s' "${VERSION_TXT}" | head -1 | awk '{print $2}')"
BUILD_KEY="$(printf '%s' "${VERSION_TXT}" | sed -n 's/^build: *//p' | head -1)"
echo "分发源版本：${VER}（build ${BUILD_KEY}）"

if [ -f "${TARGET}" ] && [ "${FORCE}" -eq 0 ]; then
    HAVE="$("${TARGET}" --version 2>/dev/null | head -1 || echo '')"
    echo "已存在：${TARGET}"
    [ -n "${HAVE}" ] && echo "        ${HAVE}"
    echo "        要换成分发源那一版就加 --force"
    exit 0
fi

mkdir -p "${OUT}"
TMP="$(mktemp)"
trap 'rm -f "${TMP}"' EXIT

URL="${BASE}/bin/${REMOTE}/tke.gz?b=${BUILD_KEY}"
echo "下载 ${REMOTE} → ${TARGET}"
curl -fsSL --max-time 600 "${URL}" -o "${TMP}" || {
    echo "下载失败：${URL}" >&2; exit 1; }

# 同上：**必须验内容**。取回一个网页而不报错的话，解压才会炸，而错误信息离原因很远
HEAD2="$(head -c2 "${TMP}" | od -An -tx1 | tr -d ' \n')"
[ "${HEAD2}" = "1f8b" ] || {
    echo "取回的不是 gzip（多半是这个平台的 404 兜底页面）：${URL}" >&2; exit 1; }

gunzip -c "${TMP}" > "${TARGET}"
chmod +x "${TARGET}"
echo "✓ ${TARGET}（$(du -h "${TARGET}" | cut -f1)）"

# 依赖（adb / chromedriver / aapt / go-ios）不在这里管：它们由 tke 自己按需下载，
# 而且**要落在 tke 同目录**（tke 运行时从自己旁边找）。装完跑一次：
echo "  依赖用 \"${TARGET}\" doctor --fix 补（adb / chromedriver / aapt / go-ios）"
