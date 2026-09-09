"""The loop may change the product. It may not change the evidence."""
import json


def test_free_files_change_without_objection(open_round, cli):
    (open_round / "src" / "app.py").write_text("def escalate(i):\n    return 'done'\n")
    code, out = cli("guard")
    assert code == 0
    assert "src/app.py" in out
    assert "no boundary crossed" in out


def test_frozen_file_cannot_be_modified(open_round, cli):
    (open_round / "goals" / "corpus" / "overdue-invoices.txt").write_text("easier\n")
    code, out = cli("guard")
    assert code == 1
    assert "FROZEN modified" in out


def test_frozen_file_cannot_be_added(open_round, cli):
    (open_round / "goals" / "corpus" / "convenient.txt").write_text("a request I wrote\n")
    code, out = cli("guard")
    assert code == 1
    assert "FROZEN added to" in out


def test_frozen_file_cannot_be_deleted(open_round, cli):
    (open_round / "fixtures" / "seed.sql").unlink()
    code, out = cli("guard")
    assert code == 1
    assert "FROZEN deleted" in out


def test_the_loop_cannot_edit_its_own_authority(open_round, cli):
    cfg = json.loads((open_round / "goals" / "loop.json").read_text())
    cfg["authority"]["frozen"] = []
    (open_round / "goals" / "loop.json").write_text(json.dumps(cfg, indent=2))
    code, out = cli("guard")
    assert code == 1
    assert "FROZEN modified" in out and "loop.json" in out


class TestMeasuresAreAppendOnly:
    def test_a_round_may_add_a_rung(self, open_round, cli):
        p = open_round / "tests" / "test_rung.py"
        p.write_text(p.read_text() + "\n\ndef test_new():\n    assert True\n")
        code, out = cli("guard")
        assert code == 0
        assert "rung added" in out

    def test_a_round_may_not_weaken_a_rung(self, open_round, cli):
        p = open_round / "tests" / "test_rung.py"
        p.write_text(p.read_text().replace("assert 1 + 1 == 2", "assert True"))
        code, out = cli("guard")
        assert code == 1
        assert "MEASURE weakened" in out

    def test_a_round_may_not_delete_a_rung(self, open_round, cli):
        (open_round / "tests" / "test_rung.py").unlink()
        code, out = cli("guard")
        assert code == 1
        assert "MEASURE deleted" in out


class TestEvidenceLedger:
    """witnesses.json is compared structurally: line diffs get this wrong."""

    def _ledger(self, repo):
        return json.loads((repo / "witnesses.json").read_text())

    def _write(self, repo, d):
        (repo / "witnesses.json").write_text(json.dumps(d, indent=1) + "\n")

    def test_may_add_a_witness_to_an_existing_claim(self, open_round, cli):
        d = self._ledger(open_round)
        d["challenge-401"]["witnesses"].append("sdk:TestChallengeFlow")
        self._write(open_round, d)
        code, out = cli("guard")
        assert code == 0, out
        assert "ledger grew" in out

    def test_may_add_a_new_claim(self, open_round, cli):
        d = self._ledger(open_round)
        d["secret-roundtrip"] = {"claim": "set then get", "witnesses": ["go:TestRoundtrip"]}
        self._write(open_round, d)
        assert cli("guard")[0] == 0

    def test_may_not_drop_a_witness(self, open_round, cli):
        d = self._ledger(open_round)
        d["challenge-401"]["witnesses"] = []
        self._write(open_round, d)
        code, out = cli("guard")
        assert code == 1
        assert "LEDGER weakened" in out

    def test_may_not_repoint_a_witness_at_an_easier_test(self, open_round, cli):
        d = self._ledger(open_round)
        d["challenge-401"]["witnesses"] = ["go:TestSomethingTrivial"]
        self._write(open_round, d)
        code, out = cli("guard")
        assert code == 1
        assert "LEDGER weakened" in out

    def test_may_not_soften_a_claim(self, open_round, cli):
        d = self._ledger(open_round)
        d["challenge-401"]["claim"] = "returns 401 sometimes"
        self._write(open_round, d)
        assert cli("guard")[0] == 1


def test_propose_level_change_is_recorded_not_blocked(open_round, cli):
    (open_round / "src" / "contract.py").write_text("SCHEMA = 2\n")
    code, out = cli("guard")
    assert code == 0
    assert "person decides" in out
    assert "src/contract.py" in (open_round / "goals" / "proposals.md").read_text()


def test_a_round_that_adds_no_rung_is_warned(open_round, cli):
    (open_round / "src" / "app.py").write_text("def escalate(i):\n    return 'changed'\n")
    assert "no deterministic check was added" in cli("guard")[1]
