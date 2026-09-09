"""Authority enforcement: the loop may change the metal, not the stone.

Three levels, per the design:
  free    - normal implementation levers, changed freely inside scope
  propose - crosses a boundary; the loop records evidence, a person decides
  frozen  - the stone itself (corpus, fixture, score rules, rubric, config)

Plus one rule that is not a level: measure files are APPEND-ONLY. The loop may
add a regression check for a failure it observed; it may not weaken or delete
an existing one, because that would let it grade its own repair.
"""
from __future__ import annotations

import difflib
import json
import shutil
from dataclasses import dataclass, field
from pathlib import Path

from .core import GlobSet, Loop, sha256_file, walk_repo


@dataclass
class Snapshot:
    hashes: dict          # rel -> sha256
    measure_files: list   # rels copied verbatim into the round dir
    config: dict          # authority AS OF round start — see check()

    def to_dict(self):
        return {"hashes": self.hashes, "measure_files": self.measure_files,
                "config": self.config}


def take_snapshot(loop: Loop, round_dir: Path) -> Snapshot:
    """Freeze the stone at round start so `guard` has something to compare to."""
    frozen, measure = loop.frozen(), loop.measure()
    hashes, measure_files = {}, []
    copy_root = round_dir / "_snapshot"
    for rel, path in walk_repo(loop.root):
        # Hash everything: `guard` reports a real diff, not just drift in the stone.
        hashes[rel] = sha256_file(path)
        if measure.match(rel):
            measure_files.append(rel)
            dest = copy_root / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, dest)
    # The authority is captured with the hashes. check() must judge the round by
    # the rules in force when it opened, never by rules the round itself edited:
    # a loop that can empty its own `frozen` list has no authority model at all.
    snap = Snapshot(hashes, sorted(measure_files),
                    {"authority": loop.config["authority"],
                     "measure": loop.config["measure"]})
    (round_dir / "snapshot.json").write_text(json.dumps(snap.to_dict(), indent=2) + "\n")
    return snap


@dataclass
class Verdict:
    violations: list = field(default_factory=list)   # hard stops
    proposals: list = field(default_factory=list)    # need a human decision
    warnings: list = field(default_factory=list)     # attribution smells
    changed_free: list = field(default_factory=list)
    appended_measures: list = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.violations

    def to_dict(self):
        return {"ok": self.ok, "violations": self.violations, "proposals": self.proposals,
                "warnings": self.warnings, "changed_free": self.changed_free,
                "appended_measures": self.appended_measures}


def _json_only_grew(old, new, path="$"):
    """Structural append-only for JSON evidence ledgers.

    Line diffs are the wrong tool for a file like docs/witnesses.json: adding a
    witness to an existing claim edits the previous line's comma, which reads as
    tampering when it is the opposite. So compare the parsed values instead —
    keys and array elements may be ADDED, never removed, reordered, or altered.
    Returns a list of violation strings; empty means the ledger only grew.
    """
    bad = []
    if isinstance(old, dict):
        if not isinstance(new, dict):
            return [f"{path}: object replaced by {type(new).__name__}"]
        for k, v in old.items():
            if k not in new:
                return bad + [f"{path}.{k}: removed"]
            bad += _json_only_grew(v, new[k], f"{path}.{k}")
    elif isinstance(old, list):
        if not isinstance(new, list):
            return [f"{path}: array replaced by {type(new).__name__}"]
        if len(new) < len(old):
            return [f"{path}: {len(old) - len(new)} element(s) removed"]
        for i, v in enumerate(old):
            # Existing entries must survive in place; new ones append after.
            bad += _json_only_grew(v, new[i], f"{path}[{i}]")
    elif old != new:
        bad.append(f"{path}: {old!r} -> {new!r}")
    return bad


def _is_pure_append(old: str, new: str) -> bool:
    """True when every original line survives, in order, with only insertions."""
    old_lines, new_lines = old.splitlines(), new.splitlines()
    sm = difflib.SequenceMatcher(None, old_lines, new_lines, autojunk=False)
    return all(tag in ("equal", "insert") for tag, *_ in sm.get_opcodes())


