"""
pyplot's wrappers of the package's configuration functions (``plt.rc``,
``plt.rc_context``, ``plt.rcdefaults``, ``plt.set_loglevel``), which
``pyplot`` exports.

matplotlib defines these in pyplot, calling the package's functions, so
that they are pyplot functions of their own (``get_plot_commands`` lists
them); kept close to matplotlib's.
"""

from . import is_interactive
from . import rc as _rc, rc_context as _rc_context, rcdefaults as _rcdefaults
from . import set_loglevel as _set_loglevel
from ._pyplot_figure import _copy_docstring_and_deprecators, draw_all


@_copy_docstring_and_deprecators(_set_loglevel)
def set_loglevel(level):
    return _set_loglevel(level)


@_copy_docstring_and_deprecators(_rc)
def rc(group, **kwargs):
    _rc(group, **kwargs)


@_copy_docstring_and_deprecators(_rc_context)
def rc_context(rc=None, fname=None):
    return _rc_context(rc, fname)


@_copy_docstring_and_deprecators(_rcdefaults)
def rcdefaults():
    _rcdefaults()
    if is_interactive():
        draw_all()


__all__ = ["rc", "rc_context", "rcdefaults", "set_loglevel"]
for _function in (set_loglevel, rc, rc_context, rcdefaults):
    _function.__module__ = __name__.rpartition(".")[0] + ".pyplot"
