//! Stop rules, budgets, and the boundary a person owns.

mod common;
use common::{assert_has, Repo};

mod world_health {
    use super::*;

    #[test]
    fn unhealthy_world_scores_nothing_and_opens_no_round() {
        let r = Repo::new();
        r.set_command("health", "false");
        let (code, out) = r.run(&["open", "overdue-invoices"]);
        assert_eq!(code, 2);
        assert_has(&out, "NO_SCORE");
        // Infrastructure trouble is not evidence that the feature failed, so
        // the round must not linger as if it were in progress.
        assert!(r.state()["open_round"].is_null());
        let hist = r.state()["history"].as_array().unwrap().clone();
        assert_eq!(hist.last().unwrap()["outcome"], "no_score");
    }

    #[test]
    fn a_healthy_world_opens_normally() {
        let r = Repo::new();
        assert_eq!(r.run(&["open", "overdue-invoices"]).0, 0);
        assert_eq!(r.state()["open_round"]["request"], "overdue-invoices");
    }
}

mod classification {
    use super::*;

    #[test]
    fn escalate_layers_stop_the_round() {
        for layer in ["world", "harness"] {
            let r = Repo::new();
            r.open_round();
            let (code, out) = r.run(&["classify", "--layer", layer, "--gap", "not a product gap"]);
            assert_eq!(code, 2, "layer {layer}: {out}");
            assert_has(&out, "not a product round");
        }
    }

    #[test]
    fn product_layers_continue() {
        let r = Repo::new();
        r.open_round();
        let (code, out) = r.run(&[
            "classify",
            "--layer",
            "domain",
            "--gap",
            "no persistent effect",
        ]);
        assert_eq!(code, 0, "{out}");
        assert!(r.path("goals/rounds/001/gap.md").exists());
    }

    #[test]
    fn an_unknown_layer_is_refused() {
        let r = Repo::new();
        r.open_round();
        assert_ne!(
            r.run(&["classify", "--layer", "frontend", "--gap", "x"]).0,
            0
        );
    }
}

mod closing {
    use super::*;

    #[test]
    fn a_round_cannot_close_over_an_authority_violation() {
        let r = Repo::new();
        r.open_round();
        r.write("goals/corpus/overdue-invoices.txt", "easier\n");
        let (code, out) = r.run(&["close", "--outcome", "improved"]);
        assert_eq!(code, 2);
        assert_has(&out, "block the close");
        assert!(!r.state()["open_round"].is_null());
    }

    #[test]
    fn a_clean_round_closes_and_is_recorded() {
        let r = Repo::new();
        r.open_round();
        r.run(&[
            "classify",
            "--layer",
            "domain",
            "--gap",
            "no persistent effect",
        ]);
        r.append(
            "tests/test_rung.py",
            "\n\ndef test_added():\n    assert True\n",
        );
        let (code, out) = r.run(&["close", "--outcome", "improved"]);
        assert_eq!(code, 0, "{out}");
        let state = r.state();
        let entry = state["history"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(entry["outcome"], "improved");
        assert_eq!(entry["layer"], "domain");
        assert!(state["open_round"].is_null());
    }
}

mod batch_boundary {
    use super::*;

    fn spend_the_batch(r: &Repo) {
        for _ in 0..3 {
            r.run(&["open", "overdue-invoices"]);
            r.run(&["close", "--outcome", "unchanged"]);
        }
    }

    #[test]
    fn the_budget_stops_the_loop_for_a_person() {
        let r = Repo::new();
        spend_the_batch(&r);
        let (code, out) = r.run(&["open", "overdue-invoices"]);
        assert_eq!(code, 2);
        assert_has(&out, "batch 1 is spent");
    }

    #[test]
    fn a_person_can_continue_the_queue() {
        let r = Repo::new();
        spend_the_batch(&r);
        assert_eq!(r.run(&["batch", "--continue"]).0, 0);
        assert_eq!(r.run(&["open", "overdue-invoices"]).0, 0);
    }

    #[test]
    fn a_person_can_redirect_the_next_batch() {
        let r = Repo::new();
        r.run(&[
            "batch",
            "--redirect",
            "close the read path; then the write path",
        ]);
        assert_eq!(
            r.state()["queue"],
            serde_json::json!(["close the read path", "then the write path"])
        );
    }
}

mod friction {
    use super::*;

    #[test]
    fn first_occurrence_is_evidence_not_a_rule() {
        let r = Repo::new();
        let (code, out) = r.run(&["friction", "the driver never records effects"]);
        assert_eq!(code, 0);
        assert_has(&out, "not a rule");
    }

    #[test]
    fn repeated_friction_earns_a_change() {
        let r = Repo::new();
        for _ in 0..3 {
            r.run(&["friction", "the driver never records effects"]);
        }
        assert_has(&r.run(&["status"]).1, "(3x)");
    }
}

#[test]
fn the_brief_carries_the_authority_into_a_fresh_session() {
    let r = Repo::new();
    r.open_round();
    let out = r.run(&["brief"]).1;
    assert_has(&out, "goals/corpus/**");
    assert_has(&out, "append-only");
    assert_has(&out, "overdue-invoices");
}
