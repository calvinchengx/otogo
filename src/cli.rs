//! Commands, round lifecycle, and stop rules.
//!
//! Exit codes are part of the contract — `round.sh` and the skills branch on
//! them, so they are as much an interface as stdout is:
//!   0  proceed
//!   1  the round crossed a boundary, or the floor went red
//!   2  stop: a person is needed
//!   3  a driver could not run at all

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use crate::core::{
    default_config, err, now, pretty, run, truncate, GoalLoop, LoopError, Res, Run,
    ESCALATE_LAYERS, LAYERS,
};
use crate::{guard, templates};

// ------------------------------------------------------------------ output

struct Style {
    bold: &'static str,
    dim: &'static str,
    red: &'static str,
    grn: &'static str,
    yel: &'static str,
    cyn: &'static str,
    off: &'static str,
}

fn style() -> Style {
    if std::io::stdout().is_terminal() {
        Style {
            bold: "\x1b[1m",
            dim: "\x1b[2m",
            red: "\x1b[31m",
            grn: "\x1b[32m",
            yel: "\x1b[33m",
            cyn: "\x1b[36m",
            off: "\x1b[0m",
        }
    } else {
        Style {
            bold: "",
            dim: "",
            red: "",
            grn: "",
            yel: "",
            cyn: "",
            off: "",
        }
    }
}

fn head(msg: &str) {
    let s = style();
    println!("\n{}{}{}", s.bold, msg, s.off);
}
fn ok(msg: &str) {
    let s = style();
    println!("  {}ok{}   {}", s.grn, s.off, msg);
}
fn bad(msg: &str) {
    let s = style();
    println!("  {}fail{} {}", s.red, s.off, msg);
}
fn warn(msg: &str) {
    let s = style();
    println!("  {}warn{} {}", s.yel, s.off, msg);
}
fn note(msg: &str) {
    let s = style();
    println!("  {}{}{}", s.dim, msg, s.off);
}

// ------------------------------------------------------------------ args

