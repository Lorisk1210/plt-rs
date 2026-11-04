"""
pltrs (Python): matplotlib's API, drawn by pltrs (Rust).

Code written for matplotlib runs on pltrs with one change, the import::

    import pltrs.pyplot as plt          # import matplotlib.pyplot as plt
    from pltrs import colors            # from matplotlib import colors

The classes and functions here keep matplotlib's names, signatures and
behaviour; the work (validation, layout, drawing, saving) is done by
pltrs (Rust), a reimplementation of matplotlib, through the native module
``pltrs._native``. See ``python/DESIGN.md`` for how the two fit together.

Figures take pltrs' modern look by default, as in pltrs (Rust): the
``pltrs-modern`` style sheet, Inter type and constrained layout. Setting
the environment variable ``PLTRS_LOOK=matplotlib`` before the import
selects matplotlib's look, in which the output is matplotlib's.
``rcParamsDefault``, `rcdefaults` and ``style.use("default")`` give
matplotlib's defaults in either look.

This module holds what matplotlib's top level holds: ``rcParams`` and its
helpers (``rc``, ``rc_context``, ``rcdefaults``), backend selection
(``use``, ``get_backend``) and ``__version__``.
"""

from collections import namedtuple
import atexit
import contextlib
import functools
import logging
import os
import pprint
import re
import shutil
import subprocess  # noqa: F401  (matplotlib.subprocess, which tests mock)
import sys
from collections.abc import MutableMapping

from . import _api
from ._api import MatplotlibDeprecationWarning  # noqa: F401  (matplotlib exports it here)
from ._executables import ExecutableNotFoundError, _get_executable_info  # noqa: F401
from ._preprocess import _preprocess_data  # noqa: F401
from ._native import cbook as _cbook
from ._native import patheffects as _patheffects
from ._native import rcsetup as _rcsetup
from ._native import register as _register
from . import rcsetup
from .rcsetup import cycler  # noqa: F401  (matplotlib exports it here)

import numpy as _np

# The exceptions of numpy's linear algebra (singular matrices), which
# pltrs (Rust) raises where matplotlib calls numpy.
_register("LinAlgError", _np.linalg.LinAlgError)


def _log_warning(module, message):
    """Log *message* as a warning of the package's module *module*
    (``"backends.backend_ps"``), where matplotlib's module logs it: pltrs
    (Rust) logs through this."""
    logging.getLogger(f"{__name__}.{module}").warning(message)


_register("log_warning", _log_warning)

_log = logging.getLogger(__name__)

_VersionInfo = namedtuple('_VersionInfo',
                          'major, minor, micro, releaselevel, serial')

#: The parts of a PEP 440 version string that matplotlib's versions use.
_VERSION_PATTERN = re.compile(r"""
    (?P<release>\d+(?:\.\d+)*)
    (?:(?P<pre_l>a|b|rc)(?P<pre_n>\d+))?
    (?:\.post(?P<post>\d+))?
    (?:\.dev(?P<dev>\d+))?
    (?:\+[a-z0-9.]+)?$""", re.VERBOSE | re.IGNORECASE)


def _parse_to_version_info(version_str):
    """
    Parse a version string to a namedtuple analogous to sys.version_info.

    See:
    https://packaging.pypa.io/en/latest/version.html#packaging.version.parse
    https://docs.python.org/3/library/sys.html#sys.version_info
    """
    match = _VERSION_PATTERN.match(version_str)
    if match is None:
        raise ValueError(f"Invalid version: {version_str!r}")
    major, minor, micro = ([*map(int, match['release'].split('.')), 0, 0])[:3]
    if match['pre_l'] is None and match['post'] is None and match['dev'] is None:
        return _VersionInfo(major, minor, micro, 'final', 0)
    elif match['dev'] is not None:
        return _VersionInfo(major, minor, micro, 'alpha', int(match['dev']))
    elif match['pre_l'] is not None:
        releaselevel = {
            'a': 'alpha',
            'b': 'beta',
            'rc': 'candidate'}[match['pre_l'].lower()]
        return _VersionInfo(major, minor, micro, releaselevel,
                            int(match['pre_n']))
    else:
        # fallback for post: guess-next-dev scheme from setuptools_scm
        return _VersionInfo(major, minor, micro + 1, 'alpha', int(match['post']))


