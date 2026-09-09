#!/usr/bin/env python3
"""otogo — a harness for autonomous goal loops.

A otogo is the black stone you rub gold against to read its purity. The
stone is never modified; the metal is. Here the stone is the corpus, fixture,
score rules, and rubric — the loop rubs the product against it every round and
is not permitted to reshape the stone when the reading disappoints.

The loop closes one capability per round; the harness decides what the loop is
allowed to change, and what counts as proof that it worked.
"""
from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path

from . import guard
from . import templates
from .core import (DEFAULT_CONFIG, ESCALATE_LAYERS, LAYERS, Loop, LoopError,
                  now, run)

# ------------------------------------------------------------------ output

BOLD, DIM, RED, GRN, YEL, CYN, OFF = (
    "\033[1m", "\033[2m", "\033[31m", "\033[32m", "\033[33m", "\033[36m", "\033[0m")
if not sys.stdout.isatty():
    BOLD = DIM = RED = GRN = YEL = CYN = OFF = ""


def say(msg=""):
    print(msg)


def head(msg):
    say(f"\n{BOLD}{msg}{OFF}")


def ok(msg):
    say(f"  {GRN}ok{OFF}   {msg}")


def bad(msg):
    say(f"  {RED}fail{OFF} {msg}")


def warn(msg):
    say(f"  {YEL}warn{OFF} {msg}")


def note(msg):
    say(f"  {DIM}{msg}{OFF}")


# ------------------------------------------------------------------ helpers

def record(round_dir: Path, name: str, r):
    (round_dir / f"{name}.json").write_text(json.dumps(r.to_dict(), indent=2) + "\n")
    log = round_dir / f"{name}.log"
    log.write_text(f"$ {r.cmd}\n\n[stdout]\n{r.out}\n[stderr]\n{r.err}\n[exit {r.code}]\n")


def rdir(loop: Loop) -> Path:
    return loop.round_dir(loop.open_round()["n"])


def set_phase(loop: Loop, phase: str, **kw):
    loop.open_round().update(phase=phase, **kw)
    loop.save_state()


# ------------------------------------------------------------------ init

def cmd_init(args):
    root = Path(args.path or ".").resolve()
    goals = root / "goals"
    if (goals / "loop.json").exists() and not args.force:
        raise LoopError(f"{goals/'loop.json'} already exists. Use --force to overwrite.")
    for d in (goals, goals / "corpus", goals / "rounds", goals / "score"):
        d.mkdir(parents=True, exist_ok=True)

    written = []

    def put(rel, text):
        p = goals / rel
        if p.exists() and not args.force:
            return
        p.write_text(text)
        written.append(f"goals/{rel}")

    put("goal.md", templates.GOAL_MD)
    put("facts.md", templates.FACTS_MD)
    put("plan.md", templates.PLAN_MD)
    put("LOOP.md", templates.LOOP_MD)
    put("rubric.md", templates.RUBRIC_MD)
    put("proposals.md", templates.PROPOSALS_MD)
    put("corpus/README.md", templates.README_CORPUS)
    put("corpus/overdue-invoices.txt", templates.EXAMPLE_REQUEST)
    put("loop.json", json.dumps(DEFAULT_CONFIG, indent=2) + "\n")

    loop = Loop(root=root, config=DEFAULT_CONFIG)
    loop.state = loop._read_state()
    loop.save_state()
    written += ["goals/STATE.json", "goals/STATE.md"]

    head("Initialized the control plane")
    for w in written:
        ok(w)
    head("Next")
    note("1. Replace the example corpus request with three requests a person approved.")
    note("2. Fill in goals/loop.json commands: reset, health, floor, drive, score.")
    note("3. Write goal.md's completion condition, then `otogo open <request-id>`.")
    say()


# ------------------------------------------------------------------ world

def _preflight(loop: Loop, round_dir: Path) -> str:
    """Reset the fixture and prove the world matches the recorded run."""
    reset, health = loop.cmd("reset"), loop.cmd("health")
    if not health:
        warn("no health command configured — the loop cannot tell infrastructure "
             "trouble from a feature failure. Every score below is suspect.")
        return "unchecked"
    if reset:
        r = run(reset, loop.root)
        record(round_dir, "reset", r)
        if not r.ok:
            bad(f"reset failed (exit {r.code}) — see {round_dir.name}/reset.log")
            return "unhealthy"
        ok("fixture restored")
    r = run(health, loop.root)
    record(round_dir, "health", r)
    if not r.ok:
        bad(f"world unhealthy (exit {r.code}) — see {round_dir.name}/health.log")
        return "unhealthy"
    ok("world healthy")
    return "healthy"


