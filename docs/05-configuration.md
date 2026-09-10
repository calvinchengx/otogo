# Configuration

Everything lives in `goals/loop.json`. otogo runs commands you supply; it has
no opinion about what they do.

```json
{
  "commands": {
    "reset":  "make clean && make up",
    "health": "make status",
    "floor":  "make test",
    "drive":  "sh harness/drive.sh {request} {out}",
    "score":  "python3 harness/score.py --round {round_dir} --phase {phase}"
  },
  "authority": {
    "frozen":  ["goals/corpus/**", "goals/rubric.md", "goals/loop.json", "fixtures/**"],
    "propose": ["go.mod", "Dockerfile", "Makefile"]
  },
  "measure": {
    "globs": ["**/*_test.go", "e2e/**", "docs/witnesses.json"],
    "append_only": true,
    "json_ledgers": ["docs/witnesses.json"]
  },
  "budget": { "rounds_per_batch": 3, "attribution_warn_areas": 3 }
}
```

## The five commands

| command | must do | must exit non-zero when |
|---|---|---|
| `reset` | return the world to the recorded starting point | the reset failed |
| `health` | prove the world matches that point | anything is stale, missing, or down |
| `floor` | run the deterministic checks that already passed | any earned behaviour broke |
| `drive` | send ONE request through the surface a user would use | the driver itself failed |
| `score` | read behaviour **and persistent effects** | never — the score is the output |

Look for these in your repo before inventing them. Most mature projects already
have all five under different names — a Makefile, a compose file, an `e2e/`
directory.

`health` is the one people skip and the one that pays for itself fastest.
Without it, missing data looks like weak reasoning and stale code looks like a
broken contract, and the loop spends rounds repairing things that were never
broken.

## Substitutions

`{request}`, `{out}`, `{round_dir}` and `{phase}` are replaced in the command
string. The same values arrive as environment variables for drivers that prefer
to read them:

```
OTOGO_REQUEST   absolute path to the corpus request
OTOGO_OUT       where this phase's evidence goes
OTOGO_ROUND     the round directory
OTOGO_PHASE     "drive" or "verify"
```

## The corpus

Three requests to start. Each is **one real thing a person actually asks the
system to do**, written or approved by a human, sent unchanged through a fresh
interaction. Not a test case — a use. Each should state the persistent effect a
user would expect, so the driver knows what to look for beyond the response.

Do not write ten. The budget is three rounds; the corpus should not outrun it.

## The control plane

```
goals/
  goal.md       objective, exclusions, completion condition
  facts.md      decisions later rounds must not rediscover
  plan.md       initial floor, expected capability order
  LOOP.md       procedure, authority, budget, stop rules
  rubric.md     how a run is judged                        (frozen)
  STATE.md      the queue, failures, next action
  corpus/       approved requests                          (frozen)
  rounds/NNN/   request, transcript, effects, scores, gap.md, guard.json
  proposals.md  boundary changes awaiting a person
```

`LOOP.md` changes when the *process* changes. `STATE.md` changes after each
round. That distinction is what makes sessions disposable without making the
work forgetful.

`otogo brief` prints the packet a fresh agent session reads instead of the
conversation it does not have: goal, procedure, facts, state, current request,
current gap, and exactly which paths are frozen, propose-level, and append-only.
