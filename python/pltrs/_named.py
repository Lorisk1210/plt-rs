"""
Methods named as matplotlib defines them.

Python names a function in the errors of its calls by its code's
qualified name (``Text.update_from() got an unexpected keyword argument
'x'``). A method pltrs defines once for several classes (in a mixin), or
in a class where matplotlib's comes from a base class, would be named
after where pltrs defines it: `named` and `defined_in` give it
matplotlib's name.
"""

import functools
import types


def named(qualname, function=None):
    """A copy of *function* named *qualname* (``"Line2D.update_from"``)
    in the errors of its calls and by its ``__qualname__``, as the method
    matplotlib defines there; *function* itself stays as it was. Without
    *function*, a decorator doing so."""
    if function is None:
        return functools.partial(named, qualname)
    code = function.__code__.replace(co_qualname=qualname)
    copy = types.FunctionType(code, function.__globals__, function.__name__,
                              function.__defaults__, function.__closure__)
    functools.update_wrapper(copy, function, updated=())
    del copy.__wrapped__
    copy.__kwdefaults__ = function.__kwdefaults__
    copy.__qualname__ = qualname
    return copy


def defined_in(owner):
    """Class decorator for a mixin of one class: name its public methods
    as methods of the matplotlib class *owner* (``"Line2D"``), which
    defines them there."""
    def name_methods(cls):
        for name, value in vars(cls).items():
            if isinstance(value, types.FunctionType) and not name.startswith("_"):
                value.__code__ = value.__code__.replace(co_qualname=f"{owner}.{name}")
                value.__qualname__ = f"{owner}.{name}"
        return cls
    return name_methods
