#!/usr/bin/env bash
# Download the Qwen3.5-0.8B GGUF model + mmproj projector weights into the platform data dir.
# Downloads from ModelScope by default; set MODEL_SOURCE=huggingface for HuggingFace (uses a mirror).
set -euo pipefail

MODEL_NAME="${MODEL_NAME:-Qwen3.5-0.8B-Q4_K_M.gguf}"
MMPROJ_NAME="${MMPROJ_NAME:-mmproj-F16.gguf}"
REPO="${MODEL_REPO:-unsloth/Qwen3.5-0.8B-GGUF}"
SOURCE="${MODEL_SOURCE:-modelscope}"

if [[ "$(uname)" == "Darwin" ]]; then
  DEST_DIR="$HOME/Library/Application Support/qwen-translate"
else
  DEST_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/qwen-translate"
fi
mkdir -p "$DEST_DIR"

download_one() {
  local name="$1"
  local dest="$DEST_DIR/$name"
  if [[ -f "$dest" ]]; then
    echo "Already exists: $dest" >&2
    return 0
  fi

  local urls=()
  case "$SOURCE" in
    modelscope)
      urls=("https://modelscope.cn/models/$REPO/resolve/master/$name")
      ;;
    huggingface|hf)
      urls=(
        "https://hf-mirror.com/$REPO/resolve/main/$name"
        "https://huggingface.co/$REPO/resolve/main/$name"
      )
      ;;
    *)
      echo "MODEL_SOURCE must be modelscope or huggingface" >&2
      exit 1
      ;;
  esac

  local url
  for url in "${urls[@]}"; do
    echo "Downloading: $url" >&2
    if curl -fL --connect-timeout 15 -o "$dest.tmp" "$url"; then
      mv "$dest.tmp" "$dest"
      echo "Done: $dest" >&2
      return 0
    fi
  done

  rm -f "$dest.tmp"
  echo "Download failed: $name" >&2
  return 1
}

download_one "$MODEL_NAME"
download_one "$MMPROJ_NAME"