#: The matplotlib version whose API pltrs implements.
__version__ = "3.11.2"
__version_info__ = _parse_to_version_info(__version__)

#: The citation of matplotlib, whose API pltrs implements.
__bibtex__ = r"""@Article{Hunter:2007,
  Author    = {Hunter, J. D.},
  Title     = {Matplotlib: A 2D graphics environment},
  Journal   = {Computing in Science \& Engineering},
  Volume    = {9},
  Number    = {3},
  Pages     = {90--95},
  abstract  = {Matplotlib is a 2D graphics package used for Python
  for application development, interactive scripting, and
  publication-quality image generation across user
  interfaces and operating systems.},
  publisher = {IEEE COMPUTER SOC},
  year      = 2007
}"""

#: Set by the test configuration (``testing.conftest``) while pytest runs.
_called_from_pytest = False

#: rcParams pltrs keeps in Python: the backend is chosen by Python code,
#: by pyplot the first time it needs one unless one is set (matplotlib's
#: ``rcsetup._auto_backend_sentinel``).
_PYTHON_PARAMS = {"backend": rcsetup._auto_backend_sentinel}

#: The rcParams pltrs (Rust) keeps (matplotlib's, without pltrs (Rust)'s own).
_NATIVE_KEYS = frozenset(_rcsetup.keys())

#: rcParams whose matplotlib validators warn about a valid value (``toolbar
#: = 'toolmanager'``): pltrs (Rust) validates every value, and these
#: validators run again on assignment for their warnings, but not on
#: reading (they choose among keywords, which pltrs (Rust) keeps in
#: matplotlib's spelling).
_WARNING_VALIDATORS = frozenset({"toolbar"})

#: rcParams that ``rcdefaults`` and styles leave alone (matplotlib's
#: ``style.core.STYLE_BLACKLIST``).
_STYLE_BLACKLIST = {
    "interactive", "backend", "webagg.port", "webagg.address",
    "webagg.port_retries", "webagg.open_in_browser", "backend_fallback",
    "toolbar", "timezone", "figure.max_open_warning", "figure.raise_window",
    "savefig.directory", "tk.window_focus", "docstring.hardcopy",
    "date.epoch",
}


#: The types of parameter values the current `RcParams` reuses until the
#: parameters change (a program may change a list or dict it was given).
_IMMUTABLE_VALUES = frozenset([bool, int, float, str, type(None)])


