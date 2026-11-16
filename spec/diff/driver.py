"""The differential tester: programs run under matplotlib and pltrs, compared.

    python driver.py [--suite gallery|generated|all|corpus] [PATTERN...] [--jobs N]
                     [--failures] [--reduce] [--baseline REPORT]

``cargo xtask diff`` runs this in the reference venv (after building
pltrs (Python)'s extension), with ``--pltrs-python`` naming the pltrs
venv's interpreter (and, for the corpora, ``--mpl-python`` the
interpreter of a reference venv of their own, which holds the packages
their programs import). It

1. collects the programs: matplotlib's gallery (``--gallery``, the
   sdist's ``galleries/``) and the generated programs (``generate.py``
   writes them under ``OUT/programs``), or (``--suite corpus``) the
   programs of the corpora of ``corpora.toml``, which ``corpora.py``
   fetches into ``OUT/corpora`` (the corpora a PATTERN names, or all);
2. runs each under both implementations (``trace.py``), the reference
   runs cached by the program's text (``OUT/runs/<program>/mpl``), every
   program offline, and a corpus' programs each with a home directory of
   its own, empty but for the directories a desktop's home has
   (``HOME_DIRECTORIES``, in ``OUT/runs/<program>/home``), so that what they would
   keep in a cache there stays in ``OUT`` and reaches no other program,
   and a configuration directory of their own (`Setup.seed_config`: the
   font cache a program may add fonts to is a copy), and their linear
   algebra on one thread;
3. compares them (``compare.py``), drops the differences the allowlist
   (``allowlist.txt``) proves intended, and groups the rest by cause;
4. writes ``OUT/report.json`` (every difference) and ``OUT/report.md``
   (the groups, the biggest first, each with its smallest reproduction,
   the programs of the gallery and the corpora that fail, with their
   causes, and each corpus' programs and clean ones), and prints the
   groups' counts; ``--baseline`` adds the counts of an earlier report.

Every program runs offline, as the ecosystem suites run (``OFFLINE``):
a program asking the network for something (the gallery's timeline,
GitHub's list of matplotlib's releases) takes the same fallback under
both implementations, in every run, and a corpus' program reading its
data from the web (seaborn's datasets, CSV files) fails under both, so is
skipped. Every program runs headless too: on Linux the tester has no
display, and on macOS and Windows, which always have the user's screen,
``headless/sitecustomize.py`` makes the GUI toolkits, matplotlib's
interactive backends and the web browser fail as they fail there.

A program matplotlib itself cannot run here (a GUI toolkit, a network
download) is listed as skipped, with matplotlib's error. ``--reduce``
reduces the first difference of each group to the fewest statements that
still show it (``reduce.py``). PATTERN keeps the programs whose names
contain it.
"""

from __future__ import annotations

import argparse
import ast
import concurrent.futures
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare  # noqa: E402
import corpora  # noqa: E402
import install  # noqa: E402

