//! A throwaway repository with a product, a rung, and an evidence ledger.
//!
//! Each integration-test binary compiles this module separately, so not every
//! helper is used by every one of them.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Repo {
    pub root: PathBuf,
}

static N: AtomicUsize = AtomicUsize::new(0);

impl Repo {
    pub fn new() -> Self {
        let id = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("otogo-test-{}-{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::create_dir_all(root.join("fixtures")).unwrap();
        let r = Repo { root };
        r.write("src/app.py", "def escalate(i):\n    return True\n");
        r.write(
            "tests/test_rung.py",
            "def test_existing():\n    assert 1 + 1 == 2\n",
        );
        r.write("fixtures/seed.sql", "-- seed\n");
        r.write_json("witnesses.json", &ledger_base());
        assert_eq!(r.run(&["init", "."]).0, 0);
        r.configure();
        r
    }

    fn configure(&self) {
        let cfg = serde_json::json!({
            "version": 1,
            "commands": {
                "reset": "true", "health": "true", "floor": "true",
                "drive": "sh -c 'mkdir -p {out}; echo drove > {out}/t.txt'",
                "score": "true"
            },
            "authority": {
                "frozen": ["goals/corpus/**", "goals/loop.json", "fixtures/**"],
                "propose": ["src/contract.py"],
                "free": ["**"]
            },
            "measure": {
                "globs": ["tests/**", "witnesses.json"],
                "append_only": true,
                "json_ledgers": ["witnesses.json"]
            },
            "budget": { "rounds_per_batch": 3, "attribution_warn_areas": 3 }
        });
        self.write_json("goals/loop.json", &cfg);
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn write(&self, rel: &str, text: &str) {
        let p = self.path(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, text).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.path(rel)).unwrap_or_default()
    }

    pub fn append(&self, rel: &str, text: &str) {
        let cur = self.read(rel);
        self.write(rel, &format!("{cur}{text}"));
    }

    pub fn remove(&self, rel: &str) {
        std::fs::remove_file(self.path(rel)).unwrap();
    }

    pub fn json(&self, rel: &str) -> serde_json::Value {
        serde_json::from_str(&self.read(rel)).unwrap()
    }

    pub fn write_json(&self, rel: &str, v: &serde_json::Value) {
        self.write(
            rel,
            &format!("{}\n", serde_json::to_string_pretty(v).unwrap()),
        );
    }

    /// Invoke the CLI the way a user does: (exit code, stdout + stderr).
    pub fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_otogo"))
            .args(args)
            .current_dir(&self.root)
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.root)
            .output()
            .expect("failed to run otogo");
        (
            out.status.code().unwrap_or(-1),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }

    pub fn open_round(&self) -> &Self {
        assert_eq!(self.run(&["open", "overdue-invoices"]).0, 0);
        self
    }

    pub fn state(&self) -> serde_json::Value {
        self.json("goals/STATE.json")
    }

    /// Make this a real git repo with a .gitignore, so the guard's
    /// ignore-awareness can be exercised.
    pub fn git_init(&self, ignore: &str) {
        self.write(".gitignore", ignore);
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["config", "user.email", "t@example.com"],
            vec!["config", "user.name", "t"],
            vec!["add", "-A"],
            vec!["-c", "commit.gpgsign=false", "commit", "-qm", "init"],
        ] {
            let ok = Command::new("git")
                .args(&args)
                .current_dir(&self.root)
                .output()
                .expect("git");
            assert!(
                ok.status.success(),
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&ok.stderr)
            );
        }
    }

    /// Set a value in loop.json by JSON pointer, e.g. "/timeouts/drive".
    pub fn set_config(&self, pointer: &str, value: serde_json::Value) {
        let mut cfg = self.json("goals/loop.json");
        let mut cur = &mut cfg;
        let parts: Vec<&str> = pointer.trim_start_matches('/').split('/').collect();
        for k in &parts[..parts.len() - 1] {
            if cur.get(*k).is_none() {
                cur[*k] = serde_json::json!({});
            }
            cur = cur.get_mut(*k).unwrap();
        }
        cur[parts[parts.len() - 1]] = value;
        self.write_json("goals/loop.json", &cfg);
    }

    pub fn set_command(&self, key: &str, value: &str) {
        let mut cfg = self.json("goals/loop.json");
        cfg["commands"][key] = serde_json::json!(value);
        self.write_json("goals/loop.json", &cfg);
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub fn ledger_base() -> serde_json::Value {
    serde_json::json!({
        "challenge-401": {
            "claim": "401 + WWW-Authenticate",
            "witnesses": ["go:TestChallengeShape"]
        }
    })
}

#[track_caller]
pub fn assert_has(out: &str, needle: &str) {
    assert!(
        out.contains(needle),
        "expected {needle:?} in output:\n{out}"
    );
}
