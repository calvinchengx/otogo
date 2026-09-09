//! Authority enforcement: the loop may change the product, not the exam.
//!
//!   free    — implementation levers, changed freely inside scope
//!   propose — crosses a boundary; recorded, a person decides
//!   frozen  — the exam: corpus, fixture, score rules, rubric, loop.json
//!
//! Plus one rule that is not a level: measures are APPEND-ONLY. A round may add
//! a rung for a failure it observed; it may not weaken or delete one, because
//! that would let it grade its own repair.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::core::{pretty, sha256_file, walk_repo, GoalLoop, LoopError, Res};
use crate::glob::GlobSet;

pub struct Verdict {
    pub violations: Vec<String>,
    pub proposals: Vec<String>,
    pub warnings: Vec<String>,
    pub changed_free: Vec<String>,
    pub appended_measures: Vec<String>,
}

impl Verdict {
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "ok": self.ok(),
            "violations": self.violations,
            "proposals": self.proposals,
            "warnings": self.warnings,
            "changed_free": self.changed_free,
            "appended_measures": self.appended_measures,
        })
    }
}

/// Freeze the exam at round start. The authority is captured alongside the
/// hashes: a round must be judged by the rules in force when it opened, never
/// by rules the round itself edited — a loop that can empty its own `frozen`
/// list has no authority model at all.
pub fn take_snapshot(l: &GoalLoop, round_dir: &Path) -> Res<usize> {
    let measure = l.measure();
    let mut hashes = BTreeMap::new();
    let mut measure_files = Vec::new();
    let copy_root = round_dir.join("_snapshot");

    for (rel, path) in walk_repo(&l.root) {
        hashes.insert(rel.clone(), sha256_file(&path)?);
        if measure.is_match(&rel) {
            measure_files.push(rel.clone());
            let dest = copy_root.join(&rel);
            if let Some(p) = dest.parent() {
                fs::create_dir_all(p).ok();
            }
            fs::copy(&path, &dest).map_err(|e| LoopError(format!("{rel}: {e}")))?;
        }
    }
    let n = hashes.len();
    let snap = json!({
        "hashes": hashes,
        "measure_files": measure_files,
        "config": {
            "authority": l.config.get("authority").cloned().unwrap_or(Value::Null),
            "measure": l.config.get("measure").cloned().unwrap_or(Value::Null),
        }
    });
    fs::write(
        round_dir.join("snapshot.json"),
        format!("{}\n", pretty(&snap)),
    )
    .map_err(|e| LoopError(e.to_string()))?;
    Ok(n)
}

