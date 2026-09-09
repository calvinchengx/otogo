# otogo

**Autonomous goal loops for product design, software builds, and day-to-day work.**
Every claim names its witnesses — the loop can add evidence, never rewrite it.

Built from Jarred Kenny's
[*Building Autonomous Goal Loops That Deliver*](https://jx0.ca/building-autonomous-goal-loops-that-deliver/).

---

The agent owns the turns. A person owns the rounds.

That boundary is the whole design. Inside a round the loop runs unattended:
restore the world, drive one real request, score what happened, name the single
largest gap, close it through every layer, add a rung, and drive the same
request again. At the batch boundary it stops and waits, because a score cannot
decide direction.

The reason to trust what comes out is not that the loop worked hard. It is that
the loop was never able to touch the thing that decides whether it succeeded.

## The two ideas that are enforced, not advised

### Claims carry witnesses

A claim is a capability in software and a completed task in day-to-day work.
Both are worthless unattested. `otogo` keeps an evidence ledger — the claim, and the
named things that vouch for it:

```json
"token-validation-rs256": {
  "claim": "Signature verified before any claim is read",
  "witnesses": ["ci:host-routed", "go:TestJWKSFailureModes", "sdk:TestChallengeFlow"]
}
```

A round may add a witness, or add a claim. It may not drop a witness, repoint
one at an easier test, or soften a claim's text. Ledgers are compared
**structurally**, not by line diff — because adding a witness to an existing
claim edits the previous line's comma, which reads as tampering when it is the
opposite:

```
fail LEDGER weakened: docs/witnesses.json — $.token-validation.witnesses[1]:
     'go:TestJWKSFailureModes' -> 'go:TestSomethingEasier'.
     Evidence ledgers may gain entries, never lose or rewrite them.
```

### The loop cannot grade its own repair

Three levels of authority:

| level | meaning |
|---|---|
| **free** | implementation levers, changed freely inside scope |
| **propose** | crosses a boundary — recorded in `proposals.md`, a person decides |
| **frozen** | the exam: corpus, fixture, score rules, rubric, and `loop.json` itself |

`loop.json` is frozen against itself, and the authority in force is captured at
round start. A loop that empties its own `frozen` list has no authority model at
all — so `otogo guard` judges every round by the rules that existed when it
opened, never by the rules it left behind.

Deterministic checks are append-only. A round may add a rung for a failure it
observed. It may never weaken one or lower a threshold.

## The round

```bash
otogo open sdk-python-secret-roundtrip   # reset, health, floor, snapshot the exam
otogo drive                              # one approved request, fresh interaction
otogo score                              # read what happened
otogo classify --layer domain --gap "set returns the envelope but never writes"
#   ... close that whole path, then add a rung ...
otogo verify                             # health + floor + authority + the same request
otogo close --outcome improved
```

`classify` takes one of seven layers — `world domain contract runtime steering
surface harness`. Without it, every agent failure becomes a prompt problem and
every visual failure a frontend problem: the loop changes the nearest thing
instead of the right thing. Two layers are not product rounds and stop the round
immediately — `world` (fix the world, reopen) and `harness` (the harness is the
failure).

It writes a `gap.md` with the path the claim must close end to end:

```
request → representation → operation → persistent effect → visible proof
```

Partial paths are the most convincing failures there are: the screen that
renders and stores nothing, the tool that returns success while state stays put,
the page that looks complete and answers nothing.

## Three loops at three speeds

| loop | closes | decided by |
|---|---|---|
| product | one capability per round | the evidence |
| harness | one development failure | repeated evidence — `otogo friction` |
| direction | continue, redirect, or stop | a person — `otogo batch` |

`friction` counts. One occurrence prints *"first occurrence is evidence, not a
rule."* Two or more surfaces in `status` as earning a change to the driver,
facts, or procedure. The loop does not rewrite its operating system after every
surprise, and sometimes the right change is subtraction.

## The repository is the control plane

Sessions end and context compresses. `otogo init` scaffolds the package that
makes sessions disposable without making the work forgetful:

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

`otogo brief` prints the packet a fresh agent session reads instead of the
conversation it doesn't have — goal, procedure, facts, state, current request,
current gap, and exactly which paths are frozen, propose-level, and append-only.

## Beyond feature work

The five commands in `loop.json` are domain-specific. Everything above them is
not:

| software | general |
|---|---|
| fixture | the starting conditions you can restore |
| health check | is the world in a state where a result would mean anything |
| floor | what must not get worse |
| corpus | real demand, written by a person, sent unchanged |
| witness | what vouches for this claim |
| round | one gap, one causal claim |

Recurring day-to-day work fits this shape more naturally than feature work
does, because **the corpus arrives on its own**. The triage queue, the weekly
report, the review pass, the runbook you execute every deploy — these are real
demand, written by other people, already approved by the fact that someone asked
for them. You do not have to invent representative requests; you have to pick
three and stop editing them.

The failure mode is the same one, wearing different clothes. A task agent that
*describes* the right action without performing it looks exactly like an
emulator that answers without storing. The report gets written and the number in
it was never recomputed. The ticket gets a comment and no field changes. So the
driver still has to read the effect from the system underneath, never from what
the agent said it did.

What day-to-day work adds is a floor that is easy to lose. Quality drifts
silently on recurring tasks — nobody notices the digest got vaguer, or the
review stopped checking the thing it used to check. That is precisely what a
rung is for: the round that first catches it writes down the check, and no
later round can quietly remove it.

Where it does **not** earn its cost: one-off tasks. If the work happens once,
there is no second round to preserve anything for, and the harness is pure
overhead. Use it when the task recurs, the output has an effect you can observe,
and the quality is the kind that erodes without anyone deciding to erode it.

## Install

```bash
cargo install --path .
otogo init
```

A single static binary, ~550 KB, three dependencies (`clap`, `serde_json`,
`sha2`). Globbing, directory walking and timestamps are hand-rolled rather than
pulling in `regex`, `walkdir` and `chrono` — the binary runs eight or nine times
per round, so startup cost and size matter more than convenience.

```
                    per invocation    peak RSS
  reference impl        264 ms          21.0 MB
  otogo                  13 ms           1.9 MB
```

## The CLI enforces; the skills teach

otogo contains no model. It shells out, hashes files, and compares JSON — the
referee, not the player. That separation is deliberate: **a skill is advisory and a
subprocess exit code is not.** An agent that decides the corpus is wrong can
argue its way past an instruction; it cannot argue its way past a hash
comparison. Authority has to live outside the model's control or it is not
authority.

Two skills drive the CLI from a normal agent session:

```bash
make install-skills                       # into ./.claude/skills
make install-skills DEST=~/.claude/skills # or globally
```

| skill | for |
|---|---|
| [`otogo-setup`](skills/otogo-setup/) | wiring a loop into a repo — the five commands, what to freeze, the first three requests |
| [`otogo-round`](skills/otogo-round/) | running one round — drive, classify, close one path, add a rung, stop at the stop rules |

Neither skill restates the authority list. They point the agent at `otogo brief`,
which reads it from `loop.json` at runtime, because a second copy of the rules
goes stale silently and a stale authority list is worse than none. A test
enforces this.

## Examples

- [`examples/emulator-parity/`](examples/emulator-parity/) — growing emulator
  parity against a real, unmodified SDK, anchored on `azure-keyvault-emulator`.
- [`examples/agent-product/`](examples/agent-product/) — when the product has an
  agent inside it, and the two agents must not share context.
- [`examples/round.sh`](examples/round.sh) — one autonomous round.
- [`examples/batch.sh`](examples/batch.sh) — three, then it waits for you.

## When not to use it

Most tickets need none of this. If complete tests describe the work, give the
agent the tests and let it finish. The harness earns its cost when use must
reveal the capability sequence, and when the system exposes effects it can
observe.

Start with three requests, one fixture, one score command, one loop file, one
state file, and a three-round budget. Add a control only after a failure
justifies it.
