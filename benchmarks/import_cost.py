"""What importing pltrs (Python) and matplotlib costs, as one table.

For ``import PACKAGE`` and ``import PACKAGE.pyplot`` of each side (by
default pltrs from this tree in ``.venv-pltrs`` and matplotlib in
``.venv``), fresh interpreters measure:

* the modules loaded: all of them, the package's own (its Python modules
  and, apart, the native submodules pltrs registers) and the rest;
* ``-X importtime``'s self times summed per top-level package;
* the process' user CPU time, and its user and system time together
  (the kernel splits them by sampling; the sum is exact);
* its peak and resident memory after the import (``VmHWM``/``VmRSS`` on
  Linux, the peak and current working set on Windows, ``ru_maxrss`` and
  the resident size on macOS) and its page faults;
* the files Python opened (``open`` audit events: sources, bytecode, data
  files; files the extension opens itself are not counted) and the bytes
  read by any ``read`` call (``rchar`` on Linux, the I/O counters on
  Windows; not measured on macOS).

Times and memory are the minimum over ``--runs`` runs (30 by default) of
each, which a busy machine blurs least; runs are interleaved across the
sides so that drifting load treats them alike. One untimed run per side
first warms the caches (bytecode, font lists) in a configuration
directory of the side's own under ``--out``.

Run it with any Python 3.12 or later, from the repository root or
elsewhere::

    python benchmarks/import_cost.py
    python benchmarks/import_cost.py --side main pltrs .venv-pltrs/bin/python python \\
        --side old pltrs .venv-pltrs/bin/python target/old/python \\
        --side matplotlib matplotlib .venv/bin/python

A side is ``LABEL PACKAGE PYTHON [PYTHONPATH]``. ``--modules`` also prints
each import's own modules, ``--json FILE`` writes every measurement.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Run in each fresh interpreter: import the module named by argv[1] and
# print, as JSON, what the process measured of itself.
PROBE = r"""
import os, sys
opened = set()
if sys.argv[2] == "audit":
    def hook(event, args):
        if event == "open" and args[0] is not None and not isinstance(args[0], int):
            opened.add(os.fsdecode(args[0]))
    sys.addaudithook(hook)
__import__(sys.argv[1])
times = os.times()
result = {"user": times.user, "system": times.system, "opened": len(opened)}
# Memory first, before the probe's own imports below add to it.
if sys.platform == "linux":
    with open("/proc/self/status") as status:
        for line in status:
            key, _, value = line.partition(":")
            if key in ("VmHWM", "VmRSS"):
                result[key] = int(value.split()[0]) * 1024
    with open("/proc/self/io") as io:
        result["read"] = int(io.read().partition("rchar: ")[2].split()[0])
result["modules"] = sorted(sys.modules)
result["native"] = sorted(
    name for name, module in list(sys.modules.items())
    if not hasattr(module, "__path__")
    and not isinstance(getattr(module, "__file__", None), str))
if sys.platform != "win32":
    import resource
    usage = resource.getrusage(resource.RUSAGE_SELF)
    result.update(user=usage.ru_utime, system=usage.ru_stime,
                  faults=usage.ru_minflt + usage.ru_majflt)
if sys.platform == "win32":
    import ctypes
    from ctypes import wintypes

    class Memory(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
            (name, ctypes.c_size_t) for name in (
                "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
                "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage",
                "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage")]

    class Io(ctypes.Structure):
        _fields_ = [(name, ctypes.c_ulonglong) for name in (
            "ReadOperationCount", "WriteOperationCount", "OtherOperationCount",
            "ReadTransferCount", "WriteTransferCount", "OtherTransferCount")]

    kernel32 = ctypes.WinDLL("kernel32")
    kernel32.GetCurrentProcess.restype = wintypes.HANDLE
    process = kernel32.GetCurrentProcess()
    memory = Memory(cb=ctypes.sizeof(Memory))
    ctypes.WinDLL("psapi").GetProcessMemoryInfo(
        process, ctypes.byref(memory), memory.cb)
    io = Io()
    kernel32.GetProcessIoCounters(process, ctypes.byref(io))
    result.update(VmHWM=memory.PeakWorkingSetSize, VmRSS=memory.WorkingSetSize,
                  faults=memory.PageFaultCount, read=io.ReadTransferCount)