#: Seconds a program may run under one implementation.
TIMEOUT = 300
#: The directories a corpus program's home has, as a desktop's has them
#: (ultraplot makes its own in ``~/.config`` when imported).
HOME_DIRECTORIES = (".config", ".cache", ".local/share")
#: The variables holding numpy and its BLAS to another processor's code.
PROCESSOR_LEVEL = ("NPY_DISABLE_CPU_FEATURES", "OPENBLAS_CORETYPE")
#: The files of an implementation's configuration directory a corpus
#: program gets copies of (the rest it shares, linked): matplotlib's font
#: cache, which a program may write its own fonts into (ultraplot does),
#: to reach no other program.
OWN_CONFIG_FILES = "fontlist-v*.json"
#: The longest name of a program's run directory on Windows (see
#: `Program.directory`).
WINDOWS_NAME = 40
#: Gallery directories run, under the sdist's ``galleries/``.
GALLERY_PARTS = ("examples", "plot_types", "tutorials", "users_explain")
#: The pinned list of the corpora's venvs, on which their reference runs
#: depend.
CORPUS_REQUIREMENTS = HERE.parents[1] / "python" / "requirements-corpora.txt"
#: The proxy refusing every connection: nothing listens on the discard
#: port of the loopback interface.
REFUSING_PROXY = "http://127.0.0.1:9"
#: The variables running a program offline: HTTP and HTTPS through
#: `REFUSING_PROXY`, which urllib, requests and pooch heed, the loopback
#: interface spared (``cargo xtask ecosystem suite``'s).
OFFLINE = {
    **{name: REFUSING_PROXY for name in ("http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY")},
    **{name: "localhost,127.0.0.1,::1" for name in ("no_proxy", "NO_PROXY")},
}
#: The directory of the ``sitecustomize`` keeping the programs off the
#: screen (``headless/sitecustomize.py``), put first on their
#: ``PYTHONPATH`` where a GUI toolkit always finds one (`NEEDS_HEADLESS`).
HEADLESS = HERE / "headless"
#: Whether the programs need `HEADLESS`: on every system but Linux, where
#: the tester runs without a display.
NEEDS_HEADLESS = not sys.platform.startswith("linux")


def _home_variables(home: pathlib.Path) -> dict[str, str]:
    """The variables making *home* a run's home directory: ``HOME``, and
    on Windows ``USERPROFILE``, where Python finds ``~`` there."""
    names = ("HOME", "USERPROFILE") if os.name == "nt" else ("HOME",)
    return dict.fromkeys(names, str(home))


