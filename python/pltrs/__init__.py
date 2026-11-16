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

# What ``from matplotlib import *`` takes: matplotlib's list.
__all__ = [
    "__bibtex__",
    "__version__",
    "__version_info__",
    "set_loglevel",
    "ExecutableNotFoundError",
    "get_configdir",
    "get_cachedir",
    "get_data_path",
    "matplotlib_fname",
    "MatplotlibDeprecationWarning",
    "RcParams",
    "rc_params",
    "rc_params_from_file",
    "rcParamsDefault",
    "rcParams",
    "rcParamsOrig",
    "defaultParams",
    "rc",
    "rcdefaults",
    "rc_file_defaults",
    "rc_file",
    "rc_context",
    "use",
    "get_backend",
    "interactive",
    "is_interactive",
    "colormaps",
    "multivar_colormaps",
    "bivar_colormaps",
    "color_sequences",
]

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
from ._preprocess import _label_from_arg, _preprocess_data, _replacer  # noqa: F401
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

# pltrs (Rust) computes at the processor level numpy runs at (DESIGN.md,
# "Processor levels"): the copy of its loops numpy dispatches to.
from ._native import numeric as _numeric  # noqa: E402



def _numpy_runs(*names):
    """For each of *names* (a group or a feature of numpy's dispatch),
    whether numpy runs its copy of its loops: the processor has it and
    numpy holds a copy for it (its baseline and dispatch lists: a wheel
    may hold none, as numpy's for Windows holds none for ``X86_V4``);
    `None` for a name this numpy does not know."""
    umath = (_np._core if hasattr(_np, "_core") else _np.core)._multiarray_umath
    features = umath.__cpu_features__
    copies = {*umath.__cpu_baseline__, *umath.__cpu_dispatch__}
    return [name in copies and bool(features[name]) if name in features else None
            for name in names]


_numeric.follow_numpy(*_numpy_runs("X86_V3", "X86_V4", "AVX2", "AVX512_SKX"))
# ... and orders equal values as this numpy's argsort does: numpy's C
# method sorts what is longer than pltrs (Rust) replays.
_register("argsort", _np.ndarray.argsort)


def _silently(ufunc):
    """*ufunc* without numpy's warnings: pltrs (Rust) issues those of the
    computations it replays itself."""
    def call(*arrays):
        with _np.errstate(all="ignore"):
            return ufunc(*arrays)
    return call


# numpy's own values where their last bit decides something discrete (the
# power a tick is rounded to, an edge on a pixel's centre).
_numeric.use_ufuncs(*map(_silently, (_np.arcsinh, _np.sinh, _np.log, _np.log2, _np.log10,
                                     _np.exp, _np.exp2, _np.power)))


def _log_warning(module, message):
    """Log *message* as a warning of the package's module *module*
    (``"backends.backend_ps"``), where matplotlib's module logs it: pltrs
    (Rust) logs through this."""
    logging.getLogger(f"{__name__}.{module}").warning(message)


_register("log_warning", _log_warning)

_log = logging.getLogger(__name__)

_VersionInfo = namedtuple('_VersionInfo',
                          'major, minor, micro, releaselevel, serial')

