//! Config, repository walking, hashing, state, and shelling out.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::glob::GlobSet;

pub const GOALS: &str = "goals";
pub const CONFIG: &str = "loop.json";
pub const STATE: &str = "STATE.json";

/// Where a gap's cause lives. Naming this is what stops a loop from changing
/// the nearest thing instead of the right thing.
pub const LAYERS: [&str; 7] = [
    "world", "domain", "contract", "runtime", "steering", "surface", "harness",
];

/// Layers that are not product rounds: the round stops and a person decides.
pub const ESCALATE_LAYERS: [&str; 2] = ["world", "harness"];

const IGNORE_DIRS: [&str; 11] = [
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".pytest_cache",
    ".mypy_cache",
    "target",
];

#[derive(Debug)]
pub struct LoopError(pub String);

impl std::fmt::Display for LoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LoopError {}

pub type Res<T> = Result<T, LoopError>;

pub fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(LoopError(msg.into()))
}

// ------------------------------------------------------------------ config

pub fn default_config() -> Value {
    json!({
        "version": 1,
        "commands": { "reset": "", "health": "", "floor": "", "drive": "", "score": "" },
        "authority": {
            "frozen": [
                "goals/corpus/**", "goals/rubric.md", "goals/score/**",
                "goals/loop.json", "fixtures/**"
            ],
            "propose": [],
            "free": ["**"]
        },
        "measure": { "globs": ["tests/**"], "append_only": true, "json_ledgers": [] },
        "budget": { "rounds_per_batch": 3, "attribution_warn_areas": 3 },
        // Per-command budgets. `default` covers anything not named. A command
        // that outruns its budget is killed and reported as a timeout rather
        // than waited on: an infinite loop in a driver produces no output, so
        // nothing else distinguishes it from slow work.
        "timeouts": { "default": 1800 }
    })
}

/// Deep-merge `over` onto `base`, objects recursively and everything else by
/// replacement — so a partial `loop.json` inherits the defaults it omits.
pub fn merge(base: &Value, over: &Value) -> Value {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            let mut out = b.clone();
            for (k, v) in o {
                let merged = match b.get(k) {
                    Some(bv) => merge(bv, v),
                    None => v.clone(),
                };
                out.insert(k.clone(), merged);
            }
            Value::Object(out)
        }
        _ => over.clone(),
    }
}

// ------------------------------------------------------------------ time

/// UTC in `YYYY-MM-DDTHH:MM:SSZ`. Hand-rolled to keep `chrono` out of the tree;
/// these timestamps are records, and UTC is the unambiguous thing to record.
pub fn now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

// ------------------------------------------------------------------ files

pub fn sha256_file(path: &Path) -> Res<String> {
    let mut f = fs::File::open(path).map_err(|e| LoopError(format!("{}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| LoopError(format!("{}: {e}", path.display())))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn is_bookkeeping(rel: &str) -> bool {
    // Rewritten every round by the tool itself; never a product change.
    matches!(
        rel,
        "goals/STATE.json" | "goals/STATE.md" | "goals/proposals.md"
    )
}

/// Paths git would keep: tracked, plus untracked files that are not ignored.
///
/// Returns None when this is not a git repository, or git is unavailable — the
/// caller then guards everything, which is the safe direction to fail.
pub fn git_visible(root: &Path) -> Option<std::collections::HashSet<String>> {
    let out = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
    )
}

/// Every file the guard may consider, as (repo-relative path, absolute path).
pub fn walk_repo(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if IGNORE_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(root, &path, out);
        } else {
            let rel = match path.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            // rounds/ is the loop's own evidence log; never guarded.
            if rel.starts_with(&format!("{GOALS}/rounds/")) || is_bookkeeping(&rel) {
                continue;
            }
            out.push((rel, path));
        }
    }
}

// ------------------------------------------------------------------ shell

pub struct Run {
    pub cmd: String,
    pub code: i32,
    pub out: String,
    pub err: String,
    pub seconds: f64,
    /// The command was killed for exceeding its budget. Worth its own flag: a
    /// hanging command is indistinguishable from a slow one until something
    /// says so, and a driver that loops forever produces no output to read.
    pub timed_out: bool,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    pub fn to_json(&self) -> Value {
        json!({
            "cmd": self.cmd,
            "exit": self.code,
            "seconds": (self.seconds * 100.0).round() / 100.0,
            "timed_out": self.timed_out,
            "stdout": tail(&self.out, 20_000),
            "stderr": tail(&self.err, 20_000),
        })
    }
}

fn tail(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        s[s.len() - n..].to_string()
    }
}

/// Conventional exit code for "killed after a timeout", as `timeout(1)` uses.
pub const EXIT_TIMEOUT: i32 = 124;

pub fn run(cmd: &str, cwd: &Path, env_extra: &[(&str, String)]) -> Run {
    run_within(cmd, cwd, env_extra, None)
}

