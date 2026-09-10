# How otogo is tested

otogo is an authority boundary, so its own tests are load-bearing: a guard that
silently stops guarding is worse than no guard, because the rounds still look
green.

## The suite

58 tests, 88.55% line coverage.

| file | covers |
|---|---|
| `tests/guard.rs` | frozen / propose / free, append-only rungs, evidence ledgers |
| `tests/lifecycle.rs` | stop rules, classification, closing, batch boundary, friction |
| `tests/commands.rs` | preflight, floor, drive, score, verify, abort |
| `tests/skills.rs` | skill frontmatter, and drift between skills and the CLI |
| unit tests | glob semantics, subsequence append check, structural ledger diff |

Integration tests invoke the real binary through `CARGO_BIN_EXE_otogo` and
assert only on **exit codes and stdout**. That is deliberate: exit codes are the
interface a runner script and a skill both branch on, so testing them tests the
contract rather than the implementation.

It also made the Rust port verifiable. The suite was written against the
reference Python implementation, retargeted with one environment variable, and
run unchanged against the compiled binary before a single Rust test existed.
Equivalence demonstrated rather than claimed.

## Coverage as a floor

```bash
make coverage        # summary
make coverage-html   # browsable
```

CI fails below 85%. The number is a floor rather than a target — it exists so
an untested command cannot land quietly, which is exactly what had happened:
`verify`, the command that decides whether a round improved, had no test at all
until coverage was measured. Six commands were unexercised; adding them moved
line coverage from 76.7% to 88.55%.

## Tests that guard against drift

Three tests exist because instructions to agents keep wanting to restate rules,
and the restatement is what goes stale:

- a skill may not hardcode a frozen glob — it must point at `otogo brief`;
- a skill may not teach a command the CLI does not have;
- `examples/round.sh` must feed the agent the skill file, not a copy.

Each was verified to actually fail when the fault is introduced, rather than
trusted because it was green.

## A constraint the tests discovered

A test cannot reconfigure the harness after `otogo open`, because `loop.json`
is frozen and the guard fires before whatever the test meant to exercise. The
fixtures degrade the world the way a real round would instead — a product with
a broken invariant, a health check that fails once a marker file appears.

That is the tool enforcing its own rule against its own test suite, which is a
reasonable sign the rule is real.