#[derive(Parser)]
#[command(
    name = "otogo",
    version,
    about = "Goal loops where the agent owns the turns and a person owns the rounds",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold the goals/ control plane
    Init {
        path: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// Restore the fixture and prove the world is healthy
    Preflight,
    /// Run the deterministic floor
    Floor,
    /// Start a round on one approved request
    Open {
        request: String,
        /// Open past the batch budget
        #[arg(long)]
        force: bool,
    },
    /// Send the request through a fresh interaction
    Drive {
        #[arg(long)]
        verify: bool,
    },
    /// Read what happened
    Score {
        #[arg(long)]
        verify: bool,
    },
    /// Name the largest gap and its layer
    Classify {
        #[arg(long)]
        layer: String,
        /// One causal claim
        #[arg(long)]
        gap: String,
    },
    /// Check what the round changed against its authority
    Guard,
    /// Floor + authority + the same request again
    Verify,
    /// Close the round and record what remains
    Close {
        #[arg(long)]
        outcome: String,
        /// Answer the harness question inline
        #[arg(long)]
        friction: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// End the round without counting it as progress
    Abort { reason: String },
    /// Record what cost time this round
    Friction { note: String },
    /// The human boundary between batches
    Batch {
        #[arg(long = "continue")]
        continue_: bool,
        /// New queue, entries separated by ';'
        #[arg(long)]
        redirect: Option<String>,
        #[arg(long, num_args = 0..=1, default_missing_value = "operator")]
        stop: Option<String>,
    },
    /// Where the loop is and what comes next
    Status,
    /// The packet a fresh agent session should read
    Brief {
        #[arg(long)]
        out: Option<String>,
    },
}

pub fn main() -> i32 {
    let cli = Cli::parse();
    match dispatch(cli.cmd) {
        Ok(code) => code,
        Err(LoopError(msg)) => {
            let s = style();
            println!("\n{}error{} {}\n", s.red, s.off, msg);
            2
        }
    }
}

fn dispatch(cmd: Cmd) -> Res<i32> {
    match cmd {
        Cmd::Init { path, force } => cmd_init(path, force),
        Cmd::Preflight => cmd_preflight(),
        Cmd::Floor => cmd_floor(),
        Cmd::Open { request, force } => cmd_open(&request, force),
        Cmd::Drive { verify } => cmd_drive(verify),
        Cmd::Score { verify } => cmd_score(verify),
        Cmd::Classify { layer, gap } => cmd_classify(&layer, &gap),
        Cmd::Guard => cmd_guard(),
        Cmd::Verify => cmd_verify(),
        Cmd::Close {
            outcome,
            friction,
            force,
        } => cmd_close(&outcome, friction, force),
        Cmd::Abort { reason } => cmd_abort(&reason),
        Cmd::Friction { note } => cmd_friction(&note),
        Cmd::Batch {
            continue_,
            redirect,
            stop,
        } => cmd_batch(continue_, redirect, stop),
        Cmd::Status => cmd_status(),
        Cmd::Brief { out } => cmd_brief(out),
    }
}

// ------------------------------------------------------------------ helpers

fn record(round_dir: &Path, name: &str, r: &Run) {
    fs::write(
        round_dir.join(format!("{name}.json")),
        format!("{}\n", pretty(&r.to_json())),
    )
    .ok();
    fs::write(
        round_dir.join(format!("{name}.log")),
        format!(
            "$ {}\n\n[stdout]\n{}\n[stderr]\n{}\n[exit {}]\n",
            r.cmd, r.out, r.err, r.code
        ),
    )
    .ok();
}

fn rdir(l: &GoalLoop) -> Res<PathBuf> {
    let n = l
        .open_round()?
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    Ok(l.round_dir(n))
}

fn set_phase(l: &mut GoalLoop, phase: &str) -> Res<()> {
    l.open_round_mut()?.insert("phase".into(), json!(phase));
    l.save_state()
}

fn subst(tmpl: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = tmpl.to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

// ------------------------------------------------------------------ init

fn cmd_init(path: Option<String>, force: bool) -> Res<i32> {
    let root = fs::canonicalize(path.unwrap_or_else(|| ".".into()))
        .map_err(|e| LoopError(e.to_string()))?;
    let goals = root.join("goals");
    if goals.join("loop.json").exists() && !force {
        return err(format!(
            "{} already exists. Use --force to overwrite.",
            goals.join("loop.json").display()
        ));
    }
    for d in ["", "corpus", "rounds", "score"] {
        fs::create_dir_all(goals.join(d)).map_err(|e| LoopError(e.to_string()))?;
    }
    let mut written = Vec::new();
    let mut put = |rel: &str, text: &str| {
        let p = goals.join(rel);
        if p.exists() && !force {
            return;
        }
        if fs::write(&p, text).is_ok() {
            written.push(format!("goals/{rel}"));
        }
    };
    put("goal.md", templates::GOAL_MD);
    put("facts.md", templates::FACTS_MD);
    put("plan.md", templates::PLAN_MD);
    put("LOOP.md", templates::LOOP_MD);
    put("rubric.md", templates::RUBRIC_MD);
    put("proposals.md", templates::PROPOSALS_MD);
    put("corpus/README.md", templates::README_CORPUS);
    put("corpus/overdue-invoices.txt", templates::EXAMPLE_REQUEST);
    put("loop.json", &format!("{}\n", pretty(&default_config())));

    let l = GoalLoop {
        root,
        config: default_config(),
        state: json!({"batch": 1, "rounds_in_batch": 0, "open_round": null,
                      "queue": [], "history": []}),
    };
    l.save_state()?;
    written.push("goals/STATE.json".into());
    written.push("goals/STATE.md".into());

    head("Initialized the control plane");
    for w in &written {
        ok(w);
    }
    head("Next");
    note("1. Replace the example corpus request with three requests a person approved.");
    note("2. Fill in goals/loop.json commands: reset, health, floor, drive, score.");
    note("3. Write goal.md's completion condition, then `otogo open <request-id>`.");
    println!();
    Ok(0)
}

// ------------------------------------------------------------------ world

fn preflight(l: &GoalLoop, round_dir: &Path) -> String {
    let (reset, health) = (l.cmd("reset"), l.cmd("health"));
    if health.is_empty() {
        warn(
            "no health command configured — the loop cannot tell infrastructure trouble from \
             a feature failure. Every score below is suspect.",
        );
        return "unchecked".into();
    }
    if !reset.is_empty() {
        let r = run(&reset, &l.root, &[]);
        record(round_dir, "reset", &r);
        if !r.ok() {
            bad(&format!(
                "reset failed (exit {}) — see {}/reset.log",
                r.code,
                round_dir.file_name().unwrap_or_default().to_string_lossy()
            ));
            return "unhealthy".into();
        }
        ok("fixture restored");
    }
    let r = run(&health, &l.root, &[]);
    record(round_dir, "health", &r);
    if !r.ok() {
        bad(&format!(
            "world unhealthy (exit {}) — see {}/health.log",
            r.code,
            round_dir.file_name().unwrap_or_default().to_string_lossy()
        ));
        return "unhealthy".into();
    }
    ok("world healthy");
    "healthy".into()
}

fn cmd_preflight() -> Res<i32> {
    let l = GoalLoop::find()?;
    let out = l.rounds().join("_preflight");
    fs::create_dir_all(&out).ok();
    head("Preflight");
    let s = style();
    if preflight(&l, &out) == "unhealthy" {
        println!(
            "\n{}NO_SCORE{} — infrastructure trouble is not evidence that the feature failed.\n",
            s.red, s.off
        );
        return Ok(2);
    }
    Ok(0)
}

fn floor(l: &GoalLoop, round_dir: &Path) -> String {
    let floor = l.cmd("floor");
    if floor.is_empty() {
        warn("no floor command configured — nothing preserves prior progress.");
        return "unconfigured".into();
    }
    let r = run(&floor, &l.root, &[]);
    record(round_dir, "floor", &r);
    if r.ok() {
        ok("floor green");
        "green".into()
    } else {
        bad(&format!(
            "floor RED (exit {}) — see {}/floor.log",
            r.code,
            round_dir.file_name().unwrap_or_default().to_string_lossy()
        ));
        "red".into()
    }
}

fn cmd_floor() -> Res<i32> {
    let l = GoalLoop::find()?;
    let out = l.rounds().join("_floor");
    fs::create_dir_all(&out).ok();
    head("Deterministic floor");
    Ok(
        if matches!(floor(&l, &out).as_str(), "green" | "unconfigured") {
            0
        } else {
            1
        },
    )
}

// ------------------------------------------------------------------ round

fn find_request(l: &GoalLoop, id: &str) -> Res<PathBuf> {
    let mut hits: Vec<PathBuf> = fs::read_dir(l.corpus())
        .map_err(|e| LoopError(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name().map(|n| n != "README.md").unwrap_or(false)
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with(id))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    if let Some(h) = hits.into_iter().next() {
        return Ok(h);
    }
    let mut avail: Vec<String> = fs::read_dir(l.corpus())
        .map_err(|e| LoopError(e.to_string()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.file_name().map(|n| n != "README.md").unwrap_or(false))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    avail.sort();
    err(format!(
        "no corpus request matching '{id}'. Available: {}",
        if avail.is_empty() {
            "(none)".into()
        } else {
            avail.join(", ")
        }
    ))
}

fn cmd_open(request: &str, force: bool) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    if let Some(r) = l.state.get("open_round").and_then(|v| v.as_object()) {
        let n = r.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
        return err(format!(
            "round {n:03} is still open. Close it with `otogo close --outcome ...` or `otogo abort`."
        ));
    }
    let budget = l.budget();
    let used = l
        .state
        .get("rounds_in_batch")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if used >= budget && !force {
        let batch = l.state.get("batch").and_then(|v| v.as_u64()).unwrap_or(1);
        return err(format!(
            "batch {batch} is spent ({used}/{budget} rounds).\n\
             A person decides at the boundary: continue the queue, redirect the next\n\
             batch, or stop. Run `otogo batch --continue|--redirect \"...\"|--stop`."
        ));
    }

    let req = find_request(&l, request)?;
    let stem = req
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut n = 0u64;
    if let Ok(entries) = fs::read_dir(l.rounds()) {
        for e in entries.flatten() {
            if let Ok(v) = e.file_name().to_string_lossy().parse::<u64>() {
                n = n.max(v);
            }
        }
    }
    n += 1;
    let rd = l.round_dir(n);
    fs::create_dir_all(&rd).map_err(|e| LoopError(e.to_string()))?;
    let ext = req
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or(".txt".into());
    fs::copy(&req, rd.join(format!("request{ext}"))).ok();

    head(&format!("Round {n:03} — request `{stem}`"));
    let world = preflight(&l, &rd);
    let fl = if world == "unhealthy" {
        "skipped".to_string()
    } else {
        floor(&l, &rd)
    };
    let count = guard::take_snapshot(&l, &rd)?;
    ok(&format!("stone snapshotted ({count} guarded files)"));

    let rel = req
        .strip_prefix(&l.root)
        .unwrap_or(&req)
        .to_string_lossy()
        .to_string();
    l.state["open_round"] = json!({
        "n": n, "request": stem, "request_file": rel, "opened": now(),
        "phase": "opened", "world": world, "floor": fl, "gap": null, "layer": null
    });
    l.save_state()?;

    let s = style();
    if world == "unhealthy" {
        // An unhealthy world is not a round. Keep the evidence, drop the round.
        l.state["open_round"] = Value::Null;
        let hist = l.state["history"].as_array_mut().unwrap();
        hist.push(json!({"n": n, "request": stem, "layer": "world",
                         "gap": "world unhealthy", "outcome": "no_score", "closed": now()}));
        l.save_state()?;
        println!(
            "\n{}NO_SCORE{} — the world is unhealthy. Fix the world, then reopen.",
            s.red, s.off
        );
        println!(
            "{}Missing data looks like weak reasoning; stale code looks like a broken contract.{}\n",
            s.dim, s.off
        );
        return Ok(2);
    }
    if fl == "red" {
        println!(
            "\n{}The floor is red.{} That regression is this round's job — the driver comes after.\n",
            s.yel, s.off
        );
        return Ok(0);
    }
    note("next: otogo drive");
    println!();
    Ok(0)
}

fn cmd_drive(verify: bool) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    let (n, req, req_file) = {
        let r = l.open_round()?;
        (
            r.get("n").and_then(|v| v.as_u64()).unwrap_or(0),
            r.get("request")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            r.get("request_file")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        )
    };
    let rd = l.round_dir(n);
    let drive = l.cmd("drive");
    if drive.is_empty() {
        return err(
            "no drive command in goals/loop.json. The driver must approach the feature \
             through the same surface a user would.",
        );
    }
    head(&format!("Drive round {n:03} — `{req}`"));
    note("fresh interaction; the request goes through unchanged");
    let label = if verify { "verify" } else { "drive" };
    let out_dir = rd.join(label);
    fs::create_dir_all(&out_dir).ok();

    let abs_req = l.root.join(&req_file).to_string_lossy().to_string();
    let (o, r_) = (
        out_dir.to_string_lossy().to_string(),
        rd.to_string_lossy().to_string(),
    );
    let cmd = subst(
        &drive,
        &[
            ("request", &abs_req),
            ("request_file", &abs_req),
            ("out", &o),
            ("out_dir", &o),
            ("round_dir", &r_),
        ],
    );
    let res = run(
        &cmd,
        &l.root,
        &[
            ("OTOGO_REQUEST", abs_req),
            ("OTOGO_OUT", o),
            ("OTOGO_ROUND", r_),
            ("OTOGO_PHASE", label.to_string()),
        ],
    );
    record(&rd, label, &res);
    let msg = format!(
        "driver exit {} — evidence in {}/{label}/",
        res.code,
        rd.file_name().unwrap_or_default().to_string_lossy()
    );
    if res.ok() {
        ok(&msg)
    } else {
        bad(&msg)
    }
    note("record: answer, tool calls, rejected actions, visible result, persistent effects");
    set_phase(&mut l, label)?;
    println!();
    Ok(if res.ok() { 0 } else { 1 })
}

fn cmd_score(verify: bool) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    let n = l
        .open_round()?
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let rd = l.round_dir(n);
    let score = l.cmd("score");
    if score.is_empty() {
        return err("no score command in goals/loop.json.");
    }
    let label = if verify { "score-verify" } else { "score" };
    let phase = if verify { "verify" } else { "drive" };
    head(&format!("Score round {n:03} ({label})"));
    let r_ = rd.to_string_lossy().to_string();
    let cmd = subst(
        &score,
        &[("round_dir", &r_), ("out", &r_), ("phase", phase)],
    );
    let res = run(
        &cmd,
        &l.root,
        &[("OTOGO_ROUND", r_), ("OTOGO_PHASE", phase.to_string())],
    );
    record(&rd, label, &res);
    let msg = format!("scorer exit {}", res.code);
    if res.ok() {
        ok(&msg)
    } else {
        bad(&msg)
    }
    for line in res
        .out
        .trim_end()
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        note(line);
    }
    l.open_round_mut()?
        .insert(format!("{label}_exit"), json!(res.code));
    l.save_state()?;
    println!();
    Ok(0)
}

