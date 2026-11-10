"""
Imports made when first needed rather than when a module loads.

* Packages matplotlib's modules import that pltrs's do not need, which
  programs may still read from those modules (``backend_ps.fontTools``,
  ``backend_webagg.tornado``): the module's ``__getattr__`` (`getter`)
  imports such a package the first time it is read, where it is installed,
  so that importing the module costs nothing for it.
* Modules of the package that a module uses only inside its functions, for
  what not every program does (matplotlib's code for unusual arguments,
  legends, contours, artists moved between figures): `module` stands for
  one until the module first reads one of its attributes, so that
  importing pltrs loads none of them.
"""

import importlib


def getter(module_name, **packages):
    """
    The module ``__getattr__`` of the module *module_name* giving each
    package of *packages* (attribute name -> the modules of the package
    matplotlib's module imports) once it is read; a package that is not
    installed is an attribute the module does not have.
    """
    def __getattr__(name):
        modules = packages.get(name)
        if modules is not None:
            try:
                for module in modules:
                    importlib.import_module(module)
            except ImportError as err:
                raise AttributeError(
                    f"module {module_name!r} has no attribute {name!r} "
                    f"({name} is not installed)") from err
            return importlib.import_module(name)
        raise AttributeError(f"module {module_name!r} has no attribute {name!r}")
    return __getattr__


class _Deferred:
    """
    The module *name*, not imported yet, bound as *bound* in the namespace
    *namespace* of the module using it: reading one of its attributes
    imports it and binds the module itself there instead, so that later
    reads go to it directly.
    """

    __slots__ = ("_name", "_namespace", "_bound")

    def __init__(self, name, namespace, bound):
        self._name = name
        self._namespace = namespace
        self._bound = bound

    def __getattr__(self, attribute):
        if attribute in _Deferred.__slots__:
            # Not set yet (a copy being made): no module to import.
            raise AttributeError(attribute)
        module = importlib.import_module(self._name)
        self._namespace[self._bound] = module
        return getattr(module, attribute)

    def __repr__(self):
        return f"<module {self._name!r} (imported when first used)>"


def module(namespace, name):
    """
    The module *name*, relative to the module whose globals are *namespace*
    as in its ``from`` imports (``".axes._sharing"`` for ``from .axes import
    _sharing``, ``".._mirrors"`` for ``from .. import _mirrors``), imported
    when that module first reads one of its attributes, to be bound under
    the last part of *name*::

        _quiver_reference = _lazy_imports.module(globals(), "._quiver_reference")
    """
    relative = name.lstrip(".")
    package = namespace["__package__"].rsplit(".", len(name) - len(relative) - 1)[0]
    return _Deferred(f"{package}.{relative}", namespace, relative.rpartition(".")[2])