class RcParams(MutableMapping, dict):
    """
    A dict-like key-value store for config parameters, including validation.

    Values are validated and stored by pltrs (Rust): assigning an invalid value
    raises the ``ValueError`` matplotlib raises, an unknown key the
    ``KeyError``. Reading a value gives matplotlib's Python value: the
    parameter's validator (`.rcsetup`) applied to the rc text pltrs (Rust)
    keeps, or to the value given, while pltrs (Rust) holds that value (so
    that ``'dashed'`` stays ``'dashed'`` where pltrs (Rust) keeps ``--``).
    ``matplotlib.rcParams`` holds the current parameters; other instances
    (``rcParamsDefault``, ``RcParams({...})``, copies) hold their own, and
    only the keys given to them.
    """

    validate = rcsetup._validators

    def __init__(self, *args, **kwargs):
        self._store = _rcsetup.defaults()
        self._keys = set()
        self._python = {}
        self._given = {}
        self.update(*args, **kwargs)

    @classmethod
    def _current(cls):
        """The instance holding the current parameters (``rcParams``)."""
        params = cls.__new__(cls)
        params._store = None
        params._keys = None
        params._python = dict(_PYTHON_PARAMS)
        params._given = {}
        # Immutable values read, by key, with the generation of the
        # parameters they were read from (`_rcsetup.generation`).
        params._read = {}
        return params

    def _has(self, key):
        """Whether *key* is in this mapping."""
        if key in _PYTHON_PARAMS:
            return key in self._python
        if self._keys is None:
            return key in _NATIVE_KEYS
        return key in self._keys

    def _set(self, key, val):
        """Set *key* without the deprecation checks (matplotlib's ``_set``)."""
        if key in _PYTHON_PARAMS:
            self._python[key] = val
            return
        if key == "path.effects":
            # Path effect objects, which rc text cannot spell.
            _patheffects.set_rc_path_effects(
                self._store, [effect._native_spec() for effect in val])
        elif self._typed_color(key, val):
            # A colour as a tuple, which matplotlib's validator of the key
            # takes from Python but not as rc text (``legend.labelcolor``).
            _rcsetup.set_color(self._store, key, self.validate[key](val))
        else:
            _rcsetup.set(self._store, key, rcsetup._rc_value(val))
        self._given[key] = (val, _rcsetup.text(self._store, key))
        if self._keys is not None:
            self._keys.add(key)

    def _typed_color(self, key, val):
        """Whether *val* is a colour that is no string, for a key whose
        matplotlib validator takes it but whose rc text parser (in pltrs
        (Rust), as matplotlib's for rc files) refuses its spelling."""
        if isinstance(val, str) or not _np.iterable(val):
            return False
        from .colors import is_color_like
        validator = self.validate.get(key)
        if validator is None or not is_color_like(val):
            return False
        try:
            validator(val)
        except ValueError:
            return False  # refused as rc text too, with matplotlib's message
        try:
            _rcsetup.set(_rcsetup.copy(self._store), key, rcsetup._rc_value(val))
        except ValueError:
            return True
        return False

    def _get(self, key):
        """Get *key* without the deprecation checks (matplotlib's ``_get``)."""
        if self._store is not None or key in _PYTHON_PARAMS:
            return self._get_uncached(key)
        # The current parameters: a value read before holds until they change.
        generation = _rcsetup.generation()
        read = self._read.get(key)
        if read is not None and read[0] == generation:
            return read[1]
        value = self._get_uncached(key)
        if type(value) in _IMMUTABLE_VALUES:
            self._read[key] = (generation, value)
        return value

    def _get_uncached(self, key):
        """*key*'s value, read from pltrs (Rust) and validated."""
        if not self._has(key):
            # matplotlib's dict lookup: the error names the key alone.
            raise KeyError(key)
        if key in _PYTHON_PARAMS:
            return self._python[key]
        validator = self.validate.get(key)
        if validator is None:
            return _rcsetup.get(self._store, key)
        text = _rcsetup.text(self._store, key)
        if key in _WARNING_VALIDATORS:
            return text
        given, held = self._given.get(key, (text, text))
        return validator(given if held == text else text)

    def __setitem__(self, key, val):
        if (key == "backend" and val is rcsetup._auto_backend_sentinel
                and "backend" in self):
            return
        if key not in _PYTHON_PARAMS and not key.startswith("pltrs."):
            _api.getitem_checked(self.validate, rcParam=key, _error_cls=KeyError)
        self._set(key, val)
        if key in _WARNING_VALIDATORS:
            self.validate[key](val)
        if key in {"text.hinting_factor", "text.kerning_factor"} and val is not None:
            _api.warn_deprecated("3.11", name=key, obj_type="rcParam")

    def __getitem__(self, key):
        # In theory, this should only ever be used after the global rcParams
        # has been set up, but better be safe e.g. in presence of breakpoints.
        if key == "backend" and self is globals().get("rcParams"):
            val = self._get(key)
            if val is rcsetup._auto_backend_sentinel:
                from . import pyplot as plt
                plt.switch_backend(rcsetup._auto_backend_sentinel)
        return self._get(key)

    def _get_backend_or_none(self):
        """Get the requested backend, if any, without triggering resolution."""
        backend = self._get("backend")
        return None if backend is rcsetup._auto_backend_sentinel else backend

    def __delitem__(self, key):
        if self._keys is None or key not in self._keys:
            raise KeyError(key)
        self._keys.discard(key)

    def __iter__(self):
        keys = _NATIVE_KEYS if self._keys is None else self._keys
        yield from sorted(keys | self._python.keys())

    def __len__(self):
        return sum(1 for _ in self)

    def __repr__(self):
        class_name = self.__class__.__name__
        indent = len(class_name) + 1
        with _api.suppress_matplotlib_deprecation_warning():
            repr_split = pprint.pformat(dict(self), indent=1,
                                        width=80 - indent).split('\n')
        repr_indented = ('\n' + ' ' * indent).join(repr_split)
        return f'{class_name}({repr_indented})'

    def __str__(self):
        return '\n'.join(map('{0[0]}: {0[1]}'.format, sorted(self.items())))

    def update(self, other=(), /, **kwargs):
        """Update from a mapping or pairs, validating each value; another
        `RcParams` is copied as pltrs (Rust) holds it."""
        if isinstance(other, RcParams):
            self._update_from(other, list(other))
            other = ()
        super().update(other, **kwargs)

    def _update_from(self, other, keys):
        """Copy *keys* of the `RcParams` *other* into this one."""
        native = [key for key in keys if key not in _PYTHON_PARAMS]
        _rcsetup.update_from(self._store, other._store, native)
        if self._keys is not None:
            self._keys.update(native)
        for key in native:
            self._given.pop(key, None)
            if key in other._given:
                self._given[key] = other._given[key]
        for key in keys:
            if key in _PYTHON_PARAMS:
                self._python[key] = other._get(key)

    def __or__(self, other):
        merged = dict(self)
        merged.update(other)
        return merged

    def __ror__(self, other):
        merged = dict(other)
        merged.update(self)
        return merged

    def __ior__(self, other):
        self.update(other)
        return self

    def __copy__(self):
        return self.copy()

    def __deepcopy__(self, memo):
        return self.copy()

    def __reduce__(self):
        return RcParams, (dict(self),)

    def find_all(self, pattern):
        """
        Return the subset of this RcParams dictionary whose keys match,
        using :func:`re.search`, the given ``pattern``.
        """
        pattern_re = re.compile(pattern)
        return RcParams((key, value) for key, value in self.items()
                        if pattern_re.search(key))

    def copy(self):
        """Copy this RcParams instance."""
        params = RcParams.__new__(RcParams)
        params._store = _rcsetup.copy(self._store)
        params._keys = set(self) - _PYTHON_PARAMS.keys()
        params._python = dict(self._python)
        params._given = dict(self._given)
        return params