/// Run a command, killing it if it outruns `limit`.
///
/// stdout and stderr are drained on their own threads. Without that, a command
/// that fills a pipe buffer blocks on write while this side waits for it to
/// exit — a deadlock that looks exactly like the hang the timeout exists to
/// catch, which would be an unfortunate way to implement it.
pub fn run_within(
    cmd: &str,
    cwd: &Path,
    env_extra: &[(&str, String)],
    limit: Option<Duration>,
) -> Run {
    let t0 = SystemTime::now();
    let mut c = Command::new("sh");
    c.arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env_extra {
        c.env(k, v);
    }

    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(e) => {
            return Run {
                cmd: cmd.to_string(),
                code: 127,
                out: String::new(),
                err: format!("{e}"),
                seconds: 0.0,
                timed_out: false,
            }
        }
    };

    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_h = thread::spawn(move || drain(&mut out_pipe));
    let err_h = thread::spawn(move || drain(&mut err_pipe));

    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {}
            Err(_) => break -1,
        }
        if let Some(limit) = limit {
            if t0.elapsed().map(|d| d > limit).unwrap_or(false) {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break EXIT_TIMEOUT;
            }
        }
        thread::sleep(Duration::from_millis(50));
    };

    let out = out_h.join().unwrap_or_default();
    let mut err = err_h.join().unwrap_or_default();
    if timed_out {
        let secs = limit.map(|l| l.as_secs()).unwrap_or(0);
        err.push_str(&format!(
            "\n[otogo] killed after {secs}s: the command exceeded its budget. \
             A command that hangs looks exactly like one that is slow.\n"
        ));
    }

    Run {
        cmd: cmd.to_string(),
        code,
        out,
        err,
        seconds: t0.elapsed().map(|d| d.as_secs_f64()).unwrap_or(0.0),
        timed_out,
    }
}

fn drain(pipe: &mut Option<impl Read>) -> String {
    let mut buf = String::new();
    if let Some(p) = pipe {
        let _ = p.read_to_string(&mut buf);
    }
    buf
}

// ------------------------------------------------------------------ loop

pub struct GoalLoop {
    pub root: PathBuf,
    pub config: Value,
    pub state: Value,
}

impl GoalLoop {
    pub fn find() -> Res<Self> {
        let cwd = std::env::current_dir().map_err(|e| LoopError(e.to_string()))?;
        let mut cur: Option<&Path> = Some(cwd.as_path());
        while let Some(dir) = cur {
            let cfg_path = dir.join(GOALS).join(CONFIG);
            if cfg_path.is_file() {
                let raw = fs::read_to_string(&cfg_path).map_err(|e| LoopError(e.to_string()))?;
                let parsed: Value = serde_json::from_str(&raw)
                    .map_err(|e| LoopError(format!("{}: {e}", cfg_path.display())))?;
                let config = merge(&default_config(), &parsed);
                let mut l = GoalLoop {
                    root: dir.to_path_buf(),
                    config,
                    state: Value::Null,
                };
                l.state = l.read_state();
                return Ok(l);
            }
            cur = dir.parent();
        }
        err(
            "no goals/loop.json found in this directory or any parent.\n\
             Run `otogo init` at the repository root first.",
        )
    }

    pub fn goals(&self) -> PathBuf {
        self.root.join(GOALS)
    }
    pub fn rounds(&self) -> PathBuf {
        self.goals().join("rounds")
    }
    pub fn corpus(&self) -> PathBuf {
        self.goals().join("corpus")
    }
    pub fn state_path(&self) -> PathBuf {
        self.goals().join(STATE)
    }
    pub fn round_dir(&self, n: u64) -> PathBuf {
        self.rounds().join(format!("{n:03}"))
    }

    fn read_state(&self) -> Value {
        fs::read_to_string(self.state_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| {
                json!({"batch": 1, "rounds_in_batch": 0, "open_round": null,
                       "queue": [], "history": []})
            })
    }

    pub fn save_state(&self) -> Res<()> {
        fs::create_dir_all(self.goals()).ok();
        fs::write(self.state_path(), format!("{}\n", pretty(&self.state)))
            .map_err(|e| LoopError(e.to_string()))?;
        self.render_state_md()
    }

