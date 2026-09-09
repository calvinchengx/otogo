"""Shared primitives: config, paths, globs, hashing, state."""
from __future__ import annotations

import fnmatch
import hashlib
import json
import os
import re
import shutil
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path

GOALS = "goals"
CONFIG = "loop.json"
STATE = "STATE.json"

LAYERS = [
    "world",     # fixture, services, model, served code
    "domain",    # data model / business rules
    "contract",  # tool + API surface between agent and system
    "runtime",   # execution, orchestration, error handling
    "steering",  # agent instructions / prompts
    "surface",   # UI, rendering, visible proof
    "harness",   # driver, corpus, scorer, procedure
]

# Layers that are NOT product gaps: the round must stop and escalate.
ESCALATE_LAYERS = {"world", "harness"}


class LoopError(Exception):
    """Operator-facing failure. Printed without a traceback."""


# ---------------------------------------------------------------- globbing

def _glob_re(pattern: str) -> re.Pattern:
    """Translate a `**`-aware glob into a regex anchored at the repo root."""
    i, out = 0, []
    while i < len(pattern):
        c = pattern[i]
        if pattern.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        elif c == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(c))
            i += 1
    return re.compile("^" + "".join(out) + "$")


class GlobSet:
    def __init__(self, patterns):
        self.patterns = list(patterns or [])
        self._res = [(p, _glob_re(p)) for p in self.patterns]

    def match(self, rel: str):
        """Return the first matching pattern, or None."""
        for pat, rx in self._res:
            if rx.match(rel):
                return pat
        return None

    def __bool__(self):
        return bool(self.patterns)


# ---------------------------------------------------------------- hashing

def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


# The loop's own bookkeeping: rewritten every round, never a product change.
BOOKKEEPING = {f"{GOALS}/STATE.json", f"{GOALS}/STATE.md", f"{GOALS}/proposals.md"}

IGNORE_DIRS = {".git", "node_modules", "__pycache__", ".venv", "venv",
               "dist", "build", ".next", ".pytest_cache", ".mypy_cache"}


def walk_repo(root: Path):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in IGNORE_DIRS]
        for fn in filenames:
            p = Path(dirpath) / fn
            rel = p.relative_to(root).as_posix()
            # rounds/ is the loop's own evidence log; never guarded.
            if rel.startswith(f"{GOALS}/rounds/") or rel in BOOKKEEPING:
                continue
            yield rel, p


# ---------------------------------------------------------------- config

DEFAULT_CONFIG = {
    "version": 1,
    "commands": {
        "reset": "",
        "health": "",
        "floor": "",
        "drive": "",
        "score": "",
    },
    "authority": {
        "frozen": [
            "goals/corpus/**",
            "goals/rubric.md",
            "goals/score/**",
            "goals/loop.json",
            "fixtures/**",
        ],
        "propose": [],
        "free": ["**"],
    },
    "measure": {
        "globs": ["tests/**"],
        "append_only": True,
        # JSON evidence ledgers compared structurally, not line by line.
        "json_ledgers": [],
    },
    "budget": {
        "rounds_per_batch": 3,
        "attribution_warn_areas": 3,
    },
}


