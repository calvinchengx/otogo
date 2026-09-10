//! What the guard counts as a change.

mod common;
use common::{assert_has, Repo};

#[test]
fn gitignored_build_output_is_not_a_product_change() {
    let r = Repo::new();
    r.git_init("artifacts/\n*.log\n");
    r.open_round();

    r.write("artifacts/binary", "compiled bytes");
    r.write("run.log", "noise from the last reset");
    r.write("src/app.py", "def escalate(i):\n    return 'done'\n");

    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 0, "{out}");
    assert_has(&out, "src/app.py");
    assert!(
        !out.contains("artifacts/binary"),
        "build output counted as a change:\n{out}"
    );
    assert!(
        !out.contains("run.log"),
        "a log counted as a change:\n{out}"
    );
    assert!(
        !out.contains("attribution:"),
        "attribution warned on noise:\n{out}"
    );
}

#[test]
fn a_gitignored_frozen_file_is_still_frozen() {
    // The exception that matters. Fixtures are routinely gitignored because
    // they are large; un-freezing the exam because of a line in .gitignore
    // would defeat the whole authority model.
    let r = Repo::new();
    r.git_init("fixtures/\n");
    r.open_round();

    r.write("fixtures/seed.sql", "-- a much more convenient fixture\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 1, "a gitignored frozen file was not guarded:\n{out}");
    assert_has(&out, "FROZEN modified");
}

#[test]
fn a_gitignored_rung_is_still_append_only() {
    let r = Repo::new();
    r.git_init("tests/\n");
    r.open_round();

    r.write(
        "tests/test_rung.py",
        "def test_existing():\n    assert True\n",
    );
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 1, "a gitignored measure file was not guarded:\n{out}");
    assert_has(&out, "MEASURE weakened");
}

#[test]
fn ignoring_a_file_mid_round_is_not_reported_as_a_deletion() {
    let r = Repo::new();
    r.git_init("# nothing ignored yet\n");
    r.open_round();

    // The file is still on disk; it just stopped being visible to git.
    r.write(".gitignore", "src/app.py\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        !out.contains("(deleted)"),
        "a still-present file reported deleted:\n{out}"
    );
}

#[test]
fn a_non_git_directory_guards_everything() {
    // Failing safe: with no git to ask, nothing is skipped.
    let r = Repo::new();
    r.open_round();
    r.write("artifacts/binary", "compiled bytes");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 0, "{out}");
    assert_has(&out, "artifacts/binary");
}

#[test]
fn a_tracked_build_directory_is_still_guarded() {
    // otogo's fallback walk skips conventional build directories by name. A
    // repo whose product genuinely lives in one would have those files
    // silently unguarded — frozen ones included. When git can answer, git
    // decides, and a tracked build/ is ordinary source.
    let r = Repo::new();
    r.write("build/app.py", "def escalate(i):\n    return True\n");
    r.git_init("# build/ is real source here, not output\n");
    r.open_round();

    r.write("build/app.py", "def escalate(i):\n    return 'changed'\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 0, "{out}");
    assert_has(&out, "build/app.py");
}

#[test]
fn a_frozen_file_in_a_skipped_directory_is_still_frozen() {
    let r = Repo::new();
    r.set_config(
        "/authority/frozen",
        serde_json::json!(["goals/corpus/**", "build/**"]),
    );
    r.write("build/exam.txt", "the question\n");
    r.git_init("# nothing ignored\n");
    r.open_round();

    r.write("build/exam.txt", "an easier question\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(
        code, 1,
        "a frozen file under build/ was not guarded:\n{out}"
    );
    assert_has(&out, "FROZEN modified");
}
