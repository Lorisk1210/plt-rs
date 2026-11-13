"""
The two distributions the backend builds, as ``pyproject.toml``'s
``[tool.pltrs-backend]`` describes them, and their core metadata.

* ``pltrs``: the package ``pltrs`` with its native extension. Its version
  is the Cargo workspace's.
* ``matplotlib``: the packages ``matplotlib``, ``mpl_toolkits`` (its
  three toolkits) and the module ``pylab``, standing for pltrs' own
  (``python/alias``, with ``_pltrs_matplotlib_alias``), so that code
  importing matplotlib by name (pandas, seaborn) runs on pltrs. It is
  named ``matplotlib`` so that pip counts it as matplotlib: what requires
  matplotlib is satisfied by it, and installing it replaces a real
  matplotlib, or a real matplotlib it, openly, rather than both writing
  ``matplotlib/`` into the same ``site-packages``. Its version is the
  matplotlib version pltrs implements with pltrs' as a local label
  (``3.11.2+pltrs.0.1.0``), and it requires that pltrs.

``[tool.pltrs-backend]`` holds what both share (``urls``, and
``license-files``, the patterns of the licence files every distribution
carries), and a table per distribution: ``summary``, ``requires-python``,
and optionally ``readme``, ``license`` (an SPDX expression),
``keywords``, ``classifiers`` and ``dependencies``.
"""

import pathlib
import re
import tomllib

#: The distributions, by the name ``-C distribution=...`` gives.
DISTRIBUTIONS = ("pltrs", "matplotlib")