fn cmd_classify(layer: &str, gap: &str) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    if !LAYERS.contains(&layer) {
        return err(format!(
            "unknown layer '{layer}'. One of: {}",
            LAYERS.join(", ")
        ));
    }
    let n = {
        let r = l.open_round_mut()?;
        r.insert("layer".into(), json!(layer));
        r.insert("gap".into(), json!(gap));
        r.get("n").and_then(|v| v.as_u64()).unwrap_or(0)
    };
    set_phase(&mut l, "classified")?;
    head(&format!("Round {n:03} gap classified"));
    ok(&format!("layer `{layer}` — {gap}"));
    fs::write(
        l.round_dir(n).join("gap.md"),
        format!(
            "# Gap\n\n- layer: `{layer}`\n- claim: {gap}\n- classified: {}\n\n\
             ## Path this claim must close\n\n\
             request -> representation -> operation -> persistent effect -> visible proof\n\n\
             - [ ] request\n- [ ] representation\n- [ ] operation\n- [ ] persistent effect\n\
             - [ ] visible proof\n- [ ] deterministic check added\n",
            now()
        ),
    )
    .ok();
    if ESCALATE_LAYERS.contains(&layer) {
        let s = style();
        println!(
            "\n{}Stop.{} A `{layer}` gap is not a product round.",
            s.yel, s.off
        );
        if layer == "world" {
            note("Fix the fixture/services/served code, then reopen the round. NO_SCORE.");
        } else {
            note("The harness itself is the failure. Record it with `otogo friction`;");
            note("change the driver, facts, or procedure only on repeated evidence.");
        }
        println!();
        return Ok(2);
    }
    note("Close that whole path, then add a rung for the behavior, then `otogo verify`.");
    println!();
    Ok(0)
}

