#!/usr/bin/env python3
"""Scorer: read what happened. Deterministic where it can be.

Writes score.json next to the phase directory. The controller compares the
drive-phase score with the verify-phase score to decide the round's outcome;
neither this file nor the rubric it implements is the loop's to edit.
"""
import argparse, json, pathlib, sys

ap = argparse.ArgumentParser()
ap.add_argument("--round", required=True)
ap.add_argument("--phase", default="drive")
a = ap.parse_args()

rd = pathlib.Path(a.round)
phase = rd / a.phase
def read(name, default=""):
    p = phase / name
    return p.read_text(errors="replace") if p.exists() else default

behaviour_ok = read("exit", "1").strip() == "0"
effect_ok    = read("effects_exit", "1").strip() == "0"
rejected     = [l for l in read("rejected.txt").splitlines() if l.strip()]

score = {
    "phase": a.phase,
    "behaviour_ok": behaviour_ok,     # did the client's calls succeed
    "effect_ok": effect_ok,           # did a FRESH client still see the state
    "rejected": rejected[:20],
    # The distinction the essay insists on: answering correctly is not the same
    # as having done anything.
    "partial_path": behaviour_ok and not effect_ok,
    "pass": behaviour_ok and effect_ok,
}
(rd / f"score-{a.phase}.json").write_text(json.dumps(score, indent=2) + "\n")

print(f"behaviour_ok={behaviour_ok}  effect_ok={effect_ok}  pass={score['pass']}")
if score["partial_path"]:
    print("PARTIAL PATH: the client was answered but nothing was stored.")
for r in rejected[:5]:
    print(f"  rejected: {r.strip()[:90]}")
sys.exit(0)   # scoring always succeeds; the SCORE is the output, not the exit
