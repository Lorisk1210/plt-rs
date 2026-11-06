"""
matplotlib's names for pltrs (Python): what the ``matplotlib``
distribution of pltrs installs (``pip install . -C distribution=matplotlib``)
runs this, so that code importing matplotlib by name (pandas, seaborn)
gets pltrs.

A process has one pltrs, under the names of the import that came first:

* ``import matplotlib`` (or a toolkit) before any ``import pltrs``:
  pltrs's sources load under matplotlib's names, as matplotlib's own
  would (``matplotlib.figure`` is pltrs's ``figure.py`` loaded as
  ``matplotlib.figure``), so that its classes, functions, loggers, reprs
  and pickles name matplotlib's modules, which libraries read
  (arviz-plots picks its plotting backend by ``type(fig).__module__``,
  doctests print reprs). pltrs's own names stand for them:
  ``pltrs`` and ``pltrs.x`` for ``matplotlib`` and ``matplotlib.x``, and
  ``pltrs.mpl_toolkits.<kit>`` (which pltrs's toolkits import) for
  ``mpl_toolkits.<kit>``;
* ``import pltrs`` first: pltrs keeps its names, and matplotlib's stand
  for its modules: ``matplotlib`` and ``matplotlib.x`` for ``pltrs`` and
  ``pltrs.x``, ``mpl_toolkits.<kit>`` and ``mpl_toolkits.<kit>.x`` for
  pltrs's toolkits (``pltrs.mpl_toolkits.<kit>``).

Each name standing for another is an alias of its module, never a second
copy of it. The namespace package ``mpl_toolkits`` stays open to other
distributions' toolkits either way.
"""

import importlib
import importlib.abc
import importlib.util
import os
import sys

#: pltrs's toolkits, by their names in matplotlib.
TOOLKITS = ("mpl_toolkits.axes_grid1", "mpl_toolkits.axisartist", "mpl_toolkits.mplot3d")
#: The aliases (alias prefix, real prefix), the longest first, with
#: pltrs's sources loaded under matplotlib's names (pltrs's projections
#: import its toolkits relatively, as ``matplotlib.mpl_toolkits.<kit>``);
AS_MATPLOTLIB = (
    ("matplotlib.mpl_toolkits", "mpl_toolkits"),
    ("pltrs.mpl_toolkits", "mpl_toolkits"),
    ("pltrs", "matplotlib"),
)
#: and with pltrs imported as pltrs.
AS_PLTRS = (*((kit, f"pltrs.{kit}") for kit in TOOLKITS), ("matplotlib", "pltrs"))


class _AliasLoader(importlib.abc.Loader):
    """Loads an alias as the module it stands for."""

    def __init__(self, real):
        self.real = real
        self.real_spec = None

    def create_module(self, spec):
        module = importlib.import_module(self.real)
        self.real_spec = module.__spec__
        return module

    def exec_module(self, module):
        """
        The real module ran already; importing the alias gave it the
        alias' ``__spec__``, which would break the relative imports its
        functions make when called (``__package__ != __spec__.parent``):
        it gets its own back.
        """
        module.__spec__ = self.real_spec
        _alias_imported_toolkits()


class _Finder(importlib.abc.MetaPathFinder):
    """
    Finds the aliases among matplotlib's and pltrs's names and the
    packages of pltrs's sources loaded under matplotlib's names
    (``matplotlib`` and the toolkits; their modules are found below them,
    as any package's are).
    """

    def __init__(self, aliases, packages):
        #: `AS_MATPLOTLIB` or `AS_PLTRS`.
        self.aliases = aliases
        #: The directories of the packages loaded under matplotlib's
        #: names, by name (none with `AS_PLTRS`).
        self.packages = packages

    def real_name(self, name):
        """The module the alias *name* stands for, or None."""
        for alias, real in self.aliases:
            if name == alias or name.startswith(alias + "."):
                return real + name[len(alias):]
        return None

    def find_spec(self, name, path=None, target=None):
        real = self.real_name(name)
        if real is not None:
            return importlib.util.spec_from_loader(name, _AliasLoader(real))
        directory = self.packages.get(name)
        if directory is None:
            return None
        return importlib.util.spec_from_file_location(
            name, os.path.join(directory, "__init__.py"),
            submodule_search_locations=[directory])


def _finder():
    """The finder of matplotlib's names, first in ``sys.meta_path`` from
    the first call on; it serves `AS_PLTRS` if pltrs was imported (as
    pltrs) by then, `AS_MATPLOTLIB` otherwise."""
    for finder in sys.meta_path:
        if isinstance(finder, _Finder):
            return finder
    if "pltrs" in sys.modules:
        finder = _Finder(AS_PLTRS, {})
    else:
        spec = importlib.util.find_spec("pltrs")
        if spec is None:
            raise ModuleNotFoundError("No module named 'pltrs'", name="pltrs")
        # pltrs's package is matplotlib; its toolkits are within it.
        sources = spec.submodule_search_locations[0]
        finder = _Finder(AS_MATPLOTLIB, {
            "matplotlib": sources,
            **{kit: os.path.join(sources, *kit.split(".")) for kit in TOOLKITS}})
    sys.meta_path.insert(0, finder)
    return finder


def _alias_imported_toolkits():
    """With pltrs imported as pltrs, import the aliases of the toolkits it
    imported itself, as matplotlib's own import of ``mpl_toolkits.mplot3d``
    (by ``matplotlib.projections``) makes it an attribute of
    ``mpl_toolkits``, which programs reach it through
    (``mpl_toolkits.mplot3d.Axes3D``)."""
    for alias, real in _finder().aliases:
        if (alias in TOOLKITS and real in sys.modules
                and alias not in sys.modules):
            importlib.import_module(alias)


def become(name):
    """
    Makes the module being imported as *name* (``matplotlib`` or a
    toolkit) pltrs's: imports *name* again in its place, which the finder
    now finds.
    """
    _finder()
    del sys.modules[name]
    importlib.import_module(name)
    _alias_imported_toolkits()
