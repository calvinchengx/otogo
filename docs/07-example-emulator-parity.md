# Example: growing emulator parity

Anchored on `azure-keyvault-emulator`, but every sibling emulator in the
workspace has the same shape.

## Why this is the case the harness is for

The essay's test is: *does use have to reveal the capability sequence?* For an
emulator, yes, unavoidably. You cannot write the parity tests up front, because
the thing you are trying to discover is **which parts of the wire protocol an
unmodified SDK actually exercises** — the challenge it walks, the header it
insists on, the second call it makes after the first succeeds. A hand-written
test suite encodes what you already believed. Only a real `azidentity` client
hitting a real socket tells you what Key Vault actually promises.

This is also exactly the failure mode the essay warns about: an emulator can
*look* complete and store nothing. It can return `200` with a plausible body
while no state changes. That is a partial path — `request → representation →
operation → persistent effect → visible proof` with the last two missing — and
it is the most convincing kind of failure there is.

## What maps to what

Your repo already had four of the five pieces. The harness mostly names them.

| harness concept | already in the repo |
|---|---|
| **reset** | `make clean && make up` — the `restart` target is literally these two |
| **health** | `make status` — *"Report whether the pair is usable (non-zero exit if not)"* |
| **floor** | `make test` — `go build && go vet && go test ./...` |
| **driver** | `e2e/sdk/{python,js,dotnet}`, `e2e/az-cli`, `make e2e-host-routed` |
| **the rubric** | `docs/parity.md` — the 🟢/🟡/🟠/🔴 legend and the claim rows |
| **the ledger** | `docs/witnesses.json` — 49 claims, each with the tests that prove it |
| **the corpus** | *the missing piece* — see below |

`make status` deserves special mention: it is already the preflight contract the
essay asks for, written before anyone asked for it. An emulator stack that
didn't come up is not a parity failure, and without that check every round would
risk repairing a feature that was never broken.

## The corpus is the part you have to add

A corpus request is **one real operation a real client performs**, written by a
person, sent unchanged. Not a test — a use. Three to start:

```
goals/corpus/
  sdk-python-secret-roundtrip.md
  azcli-set-show-secret.md
  sdk-dotnet-key-sign-verify.md
```

Each names the client, the operation, and — critically — the *persistent effect*
a user would expect, so the driver knows what to look for beyond the response
body.

## Authority: what the loop may and may not touch

```json
"frozen":  ["goals/corpus/**", "docs/parity.md", "docs/parity-snapshots/**",
            "docker-compose.yml"],
"measure": ["**/*_test.go", "e2e/**", "docs/witnesses.json"]
```

- **`docs/parity.md` is frozen.** A 🔴 row is the exam question. A loop that can
  edit the claim can pass by rewording it.
- **`docker-compose.yml` is frozen** because it pins the emulator versions of
  the *other* services in the chain. A loop that can bump `ENTRA_EMULATOR_VERSION`
  to make its round pass has changed the world mid-exam.
- **`docs/witnesses.json` is an append-only ledger**, checked structurally
  rather than by line diff. A round may add a witness to a claim or add a new
  claim. It may not drop a witness, rename one to point at an easier test, or
  soften a claim's text. Those all read as "the evidence improved" in a line
  diff and are caught here:

```
fail LEDGER weakened: docs/witnesses.json — $.token-validation.witnesses[1]:
     'go:TestJWKSFailureModes' -> 'go:TestSomethingEasier'.
     Evidence ledgers may gain entries, never lose or rewrite them.
```

- **`Makefile` and `Dockerfile` are propose-level.** Changing how the thing is
  built or tested is a boundary crossing; the loop records it in
  `goals/proposals.md` and you decide.

## A round, concretely

The goal: *turn one 🔴 row in `docs/parity.md` 🟢, proven by an unmodified SDK.*

```bash
otogo open sdk-python-secret-roundtrip
```
`make clean && make up`, then `make status`, then `make test`, then it hashes
parity.md, the corpus, the compose file, and every `_test.go`.

```bash
otogo drive
```
Runs the real `azure-keyvault-secrets` client against the emulator in a fresh
process, and records the response **and the effect** — a second, independent
read confirming the secret is actually there.

```bash
otogo score
```

Suppose the SDK's `set_secret` returns `200` but a follow-up `get_secret` 404s.
That is the classic partial path. The development agent classifies it:

```bash
otogo classify --layer domain \
  --gap "set_secret returns the object envelope but never writes to the store, so no later read can find it"
```

Not `contract` — the wire shape was right. Not `surface` — there is no UI. The
cause is that the operation performs no persistent effect. Getting this wrong is
what the essay means by *the loop changes the nearest thing instead of the right
thing*.

The agent then closes that path in `internal/`, appends a Go test, and adds the
witness to the ledger:

```json
"secret-set-get-roundtrip": {
  "section": "Secrets (`secrets/`)",
  "claim": "Set then get returns the stored value",
  "witnesses": ["go:TestSecretRoundtrip", "sdk:TestPythonSecretRoundtrip"]
}
```

```bash
otogo verify   # make status, make test, authority check, same request again
otogo close --outcome improved
```

`verify` re-runs the floor, so if the fix broke one of the other 48 claims, the
round refuses to close and that regression becomes the next job.

## Running three of them unattended

```bash
./examples/batch.sh sdk-python-secret-roundtrip azcli-set-show-secret sdk-dotnet-key-sign-verify
```

Three rounds, then it stops and waits for you. It also stops early on any of the
essay's stop rules: unhealthy world, a `world` or `harness` classification, an
authority violation, or a floor that went red.

## Files

The runnable pieces live in [`examples/emulator-parity/`](../examples/emulator-parity/):
`loop.json`, `drive.sh`, `score.py`, and a corpus request.