class Setup:
    """Where things are, and how each implementation runs."""

    def __init__(self, args: argparse.Namespace) -> None:
        self.root = pathlib.Path(args.root).resolve()
        self.out = pathlib.Path(args.out).resolve()
        self.gallery = pathlib.Path(args.gallery).resolve()
        self.pltrs_python = args.pltrs_python
        #: The ``site-packages`` pltrs is installed in for a corpus run.
        self.installed: pathlib.Path | None = None
        self.mpl_python = args.mpl_python or sys.executable
        #: The variables of each corpus' manifest table (`corpora.environment`).
        self._corpus_environments: dict[str, dict[str, str]] = {}

    def command(self, impl: str, program: Program, out: pathlib.Path) -> list[str]:
        """The command tracing *program* under *impl* into *out* (a corpus'
        program in the copy of its tree laid out there)."""
        python = self.mpl_python if impl == "matplotlib" else self.pltrs_python
        command = [python, str(HERE / "trace.py"), "--impl", impl, "--mode", program.mode]
        if program.corpus is None:
            return [*command, str(program.path), str(out)]
        if program.corpus.corpus.shell:
            command += ["--shell", program.corpus.corpus.shell]
        tree = out / "tree"
        return [*command, "--in-tree", str(tree), str(program.corpus.lay_out(tree)), str(out)]

    def environment(self, impl: str, program: Program | None = None) -> dict[str, str]:
        """The environment of a run of *program* under *impl*: offline
        (`OFFLINE`, whatever proxy the tester's own environment names), and
        for a corpus' program with its home directory (`home`) and the
        variables of its corpus' table (`corpora.environment`)."""
        env = {k: v for k, v in os.environ.items()
               if not k.startswith(("MPL", "PLTRS", "PYTHON")) and k != "CARGO_TARGET_DIR"
               and k.lower() not in ("http_proxy", "https_proxy", "all_proxy", "no_proxy")}
        env.update(OFFLINE)
        if program is not None and program.corpus is not None:
            env.update(_home_variables(self.home(program)))
            # One thread for the linear algebra, whose sums several threads
            # add in an order of their own from run to run.
            env.update(dict.fromkeys(("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"), "1"))
        env["MPLBACKEND"] = "agg"
        # (Named apart from the package names `trace.py` replaces in messages.)
        env["MPLCONFIGDIR"] = str(self.config(impl, program))
        env["PYTHONHASHSEED"] = "0"
        path = [str(HEADLESS)] if NEEDS_HEADLESS else []
        if program is not None and program.corpus is not None:
            own = dict(self._corpus_environments.get(program.corpus.corpus.name, {}))
            path += [part for part in own.pop("PYTHONPATH", "").split(os.pathsep) if part]
            env.update(own)
        if impl == "pltrs":
            env["PLTRS_LOOK"] = "matplotlib"
            # pltrs, and the `matplotlib` distribution's aliases of it, so
            # that the libraries a program plots through (pandas) get pltrs
            # where they import matplotlib, as they do where it is installed
            # as matplotlib: for a corpus' program installed in its venv
            # (`install.py`), for the others from the tree.
            if program is None or program.corpus is None:
                path += [str(self.root / "python" / part) for part in ("", "alias")]
        if path:
            env["PYTHONPATH"] = os.pathsep.join(path)
        return env

    def prepare_environments(self, programs: list[Program]) -> None:
        """Resolve the variables of the corpora of *programs* (fetching
        the corpora they name) before the programs run side by side."""
        for program in programs:
            corpus = program.corpus.corpus if program.corpus else None
            if corpus is not None and corpus.name not in self._corpus_environments:
                self._corpus_environments[corpus.name] = corpora.environment(corpus, self.out)

    def config(self, impl: str, program: Program | None = None) -> pathlib.Path:
        """The configuration directory of *impl*'s runs: the one they
        share, or, for a corpus' program, its own (`seed_config`).
        (Named apart from the package names `trace.py` replaces in
        messages.)"""
        name = "config-reference" if impl == "matplotlib" else "config-candidate"
        if program is not None and program.corpus is not None:
            return program.directory(self) / name
        return self.out / name

    def seed_config(self, impl: str, program: Program) -> None:
        """Give *program* a configuration directory of its own for *impl*:
        copies of the shared one's `OWN_CONFIG_FILES`, links to the rest."""
        shared, own = self.config(impl), self.config(impl, program)
        shutil.rmtree(own, ignore_errors=True)
        own.mkdir(parents=True)
        copied = set(shared.glob(OWN_CONFIG_FILES))
        for entry in shared.iterdir():
            if entry in copied:
                shutil.copy2(entry, own / entry.name)
            else:
                corpora.link(entry, own / entry.name)

    def prepare_configs(self) -> None:
        """Make the shared configuration directories a corpus run seeds
        the programs' from: without a font cache a program wrote (one that
        added fonts to it, as ultraplot does), matplotlib's made afresh as
        a program run with an empty home makes it."""
        seed_home = self.out / "config-seed-home"
        shutil.rmtree(seed_home, ignore_errors=True)
        seed_home.mkdir(parents=True)
        for impl in ("matplotlib", "pltrs"):
            shared = self.config(impl)
            shared.mkdir(parents=True, exist_ok=True)
            for cache in shared.glob(OWN_CONFIG_FILES):
                cache.unlink()
        env = dict(self.environment("matplotlib"), **_home_variables(seed_home))
        subprocess.run([self.mpl_python, "-c", "import matplotlib.font_manager"],
                       env=env, check=True, capture_output=True)

    def home(self, program: Program) -> pathlib.Path:
        """The home directory of *program*'s runs, a corpus' program's:
        its own, which `_run` empties before each, so that what a program
        keeps there (a cache, a data directory) reaches neither the
        programs running beside it, racing to make it, nor those after it,
        and both implementations start from the same one."""
        return program.directory(self) / "home"


