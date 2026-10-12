#!/usr/bin/env bash
# 安装 Firefox 原生消息宿主清单（Linux/macOS）
# 用法: scripts/install-native-host.sh [translate 二进制路径]
set -euo pipefail

HOST_NAME="qwen_translate"
EXT_ID="qwen-translate@local"

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ $# -ge 1 ]]; then
  BIN="$1"
elif [[ -x "$PROJECT_ROOT/target/release/translate" ]]; then
  BIN="$PROJECT_ROOT/target/release/translate"
elif command -v translate >/dev/null 2>&1; then
  BIN="$(command -v translate)"
else
  echo "找不到 translate 二进制，请先 cargo build --release 或指定路径" >&2
  exit 1
fi
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"

if [[ "$(uname)" == "Darwin" ]]; then
  HOST_DIR="$HOME/Library/Application Support/Mozilla/NativeMessagingHosts"
  DATA_DIR="$HOME/Library/Application Support/qwen-translate"
else
  HOST_DIR="$HOME/.mozilla/native-messaging-hosts"
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/qwen-translate"
fi
mkdir -p "$HOST_DIR" "$DATA_DIR"

# 原生清单的 path 不能带参数，用 wrapper 脚本追加 --native-host
WRAPPER="$DATA_DIR/native-host.sh"
cat > "$WRAPPER" <<EOF
#!/bin/sh
exec "$BIN" --native-host "\$@"
EOF
chmod +x "$WRAPPER"

cat > "$HOST_DIR/$HOST_NAME.json" <<EOF
{
  "name": "$HOST_NAME",
  "description": "Qwen 本地翻译原生消息宿主",
  "path": "$WRAPPER",
  "type": "stdio",
  "allowed_extensions": ["$EXT_ID"]
}
EOF

echo "已安装: $HOST_DIR/$HOST_NAME.json"
echo "wrapper: $WRAPPER -> $BIN --native-host"
echo "下一步: 打开 Firefox about:debugging#/runtime/this-firefox → 临时载入附加组件 → 选择 $PROJECT_ROOT/firefox/manifest.json"
