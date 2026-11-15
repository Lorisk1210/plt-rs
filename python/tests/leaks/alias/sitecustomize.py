"""
The leak harness's reference run: imports `pltrs` and ``pltrs.<module>``
as matplotlib's modules (the very module objects), and
``pltrs.mpl_toolkits.<module>`` as ``mpl_toolkits.<module>``, so that the
scenes, written for pltrs, run under matplotlib unchanged. On
``PYTHONPATH`` of the reference venv's interpreter only.
"""

import importlib
import importlib.abc
import importlib.util
import sys


class PltrsIsMatplotlib(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    """Finds pltrs's names and loads matplotlib's modules for them."""

    def find_spec(self, name, path=None, target=None):
        if name == "pltrs" or name.startswith("pltrs."):
            return importlib.util.spec_from_loader(name, self)
        return None

    def create_module(self, spec):
        name = spec.name.removeprefix("pltrs")
        if name.startswith(".mpl_toolkits"):
            return importlib.import_module(name.removeprefix("."))
        return importlib.import_module("matplotlib" + name)

    def exec_module(self, module):
        """The module ran when matplotlib imported it."""


sys.meta_path.insert(0, PltrsIsMatplotlib())
