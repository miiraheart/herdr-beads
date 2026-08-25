#!/usr/bin/env bash
# Shared helpers for the herdr-beads launcher scripts.
# herdr launches plugins with a minimal PATH; set it and resolve herdr.
# `pane list` emits JSON on stdout by default (no --json flag). Fields we use:
# result.panes[].{pane_id, cwd, focused, workspace_id, terminal_title[_stripped]}.
#
# NOTE: the JSON is piped in on stdin, so the Python program MUST come via -c
# (a `python3 - <<HEREDOC` would consume stdin as the program and drop the pipe).

export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"
HERDR_BIN="${HERDR_BIN_PATH:-herdr}"

# cwd of the focused pane (the user's repo) - passed to the board via --env as a
# hint; the binary also self-resolves, so this is belt-and-suspenders.
focused_cwd() {
  "$HERDR_BIN" pane list 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
panes = (d.get("result") or {}).get("panes") or []
focused = [p for p in panes if p.get("focused")]
p = (focused or panes or [None])[0]
if p:
    print(p.get("cwd") or p.get("foreground_cwd") or "")
' 2>/dev/null || true
}

# pane_ids of beads panes carrying $1 in their OSC title (set by the binary:
# "herdr-beads-dock" / "herdr-beads-board"). $2 narrows the search: "ws" to the
# current HERDR_WORKSPACE_ID, "tab" to the current HERDR_TAB_ID. Used to toggle
# a surface open/closed. The dock is per-tab, so it must use "tab": a workspace
# holds many tabs, and a workspace-wide match closes another tab's dock.
beads_panes_by_title() {
  "$HERDR_BIN" pane list 2>/dev/null | MARKER="$1" SCOPE="${2:-}" python3 -c '
import json, os, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
marker = os.environ.get("MARKER", "")
scope = os.environ.get("SCOPE", "")
ws = os.environ.get("HERDR_WORKSPACE_ID", "")
tab = os.environ.get("HERDR_TAB_ID", "")
for p in (d.get("result") or {}).get("panes") or []:
    title = (p.get("terminal_title") or "") + " " + (p.get("terminal_title_stripped") or "")
    if marker not in title:
        continue
    if scope == "ws" and ws and p.get("workspace_id") != ws:
        continue
    if scope == "tab" and tab and p.get("tab_id") != tab:
        continue
    pid = p.get("pane_id")
    if pid:
        print(pid)
' 2>/dev/null || true
}

# pane_id of the focused pane.
focused_pane_id() {
  "$HERDR_BIN" pane list 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
for p in (d.get("result") or {}).get("panes") or []:
    if p.get("focused") and p.get("pane_id"):
        print(p["pane_id"])
        break
' 2>/dev/null || true
}

# pane_id and cwd of the first pane of tab $1, which for a fresh tab is its
# root pane. Printed as "<pane_id> <cwd>".
first_pane_in_tab() {
  "$HERDR_BIN" pane list 2>/dev/null | TAB="$1" python3 -c '
import json, os, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
tab = os.environ.get("TAB", "")
for p in (d.get("result") or {}).get("panes") or []:
    if p.get("tab_id") == tab and p.get("pane_id"):
        print(p["pane_id"], p.get("cwd") or p.get("foreground_cwd") or "")
        break
' 2>/dev/null || true
}

# Split the dock off pane $1, dock it on the LEFT edge, narrow it to a sidebar.
# $2 is a cwd hint for the board, $3 is "focus" or "nofocus".
#
# herdr's `pane split` only goes right/down, so the new pane is swapped left
# until it reaches the edge. Every step targets the dock by id rather than the
# focused pane, so this works whether or not the dock took focus.
open_dock_at() {
  local target="$1" cwd="$2" focus_flag="--focus"
  [ "${3:-focus}" = "nofocus" ] && focus_flag="--no-focus"

  local out dock
  out="$("$HERDR_BIN" plugin pane open \
    --plugin herdr-beads \
    --entrypoint dock \
    --placement split \
    --direction right \
    --target-pane "$target" \
    ${cwd:+--env "HERDR_BEADS_CWD=$cwd"} \
    "$focus_flag" 2>/dev/null)"

  dock="$(printf '%s' "$out" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
def find(o):
    if isinstance(o, dict):
        if "pane_id" in o and isinstance(o["pane_id"], str):
            return o["pane_id"]
        for v in o.values():
            r = find(v)
            if r:
                return r
    elif isinstance(o, list):
        for v in o:
            r = find(v)
            if r:
                return r
    return None
p = find(d)
if p:
    print(p)
' 2>/dev/null || true)"
  [ -n "$dock" ] || return 0

  for _ in 1 2 3 4 5 6; do
    "$HERDR_BIN" pane neighbor --direction left --pane "$dock" >/dev/null 2>&1 || break
    "$HERDR_BIN" pane swap --direction left --pane "$dock" >/dev/null 2>&1 || break
  done

  # The dock sits on the LEFT now, so shrink its inner (right) edge.
  "$HERDR_BIN" pane resize --direction left --amount 0.18 --pane "$dock" >/dev/null 2>&1
}

# tab_id of the focused tab in this workspace.
focused_tab_id() {
  "$HERDR_BIN" tab list ${HERDR_WORKSPACE_ID:+--workspace "$HERDR_WORKSPACE_ID"} 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    sys.exit(0)
for t in (d.get("result") or {}).get("tabs") or []:
    if t.get("focused") and t.get("tab_id"):
        print(t["tab_id"])
        break
' 2>/dev/null || true
}