class Program:
    """One program of the run."""

    def __init__(self, name: str, path: pathlib.Path, mode: str,
                 corpus: corpora.CorpusProgram | None = None) -> None:
        self.name = name
        self.path = path
        self.mode = mode
        self.corpus = corpus

    @classmethod
    def of_corpus(cls, program: corpora.CorpusProgram) -> Program:
        """A program of a corpus."""
        return cls(program.name, program.path, program.mode, program)

    @property
    def digest(self) -> str:
        """What the reference run depends on: the program, its mode, its
        corpus' tree and venvs, the tester's harness (`HEADLESS` too where
        the programs get it), the network it is refused (`OFFLINE`) and
        the processor level numpy is held to (`PROCESSOR_LEVEL`)."""
        harness = b"".join((HERE / name).read_bytes()
                           for name in ("trace.py", "describe.py", "corpora.py", "ipython_shell.py"))
        if NEEDS_HEADLESS:
            harness += (HEADLESS / "sitecustomize.py").read_bytes()
        tree = (self.corpus.corpus.sha256.encode() + CORPUS_REQUIREMENTS.read_bytes()
                + json.dumps(self.corpus.corpus.environment).encode()) if self.corpus else b""
        network = json.dumps(OFFLINE).encode()
        # The copy of its loops numpy runs, and its BLAS' kernels, where a
        # run is held to another processor level (`cargo xtask ... --level`).
        network += "".join(os.environ.get(name, "") for name in PROCESSOR_LEVEL).encode()
        if self.corpus:
            # What a corpus program starts with, besides its tree.
            network += json.dumps([HOME_DIRECTORIES, OWN_CONFIG_FILES]).encode()
        shell = self.corpus.corpus.shell.encode() if self.corpus else b""
        return hashlib.sha256(self.path.read_bytes() + self.mode.encode() + shell + tree + harness
                              + network).hexdigest()[:16]

    def directory(self, setup: Setup) -> pathlib.Path:
        """Where the runs of the program go: a directory named after the
        program, on Windows shortened past `WINDOWS_NAME` characters (to
        its start and a hash of the whole), as a corpus program's tree is
        copied below it and Windows' paths hold 259 characters at most."""
        name = re.sub(r"[^\w.-]", "_", self.name)
        if os.name == "nt" and len(name) > WINDOWS_NAME:
            digest = hashlib.sha256(name.encode()).hexdigest()[:12]
            name = f"{name[:WINDOWS_NAME - len(digest) - 1]}-{digest}"
        return setup.out / "runs" / name


def gallery_programs(gallery: pathlib.Path) -> list[Program]:
    """matplotlib's gallery, as programs run whole."""
    programs = []
    for part in GALLERY_PARTS:
        for path in sorted((gallery / part).rglob("*.py")):
            name = "gallery/" + path.relative_to(gallery).as_posix().removesuffix(".py")
            programs.append(Program(name, path, "program"))
    return programs


def generated_programs(setup: Setup) -> list[Program]:
    """The programs `generate.py` writes, run statement by statement."""
    target = setup.out / "programs"
    subprocess.run([setup.mpl_python, str(HERE / "generate.py"), str(target)], check=True,
                   env=setup.environment("matplotlib"))
    return [
        Program("generated/" + path.relative_to(target).as_posix().removesuffix(".py"), path, "steps")
        for path in sorted(target.rglob("*.py"))
    ]


def corpus_programs(setup: Setup, patterns: list[str]) -> list[Program]:
    """The programs of the corpora: of those a pattern names, or of all."""
    every = corpora.load()
    named = [corpus for corpus in every if any(corpus.name in pattern for pattern in patterns)]
    programs = []
    for corpus in named or every:
        try:
            programs += [Program.of_corpus(program) for program in corpora.programs(corpus, setup.out)]
        except corpora.CorpusUnfit as err:
            print(f"left out: {str(err).splitlines()[0]}", file=sys.stderr)
    return programs


