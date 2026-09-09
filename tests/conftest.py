import json
import subprocess
import sys
from pathlib import Path

import pytest

SRC = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(SRC))


@pytest.fixture
def repo(tmp_path, monkeypatch):
    """A minimal project with a product, a rung, and an evidence ledger."""
    (tmp_path / "src").mkdir()
    (tmp_path / "tests").mkdir()
    (tmp_path / "fixtures").mkdir()
    (tmp_path / "src" / "app.py").write_text("def escalate(i):\n    return True\n")
    (tmp_path / "tests" / "test_rung.py").write_text(
        "def test_existing():\n    assert 1 + 1 == 2\n")
    (tmp_path / "fixtures" / "seed.sql").write_text("-- seed\n")
    (tmp_path / "witnesses.json").write_text(json.dumps({
        "challenge-401": {"claim": "401 + WWW-Authenticate",
                          "witnesses": ["go:TestChallengeShape"]},
    }, indent=1) + "\n")
    monkeypatch.chdir(tmp_path)
    run(tmp_path, "init", ".")
    cfg = json.loads((tmp_path / "goals" / "loop.json").read_text())
    cfg["commands"] = {"reset": "true", "health": "true", "floor": "true",
                       "drive": "sh -c 'mkdir -p {out}; echo drove > {out}/t.txt'",
                       "score": "true"}
    cfg["authority"]["frozen"] = ["goals/corpus/**", "goals/loop.json", "fixtures/**"]
    cfg["authority"]["propose"] = ["src/contract.py"]
    cfg["measure"] = {"globs": ["tests/**", "witnesses.json"],
                      "append_only": True, "json_ledgers": ["witnesses.json"]}
    (tmp_path / "goals" / "loop.json").write_text(json.dumps(cfg, indent=2))
    return tmp_path


def run(cwd, *args):
    """Invoke the CLI the way a user does, and return (exit_code, output)."""
    p = subprocess.run([sys.executable, "-m", "otogo", *args], cwd=str(cwd),
                       capture_output=True, text=True,
                       env={"PATH": "/usr/bin:/bin", "PYTHONPATH": str(SRC),
                            "HOME": str(cwd)})
    return p.returncode, p.stdout + p.stderr


@pytest.fixture
def cli(repo):
    def _run(*args):
        return run(repo, *args)
    return _run


@pytest.fixture
def open_round(repo, cli):
    code, out = cli("open", "overdue-invoices")
    assert code == 0, out
    return repo
