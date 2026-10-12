#!/usr/bin/env bash
# Installer for the translate release tarball:
# installs the binary into ~/.local/bin and the bundled model (if any) into the data dir.
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
echo "Installed: $BIN_DIR/translate"

if [[ -f "$HERE"/model/*.gguf ]]; then
  cp "$HERE"/model/*.gguf "$DATA_DIR/"
  echo "Installed model: $DATA_DIR/$(basename "$HERE"/model/*.gguf)"
else
  echo "This package has no bundled model; it will be auto-downloaded on first run (via ModelScope)."
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "Note: $BIN_DIR is not in PATH; add it in your shell config (e.g. ~/.bashrc or ~/.zshrc)" ;;
esac

echo "Firefox extension: run $HERE/scripts/install-native-host.sh, then load $HERE/firefox/manifest.json in about:debugging"
