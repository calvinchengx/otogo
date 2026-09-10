# Examples

Runnable pieces. The prose that explains them lives in [`docs/`](../docs/) and
on the [docs site](https://calvinchengx.github.io/otogo/).

| | what it is |
|---|---|
| [`round.sh`](round.sh) | one autonomous round; exits `3` on any stop rule |
| [`batch.sh`](batch.sh) | three rounds, then it waits for a person |
| [`emulator-parity/`](emulator-parity/) | a real `loop.json`, driver, scorer and corpus request, anchored on `azure-keyvault-emulator` |

`round.sh` hands the development agent the
[`otogo-round`](../skills/otogo-round/) skill, read at run time — there is
deliberately no second copy of the procedure here to go stale.

## The scorer's contract

`round.sh` decides the outcome from the score files, not from the agent's
opinion of its own work. A scorer writes `score-<phase>.json` into the round
directory with at least:

```json
{ "behaviour_ok": true, "effect_ok": false, "partial_path": true, "pass": false }
```

`behaviour_ok` is *the client's calls succeeded*. `effect_ok` is *a fresh,
independent read still sees the state*. When the first is true and the second
is false you have a partial path — the failure worth naming separately because
it looks most like success.
