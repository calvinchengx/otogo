#!/usr/bin/env bash
# One autonomous round. The harness owns every step except the repair.
#
#   exit 0  round closed
#   exit 3  stopped on a stop rule — a person is needed
#
# The development agent is invoked exactly once, in the middle, with the brief
# and no other context. It classifies and repairs. It does not verify its own
# work and it does not close the round.
set -uo pipefail
TS="${TS:-otogo}"
REQUEST="${1:?usage: round.sh <request-id>}"
AGENT_MD="$(dirname "$0")/AGENT.md"

stop() { echo; echo "STOP: $*"; exit 3; }

# 1-2. Restore the world, prove it, run the floor, snapshot the stone.
$TS open "$REQUEST"; rc=$?
[ $rc -eq 2 ] && stop "world unhealthy — no score recorded"
[ $rc -ne 0 ] && stop "could not open the round"

# 3-4. One approved request, fresh interaction. Behaviour AND effects.
$TS drive
$TS score

# 5-7. The agent classifies one gap and closes that whole path.
claude -p "$($TS brief)

$(cat "$AGENT_MD")" \
  --allowedTools Bash Read Edit Write Grep Glob \
  --permission-mode acceptEdits
agent_rc=$?
[ $agent_rc -ne 0 ] && stop "the development agent exited $agent_rc without completing the repair"

# The agent may have hit an escalate-layer classification and stopped.
layer=$(python3 -c "import json;print(json.load(open('goals/STATE.json'))['open_round']['layer'] or '')" 2>/dev/null)
case "$layer" in
  world|harness) stop "gap classified as '$layer' — not a product round" ;;
  "")            stop "the agent never classified a gap; nothing is attributable" ;;
esac

# 8. Same request again, plus floor and authority.
$TS verify; rc=$?
[ $rc -eq 2 ] && stop "world went unhealthy during verify"
[ $rc -eq 1 ] && stop "floor went red, or the round crossed an authority boundary"

# Outcome from the scorer, not from the agent's opinion of its own work.
outcome=$(python3 - <<'PY'
import json, glob, pathlib
rd = sorted(glob.glob("goals/rounds/[0-9]*"))[-1]
def get(p):
    f = pathlib.Path(rd) / p
    return json.loads(f.read_text()) if f.exists() else {}
before, after = get("score-drive.json"), get("score-verify.json")
if after.get("pass") and not before.get("pass"):   print("improved")
elif before.get("pass") and not after.get("pass"): print("regressed")
else:                                              print("unchanged")
PY
)
$TS close --outcome "$outcome" || stop "the round could not close"
echo "round closed: $outcome"
