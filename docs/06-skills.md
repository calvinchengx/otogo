# The CLI enforces; the skills teach

otogo contains no model. It shells out, hashes files, and compares JSON — the
referee, not the player.

That separation is deliberate. **A skill is advisory and a subprocess exit code
is not.** An agent that decides the corpus request is unfair can argue its way
past an instruction telling it not to edit the corpus. It cannot argue its way
past a SHA-256 comparison. Authority has to live outside the model's control or
it is not authority.

## Two skills

```bash
make install-skills                       # into ./.claude/skills
make install-skills DEST=~/.claude/skills # or globally
```

| skill | for |
|---|---|
| `otogo-setup` | wiring a loop into a repo — the five commands, what to freeze, the first three requests |
| `otogo-round` | running one round — drive, classify, close one path, add a rung, stop at the stop rules |

## One source of truth

Neither skill restates the authority list. They point the agent at
`otogo brief`, which reads it from `loop.json` at run time.

This is not stylistic. A second copy of the rules goes stale silently, and a
stale authority list is worse than none — it teaches an agent to expect
protections that no longer exist. Three tests enforce it:

- a skill may not hardcode a frozen glob;
- a skill may not teach a command the CLI does not have;
- `examples/round.sh` must hand the agent the *skill file*, not a copy of the
  procedure kept beside it.

The third exists because that duplicate was real: an `AGENT.md` sat next to the
runner restating what the skill teaches, and the runner fed the agent that
instead. Both entry points now read the same file.

## If the product has an agent inside it

The driver must spawn it in a fresh conversation, with only the tools the
product exposes and a scratch `HOME` so no ambient config, memory, or MCP
server leaks in.

If the development agent and the product agent share context, the test is
worthless — the product agent will succeed using knowledge no user will ever
provide.
