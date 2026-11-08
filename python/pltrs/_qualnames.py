"""
Methods named as matplotlib's.

Python words the errors of a call (an unexpected keyword, too many
arguments) with the qualified name of the function's code. A method pltrs
defines on a class of its own (a base of the package's classes that
matplotlib does not have) or overrides where matplotlib inherits is named
for the class matplotlib defines it in (`named_as`), so that both read as
matplotlib's.
"""


def named_as(qualname):
    """Decorator naming a function *qualname* (``"Formatter.set_locs"``),
    its code too."""
    def name(function):
        function.__qualname__ = qualname
        function.__code__ = function.__code__.replace(co_qualname=qualname)
        return function
    return name