fn append_proposals(l: &GoalLoop, items: &[String]) {
    let p = l.goals().join("proposals.md");
    let mut text = fs::read_to_string(&p).unwrap_or_else(|_| templates::PROPOSALS_MD.to_string());
    let (n, gap, layer) = match l.state.get("open_round").and_then(|v| v.as_object()) {
        Some(r) => (
            r.get("n").and_then(|v| v.as_u64()).unwrap_or(0),
            r.get("gap")
                .and_then(|v| v.as_str())
                .unwrap_or("unclassified")
                .to_string(),
            r.get("layer")
                .and_then(|v| v.as_str())
                .unwrap_or("none")
                .to_string(),
        ),
        None => (0, "unclassified".into(), "none".into()),
    };
    text.push_str(&format!(
        "\n## Round {n:03} — {}\nGap: {gap} (layer `{layer}`)\n\n",
        now()
    ));
    for i in items {
        text.push_str(&format!("- [ ] {i}\n"));
    }
    fs::write(&p, text).ok();
    note("recorded in goals/proposals.md");
}

fn cmd_guard() -> Res<i32> {
    let l = GoalLoop::find()?;
    let rd = rdir(&l)?;
    let n = l
        .open_round()?
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    head(&format!("Authority check — round {n:03}"));
    let v = guard::check(&l, &rd)?;
    fs::write(rd.join("guard.json"), format!("{}\n", pretty(&v.to_json()))).ok();

    for m in &v.violations {
        bad(m);
    }
    for m in &v.warnings {
        warn(m);
    }
    if !v.changed_free.is_empty() {
        ok(&format!("{} free file(s) changed", v.changed_free.len()));
        for m in v.changed_free.iter().take(12) {
            note(m);
        }
        if v.changed_free.len() > 12 {
            note(&format!("... and {} more", v.changed_free.len() - 12));
        }
    }
    for m in &v.appended_measures {
        ok(&format!("rung added: {m}"));
    }
    if !v.proposals.is_empty() {
        warn(&format!(
            "{} propose-level file(s) changed — a person decides:",
            v.proposals.len()
        ));
        for m in &v.proposals {
            note(m);
        }
        append_proposals(&l, &v.proposals);
    }
    if v.ok() && v.proposals.is_empty() {
        ok("no boundary crossed");
    }
    println!();
    Ok(if v.ok() { 0 } else { 1 })
}

