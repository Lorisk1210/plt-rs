"""
The two distributions the backend builds, as ``pyproject.toml``'s
``[tool.pltrs-backend]`` describes them, and their core metadata.

* ``pltrs``: the package ``pltrs`` with its native extension. Its version
  is the Cargo workspace's.
* ``matplotlib``: the packages ``matplotlib``, ``mpl_toolkits`` (its
  three toolkits) and the module ``pylab``, standing for pltrs' own
  (``python/alias``, with ``_pltrs_matplotlib_alias``), so that code importing matplotlib by name (pandas,
  seaborn) runs on pltrs. It is named ``matplotlib`` so that pip counts
  it as matplotlib: what requires matplotlib is satisfied by it, and
  installing it replaces a real matplotlib, or a real matplotlib it,
  openly, rather than both writing ``matplotlib/`` into the same
  ``site-packages``. Its version is the matplotlib version pltrs
  implements with pltrs' as a local label (``3.11.2+pltrs.0.1.0``), and
  it requires that pltrs.
"""

import pathlib
import re
import tomllib

#: The distributions, by the name ``-C distribution=...`` gives.
DISTRIBUTIONS = ("pltrs", "matplotlib")

#: The licence files every distribution carries, relative to the root.
LICENSE_GLOBS = ("THIRD_PARTY.md", "LICENSES/*.txt")


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
            self.settings = tomllib.load(file)["tool"]["pltrs-backend"][name]

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
        """The licence files, relative to the root, sorted."""
        return sorted(
            path.relative_to(self.root).as_posix()
            for pattern in LICENSE_GLOBS for path in self.root.glob(pattern))

    def metadata(self):
        """The core metadata (``METADATA``, version 2.4) as text."""
        lines = [
            "Metadata-Version: 2.4",
            f"Name: {self.name}",
            f"Version: {self.version}",
            f"Summary: {self.settings['summary']}",
            f"Requires-Python: {self.settings['requires-python']}",
            f"Project-URL: Source, {self._workspace_package()['repository']}",
            *(f"Requires-Dist: {requirement}" for requirement in self.dependencies),
            *(f"License-File: {path}" for path in self.license_files()),
        ]
        readme = self.settings.get("readme")
        if readme is None:
            return "\n".join(lines) + "\n"
        lines.append("Description-Content-Type: text/markdown")
        description = (self.root / readme).read_text(encoding="utf-8")
        return "\n".join(lines) + "\n\n" + description

    def package_files(self):
        """
        The files the distribution installs, as ``(archive path, file)``
        pairs, sorted: pltrs' package without byte code or a built
        extension, or the alias' modules.
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
        return sorted(found)


def _is_source(path):
    """Whether *path* belongs in a distribution: not byte code, nor an
    extension built in place (``cargo xtask python``)."""
    if "__pycache__" in path.parts or path.suffix == ".pyc":
        return False
    return not path.name.startswith("_native.")
