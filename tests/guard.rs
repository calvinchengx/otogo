//! The loop may change the product. It may not change the evidence.

mod common;
use common::{assert_has, ledger_base, Repo};

#[test]
fn free_files_change_without_objection() {
    let r = Repo::new();
    r.open_round();
    r.write("src/app.py", "def escalate(i):\n    return 'done'\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 0, "{out}");
    assert_has(&out, "src/app.py");
    assert_has(&out, "no boundary crossed");
}

#[test]
fn frozen_file_cannot_be_modified() {
    let r = Repo::new();
    r.open_round();
    r.write("goals/corpus/overdue-invoices.txt", "easier\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 1);
    assert_has(&out, "FROZEN modified");
}

#[test]
fn frozen_file_cannot_be_added() {
    let r = Repo::new();
    r.open_round();
    r.write("goals/corpus/convenient.txt", "a request I wrote myself\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 1);
    assert_has(&out, "FROZEN added to");
}

#[test]
fn frozen_file_cannot_be_deleted() {
    let r = Repo::new();
    r.open_round();
    r.remove("fixtures/seed.sql");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 1);
    assert_has(&out, "FROZEN deleted");
}

#[test]
fn the_loop_cannot_disarm_its_own_authority() {
    // The authority is captured at `open`. A round judged by rules it edited
    // is not judged at all.
    let r = Repo::new();
    r.open_round();
    let mut cfg = r.json("goals/loop.json");
    cfg["authority"]["frozen"] = serde_json::json!([]);
    r.write_json("goals/loop.json", &cfg);
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 1, "{out}");
    assert_has(&out, "FROZEN modified");
    assert_has(&out, "loop.json");
}

mod measures_are_append_only {
    use super::*;

    #[test]
    fn a_round_may_add_a_rung() {
        let r = Repo::new();
        r.open_round();
        r.append(
            "tests/test_rung.py",
            "\n\ndef test_new():\n    assert True\n",
        );
        let (code, out) = r.run(&["guard"]);
        assert_eq!(code, 0, "{out}");
        assert_has(&out, "rung added");
    }

    #[test]
    fn a_round_may_not_weaken_a_rung() {
        let r = Repo::new();
        r.open_round();
        r.write(
            "tests/test_rung.py",
            "def test_existing():\n    assert True\n",
        );
        let (code, out) = r.run(&["guard"]);
        assert_eq!(code, 1);
        assert_has(&out, "MEASURE weakened");
    }

    #[test]
    fn a_round_may_not_delete_a_rung() {
        let r = Repo::new();
        r.open_round();
        r.remove("tests/test_rung.py");
        let (code, out) = r.run(&["guard"]);
        assert_eq!(code, 1);
        assert_has(&out, "MEASURE deleted");
    }
}

mod evidence_ledger {
    //! Compared structurally: a line diff gets this wrong, because adding a
    //! witness to an existing claim edits the previous line's comma.
    use super::*;

    #[test]
    fn may_add_a_witness_to_an_existing_claim() {
        let r = Repo::new();
        r.open_round();
        let mut d = ledger_base();
        d["challenge-401"]["witnesses"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("sdk:TestChallengeFlow"));
        r.write_json("witnesses.json", &d);
        let (code, out) = r.run(&["guard"]);
        assert_eq!(code, 0, "{out}");
        assert_has(&out, "ledger grew");
    }

    #[test]
    fn may_add_a_new_claim() {
        let r = Repo::new();
        r.open_round();
        let mut d = ledger_base();
        d["secret-roundtrip"] =
            serde_json::json!({"claim": "set then get", "witnesses": ["go:TestRoundtrip"]});
        r.write_json("witnesses.json", &d);
        assert_eq!(r.run(&["guard"]).0, 0);
    }

    #[test]
    fn may_not_drop_a_witness() {
        let r = Repo::new();
        r.open_round();
        let mut d = ledger_base();
        d["challenge-401"]["witnesses"] = serde_json::json!([]);
        r.write_json("witnesses.json", &d);
        let (code, out) = r.run(&["guard"]);
        assert_eq!(code, 1);
        assert_has(&out, "LEDGER weakened");
    }

    #[test]
    fn may_not_repoint_a_witness_at_an_easier_test() {
        let r = Repo::new();
        r.open_round();
        let mut d = ledger_base();
        d["challenge-401"]["witnesses"] = serde_json::json!(["go:TestSomethingTrivial"]);
        r.write_json("witnesses.json", &d);
        let (code, out) = r.run(&["guard"]);
        assert_eq!(code, 1);
        assert_has(&out, "LEDGER weakened");
    }

    #[test]
    fn may_not_soften_a_claim() {
        let r = Repo::new();
        r.open_round();
        let mut d = ledger_base();
        d["challenge-401"]["claim"] = serde_json::json!("returns 401 sometimes");
        r.write_json("witnesses.json", &d);
        assert_eq!(r.run(&["guard"]).0, 1);
    }
}

#[test]
fn propose_level_change_is_recorded_not_blocked() {
    let r = Repo::new();
    r.open_round();
    r.write("src/contract.py", "SCHEMA = 2\n");
    let (code, out) = r.run(&["guard"]);
    assert_eq!(code, 0, "{out}");
    assert_has(&out, "person decides");
    assert_has(&r.read("goals/proposals.md"), "src/contract.py");
}

#[test]
fn a_round_that_adds_no_rung_is_warned() {
    let r = Repo::new();
    r.open_round();
    r.write("src/app.py", "def escalate(i):\n    return 'changed'\n");
    assert_has(&r.run(&["guard"]).1, "no deterministic check was added");
}
