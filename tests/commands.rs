//! The commands a round actually runs, and the evidence they leave behind.

mod common;
use common::{assert_has, Repo};

mod preflight_and_floor {
    use super::*;

    #[test]
    fn preflight_passes_on_a_healthy_world() {
        let r = Repo::new();
        let (code, out) = r.run(&["preflight"]);
        assert_eq!(code, 0, "{out}");
        assert_has(&out, "world healthy");
    }

    #[test]
    fn preflight_refuses_to_score_an_unhealthy_world() {
        let r = Repo::new();
        r.set_command("health", "false");
        let (code, out) = r.run(&["preflight"]);
        assert_eq!(code, 2);
        assert_has(&out, "NO_SCORE");
        assert_has(&out, "not evidence that the feature failed");
    }

    #[test]
    fn a_missing_health_command_is_called_out_rather_than_assumed_fine() {
        // Without it, the loop cannot tell infrastructure trouble from a
        // feature failure, and every score below it is suspect.
        let r = Repo::new();
        r.set_command("health", "");
        let (_, out) = r.run(&["preflight"]);
        assert_has(&out, "no health command configured");
    }

    #[test]
    fn floor_reports_green_and_red() {
        let r = Repo::new();
        assert_eq!(r.run(&["floor"]).0, 0);
        r.set_command("floor", "false");
        let (code, out) = r.run(&["floor"]);
        assert_eq!(code, 1);
        assert_has(&out, "floor RED");
    }
}

mod driving {
    use super::*;

    #[test]
    fn the_driver_receives_the_request_and_an_output_directory() {
        let r = Repo::new();
        r.set_command(
            "drive",
            "sh -c 'mkdir -p \"$OTOGO_OUT\"; cp \"$OTOGO_REQUEST\" \"$OTOGO_OUT/seen.txt\"; \
             echo \"$OTOGO_PHASE\" > \"$OTOGO_OUT/phase.txt\"'",
        );
        r.open_round();
        let (code, out) = r.run(&["drive"]);
        assert_eq!(code, 0, "{out}");
        // The driver saw the approved request, unchanged.
        assert_has(&r.read("goals/rounds/001/drive/seen.txt"), "overdue");
        assert_eq!(r.read("goals/rounds/001/drive/phase.txt").trim(), "drive");
    }

    #[test]
    fn a_failing_driver_is_reported_not_swallowed() {
        let r = Repo::new();
        r.set_command("drive", "false");
        r.open_round();
        let (code, out) = r.run(&["drive"]);
        assert_eq!(code, 1);
        assert_has(&out, "driver exit 1");
    }

    #[test]
    fn drive_without_a_driver_configured_refuses() {
        let r = Repo::new();
        r.set_command("drive", "");
        r.open_round();
        let (code, out) = r.run(&["drive"]);
        assert_eq!(code, 2);
        assert_has(&out, "same surface a user would");
    }

    #[test]
    fn scoring_records_what_the_scorer_said() {
        let r = Repo::new();
        r.set_command("score", "echo 'effect_ok=false partial_path=true'");
        r.open_round();
        r.run(&["drive"]);
        let (code, out) = r.run(&["score"]);
        assert_eq!(code, 0, "{out}");
        assert_has(&out, "partial_path=true");
        assert_has(&r.read("goals/rounds/001/score.log"), "partial_path=true");
    }
}

mod verifying {
    //! `verify` is the command that decides whether the round earned anything.
    //!
    //! These fixtures degrade the world the way a real round would — a broken
    //! product, a service that went down — rather than by editing `loop.json`
    //! mid-round. That is itself frozen, so a test that reconfigures the
    //! harness after `open` trips the authority check before reaching what it
    //! meant to exercise.
    use super::*;

    /// A floor that is green until the product is broken, and a health check
    /// that fails once the world is marked down. Both are set before `open`.
    fn opened_and_repaired() -> Repo {
        let r = Repo::new();
        r.set_command("floor", "sh -c '! grep -q BROKEN src/app.py'");
        r.set_command("health", "sh -c '! test -f WORLD_DOWN'");
        r.open_round();
        r.run(&[
            "classify",
            "--layer",
            "domain",
            "--gap",
            "no persistent effect",
        ]);
        r.write("src/app.py", "def escalate(i):\n    return 'stored'\n");
        r.append(
            "tests/test_rung.py",
            "\n\ndef test_added():\n    assert True\n",
        );
        r
    }

    #[test]
    fn a_clean_repair_verifies() {
        let r = opened_and_repaired();
        let (code, out) = r.run(&["verify"]);
        assert_eq!(code, 0, "{out}");
        assert_has(&out, "floor green");
        assert_has(&out, "rung added");
        // It drives the SAME request again, into its own evidence directory.
        assert!(r.path("goals/rounds/001/verify").exists());
    }

