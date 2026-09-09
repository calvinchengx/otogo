---
name: otogo-round
description: Run one round of an otogo goal loop — drive an approved request, classify the largest gap, close it through every layer, add a rung, and verify. Use when the user asks to run a round, close a gap, work the queue, continue the loop, or when a repo contains goals/loop.json and the task is to advance the capability rather than fix a named bug.
---

# Running one round

You are the **development agent**. You change the product. You do not decide
whether the product improved — `otogo verify` does that, after you are done.

## First, always

```bash
otogo brief
```

This is the packet that replaces the conversation you do not have: the goal, the
procedure, the facts earlier rounds paid for, the current state, the current
request, and **the authority — which paths are frozen, propose-level, and
append-only**. Do not work from memory of a previous session, and do not assume
the authority list from another repo applies here. The brief is the source of
truth; this skill is not.

If `otogo status` shows no open round, open one:

```bash
otogo open <request-id>     # exit 2 means STOP — see stop rules below
otogo drive
otogo score
```

## Classify before you repair

This is the step that decides whether the round is worth anything.

```bash
otogo classify --layer <layer> --gap "<one causal claim>"
```

Layers: `world domain contract runtime steering surface harness`.

Read the evidence in `goals/rounds/NNN/` before forming a theory — especially
`drive/effects.*`, which is the half that catches a system describing a correct
action without performing it. Then name the layer where the **cause** lives, not
the layer where the symptom appeared. Not every agent failure is `steering`. Not
every visual failure is `surface`. Skipping this step is how a loop ends up
changing the nearest thing instead of the right thing.

`classify` exits **2** for `world` and `harness`. That is not an obstacle to
work around — those are not product rounds. Stop and tell the user.

## Close one whole path

One gap is not one file. It is one causal claim, and the claim usually crosses
layers:

```
request → representation → operation → persistent effect → visible proof
```

`gap.md` in the round directory has this as a checklist. A screen with no data
behind it, or an operation that returns success while state stays put, is a
partial path — the most convincing kind of failure, and not a closed gap.

Then **add a deterministic check for the behaviour you just made work.** A
closed gap with no rung will be re-earned and re-lost.

## Then stop

Run `otogo guard` to see what your changes crossed. Do not run `verify` or
`close` unless the user asked you to run the whole round — verification is not
the repairer's job, and the outcome is read from the scorer, never from your
opinion of your own work.

## Stop rules — report, do not route around

| condition | what it means |
|---|---|
| `open` exits 2 | the world is unhealthy; this is NO_SCORE, not a feature failure |
| `classify` exits 2 | a `world` or `harness` gap — not a product round |
| `guard` exits 1 | the round crossed a frozen path or weakened a check |
| `verify` exits 1 | the floor went red, or authority was violated |
| `open` refuses on budget | the batch is spent; a person decides at the boundary |

If a repair genuinely requires changing something frozen, do not change it.
Say so and stop — `otogo guard` records propose-level changes in
`goals/proposals.md` for a person to decide.

## Close the loop on the harness too

When the round is done, answer the question that improves the harness rather
than the product:

```bash
otogo friction "<what cost time that a rule or check could prevent>"
```

First occurrence is evidence, not a rule. Repeated friction earns a change to
the driver, the facts, or the procedure — and sometimes the right change is
subtraction.