def cmd_preflight(args):
    loop = Loop.find()
    out = loop.goals / "rounds" / "_preflight"
    out.mkdir(parents=True, exist_ok=True)
    head("Preflight")
    status = _preflight(loop, out)
    if status == "unhealthy":
        say(f"\n{RED}NO_SCORE{OFF} — infrastructure trouble is not evidence that "
            f"the feature failed.\n")
        return 2
    return 0


def _floor(loop: Loop, round_dir: Path) -> str:
    floor = loop.cmd("floor")
    if not floor:
        warn("no floor command configured — nothing preserves prior progress.")
        return "unconfigured"
    r = run(floor, loop.root)
    record(round_dir, "floor", r)
    if r.ok:
        ok("floor green")
        return "green"
    bad(f"floor RED (exit {r.code}) — see {round_dir.name}/floor.log")
    return "red"


def cmd_floor(args):
    loop = Loop.find()
    out = loop.goals / "rounds" / "_floor"
    out.mkdir(parents=True, exist_ok=True)
    head("Deterministic floor")
    return 0 if _floor(loop, out) in ("green", "unconfigured") else 1


# ------------------------------------------------------------------ round

def _find_request(loop: Loop, request_id: str) -> Path:
    hits = [p for p in sorted(loop.corpus.glob(f"{request_id}*"))
            if p.is_file() and p.name != "README.md"]
    if not hits:
        avail = ", ".join(p.stem for p in sorted(loop.corpus.iterdir())
                          if p.is_file() and p.name != "README.md") or "(none)"
        raise LoopError(f"no corpus request matching '{request_id}'. Available: {avail}")
    return hits[0]


def cmd_open(args):
    loop = Loop.find()
    if loop.state.get("open_round"):
        n = loop.state["open_round"]["n"]
        raise LoopError(f"round {n:03d} is still open. Close it with "
                        f"`otogo close --outcome ...` or `otogo abort`.")

    budget = loop.config["budget"]["rounds_per_batch"]
    used = loop.state.get("rounds_in_batch", 0)
    if used >= budget and not args.force:
        raise LoopError(
            f"batch {loop.state['batch']} is spent ({used}/{budget} rounds).\n"
            "A person decides at the boundary: continue the queue, redirect the next\n"
            "batch, or stop. Run `otogo batch --continue|--redirect \"...\"|--stop`.")

    req = _find_request(loop, args.request)
    n = max([int(p.name) for p in loop.rounds.glob("[0-9]*") if p.name.isdigit()] or [0]) + 1
    rd = loop.round_dir(n)
    rd.mkdir(parents=True, exist_ok=True)
    shutil.copy2(req, rd / f"request{req.suffix or '.txt'}")

    head(f"Round {n:03d} — request `{req.stem}`")
    world = _preflight(loop, rd)
    floor = "skipped" if world == "unhealthy" else _floor(loop, rd)

    snap = guard.take_snapshot(loop, rd)
    ok(f"stone snapshotted ({len(snap.hashes)} guarded files)")

    loop.state["open_round"] = {
        "n": n, "request": req.stem, "request_file": str(req.relative_to(loop.root)),
        "opened": now(), "phase": "opened", "world": world, "floor": floor,
        "gap": None, "layer": None,
    }
    loop.save_state()

    if world == "unhealthy":
        # An unhealthy world is not a round. Leave the evidence, drop the round,
        # and let the operator reopen once the world matches the recorded run.
        loop.state["open_round"] = None
        loop.state.setdefault("history", []).append(
            {"n": n, "request": req.stem, "layer": "world", "gap": "world unhealthy",
             "outcome": "no_score", "closed": now()})
        loop.save_state()
        say(f"\n{RED}NO_SCORE{OFF} — the world is unhealthy. Fix the world, then reopen.")
        say(f"{DIM}Missing data looks like weak reasoning; stale code looks like a "
            f"broken contract.{OFF}\n")
        return 2
    if floor == "red":
        say(f"\n{YEL}The floor is red.{OFF} That regression is this round's job — "
            f"the driver comes after.\n")
        return 0
    note(f"next: otogo drive")
    say()
    return 0


