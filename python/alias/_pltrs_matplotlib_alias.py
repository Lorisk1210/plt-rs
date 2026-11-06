"""
matplotlib's names for pltrs (Python): what the ``matplotlib``
distribution of pltrs installs (``pip install . -C distribution=matplotlib``)
runs this, so that code importing matplotlib by name (pandas, seaborn)
gets pltrs.

The names become aliases of pltrs' modules, never second copies of
them: ``import matplotlib.pyplot`` gives ``pltrs.pyplot`` itself.

* ``matplotlib`` and ``matplotlib.x`` stand for ``pltrs`` and ``pltrs.x``;
* ``mpl_toolkits.<kit>`` and ``mpl_toolkits.<kit>.x`` for pltrs' toolkits
  (``pltrs.mpl_toolkits.<kit>``), the namespace package ``mpl_toolkits``
  staying open to other distributions' toolkits;
* ``pylab`` for ``pltrs.pylab``.
"""

import importlib
import importlib.abc
import importlib.util
import sys

#: (alias prefix, real prefix), the longest aliases first.
ALIASES = (
    ("mpl_toolkits.axes_grid1", "pltrs.mpl_toolkits.axes_grid1"),
    ("mpl_toolkits.axisartist", "pltrs.mpl_toolkits.axisartist"),
    ("mpl_toolkits.mplot3d", "pltrs.mpl_toolkits.mplot3d"),
    ("matplotlib", "pltrs"),
    ("pylab", "pltrs.pylab"),
)


def real_name(name):
    """The pltrs module the alias *name* stands for, or None."""
    for alias, real in ALIASES:
        if name == alias or name.startswith(alias + "."):
            return real + name[len(alias):]
    return None


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


def _alias_imported_toolkits():
    """Import the aliases of the toolkits pltrs imported itself, as
    matplotlib's own import of ``mpl_toolkits.mplot3d`` (by
    ``matplotlib.projections``) makes it an attribute of ``mpl_toolkits``,
    which programs reach it through (``mpl_toolkits.mplot3d.Axes3D``)."""
    for alias, real in ALIASES:
        if (alias.startswith("mpl_toolkits.") and real in sys.modules
                and alias not in sys.modules):
            importlib.import_module(alias)


class _AliasFinder(importlib.abc.MetaPathFinder):
    """Finds the aliases of `ALIASES`."""

    def find_spec(self, name, path=None, target=None):
        real = real_name(name)
        if real is None:
            return None
        return importlib.util.spec_from_loader(name, _AliasLoader(real))


def become(name):
    """
    Makes the module being imported as *name* (``matplotlib``, a toolkit,
    ``pylab``) the pltrs module it stands for, and the names below it
    aliases too.
    """
    if not any(isinstance(finder, _AliasFinder) for finder in sys.meta_path):
        sys.meta_path.insert(0, _AliasFinder())
    sys.modules[name] = importlib.import_module(real_name(name))
    _alias_imported_toolkits()