fn cmd_verify() -> Res<i32> {
    let mut l = GoalLoop::find()?;
    let n = l
        .open_round()?
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let rd = l.round_dir(n);
    head(&format!("Verify round {n:03}"));
    let s = style();
    if preflight(&l, &rd) == "unhealthy" {
        println!(
            "\n{}NO_SCORE{} — cannot attribute this run.\n",
            s.red, s.off
        );
        return Ok(2);
    }
    let fl = floor(&l, &rd);
    l.open_round_mut()?.insert("floor_after".into(), json!(fl));
    l.save_state()?;

    let rc_guard = cmd_guard()?;
    cmd_drive(true)?;
    cmd_score(true)?;
    let mut l = GoalLoop::find()?;
    set_phase(&mut l, "verified")?;
    if fl == "red" {
        println!(
            "{}The floor went red.{} The repair erased earned behavior — that regression is \
             the next job.\n",
            s.red, s.off
        );
        return Ok(1);
    }
    Ok(rc_guard)
}

fn cmd_close(outcome: &str, friction: Option<String>, force: bool) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    let n = l
        .open_round()?
        .get("n")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let rd = l.round_dir(n);
    head(&format!("Close round {n:03}"));

    let v = guard::check(&l, &rd)?;
    if !v.ok() && !force {
        for m in &v.violations {
            bad(m);
        }
        return err(
            "authority violations block the close. Revert the frozen changes, or record them \
             as a proposal for a person to decide.",
        );
    }
    let floor_after = l
        .open_round()?
        .get("floor_after")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if floor_after == "red" && !force {
        return err("the floor is red. A round does not close over a regression.");
    }

    let r = l.open_round()?.clone();
    let entry = json!({
        "n": n,
        "request": r.get("request").cloned().unwrap_or(Value::Null),
        "layer": r.get("layer").cloned().unwrap_or(Value::Null),
        "gap": r.get("gap").cloned().unwrap_or(Value::Null),
        "outcome": outcome,
        "closed": now(),
        "world": r.get("world").cloned().unwrap_or(Value::Null),
        "floor_before": r.get("floor").cloned().unwrap_or(Value::Null),
        "floor_after": r.get("floor_after").cloned().unwrap_or(Value::Null),
        "rungs_added": v.appended_measures,
        "files_changed": v.changed_free.len(),
    });
    let mut merged = Value::Object(r);
    if let (Value::Object(m), Value::Object(e)) = (&mut merged, &entry) {
        for (k, val) in e {
            m.insert(k.clone(), val.clone());
        }
    }
    fs::write(rd.join("round.json"), format!("{}\n", pretty(&merged))).ok();

    l.state["history"].as_array_mut().unwrap().push(entry);
    let used = l
        .state
        .get("rounds_in_batch")
        .and_then(|x| x.as_u64())
        .unwrap_or(0)
        + 1;
    l.state["rounds_in_batch"] = json!(used);
    l.state["open_round"] = Value::Null;
    l.save_state()?;

    ok(&format!("outcome: {outcome}"));
    if !v.appended_measures.is_empty() {
        ok(&format!(
            "left behind: {} rung(s)",
            v.appended_measures.len()
        ));
    } else {
        warn("nothing new is preserved — the next session may pay for this lesson again.");
    }

    match friction {
        Some(note_text) => record_friction(&mut l, &note_text)?,
        None => {
            let s = style();
            head("The harness question");
            println!(
                "  {}What cost time that a rule or check could prevent?{}",
                s.cyn, s.off
            );
            note("Answer with: otogo friction \"...\"  (first occurrence is evidence;");
            note("repeated friction earns a change to the driver, facts, or procedure)");
        }
    }

    let batch = l.state.get("batch").and_then(|x| x.as_u64()).unwrap_or(1);
    let budget = l.budget();
    head(&format!("Batch {batch}: {used}/{budget} rounds"));
    if used >= budget {
        let s = style();
        println!("  {}Batch boundary — a person decides.{}", s.cyn, s.off);
        note("otogo batch --continue | --redirect \"...\" | --stop");
    }
    println!();
    Ok(0)
}