    pub fn cmd(&self, name: &str) -> String {
        self.config
            .pointer(&format!("/commands/{name}"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    }

    pub fn frozen(&self) -> GlobSet {
        GlobSet::from_json(self.config.pointer("/authority/frozen"))
    }
    pub fn propose(&self) -> GlobSet {
        GlobSet::from_json(self.config.pointer("/authority/propose"))
    }
    pub fn measure(&self) -> GlobSet {
        GlobSet::from_json(self.config.pointer("/measure/globs"))
    }

    /// The budget for one command, in seconds. `timeouts.<name>` wins over
    /// `timeouts.default`; 0 means no limit.
    pub fn timeout_for(&self, name: &str) -> Option<Duration> {
        let secs = self
            .config
            .pointer(&format!("/timeouts/{name}"))
            .and_then(|v| v.as_u64())
            .or_else(|| {
                self.config
                    .pointer("/timeouts/default")
                    .and_then(|v| v.as_u64())
            })
            .unwrap_or(1800);
        if secs == 0 {
            None
        } else {
            Some(Duration::from_secs(secs))
        }
    }

    pub fn budget(&self) -> u64 {
        self.config
            .pointer("/budget/rounds_per_batch")
            .and_then(|v| v.as_u64())
            .unwrap_or(3)
    }

    pub fn open_round(&self) -> Res<&Map<String, Value>> {
        match self.state.get("open_round").and_then(|v| v.as_object()) {
            Some(o) => Ok(o),
            None => err("no open round. Run `otogo open <request-id>`."),
        }
    }

    pub fn open_round_mut(&mut self) -> Res<&mut Map<String, Value>> {
        if self
            .state
            .get("open_round")
            .and_then(|v| v.as_object())
            .is_none()
        {
            return err("no open round. Run `otogo open <request-id>`.");
        }
        Ok(self
            .state
            .get_mut("open_round")
            .unwrap()
            .as_object_mut()
            .unwrap())
    }

    /// The human-readable mirror of STATE.json. Sessions are disposable; this
    /// file is one of the things that makes them so without losing the thread.
    pub fn render_state_md(&self) -> Res<()> {
        let s = &self.state;
        let mut l: Vec<String> = vec![
            "# STATE".into(),
            String::new(),
            format!(
                "_Generated by otogo at {}. Machine copy: goals/STATE.json._",
                now()
            ),
            String::new(),
        ];
        match s.get("open_round").and_then(|v| v.as_object()) {
            Some(r) => {
                let n = r.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
                let gap = r.get("gap").and_then(|v| v.as_str()).unwrap_or("");
                l.push(format!(
                    "## Open round {n:03} — request `{}`",
                    r.get("request").and_then(|v| v.as_str()).unwrap_or("")
                ));
                l.push(format!(
                    "- phase: **{}**",
                    r.get("phase").and_then(|v| v.as_str()).unwrap_or("opened")
                ));
                l.push(format!(
                    "- world: {}",
                    r.get("world").and_then(|v| v.as_str()).unwrap_or("unknown")
                ));
                l.push(format!(
                    "- floor: {}",
                    r.get("floor").and_then(|v| v.as_str()).unwrap_or("not run")
                ));
                let layer = r.get("layer").and_then(|v| v.as_str()).unwrap_or("");
                l.push(format!(
                    "- gap: {}{}",
                    if gap.is_empty() {
                        "_unclassified_"
                    } else {
                        gap
                    },
                    if layer.is_empty() {
                        String::new()
                    } else {
                        format!(" (layer: `{layer}`)")
                    }
                ));
                l.push(format!("- evidence: `goals/rounds/{n:03}/`"));
                l.push(String::new());
            }
            None => {
                l.push("## No open round".into());
                l.push(String::new());
            }
        }
        let batch = s.get("batch").and_then(|v| v.as_u64()).unwrap_or(1);
        let used = s
            .get("rounds_in_batch")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        l.push(format!("## Batch {batch}"));
        l.push(format!("- rounds used: {used} / {}", self.budget()));
        l.push(String::new());

        l.push("## Queue".into());
        let q = s
            .get("queue")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if q.is_empty() {
            l.push("_empty_".into());
        } else {
            for (i, item) in q.iter().enumerate() {
                l.push(format!("{}. {}", i + 1, item.as_str().unwrap_or("")));
            }
        }
        l.push(String::new());

        l.push("## Recent rounds".into());
        l.push(String::new());
        let hist = s
            .get("history")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if hist.is_empty() {
            l.push("_none yet_".into());
        } else {
            l.push("| # | request | layer | gap | outcome |".into());
            l.push("|---|---------|-------|-----|---------|".into());
            for e in hist.iter().rev().take(12) {
                let gap = e.get("gap").and_then(|v| v.as_str()).unwrap_or("—");
                l.push(format!(
                    "| {:03} | `{}` | {} | {} | {} |",
                    e.get("n").and_then(|v| v.as_u64()).unwrap_or(0),
                    e.get("request").and_then(|v| v.as_str()).unwrap_or(""),
                    e.get("layer")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or("—"),
                    truncate(gap, 60),
                    e.get("outcome").and_then(|v| v.as_str()).unwrap_or(""),
                ));
            }
        }
        l.push(String::new());

        let fr = s
            .get("friction")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if !fr.is_empty() {
            l.push("## Open friction (candidates for harness change)".into());
            l.push(String::new());
            let start = fr.len().saturating_sub(10);
            for f in &fr[start..] {
                l.push(format!(
                    "- ({}x) {}",
                    f.get("count").and_then(|v| v.as_u64()).unwrap_or(1),
                    f.get("note").and_then(|v| v.as_str()).unwrap_or("")
                ));
            }
            l.push(String::new());
        }
        fs::write(self.goals().join("STATE.md"), l.join("\n")).map_err(|e| LoopError(e.to_string()))
    }
}

pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n).collect()
    }
}

pub fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".into())
}
