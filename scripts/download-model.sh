#!/usr/bin/env bash
# 下载 Qwen3.5-0.8B GGUF 模型到平台数据目录
# 默认从 ModelScope 下载；MODEL_SOURCE=huggingface 切换到 HuggingFace（自动走国内镜像）
set -euo pipefail

MODEL_NAME="${MODEL_NAME:-Qwen3.5-0.8B-Q4_K_M.gguf}"
REPO="${MODEL_REPO:-unsloth/Qwen3.5-0.8B-GGUF}"
SOURCE="${MODEL_SOURCE:-modelscope}"

if [[ "$(uname)" == "Darwin" ]]; then
  DEST_DIR="$HOME/Library/Application Support/qwen-translate"
else
  DEST_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/qwen-translate"
fi
mkdir -p "$DEST_DIR"
DEST="$DEST_DIR/$MODEL_NAME"

if [[ -f "$DEST" ]]; then
  echo "模型已存在: $DEST" >&2
  exit 0
fi

case "$SOURCE" in
  modelscope)
    URLS=("https://modelscope.cn/models/$REPO/resolve/master/$MODEL_NAME")
    ;;
  huggingface|hf)
    URLS=(
      "https://hf-mirror.com/$REPO/resolve/main/$MODEL_NAME"
      "https://huggingface.co/$REPO/resolve/main/$MODEL_NAME"
    )
    ;;
  *)
    echo "MODEL_SOURCE 只支持 modelscope 或 huggingface" >&2
    exit 1
    ;;
esac

ok=0
for url in "${URLS[@]}"; do
  echo "下载: $url" >&2
  if curl -fL --connect-timeout 15 -o "$DEST.tmp" "$url"; then
    mv "$DEST.tmp" "$DEST"
    ok=1
    break
  fi
done

if [[ "$ok" != "1" ]]; then
  rm -f "$DEST.tmp"
  echo "下载失败，请检查网络后重试" >&2
  exit 1
fi
echo "完成: $DEST" >&2
