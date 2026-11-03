"""The differential tester: programs run under matplotlib and pltrs, compared.

    python driver.py [--suite gallery|generated|all] [PATTERN...] [--jobs N]
                     [--failures] [--reduce] [--baseline REPORT]

``cargo xtask diff`` runs this in the reference venv (after building
pltrs (Python)'s extension), with ``--pltrs-python`` naming the pltrs
venv's interpreter. It

1. collects the programs: matplotlib's gallery (``--gallery``, the
   sdist's ``galleries/``) and the generated programs (``generate.py``
   writes them under ``OUT/programs``);
2. runs each under both implementations (``trace.py``), the reference
   runs cached by the program's text (``OUT/runs/<program>/mpl``);
3. compares them (``compare.py``), drops the differences the allowlist
   (``allowlist.txt``) proves intended, and groups the rest by cause;
4. writes ``OUT/report.json`` (every difference) and ``OUT/report.md``
   (the groups, the biggest first, each with its smallest reproduction,
   and the gallery examples that fail, with their causes), and prints the
   groups' counts; ``--baseline`` adds the counts of an earlier report.

A program matplotlib itself cannot run here (a GUI toolkit, a network
download) is listed as skipped, with matplotlib's error. ``--reduce``
reduces the first difference of each group to the fewest statements that
still show it (``reduce.py``). PATTERN keeps the programs whose names
contain it.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
from collections import Counter, defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare  # noqa: E402

#: Seconds a program may run under one implementation.
TIMEOUT = 300
#: Gallery directories run, under the sdist's ``galleries/``.
GALLERY_PARTS = ("examples", "plot_types", "tutorials", "users_explain")


class Setup:
    """Where things are, and how each implementation runs."""

    def __init__(self, args: argparse.Namespace) -> None:
        self.root = pathlib.Path(args.root).resolve()
        self.out = pathlib.Path(args.out).resolve()
        self.gallery = pathlib.Path(args.gallery).resolve()
        self.pltrs_python = args.pltrs_python
        self.mpl_python = sys.executable

    def command(self, impl: str, mode: str, program: pathlib.Path, out: pathlib.Path) -> list[str]:
        """The command tracing *program* under *impl* into *out*."""
        python = self.mpl_python if impl == "matplotlib" else self.pltrs_python
        return [python, str(HERE / "trace.py"), "--impl", impl, "--mode", mode, str(program), str(out)]

    def environment(self, impl: str) -> dict[str, str]:
        """The environment of a run under *impl*."""
        env = {k: v for k, v in os.environ.items()
               if not k.startswith(("MPL", "PLTRS", "PYTHON")) and k != "CARGO_TARGET_DIR"}
        env["MPLBACKEND"] = "agg"
        # (Named apart from the package names `trace.py` replaces in messages.)
        env["MPLCONFIGDIR"] = str(self.out / ("config-reference" if impl == "matplotlib"
                                              else "config-candidate"))
        env["PYTHONHASHSEED"] = "0"
        if impl == "pltrs":
            env["PLTRS_LOOK"] = "matplotlib"
            env["PYTHONPATH"] = str(self.root / "python")
        return env


class Program:
    """One program of the run."""

    def __init__(self, name: str, path: pathlib.Path, mode: str) -> None:
        self.name = name
        self.path = path
        self.mode = mode

    @property
    def digest(self) -> str:
        """What the reference run depends on: the program, its mode and the
        tester's harness."""
        harness = b"".join((HERE / name).read_bytes() for name in ("trace.py", "describe.py"))
        return hashlib.sha256(self.path.read_bytes() + self.mode.encode() + harness).hexdigest()[:16]

    def directory(self, setup: Setup) -> pathlib.Path:
        """Where the runs of the program go."""
        return setup.out / "runs" / re.sub(r"[^\w.-]", "_", self.name)


def gallery_programs(gallery: pathlib.Path) -> list[Program]:
    """matplotlib's gallery, as programs run whole."""
    programs = []
    for part in GALLERY_PARTS:
        for path in sorted((gallery / part).rglob("*.py")):
            name = "gallery/" + str(path.relative_to(gallery)).removesuffix(".py")
            programs.append(Program(name, path, "program"))
    return programs


def generated_programs(setup: Setup) -> list[Program]:
    """The programs `generate.py` writes, run statement by statement."""
    target = setup.out / "programs"
    subprocess.run([setup.mpl_python, str(HERE / "generate.py"), str(target)], check=True,
                   env=setup.environment("matplotlib"))
    return [
        Program("generated/" + str(path.relative_to(target)).removesuffix(".py"), path, "steps")
        for path in sorted(target.rglob("*.py"))
    ]