class Project:
    """The source tree at *root*, and the distribution *name* of it."""

    def __init__(self, root, name="pltrs"):
        if name not in DISTRIBUTIONS:
            raise ValueError(
                f"pltrs: unknown distribution {name!r} (choose one of "
                f"{', '.join(DISTRIBUTIONS)})")
        self.root = pathlib.Path(root)
        self.name = name
        with open(self.root / "pyproject.toml", "rb") as file:
            self.shared = tomllib.load(file)["tool"]["pltrs-backend"]
        self.settings = self.shared[name]

    def _workspace_package(self):
        """The Cargo workspace's ``[workspace.package]`` table."""
        with open(self.root / "Cargo.toml", "rb") as file:
            return tomllib.load(file)["workspace"]["package"]

    @property
    def pltrs_version(self):
        """pltrs' version: the Cargo workspace's."""
        return self._workspace_package()["version"]

    @property
    def matplotlib_version(self):
        """The matplotlib version pltrs (Python) implements, its
        ``__version__``."""
        init = (self.root / "python" / "pltrs" / "__init__.py").read_text(encoding="utf-8")
        found = re.search(r'^__version__ = "([^"]+)"$', init, re.MULTILINE)
        if found is None:
            raise ValueError("pltrs: no __version__ in python/pltrs/__init__.py")
        return found.group(1)

    @property
    def version(self):
        """The distribution's version."""
        if self.name == "matplotlib":
            return f"{self.matplotlib_version}+pltrs.{self.pltrs_version}"
        return self.pltrs_version

    @property
    def dependencies(self):
        """The distributions this one requires."""
        if self.name == "matplotlib":
            return [f"pltrs=={self.pltrs_version}"]
        return list(self.settings.get("dependencies", []))

    def license_files(self):
        """The licence files, relative to the root, sorted. Each pattern
        of ``license-files`` must match a file."""
        found = set()
        for pattern in self.shared["license-files"]:
            matches = [path for path in self.root.glob(pattern) if path.is_file()]
            if not matches:
                raise ValueError(f"pltrs: the licence file pattern {pattern!r} matches nothing")
            found.update(path.relative_to(self.root).as_posix() for path in matches)
        return sorted(found)

    def metadata(self):
        """The core metadata (``METADATA``, version 2.4) as text."""
        settings = self.settings
        lines = [
            "Metadata-Version: 2.4",
            f"Name: {self.name}",
            f"Version: {self.version}",
            f"Summary: {settings['summary']}",
        ]
        if "keywords" in settings:
            lines.append(f"Keywords: {','.join(settings['keywords'])}")
        if "license" in settings:
            lines.append(f"License-Expression: {settings['license']}")
        lines += [f"License-File: {path}" for path in self.license_files()]
        lines += [f"Classifier: {classifier}" for classifier in settings.get("classifiers", [])]
        lines.append(f"Requires-Python: {settings['requires-python']}")
        lines += [f"Requires-Dist: {requirement}" for requirement in self.dependencies]
        lines += [f"Project-URL: {label}, {url}" for label, url in self.shared["urls"].items()]
        readme = settings.get("readme")
        if readme is None:
            return "\n".join(lines) + "\n"
        lines.append("Description-Content-Type: text/markdown")
        description = (self.root / readme).read_text(encoding="utf-8")
        return "\n".join(lines) + "\n\n" + description

    def package_files(self):
        """
        The files the distribution installs, as ``(archive path, file)``
        pairs, sorted: pltrs' package without byte code or a built
        extension (its stubs and ``py.typed`` included), or the alias'
        modules (with matplotlib's stubs and ``py.typed``) and the sources
        type checkers read of pltrs under matplotlib's name
        (`checked_sources`).
        """
        if self.name == "pltrs":
            base = self.root / "python"
            tops = [base / "pltrs"]
        else:
            base = self.root / "python" / "alias"
            tops = [base / "matplotlib", base / "mpl_toolkits", base / "pylab.py",
                    base / "_pltrs_matplotlib_alias.py"]
        found = []
        for top in tops:
            files = [top] if top.is_file() else sorted(top.rglob("*"))
            found.extend(
                (path.relative_to(base).as_posix(), path)
                for path in files if path.is_file() and _is_source(path))
        if self.name == "matplotlib":
            found.extend(self.checked_sources())
        return sorted(found)

    def checked_sources(self):
        """
        The sources beside the alias' stubs, as ``(archive path, file)``
        pairs, and its toolkits' modules: for each stub of ``python/alias/matplotlib`` (matplotlib's,
        and those of matplotlib's modules without one, which
        ``python/tools/matplotlib_stubs.py`` writes), pltrs' module under
        matplotlib's name (``matplotlib/units.py``), as type checkers look
        for a stub's source (pyright warns of a stub without one, and
        shows its docstrings). Not the package's ``__init__.py``, which is
        the alias' own. Python never imports them: the alias loads pltrs'
        files (``_pltrs_matplotlib_alias``).
        """
        stubs = self.root / "python" / "alias" / "matplotlib"
        package = self.root / "python" / "pltrs"
        found = []
        for stub in sorted(stubs.rglob("*.pyi")):
            source = package / stub.relative_to(stubs).with_suffix(".py")
            if source.is_file() and source != package / "__init__.py":
                found.append((f"matplotlib/{source.relative_to(package).as_posix()}", source))
        # matplotlib's toolkits are untyped, which pyright reads the code
        # of: pltrs' modules of them, but for the packages' ``__init__``
        # (the alias' loaders, whose stubs are matplotlib's code).
        found.extend((path.relative_to(package).as_posix(), path)
                     for path in sorted((package / "mpl_toolkits").rglob("*.py"))
                     if _is_source(path) and path.name != "__init__.py")
        return found

    def data_files(self):
        """
        The files the distribution installs under the installation's
        prefix, as ``(path under the prefix, file)`` pairs, sorted: pltrs'
        desktop entry and icons (``python/share``, which
        ``crates/pltrs/examples/install_desktop_entry.rs`` writes), for
        Linux desktops. The alias has none.
        """
        if self.name != "pltrs":
            return []
        base = self.root / "python"
        return sorted((path.relative_to(base).as_posix(), path)
                      for path in (base / "share").rglob("*") if path.is_file())


def _is_source(path):
    """Whether *path* belongs in a distribution: not byte code, nor an
    extension built in place (``cargo xtask python``)."""
    if "__pycache__" in path.parts or path.suffix == ".pyc":
        return False
    return not path.name.startswith("_native.")
