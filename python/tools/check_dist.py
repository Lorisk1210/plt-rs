"""
Checks pltrs' wheels and sdists, as built anywhere (``pip wheel .``, the
CI workflows, ``cargo xtask python wheels``):

    python python/tools/check_dist.py DIST...
    python python/tools/check_dist.py --install [--python PY] [--lowest] DIST...

Each archive is first checked as an index would check it (the backend's
``_audit``: file name, ``WHEEL``, ``RECORD``, core metadata, licence
files, and platform tags the extension inside really earns).

``--install`` then installs the archives together in a fresh venv of
``PY`` (default: this Python), from the archives and PyPI (an sdist is
built by pip, with cargo), and checks the installed package rather than
the source tree (which must be there, as a checkout is):

* the package imports from the venv, is typed (``py.typed`` and
  matplotlib's stubs beside its modules, `TYPED`), and pltrs' smoke tests
  (``python/tests``, run in place with only their own directory on
  ``sys.path``, as some read data from the tree) pass;
* then, with the ``matplotlib`` distribution among the archives
  installed too, a program written for matplotlib (``drop_in.py``) runs
  unchanged on pltrs and writes its figures, and ``matplotlib`` is typed
  as well (the alias carries what type checkers read of pltrs).

Before all that, the pltrs archives are installed alone, with what their
metadata requires and nothing else (no pytest), into a venv of their own,
and matplotlib (the version pltrs implements) into another, where
``bare_imports.py`` imports every module of each package and draws with
each backend that needs no display: each module matplotlib imports must
import from pltrs, one matplotlib fails to import (wanting a GUI toolkit,
tornado, IPython, pytest, ...) may fail only with matplotlib's exception,
and each backend must draw as matplotlib's does. A package pltrs imports
without declaring it shows there.

``--lowest`` installs the lowest versions the requirements allow
(their ``>=`` bounds), and holds every install of the venvs to them, so
that the bounds are known to work. The venvs,
pip's cache and the test output go to ``--work`` (default
``target/check-dist``, ``check-dist`` in the working directory outside a
source tree).
"""

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tarfile
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "python" / "backend"))

from pltrs_backend import _audit, _core_metadata  # noqa: E402

#: Files of a typed installed package, below it: PEP 561's marker and stubs.
TYPED = ("py.typed", "__init__.pyi", "pyplot.pyi", "figure.pyi", "axes/_axes.pyi")

#: The first bytes of each figure the drop-in program writes.
DROP_IN_FIGURES = {
    "figure.png": b"\x89PNG", "figure.pdf": b"%PDF", "figure.svg": b"<?xml",
    "surface.png": b"\x89PNG", "pylab.png": b"\x89PNG",
}


def audit(path):
    """Prints what is wrong with the archive at *path* (nothing when it
    holds) and returns whether it holds."""
    problems = (_audit.wheel_problems(path) if path.name.endswith(".whl")
                else _audit.sdist_problems(path))
    size = path.stat().st_size / 2 ** 20
    print(f"{path.name}: {size:.1f} MiB, {'fine' if not problems else 'WRONG'}")
    for problem in problems:
        print(f"  {problem}")
    return not problems


def metadata_of(path):
    """The core metadata of the wheel or sdist at *path*."""
    if path.name.endswith(".whl"):
        with zipfile.ZipFile(path) as archive:
            name = next(n for n in archive.namelist() if n.endswith(".dist-info/METADATA"))
            return archive.read(name).decode("utf-8")
    with tarfile.open(path, "r:gz") as archive:
        name = next(n for n in archive.getnames() if n.count("/") == 1 and n.endswith("/PKG-INFO"))
        return archive.extractfile(name).read().decode("utf-8")