@functools.cache
def _version_pattern():
    """The parts of a PEP 440 version string that matplotlib's versions use."""
    return re.compile(r"""
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
    release = version_str.split(".")
    if all(part.isdecimal() for part in release):
        # A final release (as pltrs's own version is), without the pattern.
        major, minor, micro = ([*map(int, release), 0, 0])[:3]
        return _VersionInfo(major, minor, micro, 'final', 0)
    match = _version_pattern().match(version_str)
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


def parse_version(version):
    """
    The version *version* read as `packaging.version.parse` reads it,
    which matplotlib's ``parse_version`` is; packaging is imported at the
    first call (pltrs needs it nowhere else).
    """
    from packaging.version import parse
    return parse(version)


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
_PYTHON_PARAMS = {
    "backend": rcsetup._auto_backend_sentinel,
    # A list programs change in place (``rcParams['figure.hooks'].append``,
    # the gallery's mplcvd), which pltrs (Rust) never reads.
    "figure.hooks": [],
}


#: The rcParams pltrs (Rust) keeps (matplotlib's, without pltrs (Rust)'s own).
_NATIVE_KEYS = frozenset(_rcsetup.keys())


def _kept_in_python(key):
    """Whether pltrs keeps the rcParam *key* in Python: those of
    `_PYTHON_PARAMS`, and those a program adds to ``rcParams.validate``
    (matplotlib-scalebar's ``scalebar.*``), which pltrs (Rust) knows
    nothing of."""
    return key in _PYTHON_PARAMS or (
        key not in _NATIVE_KEYS and not (isinstance(key, str) and key.startswith("pltrs.")))


#: The rcParams `_kept_in_python` says False for without being called:
#: those pltrs (Rust) keeps, but for those of `_PYTHON_PARAMS` (rcParams
#: are read thousands of times at import).
_HELD_NATIVELY = _NATIVE_KEYS - _PYTHON_PARAMS.keys()

#: rcParams whose matplotlib validators warn about a valid value (``toolbar
#: = 'toolmanager'``): they run on assignment, as every validator does, but
#: not on reading (they choose among keywords, which pltrs (Rust) keeps in
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

#: matplotlib's defaults, which a new `RcParams` starts from a copy of
#: (quicker to copy than to make again).
_MATPLOTLIB_DEFAULTS = _rcsetup.defaults()


class RcParams(MutableMapping, dict):
    """
    A dict-like key-value store for config parameters, including validation.

    This is the data structure behind `matplotlib.rcParams`.

    The complete list of rcParams can be found in :doc:`/users/explain/configuration`.
    """
    # Values are validated by matplotlib's validators (`.rcsetup`), whose
    # errors assigning an invalid value raises, then stored by pltrs (Rust),
    # which validates their rc text again. Reading a value gives
    # matplotlib's Python value: the parameter's validator applied to the
    # rc text pltrs (Rust) keeps, or to the value validated, while pltrs
    # (Rust) holds that value (so that ``'dashed'`` stays ``'dashed'`` where
    # pltrs (Rust) keeps ``--``).
    # ``matplotlib.rcParams`` holds the current parameters; other instances
    # (``rcParamsDefault``, ``RcParams({...})``, copies) hold their own, and
    # only the keys given to them.

    validate = rcsetup._validators

    #: The keys holding a value matplotlib's validator takes and pltrs
    #: (Rust) refused (see `_set`).
    _refused = frozenset()

    def __init__(self, *args, **kwargs):
        self._store = _rcsetup.copy(_MATPLOTLIB_DEFAULTS)
        self._keys = set()
        self._python = {}
        self._given = {}
        self._lists = {}
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
        # The lists handed out (see `_kept_list`): pltrs (Rust) looks at
        # those of the current parameters before every native call, which
        # may read them, and has them taken once one changed.
        params._lists = _rcsetup.watch_lists({}, params._take_changed_lists)
        return params

    def _has(self, key):
        """Whether *key* is in this mapping."""
        if key not in _HELD_NATIVELY and _kept_in_python(key):
            return key in self._python
        if self._keys is None:
            return key in _NATIVE_KEYS
        return key in self._keys

    def _set(self, key, val):
        """Set *key* without the deprecation checks (matplotlib's ``_set``)."""
        if _kept_in_python(key):
            self._python[key] = val
            return
        # A new value: the list handed out before, if any, is the
        # parameter's no more.
        self._lists.pop(key, None)
        if key == "path.effects":
            # Path effect objects, which rc text cannot spell.
            _patheffects.set_rc_path_effects(
                self._store, [effect._native_spec() for effect in val])
        elif self._typed_color(key, val):
            # A colour as a tuple, which matplotlib's validator of the key
            # takes from Python but not as rc text (``legend.labelcolor``).
            _rcsetup.set_color(self._store, key, self.validate[key](val))
        else:
            try:
                _rcsetup.set(self._store, key, rcsetup._rc_value(val))
            except ValueError:
                if self.validate.get(key) is not rcsetup._validate_marker:
                    raise
                # A marker pltrs (Rust) does not know, which matplotlib's
                # validator takes as any string: read back as given, while
                # pltrs (Rust) keeps the marker it had, and failing where
                # a line is made with it (`.markers._check_rc`).
                self._refused = self._refused | {key}
            else:
                if key in self._refused:
                    self._refused = self._refused - {key}
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
        lists = self._lists
        if lists:
            # What a program changed in the lists it holds is the parameters
            # before any is read; the lists it holds no more are forgotten.
            self._take_changed_lists()
            kept = lists.get(key)
            if kept is not None:
                return self._kept_list(key, kept)
        if self._store is not None or (key not in _HELD_NATIVELY and _kept_in_python(key)):
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
        if _kept_in_python(key):
            return self._python[key]
        validator = self.validate.get(key)
        if validator is None:
            return _rcsetup.get(self._store, key)
        text = _rcsetup.text(self._store, key)
        if key in _WARNING_VALIDATORS:
            return text
        given, held = self._given.get(key, (text, text))
        return validator(given if held == text else text)

    # matplotlib's parameters are the objects of a dict, so a list read
    # from it is the parameter itself: ``rcParams["font.sans-serif"].insert(0,
    # name)`` and ``rcParams["figure.figsize"][0] = width`` change it. pltrs
    # (Rust) holds the parameters, so a list handed out (`__getitem__`) is
    # kept with a copy of what it held and the rc text pltrs (Rust) then
    # held: read again, it is the same list while pltrs (Rust) holds that
    # text, and what the program changed in it is taken first (`_take_list`),
    # as it is before any native call for the current parameters.
    #
    # A list is kept only while a program can change it: once nothing but
    # the record holds it (its reference count says so), it is forgotten,
    # at the next read of any parameter or the next look
    # (`_take_changed_lists`). The package's own code reads list parameters
    # with `_get`, which keeps nothing, and so does a read by item that
    # walks all the parameters in their order (``dict(rcParams)``,
    # ``rcParams.items()``, `__repr__`: copies of them all, whose lists are
    # the copy's). A program changing no list in place has none looked at.

    #: The keys the last iteration gave and how many of them were read by
    #: item in that order since (a copy by item in progress), or None.
    _walk = None

    def _keep_list(self, key, value):
        """Keep the list *value*, read as *key* and handed to the program,
        as the parameter itself."""
        self._lists[key] = [value, list(value), _rcsetup.text(self._store, key)]

    def _kept_list(self, key, kept):
        """The list handed out as *key*, *kept* with what it held: taken
        first if the program changed it, and read anew once the parameter
        was set by another way (a style, the end of an `rc_context`)."""
        if _rcsetup.text(self._store, key) != kept[2]:
            self._lists.pop(key, None)
            return self._get(key)
        if kept[0] != kept[1]:
            self._take_list(key, kept)
        return kept[0]

    def _take_list(self, key, kept):
        """Hand pltrs (Rust) what the list handed out as *key* holds now. A
        content matplotlib's validator refuses stays the list's alone, pltrs
        (Rust) keeping what it had (matplotlib keeps it unchecked, and fails
        where it uses it)."""
        kept[1] = list(kept[0])
        try:
            self._set(key, self.validate[key](list(kept[0])))
        except (TypeError, ValueError):
            pass
        kept[2] = _rcsetup.text(self._store, key)
        self._lists[key] = kept

    def _take_changed_lists(self):
        """`_take_list` every list handed out that the program changed,
        unless it is the parameter no more (it was set by another way
        since: the list is nobody's, whatever it holds), and forget the
        lists no program holds any longer, which nothing can change."""
        for key, kept in list(self._lists.items()):
            if kept[0] != kept[1]:
                if _rcsetup.text(self._store, key) != kept[2]:
                    self._lists.pop(key, None)
                    continue
                self._take_list(key, kept)
            if sys.getrefcount(kept[0]) <= _UNHELD:
                self._lists.pop(key, None)

    def __setitem__(self, key, val):
        if (key == "backend" and val is rcsetup._auto_backend_sentinel
                and "backend" in self):
            return
        if isinstance(key, str) and key.startswith("pltrs."):
            # pltrs (Rust)'s own parameters, which only it validates.
            self._set(key, val)
            return
        # matplotlib's validator first, for its errors and its value; then
        # pltrs (Rust) validates the value's rc text again as it stores it.
        valid_key = _api.getitem_checked(
            self.validate, rcParam=key, _error_cls=KeyError
        )
        try:
            cval = valid_key(val)
        except ValueError as ve:
            raise ValueError(f"Key {key}: {ve}") from None
        if key in {"text.hinting_factor", "text.kerning_factor"} and cval is not None:
            _api.warn_deprecated("3.11", name=key, obj_type="rcParam")
        self._set(key, cval)

    def __getitem__(self, key):
        # In theory, this should only ever be used after the global rcParams
        # has been set up, but better be safe e.g. in presence of breakpoints.
        if key == "backend" and self is globals().get("rcParams"):
            val = self._get(key)
            if val is rcsetup._auto_backend_sentinel:
                from . import pyplot as plt
                plt.switch_backend(rcsetup._auto_backend_sentinel)
        walk = self._walk
        if walk is not None:
            keys, read = walk
            if read < len(keys) and keys[read] == key:
                # A copy by item of all the parameters: its lists are its own.
                walk[1] = read + 1
                return self._get(key)
            self._walk = None
        value = self._get(key)
        if type(value) is list and key not in self._lists and not _kept_in_python(key):
            self._keep_list(key, value)
        return value

    def _get_backend_or_none(self):
        """Get the requested backend, if any, without triggering resolution."""
        backend = self._get("backend")
        return None if backend is rcsetup._auto_backend_sentinel else backend

    def _update_raw(self, other_params):
        """
        Directly update the data from *other_params*, bypassing deprecation,
        backend and validation logic on both sides.

        This ``rcParams._update_raw(params)`` replaces the previous pattern
        ``dict.update(rcParams, params)``.

        Parameters
        ----------
        other_params : dict or `.RcParams`
            The input mapping from which to update.
        """
        if isinstance(other_params, RcParams):
            other_params = [(key, other_params._get(key)) for key in other_params]
        else:
            other_params = dict(other_params).items()
        for key, val in other_params:
            self._set(key, val)

    def _ensure_has_backend(self):
        """
        Ensure that a "backend" entry exists.

        Normally, the default matplotlibrc file contains *no* entry for "backend" (the
        corresponding line starts with ##, not #; we fill in _auto_backend_sentinel
        in that case.  However, packagers can set a different default backend
        (resulting in a normal `#backend: foo` line) in which case we should *not*
        fill in _auto_backend_sentinel.
        """
        if not self._has("backend"):
            self._set("backend", rcsetup._auto_backend_sentinel)

    def __delitem__(self, key):
        # matplotlib's RcParams inherits `MutableMapping.__delitem__`, which
        # deletes nothing (so ``pop`` and ``popitem`` fail too).
        raise KeyError

    def __iter__(self):
        """Yield sorted list of keys."""
        keys = _NATIVE_KEYS if self._keys is None else self._keys
        keys = sorted(keys | self._python.keys())
        # Reads by item in this order are a copy of all the parameters
        # (see `__getitem__`).
        self._walk = [keys, 0]
        yield from keys

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
        native = [key for key in keys if not _kept_in_python(key)]
        other._take_changed_lists()
        _rcsetup.update_from(self._store, other._store, native)
        if self._keys is not None:
            self._keys.update(native)
        for key in native:
            self._given.pop(key, None)
            if key in other._given:
                self._given[key] = other._given[key]
        for key in keys:
            if _kept_in_python(key):
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
        return self._copied()

    def __deepcopy__(self, memo):
        return self._copied()

    def _copied(self):
        """A copy as `copy` makes of matplotlib's: its protocol reads the
        items (``__reduce_ex__``), which resolves the "auto" backend of
        ``rcParams`` as reading it does (`copy` copies it unresolved)."""
        if self is globals().get("rcParams"):
            self["backend"]  # noqa: B018
        return self.copy()

    def __reduce__(self):
        return RcParams, (dict(self),)

    def find_all(self, pattern):
        """
        Return the subset of this RcParams dictionary whose keys match,
        using :func:`re.search`, the given ``pattern``.

        .. note::

            Changes to the returned dictionary are *not* propagated to
            the parent RcParams dictionary.

        """
        pattern_re = re.compile(pattern)
        return RcParams((key, value) for key, value in self.items()
                        if pattern_re.search(key))

    def copy(self):
        """Copy this RcParams instance."""
        params = RcParams.__new__(RcParams)
        params._store = _rcsetup.copy(self._store)
        params._keys = {key for key in self if not _kept_in_python(key)}
        params._python = dict(self._python)
        params._given = dict(self._given)
        params._lists = {}
        return params


def _count_unheld():
    """The reference count `sys.getrefcount` gives a list that only a
    record of `RcParams._lists` holds, counted as `_take_changed_lists`
    counts it (the interpreter's own references differ by version)."""
    kept = [[], [], ""]
    return sys.getrefcount(kept[0])


#: See `_count_unheld`.
_UNHELD = _count_unheld()


def _of(store):
    """An RcParams with every parameter of the native *store*."""
    params = RcParams.__new__(RcParams)
    params._store = store
    params._keys = set(_NATIVE_KEYS)
    params._python = dict(_PYTHON_PARAMS)
    params._given = {}
    params._lists = {}
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
    *kwargs* is a dictionary attribute name/value pairs, e.g.,::

      rc('lines', linewidth=2, color='r')

    sets the current `.rcParams` and is equivalent to::

      rcParams['lines.linewidth'] = 2
      rcParams['lines.color'] = 'r'

    The following aliases are available to save typing for interactive users:

    ======  =================
    Alias   Property
    ======  =================
    'lw'    'linewidth'
    'ls'    'linestyle'
    'c'     'color'
    'fc'    'facecolor'
    'ec'    'edgecolor'
    'mew'   'markeredgewidth'
    'aa'    'antialiased'
    'sans'  'sans-serif'
    ======  =================

    Thus you could abbreviate the above call as::

          rc('lines', lw=2, c='r')

    Note you can use python's kwargs dictionary facility to store
    dictionaries of default parameters.  e.g., you can customize the
    font rc as follows::

      font = {'family' : 'monospace',
              'weight' : 'bold',
              'size'   : 'large'}
      rc('font', **font)  # pass in the font dict as kwargs

    This enables you to easily switch between several configurations.  Use
    ``matplotlib.style.use('default')`` or :func:`~matplotlib.rcdefaults` to
    restore the default `.rcParams` after changes.

    Notes
    -----
    Similar functionality is available by using the normal dict interface, i.e.
    ``rcParams.update({"lines.linewidth": 2, ...})`` (but ``rcParams.update``
    does not support abbreviations or grouping).
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

    See Also
    --------
    matplotlib.rc_file_defaults
        Restore the `.rcParams` from the rc file originally loaded by
        Matplotlib.
    matplotlib.style.use
        Use a specific style file.  Call ``style.use('default')`` to restore
        the default style.
    """
    _import_style_core()
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

    See Also
    --------
    :ref:`customizing-with-matplotlibrc-files`

    Examples
    --------
    Passing explicit values via a dict::

        with mpl.rc_context({'interactive': False}):
            fig, ax = plt.subplots()
            ax.plot(range(3), range(3))
            fig.savefig('example.png')
            plt.close(fig)

    Loading settings from a file::

         with mpl.rc_context(fname='print.rc'):
             plt.plot(x, y)  # uses 'print.rc'

    Setting in the context body::

        with mpl.rc_context():
            # will be reset
            mpl.rcParams['lines.linewidth'] = 5
            plt.plot(x, y)

    """
    rcParams._take_changed_lists()
    saved = _rcsetup.copy(None)
    given = dict(rcParams._given)
    # The lists handed out are the parameters again once the context ends,
    # with what the program changed in them meanwhile, as matplotlib puts
    # back the very objects.
    lists = {key: (kept[0], list(kept[1]), kept[2]) for key, kept in rcParams._lists.items()}
    # The parameters kept in Python, programs' own among them, which
    # matplotlib restores too (not the backend, which it leaves as chosen).
    added = {key: value for key, value in rcParams._python.items()
             if key != "backend"}
    try:
        if fname:
            rc_file(fname)
        if rc:
            rcParams.update(rc)
        yield
    finally:
        _rcsetup.replace(saved)
        rcParams._given = given
        rcParams._python.update(added)
        rcParams._lists.clear()
        for key, (handed, taken, text) in lists.items():
            kept = rcParams._lists[key] = [handed, taken, text]
            if handed != taken:
                rcParams._take_list(key, kept)


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
    matplotlib's messages, or raise with *fail_on_error*, the values pltrs
    (Rust) refuses assigned as matplotlib assigns them, for its errors.
    """
    path = _local_path(fname)
    try:
        entries, skipped = _rcsetup.read_file(path)
    except OSError:
        _raise_undecodable(fname, path)
        raise
    config = RcParams()
    for key, value in entries:
        config._set(key, value)
    for kind, line_no, line, key, val in skipped:
        if kind in ("missing colon", "duplicate key"):
            _log.warning(f'{kind.capitalize()} in file %r, line %d (%r)',
                         fname, line_no, line)
        elif key in rcsetup._validators:
            # A value pltrs (Rust) refused, or a parameter it leaves to
            # Python (``backend``): assigned as matplotlib assigns it.
            if fail_on_error:
                config[key] = val  # try to convert to proper type or raise
            else:
                try:
                    config[key] = val  # try to convert to proper type or skip
                except Exception as msg:
                    _log.warning('Bad value in file %r, line %d (%r): %s',
                                 fname, line_no, line, msg)
        else:
            # __version__ must be looked up as an attribute to trigger the
            # module-level __getattr__.
            version = ('main' if '.post' in __version__
                       else f'v{__version__}')
            _log.warning("""
Bad key %(key)s in file %(fname)s, line %(line_no)s (%(line)r)
You probably need to get an updated matplotlibrc file from
https://github.com/matplotlib/matplotlib/blob/%(version)s/lib/matplotlib/mpl-data/matplotlibrc
or from the matplotlib source distribution""",
                         dict(key=key, fname=fname, line_no=line_no,
                              line=line, version=version))
    return config


def _raise_undecodable(fname, path):
    """Raise the `UnicodeDecodeError` matplotlib's reading of the file
    *fname* at *path* meets, if it is not UTF-8, logging it as matplotlib
    does."""
    try:
        with open(path, encoding='utf-8') as fd:
            for _ in fd:
                pass
    except UnicodeDecodeError:
        _log.warning('Cannot decode configuration file %r as utf-8.', fname)
        raise


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

    - ``$PWD/matplotlibrc``
    - ``$MATPLOTLIBRC`` if it is not a directory
    - ``$MATPLOTLIBRC/matplotlibrc``
    - ``$MPLCONFIGDIR/matplotlibrc``
    - On Linux,
        - ``$XDG_CONFIG_HOME/matplotlib/matplotlibrc`` (if ``$XDG_CONFIG_HOME``
          is defined)
        - or ``$HOME/.config/matplotlib/matplotlibrc`` (if ``$XDG_CONFIG_HOME``
          is not defined)
    - On other platforms,
      - ``$HOME/.matplotlib/matplotlibrc`` if ``$HOME`` is defined
    - Lastly, it looks in ``$MATPLOTLIBDATA/matplotlibrc``, which should always
      exist.
    """
    # - ``$PWD/pltrsrc`` - ``$PLTRSRC`` if it is not a directory -
    # ``$PLTRSRC/pltrsrc`` - ``pltrsrc`` in the configuration directory
    # (`get_configdir`), each followed by matplotlib's ``matplotlibrc``
    # (``$MATPLOTLIBRC``) there - Lastly, the template of the defaults in
    # the data directory (`get_data_path`), which pltrs (Rust) writes
    # there, as matplotlib's ``mpl-data/matplotlibrc``.
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
    _import_style_core()
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
    _import_style_core()
    rcParams._update_from(
        rcParamsOrig, [key for key in rcParamsOrig if key not in _STYLE_BLACKLIST])


def _import_style_core():
    """Import the deprecated ``style.core`` without its warning, as
    matplotlib's `rcdefaults`, `rc_file` and `rc_file_defaults` do (which
    makes it an attribute of `matplotlib.style`)."""
    with _api.suppress_matplotlib_deprecation_warning():
        from .style import core  # noqa: F401


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

    Typically, one should call ``set_loglevel("INFO")`` or
    ``set_loglevel("DEBUG")`` to get additional debugging information.

    Users or applications that are installing their own logging handlers
    may want to directly manipulate ``logging.getLogger('matplotlib')`` rather
    than use this function.

    Parameters
    ----------
    level : {"NOTSET", "DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}
        The log level as defined in `Python logging levels
        <https://docs.python.org/3/library/logging.html#logging-levels>`__.

        For backwards compatibility, the levels are case-insensitive, but
        the capitalized version is preferred in analogy to `logging.Logger.setLevel`.

    Notes
    -----
    The first time this function is called, an additional handler is attached
    to Matplotlib's root handler; this handler is reused every time and this
    function simply manipulates the logger and handler's level.

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

    See Also
    --------
    :ref:`backends`
    matplotlib.get_backend
    matplotlib.pyplot.switch_backend

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

        .. versionadded:: 3.10

        .. admonition:: Provisional

           The *auto_select* flag is provisional. It may be changed or removed
           without prior warning.

    See Also
    --------
    matplotlib.use
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
    return val if val is not None else rcParams._get(rc_name)


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


def _logged_cached(fmt, func=None, *, later=None):
    """
    Decorator that logs a function's return value, and memoizes that value.

    After ::

        @_logged_cached(fmt)
        def func(): ...

    the first call to *func* will log its return value at the DEBUG level using
    %-format string *fmt*, and memoize it; later calls to *func* will directly
    return that value.
    """
    # matplotlib's decorator: later calls take any keyword arguments, which
    # only the first call passes on. pltrs adds *later*, called with the
    # memoized value at each later call (`get_data_path` writes the data
    # directory's files then, not when importing the package primes it).
    if func is None:  # Return the actual decorator.
        return functools.partial(_logged_cached, fmt, later=later)

    called = False
    ret = None

    @functools.wraps(func)
    def wrapper(**kwargs):
        nonlocal called, ret
        if not called:
            ret = func(**kwargs)
            called = True
            _log.debug(fmt, ret)
        elif later is not None:
            later(ret)
        return ret

    return wrapper


@_logged_cached('CONFIGDIR=%s')
def get_configdir():
    """
    Return the string path of the configuration directory.

    The directory is chosen as follows:

    1. If the MPLCONFIGDIR environment variable is supplied, choose that.
    2. On Linux, follow the XDG specification and look first in
       ``$XDG_CONFIG_HOME``, if defined, or ``$HOME/.config``.  On Windows,
       use ``%LOCALAPPDATA%\\matplotlib``.  On other platforms, choose
       ``$HOME/.matplotlib``.
    3. If the chosen directory exists and is writable, use that as the
       configuration directory.
    4. Else, create a temporary directory, and use it as the configuration
       directory.
    """
    # Return the string path of the configuration directory: the one
    # ``PLTRSCONFIGDIR`` names, else the one matplotlib's ``MPLCONFIGDIR``
    # names, else on Linux ``$XDG_CONFIG_HOME/pltrs`` (by default
    # ``~/.config/pltrs``) and elsewhere ``~/.pltrs``, made if missing; a
    # temporary directory when it cannot be made or written to.
    return _cbook.config_dir()


@_logged_cached('CACHEDIR=%s')
def get_cachedir():
    """
    Return the string path of the cache directory.

    The procedure used to find the directory is the same as for
    `get_configdir`, except using ``$XDG_CACHE_HOME``/``$HOME/.cache`` instead
    on Linux.  On Windows, uses ``%LOCALAPPDATA%\\matplotlib`` (same as config).
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
from . import ft2font  # noqa: E402, F401  (matplotlib's import loads it)


@functools.cache
def _write_data_files(path):
    """Write the files of the data directory *path*, once: those pltrs (Rust)
    bundles (its fonts, ``_native.font_manager.data_path``) and the entries of
    the package's ``mpl-data`` (`_reach_shipped_data`)."""
    from ._native import font_manager as _font_manager
    _font_manager.data_path()
    _reach_shipped_data(path)


@functools.cache
def _reach_shipped_data(path):
    """Link the entries of the package's ``mpl-data`` into the data directory
    *path* (copying them where links cannot be made), as programs join
    ``get_data_path()`` and ``"sample_data"``.

    The data directory is shared by every installation of pltrs, so a link
    may lead to another installation's files, or to one since removed: such
    a link is pointed at this package's files instead. Several processes
    may write it at once (the first programs of a new configuration
    directory): what one finds there is whole."""
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
                _copy_into_place(entry, target)


def _copy_into_place(entry, target):
    """Copy the file or directory *entry* to *target* in one step: into a
    sibling of this process's own, then renamed to *target*, so that other
    processes find it whole or not at all. A copy another process put there
    meanwhile is as good as this one, which is then dropped."""
    partial = target.with_name(f".{target.name}.{os.getpid()}.partial")
    _remove_copy(partial)
    if entry.is_dir():
        shutil.copytree(entry, partial)
    else:
        shutil.copy2(entry, partial)
    try:
        os.rename(partial, target)
    except OSError:
        _remove_copy(partial)
        if not target.exists():
            raise


def _remove_copy(path):
    """Remove the file or directory tree *path*, if there is one."""
    if path.is_dir():
        shutil.rmtree(path, ignore_errors=True)
    else:
        path.unlink(missing_ok=True)


def _replace_link(entry, target):
    """Point the link *target* at *entry*, replacing it in one step, so that
    other processes always find a link there (but on Windows, which replaces
    no link to a directory: there the old link goes first)."""
    temporary = target.with_name(f".{target.name}.{os.getpid()}")
    try:
        temporary.unlink(missing_ok=True)
        temporary.symlink_to(entry, target_is_directory=entry.is_dir())
        try:
            os.replace(temporary, target)
        except PermissionError:
            if sys.platform != "win32":
                raise
            target.unlink()
            os.replace(temporary, target)
    except OSError:
        temporary.unlink(missing_ok=True)


@_logged_cached('matplotlib data path: %s', later=_write_data_files)
def get_data_path():
    """Return the path to Matplotlib data."""
    # Return the path to Matplotlib data: the directory where pltrs (Rust)
    # writes the data files it bundles (its fonts), as matplotlib's ``mpl-
    # data``, which also reaches the files the package ships (the sample data,
    # the toolbar images). Importing the package asks where it is; its files
    # are written when the path is next asked for (`_write_data_files`), as
    # writing them maps the pages of the extension around the bundled files.
    from ._native import font_manager as _font_manager
    return _font_manager.data_location()


# matplotlib loads its rc files from the data and configuration directories
# as it is imported, and the functions remember them (`_logged_cached`):
# pltrs (Rust) read its rc files from the same directories.
get_data_path()
get_configdir()


@_api.deprecated("3.10", alternative="matplotlib.cbook.sanitize_sequence")
def sanitize_sequence(data):
    from . import cbook
    return cbook.sanitize_sequence(data)


@_api.deprecated("3.10", alternative="matplotlib.rcsetup.validate_backend")
def validate_backend(s):
    return rcsetup.validate_backend(s)


# What matplotlib's module also has from its imports, which programs
# import from it.
import importlib  # noqa: E402, F401
import inspect  # noqa: E402, F401
import locale  # noqa: E402, F401
import tempfile  # noqa: E402, F401
from inspect import Parameter  # noqa: E402, F401
from pathlib import Path  # noqa: E402, F401
