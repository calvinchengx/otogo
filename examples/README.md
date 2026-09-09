# Examples

| | shape | driver approaches the product through |
|---|---|---|
| [`emulator-parity/`](emulator-parity/) | a service with a wire protocol | a real, unmodified SDK and CLI |
| [`agent-product/`](agent-product/) | a product with an agent inside it | a fresh agent session with only the product's tools |

Both share the same runners:

- [`AGENT.md`](AGENT.md) — the standing instruction handed to the development
  agent each round. It is the whole prompt; everything else it needs comes from
  `otogo brief`.
- [`round.sh`](round.sh) — one autonomous round. Exits `3` on any stop rule.
- [`batch.sh`](batch.sh) — three rounds, then it stops and waits for a person.

## The scorer's contract

`round.sh` decides the outcome from the score files, not from the agent's
opinion of its own work. A scorer writes `score-<phase>.json` into the round
directory with at least:

```json
{ "behaviour_ok": true, "effect_ok": false, "partial_path": true, "pass": false }
```

`behaviour_ok` is *the client's calls succeeded*. `effect_ok` is *a fresh,
independent read still sees the state*. When the first is true and the second is
false you have a partial path, which is the failure worth naming separately
because it is the one that looks most like success.
