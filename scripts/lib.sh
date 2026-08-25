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
