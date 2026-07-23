#!/usr/bin/env bash
# Close every beads pane so they respawn on the latest release build.
# Run after `cargo build --release` while iterating on the binary.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# shellcheck source=lib.sh
. "$DIR/lib.sh"

mapfile -t ids < <(beads_pane_ids)
for pid in "${ids[@]}"; do
  [ -n "$pid" ] && "$HERDR_BIN" pane close "$pid" >/dev/null 2>&1
done
echo "closed ${#ids[@]} beads pane(s)"
