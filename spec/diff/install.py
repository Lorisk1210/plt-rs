"""pltrs installed as its users have it, for the pltrs side of the corpora.

A user meets pltrs installed in a venv's ``site-packages``, as ``pip
install .`` and ``pip install . -C distribution=matplotlib`` lay the two
distributions out: the ``pltrs`` package with its extension, and the
``matplotlib`` distribution's aliases (``matplotlib``, ``mpl_toolkits``,
``pylab``) loading it under matplotlib's names, each with its
``.dist-info``. Programs see that: ``sys.path`` without pltrs' source tree
(a notebook printing it, aima's), modules whose files are under
``site-packages`` (dill pickles any other module by value: lmfit's
``save_model``), ``importlib.metadata.version("matplotlib")``.

`install` copies the files the build backend (``python/backend``) puts
in the two wheels into the ``site-packages`` of the corpora's pltrs venv,
which the tester owns, before a run: the tree's sources as they are
then, and the extension ``cargo xtask python`` built. Each file is
replaced whole (written beside it, then moved over it), so that the alias
is never missing; a file listing what was installed lets the next install
remove what the tree no longer has; a venv holding a ``matplotlib`` it
did not install is refused. ``cargo xtask diff --corpus`` holds the venvs
for its whole run, and the driver checks the alias before each program
(`alias_present`).

It uses the standard library and the build backend alone.
"""

from __future__ import annotations

import json
import os
import pathlib
import subprocess
import sys

#: The listing of what `install` put in a ``site-packages``.
LISTING = "pltrs-difftest-install.json"
#: The distributions installed, by the name ``-C distribution=...`` gives.
DISTRIBUTIONS = ("pltrs", "matplotlib")


class InstallError(Exception):
    """A venv pltrs cannot be installed into."""


def site_packages(python: str) -> pathlib.Path:
    """The ``site-packages`` (``purelib``) of the interpreter *python*."""
    found = subprocess.run(
        [python, "-I", "-c", "import sysconfig; print(sysconfig.get_path('purelib'))"],
        capture_output=True, text=True, check=True)
    return pathlib.Path(found.stdout.strip())


def _installed_before(site: pathlib.Path) -> set[str]:
    """The files an earlier `install` put in *site*, relative to it; refuses
    a ``matplotlib`` or ``pltrs`` installed otherwise."""
    listing = site / LISTING
    if not listing.exists():
        if (site / "matplotlib").exists() or (site / "pltrs").exists():
            raise InstallError(f"{site} holds a matplotlib or pltrs the tester did not install")
        return set()
    return set(json.loads(listing.read_text()))


def _put(data: bytes, target: pathlib.Path) -> None:
    """Writes *data* as *target*, which is never missing meanwhile: written
    beside it, then moved over it."""
    target.parent.mkdir(parents=True, exist_ok=True)
    beside = target.with_name(target.name + ".pltrs-difftest")
    beside.write_bytes(data)
    os.replace(beside, target)


def install(root: pathlib.Path, python: str) -> pathlib.Path:
    """Installs pltrs and its ``matplotlib`` distribution from the tree at
    *root* into the ``site-packages`` of *python*, over what an earlier
    install put there (no file of the alias missing at any moment; those
    the tree no longer has removed last); returns the ``site-packages``.
    Installing twice installs the same files."""
    sys.path.insert(0, str(root / "python" / "backend"))
    try:
        from pltrs_backend import _metadata as metadata
        from pltrs_backend._project import Project
        from pltrs_backend._wheel import dist_info_name
    finally:
        sys.path.remove(str(root / "python" / "backend"))
    site = site_packages(python)
    before = _installed_before(site)
    installed: dict[str, bytes] = {}
    for name in DISTRIBUTIONS:
        project = Project(root, name)
        files = list(project.package_files())
        if name == "pltrs":
            # The extension the wheel builds, as `cargo xtask python` built it.
            files += [(f"pltrs/{path.name}", path) for path in (root / "python" / "pltrs").glob("_native.*")
                      if path.suffix in (".so", ".pyd")]
        for archive_path, path in files:
            installed[archive_path] = path.read_bytes()
        info = dist_info_name(project.name, project.version)
        installed[f"{info}/METADATA"] = metadata(project).encode()
        installed[f"{info}/INSTALLER"] = b"pltrs-difftest\n"
        # The files installed, as an installer's RECORD lists them for
        # uninstalling (hashes left out, as for files an installer wrote).
        recorded = [archive_path for archive_path, _ in files]
        recorded += [f"{info}/{name}" for name in ("METADATA", "INSTALLER", "RECORD")]
        installed[f"{info}/RECORD"] = "".join(f"{path},,\n" for path in recorded).encode()
    for relative, data in installed.items():
        target = site / relative
        if not (target.is_file() and target.read_bytes() == data):
            _put(data, target)
    _put(json.dumps(sorted(installed)).encode(), site / LISTING)
    for relative in sorted(before - installed.keys(), reverse=True):
        stale = site / relative
        if stale.is_dir():
            # (A listing of an earlier layout named directories.)
            continue
        stale.unlink(missing_ok=True)
        for directory in stale.parents:
            if directory == site or any(directory.iterdir()):
                break
            directory.rmdir()
    return site


def alias_present(site: pathlib.Path) -> bool:
    """Whether the ``matplotlib`` alias `install` put in *site* is still
    there (a program run without it would import no matplotlib)."""
    return (site / "matplotlib" / "__init__.py").is_file()
