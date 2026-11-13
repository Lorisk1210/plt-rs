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

``--lowest`` installs the lowest versions the requirements allow first
(their ``>=`` bounds), so that the bounds are known to work. The venvs,
pip's cache and the test output go to ``--work`` (default
``target/check-dist``, ``check-dist`` in the working directory outside a
source tree).
"""

import argparse
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
    venv = work / "venv"
    run([python, "-m", "venv", venv], work, environment)
    interpreter = venv_python(venv)
    pip = [interpreter, "-m", "pip", "install", "--quiet"]
    pltrs = [dist for dist in dists if dist.name.startswith("pltrs-")]
    aliases = [dist for dist in dists if dist.name.startswith("matplotlib-")]
    if lowest:
        bounds = _core_metadata.lower_bounds(metadata_of(pltrs[0]))
        run([*pip, *(f"{name}=={version}" for name, version in bounds.items())], work,
            environment)
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
