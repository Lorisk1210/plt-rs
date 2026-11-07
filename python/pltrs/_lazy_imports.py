"""
Packages matplotlib's modules import that pltrs's do not need, which
programs may still read from those modules (``backend_ps.fontTools``,
``backend_webagg.tornado``): the module's ``__getattr__`` (`getter`)
imports such a package the first time it is read, where it is installed,
so that importing the module costs nothing for it.
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