elif sys.platform == "darwin":
    import ctypes
    result["VmHWM"] = usage.ru_maxrss  # Bytes on macOS.

    class Info(ctypes.Structure):  # struct rusage_info_v2
        _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [
            (name, ctypes.c_uint64) for name in (
                "user_time", "system_time", "pkg_idle_wkups", "interrupt_wkups",
                "pageins", "wired_size", "resident_size", "phys_footprint",
                "proc_start_abstime", "proc_exit_abstime", "child_user_time",
                "child_system_time", "child_pkg_idle_wkups", "child_interrupt_wkups",
                "child_pageins", "child_elapsed_abstime", "diskio_bytesread",
                "diskio_byteswritten")]

    info = Info()
    if ctypes.CDLL(None).proc_pid_rusage(os.getpid(), 2, ctypes.byref(info)) == 0:
        result["VmRSS"] = info.resident_size
import json
print(json.dumps(result))
"""

# The environment of `cargo xtask bench`'s Python processes: Agg,
# matplotlib's look, and numpy's BLAS on one thread (OpenBLAS otherwise
# starts a thread per core at import, whose spinning counts as user time).
BENCH_ENV = {"MPLBACKEND": "agg", "PLTRS_LOOK": "matplotlib", "OPENBLAS_NUM_THREADS": "1",
             "OMP_NUM_THREADS": "1", "MKL_NUM_THREADS": "1"}

# The packages whose import times get a row of their own; the rest are
# summed as "other" (mostly the standard library).
NAMED_PACKAGES = ("numpy", "PIL", "dateutil", "cycler", "pyparsing", "kiwisolver",
                  "contourpy", "fontTools", "packaging")


class Side:
    """One implementation to measure: a package under an interpreter."""

    def __init__(self, label, package, python, path, out):
        self.label = label
        self.package = package
        self.python = str(python)
        self.env = dict(os.environ, **BENCH_ENV)
        self.env.pop("PYTHONPATH", None)
        if path:
            self.env["PYTHONPATH"] = str(Path(path).resolve())
        config = out / "config" / label
        config.mkdir(parents=True, exist_ok=True)
        # Each side keeps its caches (font lists, docstring tables) apart.
        self.env["MPLCONFIGDIR"] = self.env["PLTRSCONFIGDIR"] = str(config)

    def own(self, module):
        """Whether *module* is the side's own (the toolkits included)."""
        top = module.partition(".")[0]
        if self.package == "matplotlib":
            return top in ("matplotlib", "mpl_toolkits", "pylab")
        return top == self.package

    def run(self, statement_module, *, mode, importtime=False):
        """One fresh interpreter importing *statement_module*: its probe
        result, and the ``-X importtime`` self times when asked."""
        command = [self.python]
        if importtime:
            command += ["-X", "importtime"]
        command += ["-c", PROBE, statement_module, mode]
        done = subprocess.run(command, env=self.env, capture_output=True, text=True,
                              check=False)
        if done.returncode != 0:
            raise SystemExit(f"{self.label}: `import {statement_module}` failed:\n"
                             f"{done.stderr}")
        result = json.loads(done.stdout.splitlines()[-1])
        if importtime:
            result["importtime"] = self.self_times(done.stderr)
        return result

    def self_times(self, stderr):
        """``-X importtime``'s self times in microseconds, summed per
        package row (the side's own package as ``own``)."""
        sums = {}
        for line in stderr.splitlines():
            if not line.startswith("import time:") or "self [us]" in line:
                continue
            fields = line[len("import time:"):].split("|")
            module = fields[2].strip()
            top = module.partition(".")[0]
            row = "own" if self.own(module) else top if top in NAMED_PACKAGES else "other"
            sums[row] = sums.get(row, 0) + int(fields[0])
        return sums


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--side", nargs="+", action="append", metavar="ARG",
                        help="LABEL PACKAGE PYTHON [PYTHONPATH]")
    parser.add_argument("--runs", type=int, default=30)
    parser.add_argument("--out", type=Path, default=ROOT / "target" / "import-cost",
                        help="directory of the sides' configuration (default %(default)s)")
    parser.add_argument("--modules", action="store_true",
                        help="also list each import's own modules")
    parser.add_argument("--json", type=Path, help="write every measurement here")
    return parser.parse_args()


def venv_python(venv):
    """The interpreter of the venv *venv* under the repository root."""
    if sys.platform == "win32":
        return ROOT / venv / "Scripts" / "python.exe"
    return ROOT / venv / "bin" / "python"