def cmd_drive(args):
    loop = Loop.find()
    r_ = loop.open_round()
    rd = rdir(loop)
    drive = loop.cmd("drive")
    if not drive:
        raise LoopError("no drive command in goals/loop.json. The driver must approach "
                        "the feature through the same surface a user would.")
    head(f"Drive round {r_['n']:03d} — `{r_['request']}`")
    note("fresh interaction; the request goes through unchanged")
    label = "verify" if args.verify else "drive"
    out_dir = rd / label
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = drive.format(request=str(loop.root / r_["request_file"]),
                       request_file=str(loop.root / r_["request_file"]),
                       out=str(out_dir), out_dir=str(out_dir), round_dir=str(rd))
    res = run(cmd, loop.root, {"OTOGO_REQUEST": str(loop.root / r_["request_file"]),
                               "OTOGO_OUT": str(out_dir),
                               "OTOGO_ROUND": str(rd),
                               "OTOGO_PHASE": label})
    record(rd, label, res)
    (ok if res.ok else bad)(f"driver exit {res.code} — evidence in {rd.name}/{label}/")
    note("record: answer, tool calls, rejected actions, visible result, persistent effects")
    set_phase(loop, label)
    say()
    return 0 if res.ok else 1


def cmd_score(args):
    loop = Loop.find()
    r_ = loop.open_round()
    rd = rdir(loop)
    score = loop.cmd("score")
    if not score:
        raise LoopError("no score command in goals/loop.json.")
    label = "score-verify" if args.verify else "score"
    head(f"Score round {r_['n']:03d} ({label})")
    cmd = score.format(round_dir=str(rd), out=str(rd), phase=("verify" if args.verify else "drive"))
    res = run(cmd, loop.root, {"OTOGO_ROUND": str(rd),
                               "OTOGO_PHASE": ("verify" if args.verify else "drive")})
    record(rd, label, res)
    (ok if res.ok else bad)(f"scorer exit {res.code}")
    tail = (res.out or "").strip().splitlines()[-8:]
    for line in tail:
        note(line)
    r_[f"{label}_exit"] = res.code
    loop.save_state()
    say()
    return 0


def cmd_classify(args):
    loop = Loop.find()
    r_ = loop.open_round()
    if args.layer not in LAYERS:
        raise LoopError(f"unknown layer '{args.layer}'. One of: {', '.join(LAYERS)}")
    r_["layer"] = args.layer
    r_["gap"] = args.gap
    set_phase(loop, "classified")
    head(f"Round {r_['n']:03d} gap classified")
    ok(f"layer `{args.layer}` — {args.gap}")
    (rdir(loop) / "gap.md").write_text(
        f"# Gap\n\n- layer: `{args.layer}`\n- claim: {args.gap}\n- classified: {now()}\n\n"
        "## Path this claim must close\n\n"
        "request -> representation -> operation -> persistent effect -> visible proof\n\n"
        "- [ ] request\n- [ ] representation\n- [ ] operation\n"
        "- [ ] persistent effect\n- [ ] visible proof\n- [ ] deterministic check added\n")
    if args.layer in ESCALATE_LAYERS:
        say(f"\n{YEL}Stop.{OFF} A `{args.layer}` gap is not a product round.")
        if args.layer == "world":
            note("Fix the fixture/services/served code, then reopen the round. NO_SCORE.")
        else:
            note("The harness itself is the failure. Record it with `otogo friction`;")
            note("change the driver, facts, or procedure only on repeated evidence.")
        say()
        return 2
    note("Close that whole path, then add a rung for the behavior, then `otogo verify`.")
    say()
    return 0


