//! The skills teach the procedure; the CLI enforces it.
//!
//! They may differ on emphasis. They may not differ on what commands exist — a
//! skill that teaches a verb the CLI dropped sends an agent down a dead end.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn skills() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills");
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("skills/ missing")
        .flatten()
        .map(|e| e.path().join("SKILL.md"))
        .filter(|p| p.is_file())
        .collect();
    out.sort();
    out
}

/// Subcommands as the CLI itself reports them — parsed from `--help` rather
/// than read from the source, so this tests the real interface.
fn subcommands() -> BTreeSet<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_otogo"))
        .arg("--help")
        .output()
        .expect("failed to run otogo --help");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut in_commands = false;
    let mut set = BTreeSet::new();
    for line in text.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if in_commands {
            if line.starts_with("Options:") || line.trim().is_empty() && !set.is_empty() {
                if line.starts_with("Options:") {
                    break;
                }
                continue;
            }
            if let Some(word) = line.split_whitespace().next() {
                if word.chars().all(|c| c.is_ascii_lowercase() || c == '-') && !word.is_empty() {
                    set.insert(word.to_string());
                }
            }
        }
    }
    assert!(
        set.contains("open"),
        "could not parse subcommands from --help:\n{text}"
    );
    set
}

/// Every `otogo <verb>` written as code, ignoring prose mentions.
fn invocations(text: &str) -> BTreeSet<String> {
    let mut code = String::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            code.push_str(line);
            code.push('\n');
        }
    }
    // Inline spans too.
    let mut inline = String::new();
    let mut rest = text;
    while let Some(a) = rest.find('`') {
        rest = &rest[a + 1..];
        match rest.find('`') {
            Some(b) => {
                let span = &rest[..b];
                if !span.contains('\n') {
                    inline.push_str(span);
                    inline.push('\n');
                }
                rest = &rest[b + 1..];
            }
            None => break,
        }
    }
    let mut found = BTreeSet::new();
    for chunk in [code, inline] {
        for line in chunk.lines() {
            let mut words = line.split_whitespace().peekable();
            while let Some(w) = words.next() {
                if w == "otogo" {
                    if let Some(next) = words.peek() {
                        let v: String = next
                            .chars()
                            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
                            .collect();
                        if !v.is_empty() {
                            found.insert(v);
                        }
                    }
                }
            }
        }
    }
    found
}

#[test]
fn there_are_skills_to_check() {
    assert!(!skills().is_empty(), "no skills found");
}

#[test]
fn frontmatter_name_matches_its_directory() {
    for s in skills() {
        let text = std::fs::read_to_string(&s).unwrap();
        let name = text
            .lines()
            .find_map(|l| l.strip_prefix("name: "))
            .unwrap_or_else(|| panic!("{} has no name in frontmatter", s.display()));
        let dir = s.parent().unwrap().file_name().unwrap().to_string_lossy();
        assert_eq!(name.trim(), dir, "{}", s.display());
    }
}

#[test]
fn every_skill_has_a_description_with_triggers() {
    for s in skills() {
        let text = std::fs::read_to_string(&s).unwrap();
        let desc = text
            .lines()
            .find_map(|l| l.strip_prefix("description: "))
            .unwrap_or_else(|| panic!("{} has no description", s.display()));
        assert!(
            desc.len() > 60,
            "{}: description too thin to route on",
            s.display()
        );
    }
}

#[test]
fn every_command_a_skill_teaches_exists() {
    let known = subcommands();
    for s in skills() {
        let text = std::fs::read_to_string(&s).unwrap();
        let unknown: Vec<String> = invocations(&text).difference(&known).cloned().collect();
        assert!(
            unknown.is_empty(),
            "{} teaches commands the CLI does not have: {unknown:?}",
            s.parent().unwrap().file_name().unwrap().to_string_lossy()
        );
    }
}

#[test]
fn skills_do_not_hardcode_the_authority_list() {
    // Authority lives in loop.json and reaches the agent via `otogo brief`.
    // A second copy goes stale silently, and a stale authority list is worse
    // than none.
    for s in skills() {
        let text = std::fs::read_to_string(&s).unwrap();
        assert!(
            !text.contains("goals/corpus/**"),
            "{} hardcodes a frozen glob; point at `otogo brief` instead",
            s.parent().unwrap().file_name().unwrap().to_string_lossy()
        );
    }
}
