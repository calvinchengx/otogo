#!/usr/bin/env bash
# Driver: send one corpus request through a real, unmodified SDK.
#
# The product is approached exactly as a user would approach it. Nothing about
# the harness — no fixture knowledge, no expected values, no test helpers —
# crosses into this process.
set -uo pipefail
REQUEST="${1:?request file}"; OUT="${2:?output dir}"
mkdir -p "$OUT"

case "$(basename "$REQUEST")" in
  sdk-python-*)  RUNNER="python3 e2e/sdk/run.py --lang python" ;;
  sdk-js-*)      RUNNER="python3 e2e/sdk/run.py --lang js" ;;
  sdk-dotnet-*)  RUNNER="python3 e2e/sdk/run.py --lang dotnet" ;;
  azcli-*)       RUNNER="python3 e2e/az-cli/run.py" ;;
  *)             echo "no driver for $(basename "$REQUEST")" >&2; exit 3 ;;
esac

# 1. Behaviour: what the client did and what came back.
$RUNNER --request "$REQUEST" >"$OUT/transcript.txt" 2>"$OUT/stderr.txt"
echo $? > "$OUT/exit"

# 2. Persistent effect: a SECOND, independent client, after the first exited.
#    This is the half that catches an emulator which answers without storing.
python3 e2e/sdk/run.py --lang python --readback --request "$REQUEST" \
  >"$OUT/effects.txt" 2>&1
echo $? > "$OUT/effects_exit"

# 3. Rejected actions matter as much as accepted ones.
grep -Ei '401|403|404|501|refused|unauthoriz' "$OUT/transcript.txt" \
  > "$OUT/rejected.txt" 2>/dev/null || true
exit 0