def cmd_guard(args):
    loop = Loop.find()
    rd = rdir(loop)
    head(f"Authority check — round {loop.open_round()['n']:03d}")
    v = guard.check(loop, rd)
    (rd / "guard.json").write_text(json.dumps(v.to_dict(), indent=2) + "\n")

    for m in v.violations:
        bad(m)
    for m in v.warnings:
        warn(m)
    if v.changed_free:
        ok(f"{len(v.changed_free)} free file(s) changed")
        for m in v.changed_free[:12]:
            note(m)
        if len(v.changed_free) > 12:
            note(f"... and {len(v.changed_free) - 12} more")
    for m in v.appended_measures:
        ok(f"rung added: {m}")
    if v.proposals:
        warn(f"{len(v.proposals)} propose-level file(s) changed — a person decides:")
        for m in v.proposals:
            note(m)
        _append_proposals(loop, v.proposals)
    if v.ok and not v.proposals:
        ok("no boundary crossed")
    say()
    return 0 if v.ok else 1


def _append_proposals(loop: Loop, items):
    p = loop.goals / "proposals.md"
    text = p.read_text() if p.exists() else templates.PROPOSALS_MD
    r_ = loop.state.get("open_round") or {}
    text += (f"\n## Round {r_.get('n', 0):03d} — {now()}\n"
             f"Gap: {r_.get('gap') or 'unclassified'} (layer `{r_.get('layer')}`)\n\n"
             + "".join(f"- [ ] {i}\n" for i in items))
    p.write_text(text)
    note("recorded in goals/proposals.md")


def cmd_verify(args):
    """Same request, again, after the repair — plus floor and authority."""
    loop = Loop.find()
    r_ = loop.open_round()
    rd = rdir(loop)
    head(f"Verify round {r_['n']:03d}")
    world = _preflight(loop, rd)
    if world == "unhealthy":
        say(f"\n{RED}NO_SCORE{OFF} — cannot attribute this run.\n")
        return 2
    floor = _floor(loop, rd)
    r_["floor_after"] = floor
    loop.save_state()
    rc_guard = cmd_guard(argparse.Namespace())
    args.verify = True
    cmd_drive(args)
    cmd_score(args)
    set_phase(loop, "verified")
    if floor == "red":
        say(f"{RED}The floor went red.{OFF} The repair erased earned behavior — "
            f"that regression is the next job.\n")
        return 1
    return rc_guard


def cmd_close(args):
    loop = Loop.find()
    r_ = loop.open_round()
    rd = rdir(loop)
    head(f"Close round {r_['n']:03d}")

    v = guard.check(loop, rd)
    if not v.ok and not args.force:
        for m in v.violations:
            bad(m)
        raise LoopError("authority violations block the close. Revert the frozen changes, "
                        "or record them as a proposal for a person to decide.")
    if r_.get("floor_after") == "red" and not args.force:
        raise LoopError("the floor is red. A round does not close over a regression.")

    entry = {"n": r_["n"], "request": r_["request"], "layer": r_.get("layer"),
             "gap": r_.get("gap"), "outcome": args.outcome, "closed": now(),
             "world": r_.get("world"), "floor_before": r_.get("floor"),
             "floor_after": r_.get("floor_after"), "rungs_added": v.appended_measures,
             "files_changed": len(v.changed_free)}
    (rd / "round.json").write_text(json.dumps({**r_, **entry}, indent=2) + "\n")
    loop.state.setdefault("history", []).append(entry)
    loop.state["rounds_in_batch"] = loop.state.get("rounds_in_batch", 0) + 1
    loop.state["open_round"] = None
    loop.save_state()

    ok(f"outcome: {args.outcome}")
    if v.appended_measures:
        ok(f"left behind: {len(v.appended_measures)} rung(s)")
    else:
        warn("nothing new is preserved — the next session may pay for this lesson again.")

    if args.friction:
        _friction(loop, args.friction)
    else:
        head("The harness question")
        say(f"  {CYN}What cost time that a rule or check could prevent?{OFF}")
        note("Answer with: otogo friction \"...\"  (first occurrence is evidence;")
        note("repeated friction earns a change to the driver, facts, or procedure)")

    used = loop.state["rounds_in_batch"]
    budget = loop.config["budget"]["rounds_per_batch"]
    head(f"Batch {loop.state['batch']}: {used}/{budget} rounds")
    if used >= budget:
        say(f"  {CYN}Batch boundary — a person decides.{OFF}")
        note("otogo batch --continue | --redirect \"...\" | --stop")
    say()
    return 0