fn cmd_abort(reason: &str) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    let r = l.open_round()?.clone();
    let n = r.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
    l.state["open_round"] = Value::Null;
    l.state["history"].as_array_mut().unwrap().push(json!({
        "n": n,
        "request": r.get("request").cloned().unwrap_or(Value::Null),
        "layer": r.get("layer").cloned().unwrap_or(Value::Null),
        "gap": r.get("gap").cloned().unwrap_or(Value::Null),
        "outcome": format!("aborted: {reason}"),
        "closed": now(),
    }));
    l.save_state()?;
    head(&format!("Round {n:03} aborted"));
    ok(reason);
    note("evidence kept; the round does not count as progress");
    println!();
    Ok(0)
}

// ------------------------------------------------------------------ meta

fn record_friction(l: &mut GoalLoop, note_text: &str) -> Res<()> {
    if l.state.get("friction").is_none() {
        l.state["friction"] = json!([]);
    }
    let arr = l.state["friction"].as_array_mut().unwrap();
    for f in arr.iter_mut() {
        let same = f
            .get("note")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().eq_ignore_ascii_case(note_text.trim()))
            .unwrap_or(false);
        if same {
            let c = f.get("count").and_then(|v| v.as_u64()).unwrap_or(1) + 1;
            f["count"] = json!(c);
            f["last"] = json!(now());
            l.save_state()?;
            head("Friction recorded again");
            warn(&format!("({c}x) {note_text}"));
            note(
                "Repeated friction earns a durable constraint: change the driver, facts, or \
                 LOOP.md — and consider whether the right change is subtraction.",
            );
            return Ok(());
        }
    }
    arr.push(json!({"note": note_text, "count": 1, "first": now(), "last": now()}));
    l.save_state()?;
    head("Friction recorded");
    ok(&format!("(1x) {note_text}"));
    note(
        "First occurrence is evidence, not a rule. The loop does not rewrite its operating \
         system after every surprise.",
    );
    Ok(())
}

