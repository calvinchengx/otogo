# Authority

An autonomous loop optimizes whatever measure it can reach, including a bad one.
So the harness needs an authority model, and it needs to be enforced by a
process rather than requested in a prompt.

## Three levels

| level | meaning |
|---|---|
| **free** | implementation levers, changed freely inside scope |
| **propose** | crosses a boundary — recorded in `proposals.md`, a person decides |
| **frozen** | the exam: corpus, fixture, score rules, rubric, and `loop.json` itself |

The loop can change the product. It cannot change the evidence that decides
whether the product improved.

## Deciding what to freeze

One question, asked of every candidate: **if the loop could edit this, could it
pass without improving?** If yes, freeze it.

Typically frozen: the corpus, the fixture, the rubric, the score rules, pinned
versions of services the system under test depends on, and `loop.json`.

## The authority is captured at `open`

`loop.json` is frozen against itself, and the rules in force are snapshotted
into `goals/rounds/NNN/snapshot.json` when the round opens. Every check judges
the round by those rules, never by the ones it left behind.

This is not hypothetical. The reference implementation read the *live* config,
so a loop that emptied its own `frozen` list disarmed the guard before it ran.
A test caught it:

```
FROZEN modified: goals/loop.json (matches `goals/loop.json`) — this file is part of the exam.
```

## Checks are append-only

A round may add a rung for a failure it observed — that is how progress is
preserved. It may never weaken or delete one, because that would let it grade
its own repair.

```
MEASURE weakened: tests/test_rung.py — existing lines were changed or removed.
Checks are append-only; a round may add a rung, never lower one.
```

The check is a real diff, not a hash: every original line must survive, in
order, with only insertions.

## Evidence ledgers

A claim is a capability in software and a completed task in day-to-day work.
Both are worthless unattested. An evidence ledger names what vouches for each:

```json
"token-validation-rs256": {
  "claim": "Signature verified before any claim is read",
  "witnesses": ["ci:host-routed", "go:TestJWKSFailureModes", "sdk:TestChallengeFlow"]
}
```

List these under `measure.json_ledgers` and they are compared **structurally**
rather than line by line. That matters more than it sounds: adding a witness to
an existing claim edits the previous line's comma, which reads as tampering in
a line diff when it is the opposite.

A round may add a witness or add a claim. It may not:

```
LEDGER weakened: docs/witnesses.json — $.token-validation.witnesses[1]:
  'go:TestJWKSFailureModes' -> 'go:TestSomethingEasier'.
  Evidence ledgers may gain entries, never lose or rewrite them.
```

Dropping a witness, repointing one at an easier test, and softening a claim's
text all look like improvements in a diff. None of them are.

## Attribution

`guard` warns when a round sprays across independent areas. One gap is not one
file, but it is one causal claim — and if a round changes five independent
levers, nobody knows which lesson to preserve.