def default_sides():
    return [["pltrs", "pltrs", venv_python(".venv-pltrs"), ROOT / "python"],
            ["matplotlib", "matplotlib", venv_python(".venv")]]


def measure(sides, runs):
    """Every side's measurements, ``{(label, statement): {...}}``."""
    results = {}
    for side in sides:
        for module in (side.package, f"{side.package}.pyplot"):
            side.run(module, mode="warm")  # Bytecode and caches written.
            listed = side.run(module, mode="audit")
            results[side.label, module] = {"probe": listed, "timed": [], "importtime": []}
    for _ in range(runs):
        for side in sides:
            for module in (side.package, f"{side.package}.pyplot"):
                entry = results[side.label, module]
                entry["timed"].append(side.run(module, mode="plain"))
                entry["importtime"].append(side.run(module, mode="plain",
                                                    importtime=True)["importtime"])
    return results


def summarise(side, entry):
    """The table's rows for one import of one side, and its own modules."""
    probe = entry["probe"]
    modules = probe["modules"]
    own = [name for name in modules if side.own(name)]
    native = [name for name in own if name in probe["native"]]
    rows = {
        "modules loaded": len(modules),
        "  own, Python": len(own) - len(native),
        "  own, native": len(native),
        "  other": len(modules) - len(own),
    }
    for row in ["own", *NAMED_PACKAGES, "other"]:
        if any(row in times for times in entry["importtime"]):
            rows[f"importtime self, ms: {row}"] = min(
                times.get(row, 0) for times in entry["importtime"]) / 1000
    rows["importtime self, ms: total"] = min(
        sum(times.values()) for times in entry["importtime"]) / 1000
    timed = entry["timed"]
    rows["user CPU, ms"] = min(run["user"] for run in timed) * 1000
    # The kernel splits a process' time between user and system by
    # sampling; their sum is exact.
    rows["user + system CPU, ms"] = min(run["user"] + run["system"] for run in timed) * 1000
    mib = 1024 * 1024
    for key, label in (("VmHWM", "peak memory, MiB"), ("VmRSS", "resident memory, MiB")):
        if key in timed[0]:
            rows[label] = min(run[key] for run in timed) / mib
    if "faults" in timed[0]:
        rows["page faults"] = min(run["faults"] for run in timed)
    rows["files opened by Python"] = probe["opened"]
    if "read" in timed[0]:
        rows["bytes read, MiB"] = min(run["read"] for run in timed) / mib
    return rows, own


def table(sides, results):
    """The measurements as one aligned table, a column per import."""
    columns = [(side, side.package + kind) for kind in ("", ".pyplot") for side in sides]
    summaries = {(side.label, module): summarise(side, results[side.label, module])
                 for side, module in columns}
    labels = []
    for rows, _ in summaries.values():
        labels += [label for label in rows if label not in labels]
    # The module counts, then the import times by package, then the rest.
    rank = ["own", *NAMED_PACKAGES, "other", "total"]
    first = {label: index for index, label in enumerate(labels)}

    def key(label):
        if label.startswith("importtime"):
            return 1, rank.index(label.rpartition(": ")[2])
        return (0 if "modules" in label or label.startswith("  ") else 2), first[label]

    labels.sort(key=key)
    heads = [f"{side.label}: {module}" for side, module in columns]
    widths = [max(len(head), 9) for head in heads]
    width = max(map(len, labels)) + 2
    lines = [" " * width + " | ".join(f"{head:>{w}}" for head, w in zip(heads, widths))]
    for label in labels:
        cells = []
        for (side, module), cell_width in zip(columns, widths):
            value = summaries[side.label, module][0].get(label)
            text = "-" if value is None else (
                f"{value:.1f}" if isinstance(value, float) else str(value))
            cells.append(f"{text:>{cell_width}}")
        lines.append(f"{label:<{width}}" + " | ".join(cells))
    return "\n".join(lines), summaries


def main():
    args = parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    sides = [Side(spec[0], spec[1], spec[2], spec[3] if len(spec) > 3 else None, args.out)
             for spec in (args.side or default_sides())]
    results = measure(sides, args.runs)
    text, summaries = table(sides, results)
    print(text)
    if args.modules:
        for (label, module), (_, own) in summaries.items():
            print(f"\n{label}: import {module}: {len(own)} own modules")
            print("  " + " ".join(own))
    if args.json:
        args.json.write_text(json.dumps(
            {f"{label}: import {module}": entry for (label, module), entry in results.items()},
            indent=1))


if __name__ == "__main__":
    main()
