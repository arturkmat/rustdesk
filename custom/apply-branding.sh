#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -f custom/icon.png ]]; then
  echo "custom/icon.png is missing" >&2
  exit 1
fi

PY=""
for candidate in python3 python; do
  if command -v "$candidate" >/dev/null 2>&1 \
    && "$candidate" -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 8) else 1)" >/dev/null 2>&1; then
    PY="$candidate"
    break
  fi
done

if [[ -n "$PY" ]]; then
  if ! "$PY" -c "from PIL import Image" >/dev/null 2>&1; then
    "$PY" -m pip install --user pillow icnsutil || true
  fi
  if "$PY" -c "from PIL import Image" >/dev/null 2>&1; then
    "$PY" custom/generate-icons.py
    exit 0
  fi
fi

echo "Pillow is not available; copying source images only." >&2
mkdir -p flutter/assets
cp -f custom/icon.png res/icon.png
cp -f custom/icon.png flutter/assets/icon.png
if [[ -f custom/logo.png ]]; then
  cp -f custom/logo.png flutter/assets/logo.png
  cp -f custom/logo.png flutter/assets/logo_dark.png
  cp -f custom/logo.png flutter/assets/logo_light.png
fi
