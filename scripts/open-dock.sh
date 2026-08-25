#!/usr/bin/env bash
# Toggle the beads dock in the current tab, docked on the LEFT edge and scoped
# to the focused pane's repo.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# shellcheck source=lib.sh
. "$DIR/lib.sh"

# Toggle: if a beads dock is already open in this tab, close it.
mapfile -t _open < <(beads_panes_by_title herdr-beads-dock tab)
if [ "${#_open[@]}" -gt 0 ]; then
  for pid in "${_open[@]}"; do "$HERDR_BIN" pane close "$pid" >/dev/null 2>&1; done
  exit 0
fi

TARGET="$(focused_pane_id)"
[ -n "$TARGET" ] || exit 0
open_dock_at "$TARGET" "$(focused_cwd)" focus
