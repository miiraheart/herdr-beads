#!/usr/bin/env bash
# Open the beads dock, docked on the LEFT edge of the current tab, scoped to the
# focused pane's repo. herdr's `pane split` only goes right/down, so after
# opening we `pane swap --direction left` until it reaches the edge.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# shellcheck source=lib.sh
. "$DIR/lib.sh"

# Toggle: if a beads dock is already open in this workspace, close it.
mapfile -t _open < <(beads_panes_by_title herdr-beads-dock ws)
if [ "${#_open[@]}" -gt 0 ]; then
  for pid in "${_open[@]}"; do "$HERDR_BIN" pane close "$pid" >/dev/null 2>&1; done
  exit 0
fi

CWD="$(focused_cwd)"

# NOTE: --width/--height are popup-only in herdr; a split pane must be resized
# after opening (see the pane resize below), or the open fails with invalid_params.
"$HERDR_BIN" plugin pane open \
  --plugin herdr-beads \
  --entrypoint dock \
  --placement split \
  --direction right \
  ${CWD:+--env "HERDR_BEADS_CWD=$CWD"} \
  --focus >/dev/null 2>&1

# Best-effort left-dock: swap leftward until there is no left neighbor.
for _ in 1 2 3 4 5 6; do
  "$HERDR_BIN" pane neighbor --direction left --current >/dev/null 2>&1 || break
  "$HERDR_BIN" pane swap --direction left --current >/dev/null 2>&1 || break
done

# Narrow toward a ~32% sidebar (split panes can't take --width). The dock is now
# on the LEFT, so shrink its inner (right) edge: direction = the dock's side.
"$HERDR_BIN" pane resize --direction left --amount 0.18 >/dev/null 2>&1
