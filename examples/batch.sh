#!/usr/bin/env bash
# A batch: three rounds, then a person decides. That boundary is the point.
set -uo pipefail
TS="${TS:-otogo}"
HERE="$(cd "$(dirname "$0")" && pwd)"
[ $# -eq 0 ] && { echo "usage: batch.sh <request-id> [request-id ...]"; exit 1; }

for req in "$@"; do
  echo "════════════════════════════════════════════════════════"
  echo "  round on: $req"
  echo "════════════════════════════════════════════════════════"
  "$HERE/round.sh" "$req"
  case $? in
    0) ;;                                    # closed, keep going
    3) echo; echo "Batch halted early. Read goals/STATE.md and goals/rounds/."; exit 3 ;;
    *) echo; echo "Unexpected failure. Halting."; exit 1 ;;
  esac
done

echo
echo "════════════════════════════════════════════════════════"
$TS status
cat <<'EOF'
Batch complete. Three rounds is enough evidence for a decision and not enough
room to drift. Read the evidence, then choose:

  otogo batch --continue                  keep working the queue
  otogo batch --redirect "next; after"    aim the next batch somewhere else
  otogo batch --stop                      done

The score cannot decide the product's direction.
EOF
