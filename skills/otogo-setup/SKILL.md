---
name: otogo-setup
description: Wire an otogo goal loop into a repository — scaffold goals/, choose the five commands, decide what is frozen, and write the first corpus requests. Use when the user wants to set up otogo, start a goal loop, add a harness to a project, or asks what should be frozen.
---

# Wiring a loop into a repository

```bash
otogo init
```

Scaffolds `goals/`. Everything below is the part that requires judgment.

## Ask first: does this repo need a harness?

Most work does not. If complete tests already describe the job, give the agent
the tests. A goal loop earns its cost only when **use has to reveal the
capability sequence** — you are growing a capability nobody has fully specified,
and the important failures sit outside the tests you have. It also requires that
the system **expose effects you can observe**. If you cannot read the effect
from underneath, no round can tell success from a convincing description of it.

Say so plainly if the repo does not need one.

## The five commands

Look for these in the repo before inventing them. Most mature projects already
have all five under different names — a Makefile, a compose file, an e2e
directory.

| command | must do | must exit non-zero when |
|---|---|---|
| `reset` | return the world to the recorded starting point | the reset failed |
| `health` | prove the world matches that point | anything is stale, missing, or down |
| `floor` | run the deterministic checks that already passed | any earned behaviour broke |
| `drive` | send ONE request through the surface a user would use | the driver itself failed |
| `score` | read behaviour **and persistent effects** | never — the score is the output |

`health` is the one people skip and the one that pays for itself fastest.
Without it, missing data looks like weak reasoning and stale code looks like a
broken contract, and the loop spends rounds repairing things that were never
broken.

The driver must record the answer, the tool calls, the **rejected** actions, the
visible result, and the persistent effects — the last read independently, after
the driving process exited.

## Deciding what is frozen

Ask one question of every candidate: **if the loop could edit this, could it
pass without improving?** If yes, freeze it.

Typically frozen: the corpus, the fixture, the rubric, the score rules, pinned
versions of services the system under test depends on, and `loop.json` itself.

Typically append-only rather than frozen: test files and evidence ledgers. A
round must be able to *add* a rung for a failure it observed — that is how
progress is preserved — but never weaken or remove one. For JSON ledgers, list
them under `measure.json_ledgers` so they are compared structurally; a line diff
gets this wrong, because adding an entry edits the previous line's comma.

Propose-level: build files, dependency manifests, schemas, and contracts. The
loop records the request and a person decides.

## The corpus

Three requests to start. Each is **one real thing a person actually asks the
system to do**, written or approved by a human, sent unchanged through a fresh
interaction. Not a test case — a use. Each should state the persistent effect a
user would expect, so the driver knows what to look for beyond the response.

Do not write ten. The budget is three rounds; the corpus should not outrun it.

## If the product has an agent inside it

The driver must spawn it in a fresh conversation with only the tools the product
exposes, and a scratch `HOME` so no ambient config, memory, or MCP server leaks
in. If the development agent and the product agent share context, the test is
worthless — the product agent will succeed using knowledge no user will ever
provide.

## Finally

Fill in `goals/goal.md` — objective, exclusions, and a completion condition that
names an observable state rather than "tests pass". Then run the first round
with the `otogo-round` skill.
