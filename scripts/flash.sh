#!/usr/bin/env bash
# Cargo runner: flashes the launcher to `factory` and every other binary (games) to `ota_0`.
# Usage (invoked by cargo): flash.sh <elf> [extra espflash args...]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
elf="$1"
shift

case "$(basename "$elf")" in
    launcher) partition=factory ;;
    *)        partition=ota_0 ;;
esac

echo "Flashing $(basename "$elf") to partition '$partition'"

exec espflash flash --monitor --chip esp32s3 \
    --partition-table "$root/partitions.csv" \
    --target-app-partition "$partition" \
    "$elf" "$@"
