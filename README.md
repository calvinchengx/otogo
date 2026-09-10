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

## Documentation

Full docs: **<https://calvinchengx.github.io/otogo/>** — or read the Markdown
in [`docs/`](docs/), which is the source those pages are built from.

| | |
|---|---|
| [Quickstart](docs/01-quickstart.md) | install, scaffold, run one round |
| [The round](docs/02-the-round.md) | the eight steps, the seven layers, the stop rules |
| [Authority](docs/03-authority.md) | frozen / propose / free, append-only rungs, evidence ledgers |
| [Witnesses](docs/04-witnesses.md) | why a claim names what vouches for it |
| [Configuration](docs/05-configuration.md) | the five commands, the corpus, the control plane |
| [Skills](docs/06-skills.md) | the CLI enforces, the skills teach |
| [Emulator parity](docs/07-example-emulator-parity.md) | a worked example against a real SDK |
| [Agent products](docs/08-example-agent-product.md) | when the product has an agent inside it |
| [Beyond feature work](docs/09-beyond-feature-work.md) | recurring day-to-day tasks |
| [How otogo is tested](docs/10-testing.md) | 58 tests, 88.6% line coverage |

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