def _run(setup: Setup, impl: str, program: Program, out: pathlib.Path) -> dict[str, Any] | None:
    """The record of one run, or None with the reason in ``out/failure``."""
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    try:
        done = subprocess.run(setup.command(impl, program.mode, program.path, out), env=setup.environment(impl),
                              capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        (out / "failure").write_text(f"timed out after {TIMEOUT} s")
        return None
    (out / "stderr.txt").write_text(done.stderr[-20000:])
    record = out / "record.json"
    if done.returncode != 0 or not record.exists():
        tail = done.stderr.strip().splitlines()[-1:] or [f"exit status {done.returncode}"]
        (out / "failure").write_text(f"exit status {done.returncode}: {tail[0]}")
        return None
    return json.loads(record.read_text())


def run_program(setup: Setup, program: Program) -> dict[str, Any]:
    """Runs one program under both implementations and compares them."""
    directory = program.directory(setup)
    reference = directory / "mpl"
    stamp = directory / "mpl.digest"
    cached = stamp.exists() and stamp.read_text() == program.digest
    if cached and (reference / "record.json").exists():
        mpl = json.loads((reference / "record.json").read_text())
    elif cached and (reference / "failure").exists():
        # matplotlib could not run it here (a timeout waiting for a mouse,
        # say): it would not now either.
        mpl = None
    else:
        mpl = _run(setup, "matplotlib", program, reference)
        stamp.write_text(program.digest)
    result: dict[str, Any] = {"program": program.name, "path": str(program.path)}
    if mpl is None:
        result["skipped"] = (reference / "failure").read_text()
        return result
    if program.mode == "program" and mpl.get("uncaught") and _environmental(mpl["uncaught"]):
        result["skipped"] = f"{mpl['uncaught']['type']}: {mpl['uncaught']['message'][:200]}"
        return result
    pltrs = _run(setup, "pltrs", program, directory / "pltrs")
    if pltrs is None:
        failure = (directory / "pltrs" / "failure").read_text()
        result["differences"] = [compare.Difference(
            program.name, "exception", f"crash: harness: {compare._message_key(failure)}", "the run",
            failure).as_json()]
        return result
    result["seconds"] = {"matplotlib": mpl["seconds"], "pltrs": pltrs["seconds"]}
    found = compare.differences(program.name, program.path, mpl, pltrs, reference, directory / "pltrs")
    result["differences"] = [dict(d.as_json(), program_path=str(program.path)) for d in found]
    return result


def _environmental(error: dict[str, Any]) -> bool:
    """Whether matplotlib's own run failed for want of something this
    machine lacks (a GUI toolkit, a network, a module)."""
    kinds = ("ModuleNotFoundError", "ImportError", "URLError", "ConnectionError", "TimeoutError",
             "TclError", "OSError", "FileNotFoundError")
    return error["type"].rsplit(".", 1)[-1] in kinds or "display" in error["message"].lower()


class Allowlist:
    """Differences proven intended: ``<program regex> | <cause regex> |
    reason`` lines of ``allowlist.txt`` (the fields apart by `` | ``)."""

    def __init__(self, path: pathlib.Path) -> None:
        self.entries = []
        if path.exists():
            for line in path.read_text().splitlines():
                if line.strip() and not line.lstrip().startswith("#"):
                    program, cause, reason = (part.strip() for part in line.split(" | ", 2))
                    self.entries.append((re.compile(program), re.compile(cause), reason))

    def allows(self, difference: dict[str, Any]) -> bool:
        """Whether the allowlist allows *difference*."""
        return any(p.search(difference["program"]) and c.search(difference["cause"])
                   for p, c, _ in self.entries)


def _group(results: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in results:
        for difference in result.get("differences", []):
            groups[difference["cause"]].append(difference)
    return dict(sorted(groups.items(), key=lambda item: (-len({d["program"] for d in item[1]}), -len(item[1]))))


def _repro(difference: dict[str, Any]) -> str:
    where = f"{difference['program']} {difference['where']}"
    source = f"\n      `{difference['source']}`" if difference.get("source") else ""
    detail = f"\n      {difference['detail']}" if difference.get("detail") else ""
    return f"{where}{source}{detail}"


def write_report(setup: Setup, results: list[dict[str, Any]], baseline: dict[str, int] | None,
                 failures: bool) -> str:
    """Write ``report.md`` and ``report.json``; return the Markdown."""
    groups = _group(results)
    kinds = Counter(d["kind"] for found in groups.values() for d in found)
    ran = [r for r in results if "skipped" not in r]
    clean = [r for r in ran if not r.get("differences")]
    lines = [
        "# Differential test of pltrs (Python) against matplotlib",
        "",
        f"{len(results)} programs: {len(ran)} ran under both, {len(clean)} without a difference, "
        f"{len(results) - len(ran)} skipped (matplotlib cannot run them here).",
        "",
        "Differences by kind: " + ", ".join(f"{k} {n}" for k, n in kinds.most_common()) + ".",
        "",
        "## Groups (programs, differences" + (", before" if baseline is not None else "") + ")",
        "",
    ]
    for cause, found in groups.items():
        programs = len({d["program"] for d in found})
        before = f", before {baseline.get(cause, 0)}" if baseline is not None else ""
        lines.append(f"* **{cause}** ({programs}, {len(found)}{before})")
        reduced = found[0].get("reduced")
        lines.append(f"  - {_repro(found[0])}")
        if reduced:
            lines.append("  - reduced:\n\n```python\n" + reduced + "\n```")
        if failures:
            for difference in found[1:]:
                lines.append(f"  - {_repro(difference)}")
    if baseline is not None:
        gone = sorted(set(baseline) - set(groups))
        if gone:
            lines += ["", "## Groups gone since the baseline", ""]
            lines += [f"* {cause} (before {baseline[cause]})" for cause in gone]
    gallery = [r for r in ran if r["program"].startswith("gallery/") and r.get("differences")]
    if gallery:
        lines += ["", "## Gallery examples with differences", ""]
        for result in gallery:
            causes = Counter(d["cause"] for d in result["differences"])
            lines.append(f"* {result['program']}: " + "; ".join(
                f"{c}" + (f" ({n}x)" if n > 1 else "") for c, n in causes.most_common(6)))
    skipped = [r for r in results if "skipped" in r]
    if skipped:
        lines += ["", "## Skipped (matplotlib cannot run them here)", ""]
        lines += [f"* {r['program']}: {r['skipped']}" for r in skipped]
    report = "\n".join(lines) + "\n"
    (setup.out / "report.md").write_text(report)
    (setup.out / "report.json").write_text(json.dumps({
        "groups": {cause: len(found) for cause, found in groups.items()},
        "results": results,
    }, indent=1))
    return report


def main() -> int:
    """Run the programs, compare them and report."""
    parser = argparse.ArgumentParser()
    parser.add_argument("patterns", nargs="*")
    parser.add_argument("--suite", choices=["gallery", "generated", "all"], default="all")
    parser.add_argument("--root", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--gallery", required=True)
    parser.add_argument("--pltrs-python", required=True)
    parser.add_argument("--jobs", type=int, default=8)
    parser.add_argument("--failures", action="store_true")
    parser.add_argument("--reduce", action="store_true")
    parser.add_argument("--baseline")
    args = parser.parse_intermixed_args()
    setup = Setup(args)
    setup.out.mkdir(parents=True, exist_ok=True)

    programs: list[Program] = []
    if args.suite in ("gallery", "all"):
        if not setup.gallery.is_dir():
            parser.error(f"no gallery at {setup.gallery} (MPL_SDIST names matplotlib's sdist)")
        programs += gallery_programs(setup.gallery)
    if args.suite in ("generated", "all"):
        programs += generated_programs(setup)
    if args.patterns:
        programs = [p for p in programs if any(pattern in p.name for pattern in args.patterns)]

    results: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        futures = {pool.submit(run_program, setup, program): program for program in programs}
        for done, future in enumerate(concurrent.futures.as_completed(futures), 1):
            result = future.result()
            results.append(result)
            marker = "skip" if "skipped" in result else len(result.get("differences", []))
            print(f"[{done}/{len(programs)}] {result['program']}: {marker}", file=sys.stderr, flush=True)
    results.sort(key=lambda r: r["program"])

    allowlist = Allowlist(HERE / "allowlist.txt")
    for result in results:
        if "differences" in result:
            result["allowed"] = [d for d in result["differences"] if allowlist.allows(d)]
            result["differences"] = [d for d in result["differences"] if not allowlist.allows(d)]

    if args.reduce:
        import reduce
        for found in _group(results).values():
            reduce.reduce(setup, found[0], run_program)

    baseline = None
    if args.baseline:
        baseline = json.loads(pathlib.Path(args.baseline).read_text())["groups"]
    report = write_report(setup, results, baseline, args.failures)
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