def _run(setup: Setup, impl: str, program: Program, out: pathlib.Path) -> dict[str, Any] | None:
    """The record of one run, or None with the reason in ``out/failure``."""
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    if program.corpus is not None:
        home = setup.home(program)
        shutil.rmtree(home, ignore_errors=True)
        for directory in HOME_DIRECTORIES:
            (home / directory).mkdir(parents=True)
        setup.seed_config(impl, program)
    # A temporary directory of the run's own: the tracer seeds `random`, so
    # `tempfile` names repeat from run to run, and what a crashed program
    # left in a shared one makes every name collide. Records read it as the
    # shared one (`_records_shared_tmp`), as the cached reference runs do.
    tmp = out / "tmp"
    tmp.mkdir()
    env = setup.environment(impl, program) | {"TMPDIR": str(tmp)}
    try:
        done = subprocess.run(setup.command(impl, program, out), env=env,
                              capture_output=True, text=True, errors="replace", timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        (out / "failure").write_text(f"timed out after {TIMEOUT} s", encoding="utf-8")
        return None
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    (out / "stderr.txt").write_text(done.stderr[-20000:], encoding="utf-8")
    record = out / "record.json"
    if done.returncode != 0 or not record.exists():
        tail = done.stderr.strip().splitlines()[-1:] or [f"exit status {done.returncode}"]
        (out / "failure").write_text(f"exit status {done.returncode}: {tail[0]}", encoding="utf-8")
        return None
    return json.loads(_records_shared_tmp(record.read_text(), tmp))


def _records_shared_tmp(text: str, tmp: pathlib.Path) -> str:
    """The record *text* of a run given the temporary directory *tmp*, as
    if it had the shared one (`tempfile.gettempdir`), JSON's escapes
    included: the random names in it read ``<tmp>`` (as ``describe.py``
    has those of the shared one), the rest the shared directory. The
    tracer names pltrs' package matplotlib's in what it records, the run's
    directory in the path too. Paths are found as written and as a
    ``repr`` writes them (Windows' backslashes doubled, as in a warning
    naming a ``TemporaryDirectory``)."""
    def escaped(path: str) -> str:
        return json.dumps(path)[1:-1]

    def as_repr(path: str) -> str:
        return path.replace("\\", "\\\\")

    for path in {str(tmp), str(tmp).replace("pltrs", "matplotlib")}:
        for spelled in (str, as_repr):
            own = re.escape(escaped(spelled(path)))
            separator = re.escape(escaped(spelled(os.sep)))
            text = re.sub(own + separator + r"tmp[\w-]{8}", "<tmp>", text)
            shared = escaped(spelled(tempfile.gettempdir()))
            text = re.sub(own, lambda _, shared=shared: shared, text)
    return text


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
        result["skipped"] = (reference / "failure").read_text(encoding="utf-8")
        return result
    if program.mode == "program" and mpl.get("uncaught") and _environmental(mpl["uncaught"]):
        result["skipped"] = f"{mpl['uncaught']['type']}: {mpl['uncaught']['message'][:200]}"
        return result
    lacking = [error for error in mpl["errors"] if _environmental(error)]
    if program.mode == "cells" and lacking and not mpl["figures"]:
        # A notebook whose figures all need what this machine lacks (a
        # package its first cell imports, say) tests nothing.
        result["skipped"] = f"{lacking[0]['type']}: {lacking[0]['message'][:200]}"
        return result
    if setup.installed is not None and not install.alias_present(setup.installed):
        raise install.InstallError(f"the matplotlib alias left {setup.installed} during the run: "
                                   "something changed the venv under it")
    pltrs = _run(setup, "pltrs", program, directory / "pltrs")
    if pltrs is None:
        failure = (directory / "pltrs" / "failure").read_text(encoding="utf-8")
        result["differences"] = [compare.Difference(
            program.name, "exception", f"crash: harness: {compare._message_key(failure)}", "the run",
            failure).as_json()]
        return result
    result["seconds"] = {"matplotlib": mpl["seconds"], "pltrs": pltrs["seconds"]}
    result["matplotlib"] = how_it_ran(program.path, mpl)
    found = compare.differences(program.name, program.path, mpl, pltrs, reference, directory / "pltrs")
    result["differences"] = [dict(d.as_json(), program_path=str(program.path)) for d in found]
    return result


def how_it_ran(path: pathlib.Path, record: dict[str, Any]) -> dict[str, Any]:
    """How much of a program matplotlib ran, from its *record*: whether it
    ran to its end (no statement or cell stopped at an exception), whether
    it drew or wrote a figure, and the exceptions it stopped at, those of
    a failed import naming the module. A program both implementations stop
    alike at an exception compares clean though nothing after it ran: a
    cell program goes on to its next cell, and a library a shadowed module
    breaks fails the same under both (task CORP9)."""
    stops = [*record["errors"], *([record["uncaught"]] if record.get("uncaught") else [])]
    imports = _import_statements(path) if stops else {}
    raised = []
    for error in stops:
        stop = {"type": error["type"].rsplit(".", 1)[-1], "line": error.get("statement_line")}
        module = _failed_import(error, imports.get(error.get("statement_line")))
        if module is not None:
            stop["import"] = module
        raised.append(stop)
    return {"ended": not stops, "drew": bool(record["figures"] or record.get("written")), "raised": raised}


def _import_statements(path: pathlib.Path) -> dict[int, str]:
    """The top-level import statements of the program at *path*, by line,
    each with the first module it imports."""
    try:
        tree = ast.parse(path.read_text(errors="replace"))
    except (OSError, SyntaxError, ValueError):
        return {}
    found = {}
    for node in tree.body:
        if isinstance(node, ast.Import):
            found[node.lineno] = node.names[0].name
        elif isinstance(node, ast.ImportFrom) and node.module:
            found[node.lineno] = node.module
    return found


#: The module an ImportError names: ``No module named 'x'``, ``cannot
#: import name 'y' from 'x'``.
_IMPORTED = re.compile(r"(?:No module named|from) '([\w.]+)'")


def _failed_import(error: dict[str, Any], statement: str | None) -> str | None:
    """The module whose import *error* is, an exception of the ImportError
    kind or any raised by an import statement (importing *statement*'s
    module), else None."""
    kind = error["type"].rsplit(".", 1)[-1]
    if kind in ("ImportError", "ModuleNotFoundError") or "ImportError" in error.get("mro", ()):
        named = _IMPORTED.search(error["message"])
        if named:
            return named[1]
        # A library's own ImportError for an optional dependency, named
        # by its message (pywaffle's "requires Font Awesome").
        return statement or f"{kind}: {error['message'].strip()[:60]}"
    return statement


def _environmental(error: dict[str, Any]) -> bool:
    """Whether matplotlib's own run failed for want of something this
    machine lacks (a GUI toolkit, a network, a module): an exception of one
    of those kinds, or of a subclass of one (requests' ProxyError, a
    ConnectionError)."""
    kinds = {"ModuleNotFoundError", "ImportError", "URLError", "ConnectionError", "TimeoutError",
             "TclError", "OSError", "FileNotFoundError"}
    return (error["type"].rsplit(".", 1)[-1] in kinds or not kinds.isdisjoint(error.get("mro", ()))
            or "display" in error["message"].lower())


class Allowlist:
    """Differences proven intended: ``<program regex> | <cause regex>[ @
    <where regex>] | reason`` lines of ``allowlist.txt`` (the fields apart
    by `` | ``; a where regex narrows the entry to the places it matches,
    a value's key, say)."""

    def __init__(self, path: pathlib.Path) -> None:
        self.entries = []
        if path.exists():
            for line in path.read_text(encoding="utf-8").splitlines():
                if line.strip() and not line.lstrip().startswith("#"):
                    program, cause, reason = (part.strip() for part in line.split(" | ", 2))
                    cause, _, where = cause.partition(" @ ")
                    self.entries.append((re.compile(program), re.compile(cause), re.compile(where), reason))

    def allows(self, difference: dict[str, Any]) -> bool:
        """Whether the allowlist allows *difference*."""
        return any(p.search(difference["program"]) and c.search(difference["cause"])
                   and w.search(difference.get("where", ""))
                   for p, c, w, _ in self.entries)


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
                 failures: bool, baseline_corpora: dict[str, dict[str, int]] | None = None) -> str:
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
    corpus_counts = _corpus_counts(results)
    if corpus_counts:
        total = _corpus_totals(corpus_counts)
        compared = total["runs"]["ended, drew"]
        lines += [
            "", "## Corpora", "",
            f"Of the {compared['ran']} corpus programs matplotlib runs to their end, drawing or writing "
            f"a figure, {compared['clean']} are clean. A program matplotlib stops at an exception "
            "(a cell of a notebook, a failed import) compares clean when pltrs raises alike, "
            "though what follows did not run: clean of programs by how far matplotlib ran them: "
            + _runs_line(total) + f". Clean of those that ran under both: {total['clean']}/{total['ran']}.",
        ]
        if total["failed_imports"]:
            lines += ["", "Failed imports in clean programs (module: programs): " + ", ".join(
                f"{module} {n}" for module, n in total["failed_imports"].items()) + "."]
        lines += ["", "Per corpus (programs, ran under both, clean" + (", clean before" if baseline else "")
                  + "; clean/ran by how far matplotlib ran them; failed imports):", ""]
        for name, counts in corpus_counts.items():
            before = (baseline_corpora or {}).get(name, {}).get("clean")
            imports = ", ".join(f"{module} {n}" for module, n in sorted(counts["failed_imports"].items()))
            lines.append(f"* {name}: {counts['programs']}, {counts['ran']}, {counts['clean']}"
                         + (f", before {before}" if before is not None else "")
                         + f"; {_runs_line(counts)}" + (f"; imports failed: {imports}" if imports else ""))
    gallery = [r for r in ran if r["program"].startswith(("gallery/", "corpus/")) and r.get("differences")]
    if gallery:
        lines += ["", "## Gallery examples and corpus programs with differences", ""]
        for result in gallery:
            causes = Counter(d["cause"] for d in result["differences"])
            lines.append(f"* {result['program']}: " + "; ".join(
                f"{c}" + (f" ({n}x)" if n > 1 else "") for c, n in causes.most_common(6)))
    skipped = [r for r in results if "skipped" in r]
    if skipped:
        lines += ["", "## Skipped (matplotlib cannot run them here)", ""]
        lines += [f"* {r['program']}: {r['skipped']}" for r in skipped]
    report = "\n".join(lines) + "\n"
    (setup.out / "report.md").write_text(report, encoding="utf-8")
    (setup.out / "report.json").write_text(json.dumps({
        "groups": {cause: len(found) for cause, found in groups.items()},
        "corpora": corpus_counts,
        "corpora_total": _corpus_totals(corpus_counts) if corpus_counts else None,
        "results": results,
    }, indent=1))
    return report


