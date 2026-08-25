#!/usr/bin/env bash
# Open the beads board as a floating popup, scoped to the focused pane's repo.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# shellcheck source=lib.sh
. "$DIR/lib.sh"

# No toggle-to-close here: a herdr popup is a session resource, not a pane. It
# has no pane ID and never shows up in `pane list`, so the lookup that closes
# the dock can never find the board. `q` inside the board is the way out.

CWD="$(focused_cwd)"

exec "$HERDR_BIN" plugin pane open \
  --plugin herdr-beads \
  --entrypoint board \
  --placement popup \
  --width 85% \
  --height 85% \
  ${CWD:+--env "HERDR_BEADS_CWD=$CWD"} \
  --focus
