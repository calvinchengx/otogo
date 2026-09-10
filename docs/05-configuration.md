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

## Timeouts

```json
"timeouts": { "default": 1800, "drive": 600, "reset": 900 }
```

Every command runs under a budget and is killed if it outruns it, reported as
`TIMED OUT` rather than a generic non-zero exit. `0` means no limit.

This is not tidiness. A command that hangs produces no output and looks exactly
like one that is slow — a driver caught in an infinite retry or paging loop will
sit there indefinitely while the loop waits on it. Set `drive` tighter than the
rest: a driver should finish in the time a user would wait.

## What counts as a change

In a git repository, **git decides** which files the guard considers: its own
list of tracked and non-ignored files. Build output and fixture data are not
product changes, and hashing them makes every round report them as changed,
which buries the real diff.

Two consequences worth knowing:

- **A path that is frozen or a measure is guarded even when git ignores it.**
  Fixtures are routinely gitignored because they are large. Un-freezing the exam
  because of a line in `.gitignore` would defeat the authority model.
- **A tracked `build/` or `target/` directory is ordinary source.** Only the
  non-git fallback skips directories by name.

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
