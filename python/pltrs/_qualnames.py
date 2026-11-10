"""
Methods named as matplotlib's.

Python words the errors of a call (an unexpected keyword, too many
arguments) with the qualified name of the function's code. A method pltrs
defines on a class of its own (a base of the package's classes that
matplotlib does not have) or overrides where matplotlib inherits is named
for the class matplotlib defines it in (`named_as`), so that both read as
matplotlib's. Where each of matplotlib's classes defines a method that
pltrs' classes share, each of pltrs' gets it as its own (`own_method`).
"""

from types import FunctionType


def named_as(qualname):
    """Decorator naming a function *qualname* (``"Formatter.set_locs"``),
    its code too."""
    def name(function):
        function.__qualname__ = qualname
        function.__code__ = function.__code__.replace(co_qualname=qualname)
        return function
    return name


def own_method(cls, name, function):
    """
    Give the class *cls* the method *name* as matplotlib's class defines
    it: of its own (a copy of *function*, named for *cls*), without a
    docstring, so that ``help`` shows what it finds on the bases, as for
    matplotlib's (whose method says ``# docstring inherited``).
    """
    method = FunctionType(function.__code__, function.__globals__, name,
                          function.__defaults__, function.__closure__)
    method.__kwdefaults__ = function.__kwdefaults__
    method.__doc__ = None
    setattr(cls, name, named_as(f"{cls.__qualname__}.{name}")(method))
