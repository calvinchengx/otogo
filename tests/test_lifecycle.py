"""Stop rules, budgets, and the boundary a person owns."""
import json

import pytest


def state(repo):
    return json.loads((repo / "goals" / "STATE.json").read_text())


def set_cmd(repo, **kw):
    p = repo / "goals" / "loop.json"
    cfg = json.loads(p.read_text())
    cfg["commands"].update(kw)
    p.write_text(json.dumps(cfg, indent=2))


class TestWorldHealth:
    def test_unhealthy_world_scores_nothing_and_opens_no_round(self, repo, cli):
        set_cmd(repo, health="false")
        code, out = cli("open", "overdue-invoices")
        assert code == 2
        assert "NO_SCORE" in out
        # Infrastructure trouble is not evidence that the feature failed, so the
        # round must not linger as if it were in progress.
        assert state(repo)["open_round"] is None
        assert state(repo)["history"][-1]["outcome"] == "no_score"

    def test_a_healthy_world_opens_normally(self, repo, cli):
        assert cli("open", "overdue-invoices")[0] == 0
        assert state(repo)["open_round"]["request"] == "overdue-invoices"


class TestClassification:
    @pytest.mark.parametrize("layer", ["world", "harness"])
    def test_escalate_layers_stop_the_round(self, open_round, cli, layer):
        code, out = cli("classify", "--layer", layer, "--gap", "not a product gap")
        assert code == 2
        assert "not a product round" in out

    def test_product_layers_continue(self, open_round, cli):
        code, out = cli("classify", "--layer", "domain", "--gap", "no persistent effect")
        assert code == 0
        assert (open_round / "goals" / "rounds" / "001" / "gap.md").exists()

    def test_an_unknown_layer_is_refused(self, open_round, cli):
        assert cli("classify", "--layer", "frontend", "--gap", "x")[0] != 0


class TestClosing:
    def test_a_round_cannot_close_over_an_authority_violation(self, open_round, cli):
        (open_round / "goals" / "corpus" / "overdue-invoices.txt").write_text("easier\n")
        code, out = cli("close", "--outcome", "improved")
        assert code == 2
        assert "block the close" in out
        assert state(open_round)["open_round"] is not None

    def test_a_clean_round_closes_and_is_recorded(self, open_round, cli):
        cli("classify", "--layer", "domain", "--gap", "no persistent effect")
        p = open_round / "tests" / "test_rung.py"
        p.write_text(p.read_text() + "\n\ndef test_added():\n    assert True\n")
        code, out = cli("close", "--outcome", "improved")
        assert code == 0, out
        entry = state(open_round)["history"][-1]
        assert entry["outcome"] == "improved" and entry["layer"] == "domain"
        assert state(open_round)["open_round"] is None


class TestBatchBoundary:
    def test_the_budget_stops_the_loop_for_a_person(self, repo, cli):
        for _ in range(3):
            cli("open", "overdue-invoices")
            cli("close", "--outcome", "unchanged")
        code, out = cli("open", "overdue-invoices")
        assert code == 2
        assert "batch 1 is spent" in out

    def test_a_person_can_continue_the_queue(self, repo, cli):
        for _ in range(3):
            cli("open", "overdue-invoices")
            cli("close", "--outcome", "unchanged")
        assert cli("batch", "--continue")[0] == 0
        assert cli("open", "overdue-invoices")[0] == 0

    def test_a_person_can_redirect_the_next_batch(self, repo, cli):
        cli("batch", "--redirect", "close the read path; then the write path")
        assert state(repo)["queue"] == ["close the read path", "then the write path"]


class TestFriction:
    def test_first_occurrence_is_evidence_not_a_rule(self, repo, cli):
        code, out = cli("friction", "the driver never records effects")
        assert code == 0
        assert "not a rule" in out

    def test_repeated_friction_earns_a_change(self, repo, cli):
        for _ in range(3):
            cli("friction", "the driver never records effects")
        assert "(3x)" in cli("status")[1]


class TestBrief:
    def test_the_brief_carries_the_authority_into_a_fresh_session(self, open_round, cli):
        out = cli("brief")[1]
        assert "goals/corpus/**" in out
        assert "append-only" in out
        assert "overdue-invoices" in out
