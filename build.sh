#!/usr/bin/env bash
# Локальная сборка под Linux/macOS: receiver-micro -> resources -> uploader-desktop.
set -euo pipefail
cd "$(dirname "$0")"

if [[ "$(uname)" == "Darwin" ]]; then
  echo "==> 1/3 receiver-micro (universal: Intel + Apple Silicon)"
  (cd apps/receiver-micro && npm install && npm run tauri build -- --no-bundle --target universal-apple-darwin)
  BIN_DIR="target/universal-apple-darwin/release"
  UPLOADER_ARGS="--target universal-apple-darwin"
else
  echo "==> 1/3 receiver-micro"
  (cd apps/receiver-micro && npm install && npm run tauri build -- --no-bundle)
  BIN_DIR="target/release"
  UPLOADER_ARGS=""
fi

echo "==> 2/3 копирование шаблона в ресурсы отправителя"
mkdir -p apps/uploader-desktop/src-tauri/resources
cp "$BIN_DIR/receiver-micro" apps/uploader-desktop/src-tauri/resources/receiver-micro

echo "==> 3/3 uploader-desktop"
# shellcheck disable=SC2086
(cd apps/uploader-desktop && npm install && npm run tauri build -- $UPLOADER_ARGS)

echo "Готово. Установщики: target/**/release/bundle/"