    #[test]
    fn a_repair_that_reddens_the_floor_fails_verification() {
        let r = opened_and_repaired();
        r.write("src/app.py", "def escalate(i):\n    return BROKEN\n");
        let (code, out) = r.run(&["verify"]);
        assert_eq!(code, 1);
        assert_has(&out, "The floor went red");
    }

    #[test]
    fn a_red_floor_then_blocks_the_close() {
        // A round does not close over a regression it caused.
        let r = opened_and_repaired();
        r.write("src/app.py", "def escalate(i):\n    return BROKEN\n");
        r.run(&["verify"]);
        let (code, out) = r.run(&["close", "--outcome", "improved"]);
        assert_eq!(code, 2);
        assert_has(&out, "floor is red");
    }

    #[test]
    fn verification_fails_when_the_round_crossed_a_boundary() {
        let r = opened_and_repaired();
        r.write("goals/corpus/overdue-invoices.txt", "an easier request\n");
        let (code, out) = r.run(&["verify"]);
        assert_eq!(code, 1);
        assert_has(&out, "FROZEN modified");
    }

    #[test]
    fn a_world_that_goes_down_mid_round_cannot_attribute_the_run() {
        let r = opened_and_repaired();
        r.write("WORLD_DOWN", "the stack fell over\n");
        let (code, out) = r.run(&["verify"]);
        assert_eq!(code, 2);
        assert_has(&out, "cannot attribute this run");
    }
}

mod aborting {
    use super::*;

    #[test]
    fn an_aborted_round_keeps_its_evidence_but_does_not_count_as_progress() {
        let r = Repo::new();
        r.open_round();
        let (code, out) = r.run(&["abort", "harness gap, not a product round"]);
        assert_eq!(code, 0, "{out}");
        assert_has(&out, "evidence kept");

        let state = r.state();
        assert!(state["open_round"].is_null());
        // The batch budget is for rounds that produced something.
        assert_eq!(state["rounds_in_batch"], 0);
        let last = state["history"].as_array().unwrap().last().unwrap().clone();
        assert_has(last["outcome"].as_str().unwrap(), "aborted");
        assert!(r.path("goals/rounds/001").exists());
    }

    #[test]
    fn aborting_without_an_open_round_says_so() {
        let r = Repo::new();
        let (code, out) = r.run(&["abort", "nothing to abort"]);
        assert_eq!(code, 2);
        assert_has(&out, "no open round");
    }
}

mod timeouts {
    //! A command that hangs is indistinguishable from one that is slow until
    //! something kills it. This is the round that cost real time before it
    //! existed: an unmodified SDK looping forever on a paging bug produced no
    //! output at all, and otogo waited.
    use super::*;

    #[test]
    fn a_hanging_driver_is_killed_and_reported_as_a_timeout() {
        let r = Repo::new();
        r.set_command("drive", "sleep 30");
        r.set_config("/timeouts/drive", serde_json::json!(1));
        r.open_round();

        let start = std::time::Instant::now();
        let (_, out) = r.run(&["drive"]);
        assert!(
            start.elapsed().as_secs() < 15,
            "otogo waited instead of killing"
        );
        assert_has(&out, "TIMED OUT");
        assert_has(&out, "raise `timeouts.drive`");
    }

    #[test]
    fn the_timeout_is_recorded_in_the_evidence() {
        let r = Repo::new();
        r.set_command("drive", "sleep 30");
        r.set_config("/timeouts/drive", serde_json::json!(1));
        r.open_round();
        r.run(&["drive"]);

        let ev = r.json("goals/rounds/001/drive.json");
        assert_eq!(ev["timed_out"], true);
        assert_eq!(ev["exit"], 124, "should use timeout(1)'s conventional code");
        assert!(
            ev["stderr"]
                .as_str()
                .unwrap_or("")
                .contains("killed after 1s"),
            "the log should say why: {ev}"
        );
    }

    #[test]
    fn a_hanging_floor_is_red_not_a_wait() {
        let r = Repo::new();
        r.set_command("floor", "sleep 30");
        r.set_config("/timeouts/floor", serde_json::json!(1));
        let (code, out) = r.run(&["floor"]);
        assert_eq!(code, 1);
        assert_has(&out, "floor TIMED OUT");
    }

    #[test]
    fn zero_means_no_limit() {
        let r = Repo::new();
        r.set_command("drive", "true");
        r.set_config("/timeouts/default", serde_json::json!(0));
        r.open_round();
        let (code, out) = r.run(&["drive"]);
        assert_eq!(code, 0, "{out}");
        assert!(!out.contains("TIMED OUT"));
    }
}
