#!/usr/bin/env bash
# tab.created hook: open the dock in the new tab, but only when the user has
# opted in by pressing A in the board, which writes the marker this checks.
# Manual toggling stays the default so existing layouts do not change.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
# shellcheck source=lib.sh
. "$DIR/lib.sh"

[ -n "${HERDR_PLUGIN_CONFIG_DIR:-}" ] || exit 0
[ -f "$HERDR_PLUGIN_CONFIG_DIR/auto-dock" ] || exit 0

# tab.created carries the new tab as {"tab": {"tab_id": ..., ...}}. The hook
# must target that tab: it is not necessarily the focused one yet.
TAB="$(printf '%s' "${HERDR_PLUGIN_EVENT_JSON:-}" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
t = (d.get("tab") or {}).get("tab_id")
if t:
    print(t)
' 2>/dev/null || true)"
[ -n "$TAB" ] || exit 0

# Never a second dock in the same tab.
mapfile -t _open < <(HERDR_TAB_ID="$TAB" beads_panes_by_title herdr-beads-dock tab)
[ "${#_open[@]}" -gt 0 ] && exit 0

read -r ROOT CWD <<<"$(first_pane_in_tab "$TAB")"
[ -n "${ROOT:-}" ] || exit 0

# Splitting and swapping pull focus to the new pane even with --no-focus, so
# remember where the user was and put them back. Opening a tab should not
# teleport them into a different one.
WAS="$(focused_tab_id)"

open_dock_at "$ROOT" "${CWD:-}" nofocus

if [ -n "$WAS" ] && [ "$WAS" != "$TAB" ]; then
  "$HERDR_BIN" tab focus "$WAS" >/dev/null 2>&1
fi
