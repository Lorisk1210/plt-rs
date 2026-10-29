"""
pltrs (Python): matplotlib's API, drawn by pltrs (Rust).

Code written for matplotlib runs on pltrs with one change, the import::

    import pltrs.pyplot as plt          # import matplotlib.pyplot as plt
    from pltrs import colors            # from matplotlib import colors

The classes and functions here keep matplotlib's names, signatures and
behaviour; the work (validation, layout, drawing, saving) is done by
pltrs (Rust), a reimplementation of matplotlib, through the native module
``pltrs._native``. See ``python/DESIGN.md`` for how the two fit together.

This module holds what matplotlib's top level holds: ``rcParams`` and its
helpers (``rc``, ``rc_context``, ``rcdefaults``), backend selection
(``use``, ``get_backend``) and ``__version__``.
"""

import contextlib
import pprint
import re
import sys
from collections.abc import MutableMapping

from . import _api
from ._api import MatplotlibDeprecationWarning  # noqa: F401  (matplotlib exports it here)
from ._executables import ExecutableNotFoundError, _get_executable_info  # noqa: F401
from ._preprocess import _preprocess_data  # noqa: F401
from ._native import cbook as _cbook
from ._native import rcsetup as _rcsetup
from ._native import register as _register

import numpy as _np

# The exceptions of numpy's linear algebra (singular matrices), which
# pltrs (Rust) raises where matplotlib calls numpy.
_register("LinAlgError", _np.linalg.LinAlgError)

#: The matplotlib version whose API pltrs implements.
__version__ = "3.11.2"
__version_info__ = (3, 11, 2, "final", 0)

#: Set by the test configuration (``testing.conftest``) while pytest runs.
_called_from_pytest = False

#: rcParams pltrs keeps in Python: the backend is chosen by Python code.
_PYTHON_PARAMS = {"backend": "agg"}

#: The rcParams pltrs (Rust) keeps (matplotlib's, without pltrs (Rust)'s own).
_NATIVE_KEYS = frozenset(_rcsetup.keys())

#: rcParams that ``rcdefaults`` and styles leave alone (matplotlib's
#: ``style.core.STYLE_BLACKLIST``).
_STYLE_BLACKLIST = {
    "interactive", "backend", "webagg.port", "webagg.address",
    "webagg.port_retries", "webagg.open_in_browser", "backend_fallback",
    "toolbar", "timezone", "figure.max_open_warning", "figure.raise_window",
    "savefig.directory", "tk.window_focus", "docstring.hardcopy",
    "date.epoch",
}


class RcParams(MutableMapping, dict):
    """
    A dict-like key-value store for config parameters, including validation.

    Values are validated and stored by pltrs (Rust): assigning an invalid value
    raises the ``ValueError`` matplotlib raises, an unknown key the
    ``KeyError``. ``matplotlib.rcParams`` holds the current parameters;
    other instances (``rcParamsDefault``, ``RcParams({...})``, copies) hold
    their own, and only the keys given to them.
    """

    def __init__(self, *args, **kwargs):
        self._store = _rcsetup.defaults()
        self._keys = set()
        self._python = {}
        self.update(*args, **kwargs)

    @classmethod
    def _current(cls):
        """The instance holding the current parameters (``rcParams``)."""
        params = cls.__new__(cls)
        params._store = None
        params._keys = None
        params._python = dict(_PYTHON_PARAMS)
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
        _rcsetup.set(self._store, key, val)
        if self._keys is not None:
            self._keys.add(key)

    def _get(self, key):
        """Get *key* without the deprecation checks (matplotlib's ``_get``)."""
        if not self._has(key):
            raise KeyError(
                f"{key} is not a valid rc parameter (see rcParams.keys() for "
                f"a list of valid parameters)")
        if key in _PYTHON_PARAMS:
            return self._python[key]
        return _rcsetup.get(self._store, key)

    def __setitem__(self, key, val):
        self._set(key, val)

    def __getitem__(self, key):
        return self._get(key)

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
        return params


def _of(store):
    """An RcParams with every parameter of the native *store*."""
    params = RcParams.__new__(RcParams)
    params._store = store
    params._keys = set(_NATIVE_KEYS)
    params._python = dict(_PYTHON_PARAMS)
    return params


rcParams = RcParams._current()
rcParamsDefault = _of(_rcsetup.defaults())
rcParamsOrig = rcParams.copy()


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
    for key, value in kept.items():
        rcParams._set(key, value)


@contextlib.contextmanager
def rc_context(rc=None, fname=None):
    """
    Return a context manager for temporarily changing rcParams.

    The :rc:`backend` will not be reset by the context manager.
    """
    if fname is not None:
        raise NotImplementedError("pltrs does not read rc files yet")
    saved = _rcsetup.copy(None)
    try:
        if rc:
            rcParams.update(rc)
        yield
    finally:
        _rcsetup.replace(saved)


def use(backend, *, force=True):
    """
    Select the backend used for rendering and GUI integration.

    If pyplot is already imported, `~matplotlib.pyplot.switch_backend` is used
    to switch the backend.

    Parameters
    ----------
    backend : str
        The backend to switch to: one of the standard backend names, which
        are case-insensitive, or a string of the form
        ``module://my.module.name``.
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


def get_backend(*, auto_select=True):
    """Return the name of the current backend."""
    return rcParams['backend']


def _val_or_rc(val, rc_name):
    """
    If *val* is None, the value of :rc:`rc_name`, else *val*.
    """
    return val if val is not None else rcParams[rc_name]


def interactive(b):
    """
    Set whether to redraw after every plotting command (e.g. `.pyplot.xlabel`).
    """
    rcParams['interactive'] = b


def is_interactive():
    """
    Return whether to redraw after every plotting command.
    """
    return rcParams['interactive']


def _init_tests():
    """Check the test environment (matplotlib checks its FreeType build;
    pltrs (Rust) ports FreeType, so there is nothing to check)."""


def get_configdir():
    """
    Return the string path of the configuration directory: the one
    ``PLTRSCONFIGDIR`` names, else ``$XDG_CONFIG_HOME/pltrs`` (by default
    ``~/.config/pltrs``), made if missing; a temporary directory when it
    cannot be made or written to.
    """
    return _cbook.config_dir()


def get_cachedir():
    """
    Return the string path of the cache directory, found as the
    configuration directory is (see `get_configdir`) in
    ``$XDG_CACHE_HOME`` (by default ``~/.cache``).
    """
    return _cbook.cache_dir()


# The colormap and colour sequence registries, as matplotlib exports them.
from .cm import _colormaps as colormaps  # noqa: E402
from .cm import _multivar_colormaps as multivar_colormaps  # noqa: E402
from .cm import _bivar_colormaps as bivar_colormaps  # noqa: E402
from .colors import _color_sequences as color_sequences  # noqa: E402
