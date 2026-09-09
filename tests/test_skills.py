"""The skills teach the procedure; the CLI enforces it.

They are allowed to disagree about emphasis. They are not allowed to disagree
about what commands exist — a skill that teaches a verb the CLI dropped sends an
agent down a path that dead-ends.
"""
import re
from pathlib import Path

import pytest

from otogo.cli import build_parser

SKILLS = sorted((Path(__file__).resolve().parents[1] / "skills").glob("*/SKILL.md"))


def subcommands():
    for action in build_parser()._subparsers._group_actions:
        return set(action.choices)
    return set()


def invocations(text):
    """Every `otogo <verb>` written as code, ignoring prose mentions."""
    fenced = "\n".join(re.findall(r"```(?:bash|sh)?\n(.*?)```", text, re.S))
    inline = "\n".join(re.findall(r"`([^`\n]*)`", text))
    return set(re.findall(r"\botogo\s+([a-z][a-z-]*)", fenced + "\n" + inline))


def test_there_are_skills_to_check():
    assert SKILLS, "no skills found"


@pytest.mark.parametrize("skill", SKILLS, ids=lambda p: p.parent.name)
class TestSkill:
    def test_frontmatter_name_matches_its_directory(self, skill):
        m = re.match(r"^---\nname: (.+?)\n", skill.read_text())
        assert m, f"{skill} has no name in frontmatter"
        assert m.group(1).strip() == skill.parent.name

    def test_has_a_description_with_triggers(self, skill):
        m = re.search(r"^description: (.+?)$", skill.read_text(), re.M)
        assert m, f"{skill} has no description"
        assert len(m.group(1)) > 60, "description too thin to route on"

    def test_every_command_it_teaches_exists(self, skill):
        unknown = invocations(skill.read_text()) - subcommands()
        assert not unknown, (
            f"{skill.parent.name} teaches commands the CLI does not have: "
            f"{sorted(unknown)}")


def test_skills_do_not_hardcode_the_authority_list():
    """Authority lives in loop.json and reaches the agent via `otogo brief`.

    Copying it into a skill creates a second source of truth that goes stale
    silently, and a stale authority list is worse than none.
    """
    for skill in SKILLS:
        text = skill.read_text()
        assert "goals/corpus/**" not in text, (
            f"{skill.parent.name} hardcodes a frozen glob; point at `otogo brief` instead")