def cmd_abort(args):
    loop = Loop.find()
    r_ = loop.open_round()
    loop.state["open_round"] = None
    loop.state.setdefault("history", []).append(
        {"n": r_["n"], "request": r_["request"], "layer": r_.get("layer"),
         "gap": r_.get("gap"), "outcome": f"aborted: {args.reason}", "closed": now()})
    loop.save_state()
    head(f"Round {r_['n']:03d} aborted")
    ok(args.reason)
    note("evidence kept; the round does not count as progress")
    say()
    return 0


# ------------------------------------------------------------------ meta loops

def _friction(loop: Loop, note_text: str):
    fr = loop.state.setdefault("friction", [])
    for f in fr:
        if f["note"].strip().lower() == note_text.strip().lower():
            f["count"] += 1
            f["last"] = now()
            loop.save_state()
            head("Friction recorded again")
            warn(f"({f['count']}x) {note_text}")
            note("Repeated friction earns a durable constraint: change the driver, "
                 "facts, or LOOP.md — and consider whether the right change is subtraction.")
            return
    fr.append({"note": note_text, "count": 1, "first": now(), "last": now()})
    loop.save_state()
    head("Friction recorded")
    ok(f"(1x) {note_text}")
    note("First occurrence is evidence, not a rule. The loop does not rewrite its "
         "operating system after every surprise.")


def cmd_friction(args):
    loop = Loop.find()
    _friction(loop, args.note)
    say()
    return 0


def cmd_batch(args):
    loop = Loop.find()
    s = loop.state
    if args.stop:
        s["stopped"] = {"at": now(), "reason": args.stop if isinstance(args.stop, str) else "operator"}
        loop.save_state()
        head("Loop stopped by a person")
        note("The score cannot decide the product's direction.")
        say()
        return 0
    if args.redirect:
        s["queue"] = [q.strip() for q in args.redirect.split(";") if q.strip()]
        s["batch"] = s.get("batch", 1) + 1
        s["rounds_in_batch"] = 0
        loop.save_state()
        head(f"Batch {s['batch']} — redirected")
        for q in s["queue"]:
            ok(q)
        say()
        return 0
    if args.continue_:
        s["batch"] = s.get("batch", 1) + 1
        s["rounds_in_batch"] = 0
        loop.save_state()
        head(f"Batch {s['batch']} — continuing the queue")
        say()
        return 0
    head(f"Batch {s.get('batch',1)}")
    note(f"{s.get('rounds_in_batch',0)}/{loop.config['budget']['rounds_per_batch']} rounds used")
    say()
    return 0


def cmd_status(args):
    loop = Loop.find()
    s = loop.state
    head("otogo")
    note(str(loop.root))
    r_ = s.get("open_round")
    if r_:
        head(f"Round {r_['n']:03d} — `{r_['request']}` ({r_.get('phase')})")
        ok(f"world {r_.get('world')} · floor {r_.get('floor')}")
        if r_.get("gap"):
            ok(f"gap [{r_['layer']}] {r_['gap']}")
        else:
            warn("gap unclassified — without it, every agent failure becomes a prompt problem")
        nxt = {"opened": "otogo drive", "drive": "otogo score",
               "score": "otogo classify --layer <layer> --gap \"...\"",
               "classified": "close the path, add a rung, then otogo verify",
               "verify": "otogo close --outcome <improved|unchanged|regressed>",
               "verified": "otogo close --outcome <improved|unchanged|regressed>"}
        note("next: " + nxt.get(r_.get("phase"), "otogo close"))
    else:
        head("No open round")
        note("next: otogo open <request-id>")
    head(f"Batch {s.get('batch',1)}: {s.get('rounds_in_batch',0)}"
         f"/{loop.config['budget']['rounds_per_batch']} rounds")
    hist = list(reversed(s.get("history") or []))[:5]
    if hist:
        head("Recent")
        for e in hist:
            note(f"{e['n']:03d} [{e.get('layer') or '—'}] {e.get('outcome')} — "
                 f"{(e.get('gap') or '')[:60]}")
    fr = [f for f in (s.get("friction") or []) if f["count"] > 1]
    if fr:
        head("Repeated friction (earns a harness change)")
        for f in fr:
            warn(f"({f['count']}x) {f['note']}")
    say()
    return 0


