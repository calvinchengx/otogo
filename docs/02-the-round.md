# The round

One round closes one gap. Not one file — **one causal claim**.

## The eight steps

1. Restore the fixture and check the running system.
2. Run the deterministic floor.
3. Send one approved request through a fresh interaction.
4. Collect the behaviour, visible result, and persistent effects.
5. Classify the largest gap.
6. Close that gap through every required layer.
7. Add a deterministic check for the behaviour.
8. Send the same request again and record the result.

`otogo open` does 1–2, `drive` does 3–4, `classify` does 5, you (or an agent)
do 6–7, and `verify` does 8 — plus the floor and the authority check again.

## Classify before you repair

```bash
otogo classify --layer <layer> --gap "<one causal claim>"
```

| layer | the cause lives in |
|---|---|
| `world` | the fixture, services, model, or served code |
| `domain` | the data model or business rules |
| `contract` | the tool or API surface between agent and system |
| `runtime` | execution, orchestration, error handling |
| `steering` | agent instructions or prompts |
| `surface` | UI, rendering, visible proof |
| `harness` | the driver, corpus, scorer, or procedure |

Without this step every agent failure becomes a prompt problem and every visual
failure becomes a frontend problem — the loop changes the nearest thing instead
of the right thing.

**`world` and `harness` exit 2 and stop the round.** Neither is product work.
A `world` gap means fix the world and reopen; a `harness` gap means the harness
is what failed, and belongs in `otogo friction` rather than a repair.

## The path a claim must close

```
request → representation → operation → persistent effect → visible proof
```

`classify` writes this into `gap.md` as a checklist. Partial paths make the
most convincing failures there are: the screen that renders and stores nothing,
the tool that returns success while state stays put, the page that looks
complete and answers nothing.

A narrow path that works from request to effect is worth more than five layers
of unfinished possibility.

## Stop rules

| condition | exit | meaning |
|---|---|---|
| world unhealthy at `open` | 2 | NO_SCORE — the round is dropped, not recorded as failure |
| `classify --layer world\|harness` | 2 | not a product round |
| `guard` finds a violation | 1 | the round crossed a frozen path or weakened a check |
| floor red at `verify` | 1 | the repair erased earned behaviour |
| batch budget spent | 2 | a person decides at the boundary |

Infrastructure trouble is not evidence that the feature failed. That
distinction prevents a large class of false repairs: missing data looks like
weak reasoning, and stale code looks like a broken contract.
