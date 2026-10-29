"""Give pltrs's own name to the modules loaded as matplotlib.

``cargo xtask mpl-suite`` runs matplotlib's tests in a directory where
``matplotlib``, ``mpl_toolkits`` and ``pylab`` are links into
``python/pltrs``, so that ``import matplotlib`` loads pltrs's sources
under matplotlib's names, and puts this file there as
``sitecustomize.py``, which Python imports at startup (in pytest and in
every subprocess a test starts).

pltrs's toolkits import pltrs by name, as matplotlib's import
matplotlib: ``from pltrs import colors``. Here that must give the module
already loaded as ``matplotlib.colors``, never a second copy, so these
names become aliases:

* ``pltrs`` and ``pltrs.x`` for ``matplotlib`` and ``matplotlib.x``;
* ``pltrs.mpl_toolkits.x`` and ``matplotlib.mpl_toolkits.x`` for
  ``mpl_toolkits.x``, the toolkits' top-level name in matplotlib.
"""

import importlib
import importlib.abc
import importlib.util
import sys

#: (alias prefix, real prefix), the longest aliases first.
_ALIASES = (
    ("pltrs.mpl_toolkits", "mpl_toolkits"),
    ("matplotlib.mpl_toolkits", "mpl_toolkits"),
    ("pltrs", "matplotlib"),
)


class _AliasLoader(importlib.abc.Loader):
    """Loads an alias as the module it stands for."""

    def __init__(self, real_name):
        self.real_name = real_name
        self.real_spec = None

    def create_module(self, spec):
        module = importlib.import_module(self.real_name)
        self.real_spec = module.__spec__
        return module

    def exec_module(self, module):
        """
        The real module is already executed; importing the alias gave it
        the alias's ``__spec__``, which would break the relative imports
        its functions make when called (``__package__ != __spec__.parent``):
        it gets its own back.
        """
        module.__spec__ = self.real_spec


class _AliasFinder(importlib.abc.MetaPathFinder):
    """Finds the aliases of ``_ALIASES``."""

    def find_spec(self, name, path=None, target=None):
        for alias, real in _ALIASES:
            if name == alias or name.startswith(alias + "."):
                real_name = real + name[len(alias):]
                return importlib.util.spec_from_loader(name, _AliasLoader(real_name))
        return None


sys.meta_path.insert(0, _AliasFinder())