def cmd_brief(args):
    """The packet a fresh agent session reads instead of the old conversation."""
    loop = Loop.find()
    parts = []
    for rel in ("goal.md", "LOOP.md", "facts.md", "STATE.md"):
        p = loop.goals / rel
        if p.exists():
            parts.append(f"===== goals/{rel} =====\n{p.read_text().rstrip()}\n")
    r_ = loop.state.get("open_round")
    if r_:
        rd = loop.round_dir(r_["n"])
        req = next(iter(rd.glob("request.*")), None)
        if req:
            parts.append(f"===== current request ({r_['request']}) =====\n{req.read_text().rstrip()}\n")
        gap = rd / "gap.md"
        if gap.exists():
            parts.append(f"===== current gap =====\n{gap.read_text().rstrip()}\n")
    a = loop.config["authority"]
    parts.append("===== authority =====\n"
                 f"frozen  (the stone; never change): {', '.join(a['frozen']) or '—'}\n"
                 f"propose (a person decides):       {', '.join(a['propose']) or '—'}\n"
                 f"measure (append-only rungs):      {', '.join(loop.config['measure']['globs']) or '—'}\n")
    text = "\n".join(parts)
    if args.out:
        Path(args.out).write_text(text)
        ok(f"brief written to {args.out}")
    else:
        print(text)
    return 0


# ------------------------------------------------------------------ parser

def build_parser():
    p = argparse.ArgumentParser(prog="otogo", description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("init", help="scaffold the goals/ control plane")
    s.add_argument("path", nargs="?")
    s.add_argument("--force", action="store_true")
    s.set_defaults(fn=cmd_init)

    s = sub.add_parser("preflight", help="restore the fixture and prove the world is healthy")
    s.set_defaults(fn=cmd_preflight)

    s = sub.add_parser("floor", help="run the deterministic floor")
    s.set_defaults(fn=cmd_floor)

    s = sub.add_parser("open", help="start a round on one approved request")
    s.add_argument("request")
    s.add_argument("--force", action="store_true", help="open past the batch budget")
    s.set_defaults(fn=cmd_open)

    s = sub.add_parser("drive", help="send the request through a fresh interaction")
    s.add_argument("--verify", action="store_true")
    s.set_defaults(fn=cmd_drive)

    s = sub.add_parser("score", help="read what happened")
    s.add_argument("--verify", action="store_true")
    s.set_defaults(fn=cmd_score)

    s = sub.add_parser("classify", help="name the largest gap and its layer")
    s.add_argument("--layer", required=True, choices=LAYERS)
    s.add_argument("--gap", required=True, help="one causal claim")
    s.set_defaults(fn=cmd_classify)

    s = sub.add_parser("guard", help="check what the round changed against its authority")
    s.set_defaults(fn=cmd_guard)

    s = sub.add_parser("verify", help="floor + authority + the same request again")
    s.set_defaults(fn=cmd_verify)

    s = sub.add_parser("close", help="close the round and record what remains")
    s.add_argument("--outcome", required=True,
                   choices=["improved", "unchanged", "regressed", "no_score"])
    s.add_argument("--friction", help="answer the harness question inline")
    s.add_argument("--force", action="store_true")
    s.set_defaults(fn=cmd_close)

    s = sub.add_parser("abort", help="end the round without counting it as progress")
    s.add_argument("reason")
    s.set_defaults(fn=cmd_abort)

    s = sub.add_parser("friction", help="record what cost time this round")
    s.add_argument("note")
    s.set_defaults(fn=cmd_friction)

    s = sub.add_parser("batch", help="the human boundary between batches")
    s.add_argument("--continue", dest="continue_", action="store_true")
    s.add_argument("--redirect", help="new queue, entries separated by ';'")
    s.add_argument("--stop", nargs="?", const=True)
    s.set_defaults(fn=cmd_batch)

    s = sub.add_parser("status", help="where the loop is and what comes next")
    s.set_defaults(fn=cmd_status)

    s = sub.add_parser("brief", help="the packet a fresh agent session should read")
    s.add_argument("--out")
    s.set_defaults(fn=cmd_brief)

    return p


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        return args.fn(args) or 0
    except LoopError as e:
        say(f"\n{RED}error{OFF} {e}\n")
        return 2


if __name__ == "__main__":
    sys.exit(main())