def _of(store):
    """An RcParams with every parameter of the native *store*."""
    params = RcParams.__new__(RcParams)
    params._store = store
    params._keys = set(_NATIVE_KEYS)
    params._python = dict(_PYTHON_PARAMS)
    params._given = {}
    return params


def _default_spellings():
    """
    matplotlib's defaults as matplotlib spells them (``'dashed'`` where
    pltrs (Rust) keeps ``--``), by parameter, with the rc text pltrs (Rust)
    holds for each (see `RcParams`): what `RcParams` gives back while a
    parameter keeps its default.
    """
    scratch = RcParams()
    for param in rcsetup._params_list():
        if param.name not in _PYTHON_PARAMS and param.default is not None:
            scratch._set(param.name, param.default)
    return scratch._given


rcParams = RcParams._current()
rcParamsDefault = _of(_rcsetup.defaults())
rcParamsDefault._given = _default_spellings()
rcParams._given = dict(rcParamsDefault._given)
rcParamsOrig = rcParams.copy()
with _api.suppress_matplotlib_deprecation_warning():
    # Assigning to rcsetup.defaultParams is left only for backcompat.
    defaultParams = rcsetup.defaultParams = {
        key: [(rcsetup._auto_backend_sentinel if key == "backend" else
               rcParamsDefault[key]),
              validator]
        for key, validator in rcsetup._validators.items()}


def rc(group, **kwargs):
    """
    Set the current `.rcParams`.  *group* is the grouping for the rc, e.g.,
    for ``lines.linewidth`` the group is ``lines``, for
    ``axes.facecolor``, the group is ``axes``, and so on.  Group may
    also be a list or tuple of group names, e.g., (*xtick*, *ytick*).
    *kwargs* is a dictionary attribute name/value pairs.
    """
    aliases = {
        'lw': 'linewidth', 'ls': 'linestyle', 'c': 'color',
        'fc': 'facecolor', 'ec': 'edgecolor', 'mew': 'markeredgewidth',
        'aa': 'antialiased',
        'sans': 'sans-serif',
    }
    if isinstance(group, str):
        group = (group,)
    for g in group:
        for k, v in kwargs.items():
            name = aliases.get(k) or k
            key = f'{g}.{name}'
            try:
                rcParams[key] = v
            except KeyError as err:
                raise KeyError(('Unrecognized key "%s" for group "%s" and '
                                'name "%s"') % (key, g, name)) from err


def rcdefaults():
    """
    Restore the `.rcParams` from Matplotlib's internal default style.

    Style-blacklisted `.rcParams` (defined in
    ``matplotlib.style.core.STYLE_BLACKLIST``) are not updated.
    """
    kept = {key: rcParams[key] for key in _STYLE_BLACKLIST if key in rcParams}
    _rcsetup.replace(_rcsetup.defaults())
    rcParams._given = dict(rcParamsDefault._given)
    for key, value in kept.items():
        rcParams._set(key, value)


