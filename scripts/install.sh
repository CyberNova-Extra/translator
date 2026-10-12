#!/usr/bin/env bash
# translate release tarball 安装脚本
# 把二进制装到 ~/.local/bin，模型（如包内附带）装到数据目录
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"

if [[ "$(uname)" == "Darwin" ]]; then
  DATA_DIR="$HOME/Library/Application Support/qwen-translate"
else
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/qwen-translate"
fi
BIN_DIR="$HOME/.local/bin"

mkdir -p "$BIN_DIR" "$DATA_DIR"
install -m755 "$HERE/translate" "$BIN_DIR/translate"
echo "已安装: $BIN_DIR/translate"

if [[ -f "$HERE"/model/*.gguf ]]; then
  cp "$HERE"/model/*.gguf "$DATA_DIR/"
  echo "已安装模型: $DATA_DIR/$(basename "$HERE"/model/*.gguf)"
else
  echo "本包不含模型，首次运行时会自动下载（默认 ModelScope）"
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "提示: $BIN_DIR 不在 PATH 中，请加入 shell 配置（如 ~/.bashrc 或 ~/.zshrc）" ;;
esac

echo "Firefox 扩展: 运行 $HERE/scripts/install-native-host.sh，然后在 about:debugging 载入 $HERE/firefox/manifest.json"
