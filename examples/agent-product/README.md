# When the product has an agent inside it

`data-agent-service`, `data-agent-formulator` and `data-agent-voice` are this
shape. It is the case with the sharpest failure mode, and the harness's most
important rule applies here: **the two agents must not share context.**

The development agent knows the code, the fixture, and the answer. If any of
that reaches the product agent, the round proves nothing — the product agent
succeeds using knowledge no user will ever have. The test looks green and means
nothing.

So the driver spawns the product agent as a separate process, in a fresh
conversation, with only the tools the product exposes:

```bash
#!/usr/bin/env bash
# examples/agent-product/drive.sh
set -uo pipefail
REQUEST="${1:?}"; OUT="${2:?}"; mkdir -p "$OUT"

# A scratch home, so no ambient project config, memory, or MCP server the
# product doesn't ship can leak in.
export HOME="$OUT/home"; mkdir -p "$HOME"

claude -p "$(cat "$REQUEST")" \
  --mcp-config ./product.mcp.json \
  --allowedTools "mcp__product__*" \
  --output-format stream-json \
  > "$OUT/transcript.jsonl" 2>"$OUT/stderr.txt"
echo $? > "$OUT/exit"

# Tool calls and refusals, pulled out of the stream for the scorer.
python3 - "$OUT" <<'PY'
import json, sys, pathlib
out = pathlib.Path(sys.argv[1])
calls, refused = [], []
for line in (out / "transcript.jsonl").read_text().splitlines():
    try: ev = json.loads(line)
    except ValueError: continue
    for b in (ev.get("message", {}) or {}).get("content", []) or []:
        if b.get("type") == "tool_use": calls.append(b["name"])
        if b.get("type") == "tool_result" and b.get("is_error"): refused.append(str(b)[:200])
(out / "tool_calls.txt").write_text("\n".join(calls))
(out / "rejected.txt").write_text("\n".join(refused))
PY

# The effect, read from the system beneath the agent — never from what it said.
psql -tAc "select id, status from orders where updated_at > now() - interval '5 min'" \
  > "$OUT/effects.txt" 2>&1
echo $? > "$OUT/effects_exit"
```

Three things this buys you:

- **`--allowedTools "mcp__product__*"`** — the product agent cannot reach Bash,
  Edit, or the repo. It gets what a user gets.
- **`HOME` redirected into the round directory** — no `CLAUDE.md`, no memory, no
  ambient MCP servers. A fresh conversation means fresh.
- **The effect read from the database**, not from the transcript. An agent that
  says *"I've marked those two as escalated"* while writing nothing is the
  single most convincing failure in this whole category, and the only thing that
  catches it is looking at the rows.

## Scoring an agent product

Some of it is deterministic — did the rows change, was a forbidden tool called,
did it refuse something it should have done. Some of it needs a judge, and the
essay is strict about those: a learned judge stays *evidence* until its rankings
agree with repeated human rankings. Keep the rubric frozen, and record the
judge's verdict as one input among several rather than the score itself.

One perfect run proves success is possible. It says little about whether success
is reliable — so for judged rows, drive the same request several times and score
the distribution, not the best sample.