@contextlib.contextmanager
def rc_context(rc=None, fname=None):
    """
    Return a context manager for temporarily changing rcParams.

    The :rc:`backend` will not be reset by the context manager.

    rcParams changed both through the context manager invocation and
    in the body of the context will be reset on context exit.

    Parameters
    ----------
    rc : dict
        The rcParams to temporarily set.
    fname : str or path-like
        A file with Matplotlib rc settings. If both *fname* and *rc* are given,
        settings from *rc* take precedence.
    """
    saved = _rcsetup.copy(None)
    given = dict(rcParams._given)
    try:
        if fname:
            rc_file(fname)
        if rc:
            rcParams.update(rc)
        yield
    finally:
        _rcsetup.replace(saved)
        rcParams._given = given


def _local_path(fname):
    """The path of the file *fname* names: a path, or a ``file:`` URL
    (matplotlib's ``_open_file_or_url`` for local files)."""
    if isinstance(fname, str) and fname.startswith('file:'):
        from urllib.parse import unquote, urlparse
        from urllib.request import url2pathname
        return url2pathname(unquote(urlparse(fname).path))
    return os.path.expanduser(os.fspath(fname))


def _rc_params_in_file(fname, fail_on_error=False):
    """
    Construct a `RcParams` instance from file *fname*.

    Unlike `rc_params_from_file`, the configuration class only contains the
    parameters specified in the file (i.e. default values are not filled in).
    pltrs (Rust) reads the file; the lines matplotlib skips are logged with
    matplotlib's messages, or raise with *fail_on_error*.
    """
    entries, warnings = _rcsetup.read_file(_local_path(fname))
    for kind, message in warnings:
        if fail_on_error and kind == "invalid value":
            raise ValueError(message)
        _log.warning(message)
    config = RcParams()
    for key, value in entries:
        config._set(key, value)
    return config


def rc_params_from_file(fname, fail_on_error=False, use_default_template=True):
    """
    Construct a `RcParams` from file *fname*.

    Parameters
    ----------
    fname : str or path-like
        A file with Matplotlib rc settings.
    fail_on_error : bool
        If True, raise an error when the parser fails to convert a parameter.
    use_default_template : bool
        If True, initialize with default parameters before updating with those
        in the given file. If False, the configuration class only contains the
        parameters specified in the file. (Useful for updating dicts.)
    """
    config_from_file = _rc_params_in_file(fname, fail_on_error=fail_on_error)
    if not use_default_template:
        return config_from_file
    config = rcParamsDefault.copy()
    config.update(config_from_file)
    _log.debug('loaded rc file %s', fname)
    return config


def matplotlib_fname():
    """
    Get the location of the config file.

    The file location is determined in the following order

    - ``$PWD/pltrsrc``
    - ``$PLTRSRC`` if it is not a directory
    - ``$PLTRSRC/pltrsrc``
    - ``pltrsrc`` in the configuration directory (`get_configdir`)
    - Lastly, the template of the defaults in the data directory
      (`get_data_path`), which pltrs (Rust) writes there, as matplotlib's
      ``mpl-data/matplotlibrc``.
    """
    return _cbook.rc_file_or_template()


def rc_params(fail_on_error=False):
    """Construct a `RcParams` instance from the default Matplotlib rc file."""
    return rc_params_from_file(matplotlib_fname(), fail_on_error)


def rc_file(fname, *, use_default_template=True):
    """
    Update `.rcParams` from file.

    Style-blacklisted `.rcParams` (defined in
    ``matplotlib.style.core.STYLE_BLACKLIST``) are not updated.

    Parameters
    ----------
    fname : str or path-like
        A file with Matplotlib rc settings.

    use_default_template : bool
        If True, initialize with default parameters before updating with those
        in the given file. If False, the current configuration persists
        and only the parameters specified in the file are updated.
    """
    rc_from_file = rc_params_from_file(
        fname, use_default_template=use_default_template)
    rcParams._update_from(
        rc_from_file, [key for key in rc_from_file if key not in _STYLE_BLACKLIST])


