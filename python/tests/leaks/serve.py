"""
A long-running process: a server answering requests for figures, as a
web service drawing per request does. Each request makes a figure of a
random kind (a scene of `scenes.PLOTS`, `scenes.PLOTS_3D` or
`scenes.DECORATIONS`), saves it as PNG to bytes and closes it.

`compare` runs it under pltrs and under matplotlib and tabulates the
resident memory after 500, 1,000, 2,500 and 5,000 requests (or the
quarters of fewer) and the time per request over the first and the
last hundred. ``python -m leaks.serve N`` runs it in this process and
prints its JSON record.
"""

import io
import json
import random
import subprocess
import sys
import time

from . import probe, runner
from . import scenes as catalogue

#: The requests after which memory is measured, for a run of 5,000.
CHECKPOINTS = (500, 1000, 2500, 5000)

#: How many requests the start and the end are timed over.
TIMED = 100


def _kinds():
    every = {**catalogue.PLOTS, **catalogue.PLOTS_3D, **catalogue.DECORATIONS}
    return {name: scene for name, scene in sorted(every.items())
            if not scene.usetex and not scene.kept and name not in catalogue.KNOWN}


def checkpoints(requests):
    """The requests after which memory is measured."""
    if requests >= CHECKPOINTS[-1]:
        return CHECKPOINTS
    return tuple(max(1, requests * share // 4) for share in (1, 2, 3, 4))


def serve(requests, seed=0):
    """Answers *requests* requests; the record of memory and time."""
    import pltrs
    pltrs.use("agg")
    import pltrs.pyplot as plt
    import warnings
    warnings.simplefilter("ignore")
    kinds = _kinds()
    names = list(kinds)
    rng = random.Random(seed)
    base = probe._base_namespace()
    marks = set(checkpoints(requests))
    memory, times, sizes = {}, [], 0
    start = probe.measure()
    for request in range(1, requests + 1):
        began = time.perf_counter()
        namespace = dict(base)
        probe._run(kinds[rng.choice(names)].make, namespace)
        output = io.BytesIO()
        plt.gcf().savefig(output, format="png")
        sizes += output.tell()
        plt.close("all")
        probe._forget(namespace)
        del namespace
        times.append(time.perf_counter() - began)
        if request in marks:
            memory[request] = probe.measure()
    return {"start": start, "memory": memory,
            "first": sum(times[:TIMED]) / min(TIMED, len(times)),
            "last": sum(times[-TIMED:]) / min(TIMED, len(times)),
            "bytes": sizes}


def _run_under(requests, work, reference):
    work.mkdir(parents=True, exist_ok=True)
    command = [runner._interpreter(reference), "-m", "leaks.serve", str(requests)]
    done = subprocess.run(command, capture_output=True, text=True, cwd=work,
                          env=runner._environment(work, reference), check=False)
    lines = [line for line in done.stdout.splitlines() if line.startswith("{")]
    if not lines:
        raise RuntimeError(f"the server failed: {done.stderr[-2000:]}")
    return json.loads(lines[-1])


def compare(requests, work):
    """Runs the server under pltrs and matplotlib; a Markdown table of
    both."""
    records = {"pltrs": _run_under(requests, work / "pltrs", False),
               "matplotlib": _run_under(requests, work / "matplotlib", True)}
    (work / "serve.json").write_text(json.dumps(records, indent=1), "utf-8")
    lines = ["| | " + " | ".join(f"after {count:,}" for count in checkpoints(requests))
             + " | ms/request first 100 | ms/request last 100 |",
             "|---|" + "---|" * (len(checkpoints(requests)) + 2)]
    for name, found in records.items():
        cells = [f"{found['memory'][str(count)]['resident'] / 2 ** 20:,.1f} MiB"
                 for count in checkpoints(requests)]
        cells += [f"{found['first'] * 1000:.1f}", f"{found['last'] * 1000:.1f}"]
        lines.append(f"| {name} (resident) | " + " | ".join(cells) + " |")
        heap = [f"{found['memory'][str(count)]['heap'] / 2 ** 20:,.1f} MiB"
                for count in checkpoints(requests)]
        lines.append(f"| {name} (heap) | " + " | ".join(heap) + " | | |")
    return "\n".join(lines)


if __name__ == "__main__":
    print(json.dumps(serve(int(sys.argv[1]))))
