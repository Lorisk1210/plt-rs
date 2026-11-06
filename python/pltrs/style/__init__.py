"""
Core functions and attributes for the matplotlib style library:

``use``
    Select style sheet to override the current matplotlib settings.
``context``
    Context manager to use a style sheet temporarily.
``available``
    List available style sheets. Underscore-prefixed names are considered private and
    not listed, though may still be accessed directly from ``library``.
``library``
    A dictionary of style names and matplotlib settings.

From matplotlib 3.11.2's ``style`` package, under the Matplotlib License.
The bundled sheets, matplotlib's ``stylelib``, come from pltrs (Rust), which
holds them; the user's sheets are read from ``USER_LIBRARY_PATHS`` as
matplotlib reads them.
"""

import contextlib
import importlib.resources
import logging
import os
from pathlib import Path
import warnings

from .. import (RcParams, _STYLE_BLACKLIST, _api, get_configdir, rc_context,
                rc_params_from_file, rcdefaults, rcParams, rcParamsDefault)
from .._native import rcsetup as _rcsetup

_log = logging.getLogger(__name__)

__all__ = ['use', 'context', 'available', 'library', 'reload_library']


# Users may want multiple library paths, so store a list of paths.
USER_LIBRARY_PATHS = [os.path.join(get_configdir(), 'stylelib')]
_STYLE_EXTENSION = 'mplstyle'


def use(style):
    """
    Use Matplotlib style settings from a style specification.

    The style name of 'default' is reserved for reverting back to
    the default style settings.

    .. note::

       This updates the `.rcParams` with the settings from the style.
       `.rcParams` not defined in the style are kept.

    Parameters
    ----------
    style : str, dict, Path or list

        A style specification. Valid options are:

        str
            - One of the style names in `.style.available` (a builtin style or
              a style installed in the user library path).

            - A dotted name of the form "package.style_name"; in that case,
              "package" should be an importable Python package name, e.g. at
              ``/path/to/package/__init__.py``; the loaded style file is
              ``/path/to/package/style_name.mplstyle``.  (Style files in
              subpackages are likewise supported.)

            - The path or URL to a style file, which gets loaded by
              `.rc_params_from_file`.

        dict
            A mapping of key/value pairs for `matplotlib.rcParams`.

        Path
            The path to a style file, which gets loaded by
            `.rc_params_from_file`.

        list
            A list of style specifiers (str, Path or dict), which are applied
            from first to last in the list.

    Notes
    -----
    The following `.rcParams` are not related to style and will be ignored if
    found in a style specification:

    %s
    """
    if isinstance(style, (str, Path)) or hasattr(style, 'keys'):
        # If name is a single str, Path or dict, make it a single element list.
        styles = [style]
    else:
        styles = style

    style_alias = {'mpl20': 'default', 'mpl15': 'classic'}

    for style in styles:
        if isinstance(style, str):
            style = style_alias.get(style, style)
            if style == "default":
                style = rcParamsDefault
            elif style in library:
                style = library[style]
            elif "." in style:
                pkg, _, name = style.rpartition(".")
                try:
                    path = importlib.resources.files(pkg) / f"{name}.{_STYLE_EXTENSION}"
                    style = rc_params_from_file(path, use_default_template=False)
                except (ModuleNotFoundError, OSError, TypeError):
                    # There is an ambiguity whether a dotted name refers to a
                    # package.style_name or to a dotted file path: try the
                    # first form, then the second one.
                    pass
        if isinstance(style, (str, Path)):
            try:
                style = rc_params_from_file(style, use_default_template=False)
            except OSError as err:
                raise OSError(
                    f"{style!r} is not a valid package style, path of style "
                    f"file, URL of style file, or library style name (library "
                    f"styles are listed in `style.available`)") from err
        _apply(style)


