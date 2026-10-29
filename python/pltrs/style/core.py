"""
Core functions and attributes for the matplotlib style library: `use`
and `context`. plt-rs bundles matplotlib's style sheets and applies them.
"""

import contextlib

from .. import _STYLE_BLACKLIST, _api, rc_context, rcdefaults, rcParams
from .._native import rcsetup as _rcsetup

#: matplotlib's aliases of style names.
_STYLE_ALIASES = {'mpl20': 'default', 'mpl15': 'classic'}


def use(style):
    """
    Use Matplotlib style settings from a style specification.

    The style name of 'default' is reserved for reverting back to
    the default style settings.

    Parameters
    ----------
    style : str, dict, or list
        A style specification: a style name, a dict of rcParams, or a list
        of either, applied in order.
    """
    if isinstance(style, (str, dict)) or hasattr(style, "keys"):
        styles = [style]
    else:
        styles = style
    for style in styles:
        if isinstance(style, str):
            style = _STYLE_ALIASES.get(style, style)
            if style == "default":
                with _api.suppress_matplotlib_deprecation_warning():
                    rcdefaults()
            else:
                _rcsetup.use_styles([style])
        else:
            for key, value in style.items():
                if key in _STYLE_BLACKLIST:
                    _api.warn_external(
                        f"Style includes a parameter, {key!r}, that is not "
                        f"related to style.  Ignoring this parameter.")
                else:
                    rcParams[key] = value


@contextlib.contextmanager
def context(style, after_reset=False):
    """
    Context manager for using style settings temporarily.

    Parameters
    ----------
    style : str, dict, Path or list
        A style specification (see `use`).
    after_reset : bool
        If True, apply style after resetting settings to their defaults;
        otherwise, apply style on top of the current settings.
    """
    with rc_context():
        if after_reset:
            rcdefaults()
        use(style)
        yield
