#!/usr/bin/env bash
# Open the beads board as a floating popup, scoped to the focused pane's repo.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# shellcheck source=lib.sh
. "$DIR/lib.sh"

# Toggle: if the floating board is already open, close it.
mapfile -t _open < <(beads_panes_by_title herdr-beads-board)
if [ "${#_open[@]}" -gt 0 ]; then
  for pid in "${_open[@]}"; do "$HERDR_BIN" pane close "$pid" >/dev/null 2>&1; done
  exit 0
fi

CWD="$(focused_cwd)"

exec "$HERDR_BIN" plugin pane open \
  --plugin herdr-beads \
  --entrypoint board \
  --placement popup \
  --width 85% \
  --height 85% \
  ${CWD:+--env "HERDR_BEADS_CWD=$CWD"} \
  --focus
