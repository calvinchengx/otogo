//! Files written by `otogo init`. The repository is the control plane.
//!
//! Generated from the reference implementation's templates; kept verbatim so
//! a repo scaffolded by either implementation is byte-identical.

pub const GOAL_MD: &str = r##"# Goal

## Objective
<One capability, stated so a person could tell whether it exists.>

## Exclusions
- <What this goal explicitly does not cover.>

## Completion condition
<The observable state that ends the goal. Not "tests pass" — an effect a user
would notice, provable from the corpus.>
"##;

pub const FACTS_MD: &str = r##"# Facts

Decisions later rounds must not rediscover. Append only; each entry dated.
Do not put speculation here — a fact earns its place by having cost a round.

- <YYYY-MM-DD> <decision, and the evidence that settled it>
"##;

pub const PLAN_MD: &str = r##"# Plan

## Initial floor
<The deterministic checks that are green today and must stay green.>

## Expected capability order
1. <first capability the corpus should force into existence>
2. <second>
3. <third>

This order is a hypothesis. The driver decides the real one.
"##;

pub const LOOP_MD: &str = r##"# LOOP

The procedure. This file changes when the *process* changes, not after a round.

## The round
1. `otogo open <request-id>` — restore the fixture, prove the world is healthy,
   run the deterministic floor, snapshot the stone.
2. `otogo drive` — send one approved request, unchanged, through a fresh
   interaction. Record behavior, tool calls, rejected actions, visible result,
   and persistent effects.
3. `otogo score` — read what happened.
4. `otogo classify --layer <layer> --gap "<one causal claim>"` — name the
   largest gap and the layer it lives in.
5. Close that gap through every layer the claim requires:
   `request -> representation -> operation -> persistent effect -> visible proof`
6. Add a deterministic check for the behavior you just made work.
7. `otogo verify` — send the same request again, record the result.
8. `otogo close --outcome <improved|unchanged|regressed>` and answer the
   friction question.

## Authority
- **free** — implementation levers inside the assigned scope.
- **propose** — crosses a boundary. `otogo guard` records it in
  proposals.md; a person decides.
- **frozen** — the stone: corpus, fixture, existing rungs, score rules, rubric,
  and this loop's config. The loop can change the product. It cannot change the
  evidence that decides whether the product improved.

Checks are append-only. A round may add a rung for a failure it observed.
It may never weaken a rung or lower a threshold.

## Budget
Three rounds per batch. At the batch boundary a person continues, redirects,
or stops.

## Stop rules
Stop the round and escalate when:
- the world is unhealthy — record NO_SCORE, not a failure;
- the failure cannot be reproduced or attributed;
- the repair needs a frozen change;
- the gap classifies as `world` or `harness` — that is not a product round.
"##;

pub const RUBRIC_MD: &str = r##"# Rubric

How a run is judged. FROZEN — the loop cannot edit this.

## Deterministic
<Effect checks, schema validation, and rung outcomes. Machine-decidable.>

## Judged
<What needs a screenshot, repeated samples, or a person. State the ranking
question exactly as it will be asked.>

A learned judge remains *evidence* until its rankings agree with repeated human
rankings. Until then it does not decide a round.
"##;

pub const README_CORPUS: &str = r##"# Corpus

Real demand. Each request is a file a person wrote or approved, sent unchanged
through a fresh interaction. FROZEN — the loop cannot add, edit, or remove one.

One file per request. The filename (without extension) is the request id.
"##;

pub const EXAMPLE_REQUEST: &str = r##"Show me every invoice that went overdue this week and mark
the two largest as escalated.
"##;

pub const PROPOSALS_MD: &str = r##"# Proposals

Boundary changes the loop wants and a person must decide. Newest last.
"##;