def _apply(style):
    """Apply the parameters of the mapping *style* to `.rcParams`, leaving
    out (with a warning, unless *style* is `rcParamsDefault`) those of
    ``STYLE_BLACKLIST``."""
    keys = []
    for key in style:  # don't trigger RcParams.__getitem__('backend')
        if key not in _STYLE_BLACKLIST:
            keys.append(key)
        elif style is not rcParamsDefault:
            _api.warn_external(
                f"Style includes a parameter, {key!r}, that is not "
                f"related to style.  Ignoring this parameter.")
    if isinstance(style, RcParams):
        rcParams._update_from(style, keys)
    else:
        rcParams.update({key: style[key] for key in keys})


@contextlib.contextmanager
def context(style, after_reset=False):
    """
    Context manager for using style settings temporarily.

    Parameters
    ----------
    style : str, dict, Path or list
        A style specification. Valid options are:

        str
            - One of the style names in `.style.available` (a builtin style or
              a style installed in the user library path).

            - A dotted name of the form "package.style_name"; in that case,
              "package" should be an importable Python package name, e.g. at
              ``/path/to/package/__init__.py``; the loaded style file is
              ``/path/to/package/style_name.mplstyle``.  (Style files in
              subpackages are likewise supported.)

            - The path or URL to a style file, which gets loaded by
              `.rc_params_from_file`.
        dict
            A mapping of key/value pairs for `matplotlib.rcParams`.

        Path
            The path to a style file, which gets loaded by
            `.rc_params_from_file`.

        list
            A list of style specifiers (str, Path or dict), which are applied
            from first to last in the list.

    after_reset : bool
        If True, apply style after resetting settings to their defaults;
        otherwise, apply style on top of the current settings.
    """
    with rc_context():
        if after_reset:
            rcdefaults()
        use(style)
        yield


def _update_user_library(library):
    """Update style library with user-defined rc files."""
    for stylelib_path in map(os.path.expanduser, USER_LIBRARY_PATHS):
        styles = _read_style_directory(stylelib_path)
        _update_nested_dict(library, styles)
    return library


def _read_style_directory(style_dir):
    """Return dictionary of styles defined in *style_dir*."""
    styles = dict()
    for path in Path(style_dir).glob(f"*.{_STYLE_EXTENSION}"):
        with warnings.catch_warnings(record=True) as warns:
            styles[path.stem] = rc_params_from_file(path, use_default_template=False)
        for w in warns:
            _log.warning('In %s: %s', path, w.message)
    return styles


def _update_nested_dict(main_dict, new_dict, empty=RcParams):
    """
    Update nested dict (only level of nesting) with new values.

    Unlike `dict.update`, this assumes that the values of the parent dict are
    dicts (or dict-like), so you shouldn't replace the nested dict if it
    already exists. Instead you should update the sub-dict.

    pltrs: a name *main_dict* lacks gets an *empty* dict first (an
    `.RcParams` in the library; matplotlib's public function makes a
    `dict`).
    """
    # update named styles specified by user
    for name, rc_dict in new_dict.items():
        main_dict.setdefault(name, empty()).update(rc_dict)
    return main_dict


@_api.deprecated("3.11")
def update_user_library(library):
    return _update_user_library(library)


@_api.deprecated("3.11")
def read_style_directory(style_dir):
    return _read_style_directory(style_dir)


@_api.deprecated("3.11")
def update_nested_dict(main_dict, new_dict):
    return _update_nested_dict(main_dict, new_dict, dict)


def _bundled_library():
    """The sheets bundled with pltrs (matplotlib's ``stylelib``), each as an
    `.RcParams` of its settings."""
    sheets = {}
    for name, settings in _rcsetup.bundled_styles():
        params = sheets[name] = RcParams()
        for key, value in settings:
            params._set(key, value)
    return sheets


# Load style library
# ==================
_base_library = _bundled_library()
library = {}
available = []


def reload_library():
    """Reload the style library."""
    library.clear()
    library.update(_update_user_library(
        {name: params.copy() for name, params in _base_library.items()}))
    available[:] = sorted(name for name in library if not name.startswith('_'))


reload_library()

# matplotlib's deprecated alias module, which its rcdefaults imports (making
# it an attribute of this package) without the deprecation warning.
with _api.suppress_matplotlib_deprecation_warning():
    from . import core  # noqa: E402, F401
