# Quickstart

otogo runs a goal loop: one capability per round, driven by a real request
through the real surface, scored on what actually changed. The agent owns the
turns inside a round; a person owns the boundary between batches.

## Install

```bash
cargo install --path .
```

A single static binary, ~550 KB, three dependencies.

## Scaffold

```bash
cd your-repo
otogo init
```

That writes `goals/` — the control plane. See [Configuration](05-configuration.md)
for the five commands you need to fill in, and [Authority](03-authority.md) for
what to freeze.

## One round

```bash
otogo open sdk-python-secret-roundtrip   # reset, health, floor, snapshot the exam
otogo drive                              # one approved request, fresh interaction
otogo score                              # read what happened
otogo classify --layer domain --gap "set returns the envelope but never writes"
#   ... close that whole path, then add a rung ...
otogo verify                             # health + floor + authority + the same request
otogo close --outcome improved
```

Every command exits non-zero when a stop rule fires, so a runner script can
branch on it without parsing output. See [The round](02-the-round.md).

## Unattended

```bash
./examples/round.sh sdk-python-secret-roundtrip   # one round, agent in the middle
./examples/batch.sh req-a req-b req-c             # three, then it waits for you
```

## Should you use it at all?

Most work needs none of this. If complete tests already describe the job, give
the agent the tests. A goal loop earns its cost when **use has to reveal the
capability sequence** — you are growing a capability nobody has fully specified
— and when the system **exposes effects you can observe**.

If you cannot read the effect from underneath, no round can tell success from a
convincing description of it.