fn cmd_friction(note_text: &str) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    record_friction(&mut l, note_text)?;
    println!();
    Ok(0)
}

fn cmd_batch(continue_: bool, redirect: Option<String>, stop: Option<String>) -> Res<i32> {
    let mut l = GoalLoop::find()?;
    if let Some(reason) = stop {
        l.state["stopped"] = json!({"at": now(), "reason": reason});
        l.save_state()?;
        head("Loop stopped by a person");
        note("The score cannot decide the product's direction.");
        println!();
        return Ok(0);
    }
    if let Some(q) = redirect {
        let items: Vec<String> = q
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let batch = l.state.get("batch").and_then(|v| v.as_u64()).unwrap_or(1) + 1;
        l.state["queue"] = json!(items);
        l.state["batch"] = json!(batch);
        l.state["rounds_in_batch"] = json!(0);
        l.save_state()?;
        head(&format!("Batch {batch} — redirected"));
        for i in l.state["queue"].as_array().unwrap() {
            ok(i.as_str().unwrap_or(""));
        }
        println!();
        return Ok(0);
    }
    if continue_ {
        let batch = l.state.get("batch").and_then(|v| v.as_u64()).unwrap_or(1) + 1;
        l.state["batch"] = json!(batch);
        l.state["rounds_in_batch"] = json!(0);
        l.save_state()?;
        head(&format!("Batch {batch} — continuing the queue"));
        println!();
        return Ok(0);
    }
    let batch = l.state.get("batch").and_then(|v| v.as_u64()).unwrap_or(1);
    head(&format!("Batch {batch}"));
    note(&format!(
        "{}/{} rounds used",
        l.state
            .get("rounds_in_batch")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        l.budget()
    ));
    println!();
    Ok(0)
}

