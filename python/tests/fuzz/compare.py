"""
The fuzzer's comparison with matplotlib: each program that met no bug
runs again under matplotlib (the reference venv, with pltrs imported as
matplotlib), and the first line where pltrs raised another exception than
matplotlib, or raised where matplotlib did not (or the other way round),
is a difference. Lines after it are not compared: the programs' states
differ from there on.

Differences are not bugs (the differential tester, ``cargo xtask diff``,
is where pltrs is held to matplotlib's behaviour): they are grouped by
the two exceptions in ``differences.md`` in the fuzzer's directory, each
group with a program to look at, and listed one by one, with the line
where each program diverged, in ``differences.json``.
"""

import json
from concurrent.futures import ThreadPoolExecutor

#: How many programs a reference worker runs before a new one takes over.
WORKER_TASKS = 20


def _first_difference(lines, ours, theirs):
    """``(index, pltrs's exception, matplotlib's)`` of the first line
    raising differently, None if every line compared raised alike."""
    for index, (mine, reference) in enumerate(zip(ours, theirs)):
        if mine != reference:
            return index, mine, reference
    if len(ours) != len(theirs):
        index = min(len(ours), len(theirs))
        return index, ours[index] if index < len(ours) else "(ended)", \
            theirs[index] if index < len(theirs) else "(ended)"
    return None


def differences(ran, directory, jobs):
    """The differences of the programs *ran* (``(seed, lines, outcomes)``)
    from matplotlib's: ``[(seed, lines, index, pltrs's, matplotlib's)]``."""
    from .runner import Worker

    def share(programs):
        found, worker = [], None
        for count, (seed, lines, ours) in enumerate(programs):
            if worker is None or not worker.alive() or count % WORKER_TASKS == 0:
                if worker is not None:
                    worker.stop()
                worker = Worker(directory, reference=True)
            _, bugs = worker.run({"id": seed, "lines": lines})
            theirs = list(worker.outcomes)
            if bugs:
                theirs += [f"({bugs[0].kind})"]
            difference = _first_difference(lines, ours, theirs)
            if difference is not None:
                found.append((seed, lines, *difference))
        if worker is not None:
            worker.stop()
        return found

    shares = [ran[i::jobs] for i in range(jobs)]
    with ThreadPoolExecutor(jobs) as pool:
        return [found for part in pool.map(share, shares) for found in part]


def write(found, directory):
    """Writes the differences *found*, grouped by exception, to
    ``differences.md`` under *directory*, and each one, with the line it
    diverged at, to ``differences.json``."""
    groups = {}
    for seed, lines, index, ours, theirs in found:
        groups.setdefault((str(ours), str(theirs)), []).append((seed, lines, index))
    report = ["# Lines raising otherwise under pltrs than under matplotlib", "",
              "| pltrs | matplotlib | programs | first seed |", "|---|---|---|---|"]
    for (ours, theirs), programs in sorted(groups.items(), key=lambda item: -len(item[1])):
        report.append(f"| {ours} | {theirs} | {len(programs)} | {programs[0][0]} |")
    for (ours, theirs), programs in sorted(groups.items(), key=lambda item: -len(item[1])):
        seed, lines, index = programs[0]
        report += ["", f"## {ours} under pltrs, {theirs} under matplotlib (seed {seed})", "",
                   "```python", *lines[:index + 1], "```"]
    (directory / "differences.md").write_text("\n".join(report) + "\n", encoding="utf-8")
    listed = [{"seed": seed, "line": lines[index] if index < len(lines) else None,
               "index": index, "pltrs": str(ours), "matplotlib": str(theirs)}
              for seed, lines, index, ours, theirs in found]
    (directory / "differences.json").write_text(json.dumps(listed, indent=1) + "\n",
                                                encoding="utf-8")