def run(command, cwd, environment):
    """Runs *command*, echoing it, and fails unless it succeeds."""
    print("$", " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], cwd=cwd, env=environment, check=True)


def venv_python(venv):
    """The interpreter of the venv *venv*."""
    return venv / ("Scripts/python.exe" if os.name == "nt" else "bin/python")


def check_typed(interpreter, package, work, environment):
    """Fails unless the installed *package* (a name) has the files of
    `TYPED`, as found by the venv's *interpreter*."""
    run([interpreter, "-c", "import importlib.util, pathlib, sys; "
         f"spec = importlib.util.find_spec({package!r}); "
         "base = pathlib.Path(spec.submodule_search_locations[0]); "
         f"missing = [n for n in {TYPED!r} if not (base / n).is_file()]; "
         "assert base.is_relative_to(sys.prefix) and not missing, (base, missing); "
         f"print({package!r}, 'is typed')"], work, environment)


def held_to_bounds(dist, work):
    """pip's options holding an install to the lowest versions the
    requirements of *dist* allow (their ``>=`` bounds): a constraints
    file in *work*, so that what else is installed (pytest) cannot raise
    them."""
    bounds = _core_metadata.lower_bounds(metadata_of(dist))
    constraints = work / "lowest.txt"
    constraints.write_text("".join(f"{name}=={version}\n" for name, version in bounds.items()),
                           encoding="utf-8")
    return ["--constraint", constraints]


def bare_tables(python, requirements, package, work, environment):
    """Installs *requirements* (pip's arguments) alone into a fresh venv
    of *python* and returns the venv's interpreter and what
    ``bare_imports.py`` finds of *package* there."""
    venv = work / f"bare-{package}"
    run([python, "-m", "venv", venv], work, environment)
    interpreter = venv_python(venv)
    run([interpreter, "-m", "pip", "install", "--quiet", *requirements], work, environment)
    tables = venv / "tables.json"
    run([interpreter, "-I", HERE / "bare_imports.py", package, tables], venv, environment)
    return interpreter, json.loads(tables.read_text(encoding="utf-8"))


#: The packages pltrs' sources import that neither its requirements nor
#: matplotlib's optional imports give, and why each may be missing.
OPTIONAL = {
    "contourpy": "a requirement of matplotlib's that pltrs replaces: a contour set's "
                 "_contour_generator is contourpy's object, where contourpy is installed",
    "traitlets": "IPython's kernel brings it, where pltrs' inline backend runs",
    "zmq": "IPython's kernel brings it, where pltrs' event loop runs in it",
}


#: pltrs' own modules for one system only (``sys.platform``), which import
#: there alone.
SYSTEM_MODULES = {"matplotlib.backends._tkagg_win32": "win32"}


def bare_problems(ours, theirs):
    """What pltrs' tables *ours* (see ``bare_imports.py``) show that
    matplotlib's *theirs* do not: a module or backend failing where
    matplotlib's works or fails with another exception, a module of
    pltrs' own failing (but for want of an optional package, or on
    another system than its own, `SYSTEM_MODULES`), or a package imported
    that is neither required nor optional to matplotlib (nor in
    `OPTIONAL`)."""
    problems = []
    optional = (theirs["imports"].keys() - set(theirs["required"])) | OPTIONAL.keys()
    for kind in ("modules", "backends"):
        for name, failure in sorted(ours[kind].items()):
            if failure is None or SYSTEM_MODULES.get(name, sys.platform) != sys.platform:
                continue
            if name in theirs[kind]:
                expected = theirs[kind][name]
                if expected is not None and expected[0] == failure[0]:
                    continue
                under = "fine" if expected is None else expected[0]
            elif failure[2:] and failure[2] in optional:
                continue
            else:
                under = "not matplotlib's"
            problems.append(f"{name}: {failure[0]}: {failure[1]} (under matplotlib: {under})")
    for package, module in sorted(ours["imports"].items()):
        if package not in ours["required"] and package not in optional:
            problems.append(f"{module} imports {package}, which is no requirement of pltrs "
                            "nor optional to matplotlib")
    return problems


def bare_check(pltrs, python, held, work, environment):
    """Installs the archives *pltrs* alone, held by *held* (pip's
    options), into a fresh venv of *python*, and matplotlib into another,
    and fails unless pltrs imports and draws there as matplotlib does (see
    the module's description)."""
    environment = {**environment, "MPLBACKEND": "agg", "MPLCONFIGDIR": str(work / "mpl-config")}
    interpreter, ours = bare_tables(python, [*held, *pltrs], "pltrs", work, environment)
    version = subprocess.run([str(interpreter), "-I", "-c", "import pltrs; print(pltrs.__version__)"],
                             cwd=work, env=environment, check=True, stdout=subprocess.PIPE,
                             text=True).stdout.strip()
    _, theirs = bare_tables(python, [*held, f"matplotlib=={version}"], "matplotlib", work,
                            environment)
    problems = bare_problems(ours, theirs)
    if problems:
        raise RuntimeError("pltrs alone does not import or draw as matplotlib alone:\n  "
                           + "\n  ".join(problems))
    print(f"pltrs alone: {len(ours['modules'])} modules and {len(ours['backends'])} backends "
          f"as matplotlib {version} alone")


def install_check(dists, python, lowest, work):
    """Installs *dists* into a fresh venv of *python* in *work* and runs
    the checks of the module's description."""
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    # Nothing of the source tree is importable, and nothing is written into it.
    environment = {key: value for key, value in os.environ.items()
                   if key not in ("PYTHONPATH", "PYTHONHOME", "VIRTUAL_ENV")}
    config = work / "config"
    config.mkdir()
    environment.update(PIP_CACHE_DIR=str(work.parent / "pip-cache"), PLTRSCONFIGDIR=str(config),
                       PIP_DISABLE_PIP_VERSION_CHECK="1", PYTHONDONTWRITEBYTECODE="1")
    pltrs = [dist for dist in dists if dist.name.startswith("pltrs-")]
    aliases = [dist for dist in dists if dist.name.startswith("matplotlib-")]
    held = held_to_bounds(pltrs[0], work) if lowest else []
    bare_check(pltrs, python, held, work, environment)
    venv = work / "venv"
    run([python, "-m", "venv", venv], work, environment)
    interpreter = venv_python(venv)
    pip = [interpreter, "-m", "pip", "install", "--quiet", *held]
    run([*pip, *pltrs, "pytest"], work, environment)
    run([interpreter, "-c", "import pltrs, sys; "
         "assert pltrs.__file__.startswith(sys.prefix), pltrs.__file__; "
         "print('pltrs', pltrs.__version__, 'from', pltrs.__file__)"], work, environment)
    check_typed(interpreter, "pltrs", work, environment)
    # The tests run where they are, for the data some read from the tree,
    # with python/tests alone on sys.path: pltrs comes from the venv.
    run([interpreter, "-m", "pytest", "-q", "-p", "no:cacheprovider",
         f"--basetemp={work / 'pytest-tmp'}", ROOT / "python" / "tests"], work, environment)
    if aliases:
        run([*pip, *pltrs, *aliases], work, environment)
        check_typed(interpreter, "matplotlib", work, environment)
        drawn = work / "drop-in"
        drawn.mkdir()
        run([interpreter, "-c", "import matplotlib.pyplot, pltrs.pyplot; "
             "assert matplotlib.pyplot is pltrs.pyplot; print('matplotlib is pltrs')"],
            drawn, environment)
        run([interpreter, HERE / "drop_in.py"], drawn, environment)
        for name, magic in DROP_IN_FIGURES.items():
            if not (drawn / name).read_bytes().startswith(magic):
                raise RuntimeError(f"the drop-in program wrote no {name}")


def main(argv):
    """Runs the checks and exits with 1 if one fails."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("dists", nargs="+", type=pathlib.Path)
    parser.add_argument("--install", action="store_true")
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument("--lowest", action="store_true")
    default_work = ROOT / "target" / "check-dist" if (ROOT / "Cargo.toml").exists() \
        else pathlib.Path("check-dist")
    parser.add_argument("--work", type=pathlib.Path, default=default_work)
    options = parser.parse_args(argv)
    dists = [dist.resolve() for dist in options.dists]
    if not all([audit(dist) for dist in dists]):
        return 1
    if options.install:
        name = "lowest" if options.lowest else pathlib.Path(options.python).name
        try:
            install_check(dists, options.python, options.lowest, options.work.resolve() / name)
        except (subprocess.CalledProcessError, RuntimeError) as error:
            print(f"check_dist: {error}", file=sys.stderr)
            return 1
        print("installed and checked:", ", ".join(dist.name for dist in dists))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
