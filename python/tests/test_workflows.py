"""
The CI workflows (``.github/workflows``), checked without running them
(``python/tools/check_workflows.py``): the YAML reader, the rules, and
the tree's workflows against them.
"""

import json
import pathlib
import subprocess
import sys
import tomllib

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[2]
if not (ROOT / ".github" / "workflows").is_dir():
    pytest.skip("the workflows are in the source tree only", allow_module_level=True)
sys.path.insert(0, str(ROOT / "python" / "tools"))

import check_workflows  # noqa: E402

#: A workflow the rules accept.
GOOD = """\
name: x  # a comment
on:
  workflow_dispatch:
  push:
    tags: ["v*"]
permissions:
  contents: read
jobs:
  plan:
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    outputs:
      list: ${{ steps.plan.outputs.list }}
    steps:
      - id: plan
        run: |
          echo "list=[1]" >> "$GITHUB_OUTPUT"  # kept: inside the block

          echo 'a # b'
  build:
    needs: plan
    runs-on: ${{ matrix.os }}
    timeout-minutes: 30
    strategy:
      matrix:
        include: ${{ fromJSON(needs.plan.outputs.list) }}
        extra: [{ a: 1, b: "two, three" }, 'it''s']
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
      - name: Check
        run: python python/tools/check_dist.py dist/*
  publish:
    if: startsWith(github.ref, 'refs/tags/v')
    needs: [build]
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    environment: pypi
    permissions:
      id-token: write
    steps:
      - run: "true"
"""


def test_the_reader_takes_the_yaml_the_workflows_use():
    workflow = check_workflows.parse(GOOD)
    assert workflow["on"] == {"workflow_dispatch": None, "push": {"tags": ["v*"]}}
    plan = workflow["jobs"]["plan"]
    assert plan["steps"][0]["run"] == (
        'echo "list=[1]" >> "$GITHUB_OUTPUT"  # kept: inside the block\n\necho \'a # b\'\n')
    matrix = workflow["jobs"]["build"]["strategy"]["matrix"]
    assert matrix["extra"] == [{"a": "1", "b": "two, three"}, "it's"]
    assert workflow["jobs"]["build"]["steps"][0]["uses"].endswith("ba90b1")
    assert workflow["jobs"]["publish"]["needs"] == ["build"]
    with pytest.raises(check_workflows.YamlError, match="line 3: unexpected indentation"):
        check_workflows.parse("a:\n  b: 1\n    c: 2\n")
    with pytest.raises(check_workflows.YamlError, match="a given twice"):
        check_workflows.parse("a: 1\na: 2\n")


def test_a_good_workflow_passes():
    assert check_workflows.problems(GOOD) == []


@pytest.mark.parametrize("change, problem", [
    (("  push:\n    tags:", "  pull_request:\n  push:\n    tags:"), "runs for pull_request"),
    (('    tags: ["v*"]', '    branches: [main]'), "runs for pushes other than of tags"),
    (("@3d3c42e5aac5ba805825da76410c181273ba90b1", "@v7"),
     "build: actions/checkout@v7 is not pinned to a commit"),
    (("    if: startsWith(github.ref, 'refs/tags/v')", "    if: matrix.os != ''"),
     "publish: its if uses matrix, which a job's if cannot see"),
    (("    needs: [build]", "    needs: [test]"), "publish: needs the unknown job test"),
    (("needs.plan.outputs.list", "needs.plan.outputs.other"), "build: plan has no output other"),
    (("steps.plan.outputs", "steps.planned.outputs"), "plan: no step planned"),
    (("python/tools/check_dist.py", "python/tools/missing.py"),
     "build: runs python/tools/missing.py, which the tree lacks"),
    (("    environment: pypi\n", ""),
     "publish: may publish, but not for tags only in an environment"),
    (("    timeout-minutes: 30\n", ""), "build: no timeout-minutes"),
    (("  contents: read", "  contents: write"),
     "the workflow's permissions are not contents: read"),
    (("    needs: plan\n", "    needs: [plan, publish]\n"), "the jobs' needs loop"),
])
def test_the_rules_find_what_is_wrong(change, problem):
    old, new = change
    assert old in GOOD
    assert problem in check_workflows.problems(GOOD.replace(old, new))


def test_the_workflows_of_the_tree_pass():
    workflows = sorted((ROOT / ".github" / "workflows").glob("*.yml"))
    assert workflows
    for workflow in workflows:
        assert check_workflows.problems(workflow.read_text(encoding="utf-8")) == [], workflow.name


def test_the_plan_checks_the_tag_and_chooses_the_runners(tmp_path):
    with open(ROOT / "Cargo.toml", "rb") as file:
        version = tomllib.load(file)["workspace"]["package"]["version"]
    output = tmp_path / "output"

    def plan(**environment):
        output.write_text("")
        return subprocess.run(
            [sys.executable, ".github/scripts/plan.py"], cwd=ROOT, capture_output=True, text=True,
            env={"GITHUB_OUTPUT": str(output), "PATH": "", **environment})

    assert plan(EVENT="push", TAG=f"v{version}", WIN_ARM64="true",
                LINUX_ARM64_RUNNER="arm-box").returncode == 0
    outputs = dict(line.split("=", 1) for line in output.read_text().splitlines())
    linux = json.loads(outputs["linux"])
    assert {"platform": "musllinux_1_2", "arch": "aarch64", "runner": "arm-box",
            "target": "aarch64-unknown-linux-musl",
            "settings": "--config-settings target=aarch64-unknown-linux-musl"} in linux
    assert {"platform": "manylinux2014", "arch": "x86_64", "runner": "ubuntu-24.04",
            "target": "x86_64-unknown-linux-gnu",
            "settings": "--config-settings target=x86_64-unknown-linux-gnu "
                        "--config-settings glibc=2.17"} in linux
    assert json.loads(outputs["windows"]) == ["x86_64", "aarch64"]
    assert {"arch": "aarch64", "runner": "windows-11-arm", "python": "3.14"} in json.loads(
        outputs["windows_tests"])
    assert plan(EVENT="workflow_dispatch", TAG="main", WIN_ARM64="false").returncode == 0
    assert "windows=[\"x86_64\"]" in output.read_text()
    refused = plan(EVENT="push", TAG="v0.0.0-other")
    assert refused.returncode == 1 and "is not v" in refused.stderr