fn cmd_status() -> Res<i32> {
    let l = GoalLoop::find()?;
    head("otogo");
    note(&l.root.to_string_lossy());
    match l.state.get("open_round").and_then(|v| v.as_object()) {
        Some(r) => {
            let n = r.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
            let phase = r.get("phase").and_then(|v| v.as_str()).unwrap_or("");
            head(&format!(
                "Round {n:03} — `{}` ({phase})",
                r.get("request").and_then(|v| v.as_str()).unwrap_or("")
            ));
            ok(&format!(
                "world {} · floor {}",
                r.get("world").and_then(|v| v.as_str()).unwrap_or("?"),
                r.get("floor").and_then(|v| v.as_str()).unwrap_or("?")
            ));
            match r.get("gap").and_then(|v| v.as_str()) {
                Some(g) if !g.is_empty() => ok(&format!(
                    "gap [{}] {g}",
                    r.get("layer").and_then(|v| v.as_str()).unwrap_or("")
                )),
                _ => warn(
                    "gap unclassified — without it, every agent failure becomes a prompt problem",
                ),
            }
            let next = match phase {
                "opened" => "otogo drive",
                "drive" => "otogo score",
                "score" => "otogo classify --layer <layer> --gap \"...\"",
                "classified" => "close the path, add a rung, then otogo verify",
                "verify" | "verified" => "otogo close --outcome <improved|unchanged|regressed>",
                _ => "otogo close",
            };
            note(&format!("next: {next}"));
        }
        None => {
            head("No open round");
            note("next: otogo open <request-id>");
        }
    }
    head(&format!(
        "Batch {}: {}/{} rounds",
        l.state.get("batch").and_then(|v| v.as_u64()).unwrap_or(1),
        l.state
            .get("rounds_in_batch")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        l.budget()
    ));
    let hist = l
        .state
        .get("history")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !hist.is_empty() {
        head("Recent");
        for e in hist.iter().rev().take(5) {
            note(&format!(
                "{:03} [{}] {} — {}",
                e.get("n").and_then(|v| v.as_u64()).unwrap_or(0),
                e.get("layer")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("—"),
                e.get("outcome").and_then(|v| v.as_str()).unwrap_or(""),
                truncate(e.get("gap").and_then(|v| v.as_str()).unwrap_or(""), 60)
            ));
        }
    }
    let fr = l
        .state
        .get("friction")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let repeated: Vec<&Value> = fr
        .iter()
        .filter(|f| f.get("count").and_then(|c| c.as_u64()).unwrap_or(1) > 1)
        .collect();
    if !repeated.is_empty() {
        head("Repeated friction (earns a harness change)");
        for f in repeated {
            warn(&format!(
                "({}x) {}",
                f.get("count").and_then(|c| c.as_u64()).unwrap_or(1),
                f.get("note").and_then(|c| c.as_str()).unwrap_or("")
            ));
        }
    }
    println!();
    Ok(0)
}

fn cmd_brief(out: Option<String>) -> Res<i32> {
    let l = GoalLoop::find()?;
    let mut parts = Vec::new();
    for rel in ["goal.md", "LOOP.md", "facts.md", "STATE.md"] {
        if let Ok(t) = fs::read_to_string(l.goals().join(rel)) {
            parts.push(format!("===== goals/{rel} =====\n{}\n", t.trim_end()));
        }
    }
    if let Some(r) = l.state.get("open_round").and_then(|v| v.as_object()) {
        let n = r.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
        let rd = l.round_dir(n);
        if let Ok(entries) = fs::read_dir(&rd) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("request.") {
                    if let Ok(t) = fs::read_to_string(e.path()) {
                        parts.push(format!(
                            "===== current request ({}) =====\n{}\n",
                            r.get("request").and_then(|v| v.as_str()).unwrap_or(""),
                            t.trim_end()
                        ));
                    }
                }
            }
        }
        if let Ok(t) = fs::read_to_string(rd.join("gap.md")) {
            parts.push(format!("===== current gap =====\n{}\n", t.trim_end()));
        }
    }
    let dash = |v: Vec<&str>| {
        if v.is_empty() {
            "—".to_string()
        } else {
            v.join(", ")
        }
    };
    parts.push(format!(
        "===== authority =====\nfrozen  (the stone; never change): {}\n\
         propose (a person decides):       {}\nmeasure (append-only rungs):      {}\n",
        dash(l.frozen().sources()),
        dash(l.propose().sources()),
        dash(l.measure().sources())
    ));
    let text = parts.join("\n");
    match out {
        Some(p) => {
            fs::write(&p, &text).map_err(|e| LoopError(e.to_string()))?;
            ok(&format!("brief written to {p}"));
        }
        None => print!("{text}"),
    }
    Ok(0)
}