/// True when every original line survives, in order, with only insertions —
/// i.e. `old` is a subsequence of `new`.
fn is_pure_append(old: &str, new: &str) -> bool {
    let mut it = new.lines();
    for line in old.lines() {
        let mut found = false;
        for candidate in it.by_ref() {
            if candidate == line {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

/// Structural append-only for JSON evidence ledgers.
///
/// A line diff is the wrong tool here: adding a witness to an existing claim
/// edits the previous line's comma, which reads as tampering when it is the
/// opposite. Compare parsed values instead — keys and array elements may be
/// ADDED, never removed, reordered, or altered.
fn json_only_grew(old: &Value, new: &Value, path: &str, bad: &mut Vec<String>) {
    match (old, new) {
        (Value::Object(o), Value::Object(n)) => {
            for (k, v) in o {
                match n.get(k) {
                    Some(nv) => json_only_grew(v, nv, &format!("{path}.{k}"), bad),
                    None => bad.push(format!("{path}.{k}: removed")),
                }
            }
        }
        (Value::Object(_), other) => {
            bad.push(format!("{path}: object replaced by {}", kind(other)))
        }
        (Value::Array(o), Value::Array(n)) => {
            if n.len() < o.len() {
                bad.push(format!("{path}: {} element(s) removed", o.len() - n.len()));
                return;
            }
            for (i, v) in o.iter().enumerate() {
                // Existing entries must survive in place; new ones append after.
                json_only_grew(v, &n[i], &format!("{path}[{i}]"), bad);
            }
        }
        (Value::Array(_), other) => bad.push(format!("{path}: array replaced by {}", kind(other))),
        (a, b) if a != b => bad.push(format!("{path}: {a} -> {b}")),
        _ => {}
    }
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn check(l: &GoalLoop, round_dir: &Path) -> Res<Verdict> {
    let snap_path = round_dir.join("snapshot.json");
    let raw = fs::read_to_string(&snap_path).map_err(|_| {
        LoopError(format!(
            "{} missing — was the round opened?",
            snap_path.display()
        ))
    })?;
    let snap: Value = serde_json::from_str(&raw).map_err(|e| LoopError(e.to_string()))?;
    let old_hashes = snap.get("hashes").cloned().unwrap_or(Value::Null);

    // Authority as of `open`, not as it stands now.
    let cfg = snap.get("config").cloned().unwrap_or_else(
        || json!({"authority": l.config.get("authority"), "measure": l.config.get("measure")}),
    );
    let frozen = GlobSet::from_json(cfg.pointer("/authority/frozen"));
    let propose = GlobSet::from_json(cfg.pointer("/authority/propose"));
    let measure = GlobSet::from_json(cfg.pointer("/measure/globs"));
    let ledgers = GlobSet::from_json(cfg.pointer("/measure/json_ledgers"));
    let append_only = cfg
        .pointer("/measure/append_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut v = Verdict {
        violations: vec![],
        proposals: vec![],
        warnings: vec![],
        changed_free: vec![],
        appended_measures: vec![],
    };
    let mut seen: Vec<String> = Vec::new();

    for (rel, path) in walk_repo(&l.root) {
        seen.push(rel.clone());
        let digest = sha256_file(&path)?;
        let was = old_hashes.get(&rel).and_then(|h| h.as_str());
        let changed = was.map(|w| w != digest).unwrap_or(true);

        if let Some(pat) = frozen.match_of(&rel) {
            if changed {
                let kind = if was.is_none() {
                    "added to"
                } else {
                    "modified"
                };
                v.violations.push(format!(
                    "FROZEN {kind}: {rel} (matches `{pat}`) — this file is part of the exam."
                ));
            }
            continue;
        }

        if measure.is_match(&rel) {
            if !changed {
                continue;
            }
            if was.is_none() {
                v.appended_measures.push(format!("{rel} (new check file)"));
            } else if ledgers.is_match(&rel) {
                let old_text =
                    fs::read_to_string(round_dir.join("_snapshot").join(&rel)).unwrap_or_default();
                let new_text = fs::read_to_string(&path).unwrap_or_default();
                match (
                    serde_json::from_str::<Value>(&old_text),
                    serde_json::from_str::<Value>(&new_text),
                ) {
                    (Ok(o), Ok(n)) => {
                        let mut bad = Vec::new();
                        json_only_grew(&o, &n, "$", &mut bad);
                        if bad.is_empty() {
                            v.appended_measures.push(format!("{rel} (ledger grew)"));
                        } else {
                            let shown: Vec<String> = bad.iter().take(4).cloned().collect();
                            v.violations.push(format!(
                                "LEDGER weakened: {rel} — {}{}. Evidence ledgers may gain \
                                 entries, never lose or rewrite them.",
                                shown.join("; "),
                                if bad.len() > 4 { "; ..." } else { "" }
                            ));
                        }
                    }
                    _ => v.violations.push(format!("LEDGER unparseable: {rel}")),
                }
            } else if append_only {
                let old_text =
                    fs::read_to_string(round_dir.join("_snapshot").join(&rel)).unwrap_or_default();
                let new_text = fs::read_to_string(&path).unwrap_or_default();
                if is_pure_append(&old_text, &new_text) {
                    v.appended_measures.push(format!("{rel} (appended)"));
                } else {
                    v.violations.push(format!(
                        "MEASURE weakened: {rel} — existing lines were changed or removed. \
                         Checks are append-only; a round may add a rung, never lower one."
                    ));
                }
            } else {
                v.appended_measures.push(format!("{rel} (modified)"));
            }
            continue;
        }

        if let Some(pat) = propose.match_of(&rel) {
            if changed {
                v.proposals.push(format!("{rel} (matches `{pat}`)"));
            }
            continue;
        }

        if changed {
            v.changed_free.push(rel);
        }
    }

    if let Some(map) = old_hashes.as_object() {
        for rel in map.keys() {
            if seen.iter().any(|s| s == rel) {
                continue;
            }
            if frozen.is_match(rel) {
                v.violations
                    .push(format!("FROZEN deleted: {rel} — the exam cannot shrink."));
            } else if measure.is_match(rel) {
                v.violations
                    .push(format!("MEASURE deleted: {rel} — checks are append-only."));
            } else if propose.is_match(rel) {
                v.proposals.push(format!("{rel} (deleted)"));
            } else {
                v.changed_free.push(format!("{rel} (deleted)"));
            }
        }
    }

    // One gap does not mean one file, but it does mean one causal claim.
    let limit = l
        .config
        .pointer("/budget/attribution_warn_areas")
        .and_then(|x| x.as_u64())
        .unwrap_or(3) as usize;
    let mut areas: Vec<&str> = v
        .changed_free
        .iter()
        .map(|r| r.split('/').next().unwrap_or(r))
        .collect();
    areas.sort_unstable();
    areas.dedup();
    if areas.len() > limit {
        v.warnings.push(format!(
            "attribution: this round touched {} top-level areas ({}). If the round changes \
             several independent levers, no one can tell which lesson to preserve.",
            areas.len(),
            areas.join(", ")
        ));
    }
    if !v.changed_free.is_empty() && v.appended_measures.is_empty() {
        v.warnings.push(
            "no deterministic check was added. A closed gap without a new rung will be \
             re-earned and re-lost."
                .into(),
        );
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_is_allowed_and_edits_are_not() {
        assert!(is_pure_append("a\nb\n", "a\nb\nc\n"));
        assert!(is_pure_append("a\nb\n", "x\na\ny\nb\nz\n"));
        assert!(!is_pure_append("a\nb\n", "a\n"));
        assert!(!is_pure_append("assert x == 2\n", "assert True\n"));
    }

    fn grew(o: Value, n: Value) -> Vec<String> {
        let mut bad = Vec::new();
        json_only_grew(&o, &n, "$", &mut bad);
        bad
    }

    #[test]
    fn a_ledger_may_gain_a_witness_or_a_claim() {
        let base = json!({"c": {"claim": "x", "witnesses": ["go:A"]}});
        assert!(grew(
            base.clone(),
            json!({"c": {"claim": "x", "witnesses": ["go:A", "sdk:B"]}})
        )
        .is_empty());
        assert!(grew(
            base,
            json!({"c": {"claim": "x", "witnesses": ["go:A"]}, "d": {"claim": "y"}})
        )
        .is_empty());
    }

    #[test]
    fn a_ledger_may_not_lose_or_rewrite_evidence() {
        let base = json!({"c": {"claim": "x", "witnesses": ["go:A", "go:B"]}});
        assert!(!grew(
            base.clone(),
            json!({"c": {"claim": "x", "witnesses": ["go:A"]}})
        )
        .is_empty());
        assert!(!grew(
            base.clone(),
            json!({"c": {"claim": "x", "witnesses": ["go:A", "go:Easier"]}})
        )
        .is_empty());
        assert!(!grew(
            base.clone(),
            json!({"c": {"claim": "softer", "witnesses": ["go:A", "go:B"]}})
        )
        .is_empty());
        assert!(!grew(base, json!({})).is_empty());
    }
}