def rc_file_defaults():
    """
    Restore the `.rcParams` from the original rc file loaded by Matplotlib.

    Style-blacklisted `.rcParams` (defined in
    ``matplotlib.style.core.STYLE_BLACKLIST``) are not updated.
    """
    rcParams._update_from(
        rcParamsOrig, [key for key in rcParamsOrig if key not in _STYLE_BLACKLIST])


@functools.cache
def _ensure_handler():
    """
    The first time this function is called, attach a `StreamHandler` using the
    same format as `logging.basicConfig` to the Matplotlib root logger.

    Return this handler every time this function is called.
    """
    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter(logging.BASIC_FORMAT))
    _log.addHandler(handler)
    return handler


def set_loglevel(level):
    """
    Configure Matplotlib's logging levels.

    Matplotlib uses the standard library `logging` framework under the root
    logger 'matplotlib'.  This is a helper function to:

    - set Matplotlib's root logger level
    - set the root logger handler's level, creating the handler
      if it does not exist yet

    Parameters
    ----------
    level : {"NOTSET", "DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}
        The log level as defined in `Python logging levels
        <https://docs.python.org/3/library/logging.html#logging-levels>`__.
    """
    _log.setLevel(level.upper())
    _ensure_handler().setLevel(level.upper())


def interactive(b):
    """
    Set whether to redraw after every plotting command (e.g. `.pyplot.xlabel`).
    """
    rcParams['interactive'] = b


def is_interactive():
    """
    Return whether to redraw after every plotting command.

    .. note::

        This function is only intended for use in backends. End users should
        use `.pyplot.isinteractive` instead.
    """
    return rcParams['interactive']


def use(backend, *, force=True):
    """
    Select the backend used for rendering and GUI integration.

    If pyplot is already imported, `~matplotlib.pyplot.switch_backend` is used
    to switch the backend.

    Parameters
    ----------
    backend : str
        The backend to switch to.  This can either be one of the standard
        backend names, which are case-insensitive:

        - interactive backends:
          GTK3Agg, GTK3Cairo, GTK4Agg, GTK4Cairo, MacOSX, nbAgg, notebook, QtAgg,
          QtCairo, TkAgg, TkCairo, WebAgg, WX, WXAgg, WXCairo, Qt5Agg, Qt5Cairo

        - non-interactive backends:
          agg, cairo, pdf, pgf, ps, svg, template

        or a string of the form: ``module://my.module.name``.

        notebook is a synonym for nbAgg.

        Switching to an interactive backend is not possible if an unrelated
        event loop has already been started (e.g., switching to GTK3Agg if a
        TkAgg window has already been opened).  Switching to a non-interactive
        backend is always possible.

    force : bool, default: True
        If True (the default), raise an `ImportError` if the backend cannot be
        set up (either because it fails to import, or because an incompatible
        GUI interactive framework is already running); if False, silently
        ignore the failure.
    """
    from .backends import backend_registry
    if not backend_registry.is_valid_backend(backend):
        raise ValueError(
            f"'{backend}' is not a valid value for backend; supported values "
            f"are {backend_registry.list_all()}")
    if rcParams._get('backend') != backend:
        # if pyplot is not already imported, do not import it.  Doing
        # so may trigger a `plt.switch_backend` to the _default_ backend
        # before we get a chance to change to the one the user just requested
        plt = sys.modules.get(f'{__name__}.pyplot')
        # if pyplot is imported, then try to change backends
        if plt is not None:
            try:
                # we need this import check here to re-raise if the
                # user does not have the libraries to support their
                # chosen backend installed.
                plt.switch_backend(backend)
            except ImportError:
                if force:
                    raise
        # if we have not imported pyplot, then we can set the rcParam
        # value which will be respected when the user finally imports
        # pyplot
        else:
            rcParams['backend'] = backend
    # if the user has asked for a given backend, do not helpfully
    # fallback
    rcParams['backend_fallback'] = False


if os.environ.get('MPLBACKEND'):
    rcParams['backend'] = os.environ.get('MPLBACKEND')


def get_backend(*, auto_select=True):
    """
    Return the name of the current backend.

    Parameters
    ----------
    auto_select : bool, default: True
        Whether to trigger backend resolution if no backend has been
        selected so far. If True, this ensures that a valid backend
        is returned. If False, this returns None if no backend has been
        selected so far.
    """
    if auto_select:
        return rcParams['backend']
    backend = rcParams._get('backend')
    if backend is rcsetup._auto_backend_sentinel:
        return None
    return backend


