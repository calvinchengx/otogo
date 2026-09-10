# Beyond feature work

The five commands in `loop.json` are domain-specific. Everything above them is
not.

| software | general |
|---|---|
| fixture | the starting conditions you can restore |
| health check | is the world in a state where a result would mean anything |
| floor | what must not get worse |
| corpus | real demand, written by a person, sent unchanged |
| witness | what vouches for this claim |
| round | one gap, one causal claim |
| the frozen exam | the measure the doer cannot edit |

## Recurring day-to-day work

This fits the shape more naturally than feature work does, because **the corpus
arrives on its own**. The triage queue, the weekly report, the review pass, the
runbook you execute every deploy — these are real demand, written by other
people, already approved by the fact that someone asked for them. You do not
have to invent representative requests; you have to pick three and stop editing
them.

The failure mode is the same one wearing different clothes. A task agent that
*describes* the right action without performing it looks exactly like an
emulator that answers without storing. The report gets written and the number
in it was never recomputed. The ticket gets a comment and no field changes. So
the driver still has to read the effect from the system underneath, never from
what the agent said it did.

What day-to-day work adds is a floor that is easy to lose. Quality drifts
silently on recurring tasks — nobody notices the digest got vaguer, or the
review stopped checking the thing it used to check. That is precisely what a
rung is for: the round that first catches it writes down the check, and no
later round can quietly remove it.

## Where it does not earn its cost

One-off tasks. If the work happens once, there is no second round to preserve
anything for, and the harness is pure overhead.

Use it when the task recurs, the output has an effect you can observe, and the
quality is the kind that erodes without anyone deciding to erode it.
