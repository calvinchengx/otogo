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

/// A fence marker: 3+ backticks or 3+ tildes at the start of a line.
fn fence_marker(trimmed: &str) -> Option<(char, usize)> {
    for c in ['`', '~'] {
        let n = trimmed.chars().take_while(|x| *x == c).count();
        if n >= 3 {
            return Some((c, n));
        }
    }
    None
}

/// Append the inline `code spans` on one line.
fn push_inline_spans(line: &str, out: &mut String) {
    let mut rest = line;
    while let (Some(a),) = (rest.find('`'),) {
        rest = &rest[a + 1..];
        match rest.find('`') {
            Some(b) => {
                out.push_str(&rest[..b]);
                out.push('\n');
                rest = &rest[b + 1..];
            }
            None => break,
        }
    }
}

/// Everything in a Markdown document that a reader would see as code: fenced
/// blocks (backtick or tilde), four-space indented blocks, and inline spans.
///
/// Over-reading is the safe direction here. This feeds a guard, and a guard
/// that misses a region fails OPEN — it reports success while a skill teaches
/// a command that does not exist.
fn code_regions(text: &str) -> String {
    let mut out = String::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        match fence {
            // Only a marker of the SAME character and at least as long closes
            // it, so ``` inside a ~~~ block stays content.
            Some((c, n)) => match fence_marker(trimmed) {
                Some((fc, fl)) if fc == c && fl >= n => fence = None,
                _ => {
                    out.push_str(line);
                    out.push('\n');
                }
            },
            None => {
                if let Some(m) = fence_marker(trimmed) {
                    fence = Some(m);
                } else if line.starts_with("    ") || line.starts_with('\t') {
                    out.push_str(line);
                    out.push('\n');
                } else {
                    push_inline_spans(line, &mut out);
                }
            }
        }
    }
    out
}

/// True when a fence is opened and never closed. Reported on its own rather
/// than folded into the command check: an unclosed fence turns the rest of the
/// document into code, and the resulting "unknown command" would name a word
/// from the prose instead of the actual defect.
fn has_unclosed_fence(text: &str) -> bool {
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        match fence {
            Some((c, n)) => match fence_marker(trimmed) {
                Some((fc, fl)) if fc == c && fl >= n => fence = None,
                _ => {}
            },
            None => fence = fence_marker(trimmed),
        }
    }
    fence.is_some()
}

/// Every `otogo <verb>` written as code, ignoring prose mentions.
fn invocations(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in code_regions(text).lines() {
        let mut words = line.split_whitespace().peekable();
        while let Some(w) = words.next() {
            if w != "otogo" {
                continue;
            }
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

#[test]
fn the_runner_feeds_the_skill_rather_than_a_copy_of_it() {
    // examples/round.sh used to carry its own AGENT.md saying the same things.
    // Two copies of a procedure drift, and the stale one is the one an agent
    // reads at 3am. The runner reads the skill at run time instead.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runner =
        std::fs::read_to_string(root.join("examples/round.sh")).expect("examples/round.sh missing");
    assert!(
        runner.contains("skills/otogo-round/SKILL.md"),
        "round.sh must hand the agent the skill, not an inline procedure"
    );
    assert!(
        !root.join("examples/AGENT.md").exists(),
        "examples/AGENT.md is a second copy of the round procedure; the skill is the one"
    );
}

#[test]
fn every_doc_chapter_is_in_the_sidebar() {
    // A chapter written but never listed is a chapter nobody reads. Starlight
    // fails the build on a sidebar slug with no file; this is the other
    // direction — a file with no sidebar entry.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = std::fs::read_to_string(root.join("website/astro.config.mjs"))
        .expect("website/astro.config.mjs missing");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(root.join("docs"))
        .expect("docs/ missing")
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(slug) = name.strip_suffix(".md") {
            if slug.chars().take(2).all(|c| c.is_ascii_digit())
                && !config.contains(&format!("'{slug}'"))
            {
                missing.push(slug.to_string());
            }
        }
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "docs not listed in the sidebar: {missing:?}"
    );
}

mod parser {
    //! Tests for `invocations` itself.
    //!
    //! It is the drift guard's parser: if it under-reads a skill, the guard
    //! passes a skill teaching a command that does not exist. A checker that
    //! fails OPEN is worse than no checker, because it reports success.
    use super::invocations;

    fn v(text: &str) -> Vec<String> {
        invocations(text).into_iter().collect()
    }

    #[test]
    fn a_fenced_block_yields_its_verbs() {
        let got = v("```bash\notogo open req\notogo close --outcome improved\n```\n");
        assert_eq!(got, vec!["close", "open"]);
    }

    #[test]
    fn an_inline_span_yields_its_verb() {
        assert_eq!(v("First run `otogo brief` every session.\n"), vec!["brief"]);
    }

    #[test]
    fn prose_without_backticks_is_not_a_command() {
        assert!(v("Then otogo verify decides whether it worked.\n").is_empty());
    }

    #[test]
    fn a_fence_and_an_inline_span_in_one_document_both_count() {
        let doc = "Run `otogo status` first.\n\n```sh\notogo drive\n```\n\nThen `otogo score`.\n";
        assert_eq!(v(doc), vec!["drive", "score", "status"]);
    }

    #[test]
    fn a_verb_is_read_up_to_its_flags_or_punctuation() {
        assert_eq!(v("`otogo close --outcome improved`\n"), vec!["close"]);
        assert_eq!(v("`otogo verify`.\n"), vec!["verify"]);
    }

    #[test]
    fn a_trailing_otogo_names_no_verb() {
        assert!(v("the tool is `otogo`\n").is_empty());
        assert!(v("```\notogo\n```\n").is_empty());
    }

    #[test]
    fn an_indented_code_block_counts() {
        // Four-space blocks are code in every Markdown renderer, and a skill
        // written that way must be checked like any other.
        assert_eq!(
            v("Example:\n\n    otogo classify --layer domain\n\n"),
            vec!["classify"]
        );
    }

    #[test]
    fn a_tilde_fence_counts() {
        assert_eq!(v("~~~bash\notogo abort \"reason\"\n~~~\n"), vec!["abort"]);
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end_of_the_document() {
        // CommonMark's rule, and the safe direction for a guard: over-reading
        // produces a loud false positive, under-reading produces silence.
        let doc = "```bash\notogo open req\n\nLater prose mentions otogo nonsense freely.\n";
        assert_eq!(v(doc), vec!["nonsense", "open"]);
    }

    #[test]
    fn an_unclosed_fence_is_reported_on_its_own() {
        // So the failure names the real defect rather than a word from the prose.
        assert!(super::has_unclosed_fence("```bash\notogo open req\n"));
        assert!(!super::has_unclosed_fence("```bash\notogo open req\n```\n"));
        assert!(!super::has_unclosed_fence("no fences here\n"));
    }

    #[test]
    fn a_backtick_fence_inside_a_tilde_block_is_content() {
        let doc = "~~~\n```\notogo drive\n```\n~~~\n";
        assert_eq!(v(doc), vec!["drive"]);
    }

    #[test]
    fn a_multiline_inline_span_is_not_code() {
        assert!(v("a `otogo\nbogus` b\n").is_empty());
    }
}

#[test]
fn no_skill_has_an_unclosed_code_fence() {
    for s in skills() {
        let text = std::fs::read_to_string(&s).unwrap();
        assert!(
            !has_unclosed_fence(&text),
            "{} has an unclosed code fence; everything after it reads as code",
            s.parent().unwrap().file_name().unwrap().to_string_lossy()
        );
    }
}