def _val_or_rc(val, rc_name):
    """
    If *val* is None, the value of :rc:`rc_name`, else *val*.
    """
    return val if val is not None else rcParams[rc_name]


def _init_tests():
    """
    Prepare the test environment: add the ``_classic_test`` style (the
    ``classic`` style with ``_classic_test_patch``) to the style library.
    matplotlib also checks its FreeType build here; pltrs (Rust) ports
    FreeType, so there is nothing to check.
    """
    from .style import _base_library, library
    classic_test = RcParams()
    classic_test.update(_base_library['classic'])
    classic_test.update(_base_library['_classic_test_patch'])
    _base_library['_classic_test'] = library['_classic_test'] = classic_test


def get_configdir():
    """
    Return the string path of the configuration directory: the one
    ``PLTRSCONFIGDIR`` names, else the one matplotlib's ``MPLCONFIGDIR``
    names, else on Linux ``$XDG_CONFIG_HOME/pltrs`` (by default
    ``~/.config/pltrs``) and elsewhere ``~/.pltrs``, made if missing; a
    temporary directory when it cannot be made or written to.
    """
    return _cbook.config_dir()


def get_cachedir():
    """
    Return the string path of the cache directory, found as the
    configuration directory is (see `get_configdir`), on Linux in
    ``$XDG_CACHE_HOME`` (by default ``~/.cache``).
    """
    return _cbook.cache_dir()


def _log_temporary_config_dir():
    """
    Log, as matplotlib does on import, why a temporary directory stands in
    for the configuration directory, if one does: the directory
    ``PLTRSCONFIGDIR`` names, else the one matplotlib's ``MPLCONFIGDIR``
    names, else the default one cannot be made or written to. As
    matplotlib's, the temporary directory is removed when Python exits.
    """
    notice = _cbook.temp_dir_notice()
    if notice is not None:
        tmpdir, message = notice
        atexit.register(shutil.rmtree, tmpdir, ignore_errors=True)
        _log.warning(message)


_log_temporary_config_dir()


# The colormap and colour sequence registries, as matplotlib exports them.
from .cm import _colormaps as colormaps  # noqa: E402
from .cm import _multivar_colormaps as multivar_colormaps  # noqa: E402
from .cm import _bivar_colormaps as bivar_colormaps  # noqa: E402
from .colors import _color_sequences as color_sequences  # noqa: E402


def get_data_path():
    """Return the path to Matplotlib data: the directory where pltrs (Rust)
    writes the data files it bundles (its fonts), as matplotlib's
    ``mpl-data``, which also reaches the files the package ships (the
    sample data, the toolbar images)."""
    from ._native import font_manager as _font_manager
    path = _font_manager.data_path()
    _reach_shipped_data(path)
    return path


@functools.cache
def _reach_shipped_data(path):
    """Link the entries of the package's ``mpl-data`` into the data directory
    *path* (copying them where links cannot be made), as programs join
    ``get_data_path()`` and ``"sample_data"``.

    The data directory is shared by every installation of pltrs, so a link
    may lead to another installation's files, or to one since removed: such
    a link is pointed at this package's files instead."""
    from pathlib import Path
    for entry in Path(__file__).with_name("mpl-data").iterdir():
        target = Path(path, entry.name)
        if target.is_symlink():
            if target.resolve() != entry.resolve():
                _replace_link(entry, target)
        elif not target.exists():
            try:
                target.symlink_to(entry, target_is_directory=entry.is_dir())
            except FileExistsError:
                pass  # Another process linked it meanwhile.
            except OSError:
                pass  # Windows makes links only with a privilege.
            if not target.exists():
                # Copied instead (also under Wine, which reports links it
                # does not make).
                shutil.copytree(entry, target, dirs_exist_ok=True)


def _replace_link(entry, target):
    """Point the link *target* at *entry*, replacing it in one step, so that
    other processes always find a link there."""
    temporary = target.with_name(f".{target.name}.{os.getpid()}")
    try:
        temporary.unlink(missing_ok=True)
        temporary.symlink_to(entry, target_is_directory=entry.is_dir())
        os.replace(temporary, target)
    except OSError:
        temporary.unlink(missing_ok=True)
