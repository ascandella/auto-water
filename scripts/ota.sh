#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Load environment from .envrc
if [ -f .envrc ]; then
    set -a
    source .envrc
    set +a
fi

PASS="${OTA_PASSWORD:?OTA_PASSWORD not set. Add it to .envrc or export it.}"
IP="${1:?Usage: $0 <device-ip>}"

echo "==> Building release firmware..."
cargo ota-build

echo "==> Uploading to http://${IP}/ota..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer ${PASS}" \
    --data-binary "@firmware.bin" \
    "http://${IP}/ota")

if [ "$HTTP_CODE" = "200" ]; then
    echo "==> OTA update successful! Device will reboot."
else
    echo "==> OTA update failed (HTTP ${HTTP_CODE})"
    exit 1
fi