def check(loop: Loop, round_dir: Path) -> Verdict:
    snap_path = round_dir / "snapshot.json"
    if not snap_path.is_file():
        raise FileNotFoundError(snap_path)
    snap = json.loads(snap_path.read_text())
    old_hashes: dict = snap["hashes"]

    # Authority as of `open`, not as it stands now.
    cfg = snap.get("config") or {"authority": loop.config["authority"],
                                 "measure": loop.config["measure"]}
    frozen = GlobSet(cfg["authority"]["frozen"])
    propose = GlobSet(cfg["authority"]["propose"])
    measure = GlobSet(cfg["measure"]["globs"])
    json_ledgers = GlobSet(cfg["measure"].get("json_ledgers") or [])
    append_only = cfg["measure"].get("append_only", True)
    v = Verdict()
    seen = set()

    for rel, path in walk_repo(loop.root):
        seen.add(rel)
        digest = sha256_file(path)
        was = old_hashes.get(rel)
        changed = was is None or was != digest
        fpat = frozen.match(rel)
        mpat = measure.match(rel)
        ppat = propose.match(rel)

        if fpat and changed:
            kind = "added to" if was is None else "modified"
            v.violations.append(
                f"FROZEN {kind}: {rel} (matches `{fpat}`) — this file is part of the stone.")
            continue

        if mpat and changed:
            if was is None:
                v.appended_measures.append(f"{rel} (new check file)")
            elif json_ledgers.match(rel):
                old_text = (round_dir / "_snapshot" / rel).read_text(errors="replace")
                new_text = path.read_text(errors="replace")
                try:
                    diffs = _json_only_grew(json.loads(old_text), json.loads(new_text))
                except json.JSONDecodeError as e:
                    v.violations.append(f"LEDGER unparseable: {rel} — {e}")
                    continue
                if diffs:
                    v.violations.append(
                        f"LEDGER weakened: {rel} — " + "; ".join(diffs[:4])
                        + ("; ..." if len(diffs) > 4 else "")
                        + ". Evidence ledgers may gain entries, never lose or rewrite them.")
                else:
                    v.appended_measures.append(f"{rel} (ledger grew)")
            elif append_only:
                old_text = (round_dir / "_snapshot" / rel).read_text(errors="replace")
                new_text = path.read_text(errors="replace")
                if _is_pure_append(old_text, new_text):
                    v.appended_measures.append(f"{rel} (appended)")
                else:
                    v.violations.append(
                        f"MEASURE weakened: {rel} — existing lines were changed or removed. "
                        "Checks are append-only; a round may add a rung, never lower one.")
            else:
                v.appended_measures.append(f"{rel} (modified)")
            continue

        if ppat and changed:
            v.proposals.append(f"{rel} (matches `{ppat}`)")
            continue

        if changed:
            v.changed_free.append(rel)

    for rel in old_hashes:
        if rel in seen:
            continue
        if frozen.match(rel):
            v.violations.append(f"FROZEN deleted: {rel} — the stone cannot shrink.")
        elif measure.match(rel):
            v.violations.append(f"MEASURE deleted: {rel} — checks are append-only.")
        elif propose.match(rel):
            v.proposals.append(f"{rel} (deleted)")
        else:
            v.changed_free.append(f"{rel} (deleted)")

    # One gap does not mean one file, but it does mean one causal claim.
    limit = loop.config["budget"]["attribution_warn_areas"]
    areas = {r.split("/")[0] for r in v.changed_free}
    if len(areas) > limit:
        v.warnings.append(
            f"attribution: this round touched {len(areas)} top-level areas "
            f"({', '.join(sorted(areas))}). If the round changes several independent levers, "
            "no one can tell which lesson to preserve.")
    if v.changed_free and not v.appended_measures:
        v.warnings.append(
            "no deterministic check was added. A closed gap without a new rung "
            "will be re-earned and re-lost.")
    return v