#: How far matplotlib ran a program (`how_it_ran`): to its end or not,
#: drawing or writing a figure or not; the report's rows, the first the
#: programs that were compared in full.
RUNS = {
    "ended, drew": (True, True),
    "ended, no figure": (True, False),
    "raised, drew": (False, True),
    "raised, no figure": (False, False),
}


def _run_kind(result: dict[str, Any]) -> str | None:
    """The row of `RUNS` *result* counts in, None for a program skipped
    or one whose matplotlib run the harness lost."""
    ran = result.get("matplotlib")
    if ran is None:
        return None
    return next(name for name, kind in RUNS.items() if kind == (ran["ended"], ran["drew"]))


#: The committed counts of the last run of every corpus, which the README
#: states (`python/tests/test_readme_facts.py`).
OUTCOMES = HERE / "corpus-outcomes.json"


def write_outcomes(results: list[dict[str, Any]]) -> None:
    """Write `OUTCOMES`: per corpus and in all, the programs, those that
    ran under both, the clean ones, and the rows of `RUNS`."""
    counts = _corpus_counts(results)
    OUTCOMES.write_text(json.dumps({
        "total": _corpus_totals(counts),
        "corpora": counts,
    }, indent=1, sort_keys=True) + "\n")


def _corpus_counts(results: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    """Per corpus: its programs, those that ran under both, the clean ones;
    of those that ran, how many (and how many clean) matplotlib ran to
    their end or not and drew in or not (`RUNS`); and the modules whose
    import failed in clean programs, with how many programs each."""
    counts: dict[str, dict[str, Any]] = {}
    for result in results:
        if result["program"].startswith("corpus/"):
            entry = counts.setdefault(result["program"].split("/")[1], {
                "programs": 0, "ran": 0, "clean": 0,
                "runs": {name: {"ran": 0, "clean": 0} for name in RUNS}, "failed_imports": {}})
            entry["programs"] += 1
            if "skipped" in result:
                continue
            clean = not result.get("differences")
            entry["ran"] += 1
            entry["clean"] += clean
            kind = _run_kind(result)
            if kind is not None:
                entry["runs"][kind]["ran"] += 1
                entry["runs"][kind]["clean"] += clean
            if clean:
                imports = {stop["import"] for stop in result.get("matplotlib", {}).get("raised", ())
                           if "import" in stop}
                for module in imports:
                    entry["failed_imports"][module] = entry["failed_imports"].get(module, 0) + 1
    return counts


def _corpus_totals(counts: dict[str, dict[str, Any]]) -> dict[str, Any]:
    """The sums of `_corpus_counts` over the corpora."""
    total: dict[str, Any] = {"programs": 0, "ran": 0, "clean": 0,
                             "runs": {name: {"ran": 0, "clean": 0} for name in RUNS},
                             "failed_imports": Counter()}
    for entry in counts.values():
        for key in ("programs", "ran", "clean"):
            total[key] += entry[key]
        for name, row in entry["runs"].items():
            for key in ("ran", "clean"):
                total["runs"][name][key] += row[key]
        total["failed_imports"].update(entry["failed_imports"])
    total["failed_imports"] = dict(total["failed_imports"].most_common())
    return total


def _runs_line(entry: dict[str, Any]) -> str:
    """The rows of `RUNS` of a count: clean of programs, each."""
    return "; ".join(f"{name} {row['clean']}/{row['ran']}" for name, row in entry["runs"].items())


def main() -> int:
    """Run the programs, compare them and report."""
    parser = argparse.ArgumentParser()
    parser.add_argument("patterns", nargs="*")
    parser.add_argument("--suite", choices=["gallery", "generated", "all", "corpus"], default="all")
    parser.add_argument("--root", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--gallery", required=True)
    parser.add_argument("--pltrs-python", required=True)
    parser.add_argument("--mpl-python")
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
    if args.suite == "corpus":
        programs += corpus_programs(setup, args.patterns)
        setup.installed = install.install(setup.root, setup.pltrs_python)
        setup.prepare_configs()
        setup.prepare_environments(programs)
    if args.patterns:
        programs = [p for p in programs if any(pattern in p.name for pattern in args.patterns)]

    results: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        futures = {pool.submit(run_program, setup, program): program for program in programs}
        for done, future in enumerate(concurrent.futures.as_completed(futures), 1):
            try:
                result = future.result()
            except install.InstallError as err:
                # Every later program would fail alike: stop at once.
                pool.shutdown(wait=False, cancel_futures=True)
                raise SystemExit(f"driver.py: {err}") from None
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

    baseline = baseline_corpora = None
    if args.baseline:
        earlier = json.loads(pathlib.Path(args.baseline).read_text())
        baseline, baseline_corpora = earlier["groups"], earlier.get("corpora")
    report = write_report(setup, results, baseline, args.failures, baseline_corpora)
    if args.suite == "corpus" and not args.patterns:
        write_outcomes(results)
    print(report)
    return 0


if __name__ == "__main__":
    sys.exit(main())