@dataclass
class Loop:
    root: Path
    config: dict
    state: dict = field(default_factory=dict)

    # -- locations
    @property
    def goals(self) -> Path:
        return self.root / GOALS

    @property
    def rounds(self) -> Path:
        return self.goals / "rounds"

    @property
    def corpus(self) -> Path:
        return self.goals / "corpus"

    @property
    def state_path(self) -> Path:
        return self.goals / STATE

    # -- load / save
    @classmethod
    def find(cls, start: Path | None = None) -> "Loop":
        cur = (start or Path.cwd()).resolve()
        for cand in [cur, *cur.parents]:
            if (cand / GOALS / CONFIG).is_file():
                cfg = json.loads((cand / GOALS / CONFIG).read_text())
                loop = cls(root=cand, config=merge(DEFAULT_CONFIG, cfg))
                loop.state = loop._read_state()
                return loop
        raise LoopError(
            "no goals/loop.json found in this directory or any parent.\n"
            "Run `otogo init` at the repository root first."
        )

    def _read_state(self) -> dict:
        if self.state_path.is_file():
            return json.loads(self.state_path.read_text())
        return {"batch": 1, "rounds_in_batch": 0, "open_round": None,
                "queue": [], "history": []}

    def save_state(self):
        self.state_path.write_text(json.dumps(self.state, indent=2) + "\n")
        self.render_state_md()

    # -- authority
    def frozen(self) -> GlobSet:
        return GlobSet(self.config["authority"]["frozen"])

    def propose(self) -> GlobSet:
        return GlobSet(self.config["authority"]["propose"])

    def measure(self) -> GlobSet:
        return GlobSet(self.config["measure"]["globs"])

    # -- rounds
    def round_dir(self, n: int) -> Path:
        return self.rounds / f"{n:03d}"

    def open_round(self) -> dict:
        r = self.state.get("open_round")
        if not r:
            raise LoopError("no open round. Run `otogo open <request-id>`.")
        return r

    def cmd(self, name: str) -> str:
        return (self.config["commands"].get(name) or "").strip()

    # -- human-readable mirror of STATE.json
    def render_state_md(self):
        s = self.state
        lines = ["# STATE", "",
                 f"_Generated by otogo at {now()}. Machine copy: goals/STATE.json._", ""]
        r = s.get("open_round")
        if r:
            lines += [f"## Open round {r['n']:03d} — request `{r['request']}`",
                      f"- phase: **{r.get('phase', 'opened')}**",
                      f"- world: {r.get('world', 'unknown')}",
                      f"- floor: {r.get('floor', 'not run')}",
                      f"- gap: {r.get('gap') or '_unclassified_'}"
                      + (f" (layer: `{r['layer']}`)" if r.get("layer") else ""),
                      f"- evidence: `goals/rounds/{r['n']:03d}/`", ""]
        else:
            lines += ["## No open round", ""]
        budget = self.config["budget"]["rounds_per_batch"]
        lines += [f"## Batch {s.get('batch', 1)}",
                  f"- rounds used: {s.get('rounds_in_batch', 0)} / {budget}", ""]
        q = s.get("queue") or []
        lines += ["## Queue"] + ([f"{i+1}. {item}" for i, item in enumerate(q)] or ["_empty_"]) + [""]
        h = list(reversed(s.get("history") or []))[:12]
        lines += ["## Recent rounds", ""]
        if h:
            lines += ["| # | request | layer | gap | outcome |",
                      "|---|---------|-------|-----|---------|"]
            for e in h:
                lines.append(
                    f"| {e['n']:03d} | `{e.get('request','')}` | {e.get('layer','') or '—'} "
                    f"| {(e.get('gap') or '—')[:60]} | {e.get('outcome','')} |")
        else:
            lines.append("_none yet_")
        lines.append("")
        fr = s.get("friction") or []
        if fr:
            lines += ["## Open friction (candidates for harness change)", ""]
            for f in fr[-10:]:
                lines.append(f"- ({f['count']}x) {f['note']}")
            lines.append("")
        (self.goals / "STATE.md").write_text("\n".join(lines))


def merge(base: dict, over: dict) -> dict:
    out = dict(base)
    for k, v in (over or {}).items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = merge(out[k], v)
        else:
            out[k] = v
    return out


def now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S%z")


# ---------------------------------------------------------------- shell

@dataclass
class Run:
    cmd: str
    code: int
    out: str
    err: str
    seconds: float

    @property
    def ok(self) -> bool:
        return self.code == 0

    def to_dict(self):
        return {"cmd": self.cmd, "exit": self.code, "seconds": round(self.seconds, 2),
                "stdout": self.out[-20000:], "stderr": self.err[-20000:]}


def run(cmd: str, cwd: Path, env_extra: dict | None = None, timeout: int = 3600) -> Run:
    env = dict(os.environ)
    env.update({k: str(v) for k, v in (env_extra or {}).items()})
    t0 = time.time()
    try:
        p = subprocess.run(cmd, shell=True, cwd=str(cwd), env=env,
                           capture_output=True, text=True, timeout=timeout)
        return Run(cmd, p.returncode, p.stdout, p.stderr, time.time() - t0)
    except subprocess.TimeoutExpired as e:
        return Run(cmd, 124, e.stdout or "", (e.stderr or "") + f"\n[timeout after {timeout}s]",
                   time.time() - t0)
