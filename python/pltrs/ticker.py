"""
Tick locating and formatting (matplotlib's ``ticker``).

Locators decide where the ticks go and formatters write their labels. The
classes keep matplotlib's names, parameters and attributes; pltrs (Rust) computes
the ticks and the labels (``pltrs::ticker``): each locator or formatter hands
its parameters to the native side as a *spec*, with a description of the
axis it is attached to.

The formatters that format with Python's own string formatting
(`FormatStrFormatter`, `StrMethodFormatter`, `FuncFormatter`) and those
that only look labels up (`FixedFormatter`, `NullFormatter`) are
matplotlib's, kept as they are (from matplotlib 3.11.2, under the
Matplotlib License). Any formatter is a Python callable
``formatter(x, pos)``: pltrs (Rust) calls it each time it labels the ticks of the
axis it was given to.

A helper given to an axis is installed there by `._tick_helpers`: from its
spec when pltrs (Rust) can compute in its place, and installed again when
one of its attributes changes the spec (`TickHelper.__setattr__`). What
matplotlib's tick updates leave in a helper (a formatter's locations and
offset) are `._tick_state.ticked` attributes, which a helper pltrs (Rust)
ticks with natively computes again from the last ticks of its axis.
"""

from collections.abc import Mapping
import locale
import logging
import math
import numbers
from numbers import Integral
import string

import numpy as np

from . import (_api, _lazy_imports, _locators_as_matplotlib, _logit_reference,
               _program_classes, _qualnames, _tick_helpers, _tick_state, cbook)
from ._native import axis as _axis, register as _register, ticker as _ticker

_scalar_reference = _lazy_imports.module(globals(), "._scalar_reference")

_log = logging.getLogger(__name__)

# How the locale writes numbers, for the formatters pltrs (Rust) makes
# itself under :rc:`axes.formatter.use_locale`.
_ticker.follow_locale(locale.localeconv)

__all__ = ('TickHelper', 'Formatter', 'FixedFormatter',
           'NullFormatter', 'FuncFormatter', 'FormatStrFormatter',
           'StrMethodFormatter', 'ScalarFormatter', 'LogFormatter',
           'LogFormatterExponent', 'LogFormatterMathtext',
           'LogFormatterSciNotation',
           'LogitFormatter', 'EngFormatter', 'PercentFormatter',
           'Locator', 'IndexLocator', 'FixedLocator', 'NullLocator',
           'LinearLocator', 'LogLocator', 'AutoLocator',
           'MultipleLocator', 'MaxNLocator', 'AutoMinorLocator',
           'SymmetricalLogLocator', 'AsinhLocator', 'LogitLocator')


def _rc(key):
    """The rcParam *key* (imported when first needed)."""
    from . import rcParams
    return rcParams._get(key)


def _check_text_font():
    """
    What matplotlib's ``ScalarFormatter.set_useMathText(None)`` does while
    :rc:`axes.formatter.use_mathtext` is off: look the font of
    :rc:`font.family` up (logging, and caching the lookup, as matplotlib's
    font manager does), and warn when it is cmr10, whose minus sign the
    labels then lack. Every default `ScalarFormatter` is made so, pltrs
    (Rust)'s among them (`_scalar_formatters_made`). Whether it warned.
    """
    if _rc('axes.formatter.use_mathtext') is not False:
        return False
    from . import font_manager
    try:
        ufont = font_manager.findfont(
            font_manager.FontProperties(family=_rc("font.family")),
            fallback_to_default=False)
    except ValueError:
        ufont = None
    if ufont != str(cbook._get_data_path("fonts/ttf/cmr10.ttf")):
        return False
    _api.warn_external("cmr10 font should ideally be used with "
                       "mathtext, set axes.formatter.use_mathtext to True")
    return True


def _scalar_formatters_made(count):
    """
    Check the text font as each of the *count* default scalar formatters
    pltrs (Rust) made (a linear or function scale's, matplotlib's
    ``Axis._set_scale``) did as matplotlib makes it (`_check_text_font`);
    whether there was nothing to say, which the font manager's cache of
    the lookup keeps so until the parameters change.
    """
    logs, cmr10 = _ticker.text_font_check()
    if not (logs or cmr10):
        # A lookup that logs nothing and finds no cmr10 says nothing.
        return True
    warned = _check_text_font()
    if warned:
        for _ in range(count - 1):
            _check_text_font()
    return not warned


_register("scalar_formatters_made", _scalar_formatters_made)


def _val_or_rc(val, key):
    """*val*, or the rcParam *key* when *val* is None."""
    return _rc(key) if val is None else val


class _DummyAxis:
    """The axis a tick helper uses when it is not attached to one."""

    __name__ = "dummy"

    def __init__(self, minpos=0):
        self._data_interval = (0, 1)
        self._view_interval = (0, 1)
        self._minpos = minpos

    def get_view_interval(self):
        """Return the view limits ``(min, max)`` of the axis."""
        return self._view_interval

    def set_view_interval(self, vmin, vmax):
        """Set the view limits of the axis."""
        self._view_interval = (vmin, vmax)

    def get_minpos(self):
        """Return the smallest positive datum."""
        return self._minpos

    def get_data_interval(self):
        """Return the data limits ``(min, max)`` of the axis."""
        return self._data_interval

    def set_data_interval(self, vmin, vmax):
        """Set the data limits of the axis."""
        self._data_interval = (vmin, vmax)

    def get_tick_space(self):
        """Return the estimated number of ticks that can fit on the axis."""
        # Just use the long-standing default of nbins==9
        return 9


class TickHelper:
    axis = None

    def __setattr__(self, name, value):
        # What the axis' last ticks left in the helper comes first, as in
        # matplotlib, where they left it there.
        _tick_state.catch_up(self)
        super().__setattr__(name, value)
        _tick_helpers.check_native(self)

    def __getstate__(self):
        _tick_state.catch_up(self)
        return super().__getstate__()

    def set_axis(self, axis):
        self.axis = axis

    def create_dummy_axis(self, **kwargs):
        if self.axis is None:
            self.axis = _DummyAxis(**kwargs)

    #: The description pltrs (Rust) gave of the axis it is drawing, while it
    #: asks the helper about it (see `Formatter._label_axis`).
    _drawn_axis = None

    #: Whether the helper places its ticks among the axis' major ones, which
    #: only then are asked of a program's axis (whose ``get_majorticklocs``
    #: may call the major locator, as seaborn's legends' axes do).
    _reads_major_ticks = False

    def _axis_info(self):
        """
        The native description of the helper's axis, or None. A stand-in
        axis of the program's lacking some of `Axis`'s methods (one giving
        only ``get_view_interval``, as seaborn's legends make) answers the
        others as `_DummyAxis()` does.
        """
        if self._drawn_axis is not None:
            return self._drawn_axis
        axis = self.axis
        if axis is None:
            return None
        # (A held axis, matplotlib's own, is asked as a stand-in is.)
        native_info = (None if getattr(axis, "_held", False)
                       else getattr(axis, "_native_info", None))
        if native_info is not None:
            return native_info()
        scale = axis.get_scale() if hasattr(axis, "get_scale") else "linear"
        if not isinstance(scale, str):
            # A stand-in's scale object (seaborn's), as its name.
            scale = getattr(scale, "name", "linear")
        transform = getattr(getattr(axis, "_scale", None), "_transform", None)
        major = (axis.get_majorticklocs()
                 if self._reads_major_ticks and hasattr(axis, "get_majorticklocs")
                 else [])

        def asked(name, default):
            method = getattr(axis, name, None)
            return default if method is None else method()

        # A data interval the axis does not know yet (seaborn's stand-ins
        # start at ``(None, None)``) is NaN, which only `IndexLocator` reads.
        data_interval = tuple(np.nan if value is None else float(value)
                              for value in asked("get_data_interval", (0, 1)))
        return {
            "view_interval": tuple(map(float, axis.get_view_interval())),
            "data_interval": data_interval,
            "minpos": float(asked("get_minpos", 0)),
            "tick_space": int(asked("get_tick_space", 9)),
            "major_ticks": [float(v) for v in major],
            "scale": scale,
            "linthresh": getattr(transform, "linthresh", None),
        }


def _numeric_locale():
    """How the current locale writes numbers (``locale.localeconv()``), as
    pltrs (Rust) writes the labels of a `ScalarFormatter` with
    ``useLocale``."""
    conv = locale.localeconv()
    return {"decimal_point": conv["decimal_point"],
            "thousands_sep": conv["thousands_sep"],
            "grouping": list(conv["grouping"])}


def _first_steps(x, *steps):
    """
    Compute with *x*, as given, the first steps of matplotlib's formatting
    of it (*steps*, each a function of it), before pltrs (Rust) formats it
    as a number: an array, or what is no number, fails there with numpy's
    or Python's own error, as it fails in matplotlib's formatter.
    """
    for step in steps:
        step(x)


class _Unheld(Exception):
    """A parameter of a locator is no number pltrs (Rust) holds."""


def _held(value, kind=float, least=0):
    """
    *value*, a parameter of a locator, as pltrs (Rust) holds it: a real
    number (a count, an integer of at least *least*, for *kind* `int`).
    Anything else (a string, None, a sequence, a smaller count) raises
    `_Unheld`, and the locator computes with it as matplotlib's does (see
    `_NativeLocator`).
    """
    if kind is int:
        if (isinstance(value, bool) or not isinstance(value, numbers.Integral)
                or value < least):
            raise _Unheld(value)
    elif isinstance(value, bool) or not isinstance(value, numbers.Real):
        raise _Unheld(value)
    return kind(value)


def _held_all(values):
    """The parameters *values* (a sequence) as pltrs (Rust) holds them
    (see `_held`)."""
    if not np.iterable(values) or isinstance(values, str):
        raise _Unheld(values)
    return [_held(value) for value in values]


def _all_numbers(*values):
    """Whether *values*, limits a locator is asked to tick, are all real
    numbers pltrs (Rust) computes with."""
    return all(isinstance(v, numbers.Real) for v in values)


class Formatter(TickHelper):
    """
    Create a string based on a tick value and location.

    The Formatter provides four formatting methods for different use cases:

    - `format_ticks`: The public API for generating tick labels from a set of
      tick values.
    - `__call__`: The low-level primitive for formatting a single tick value,
      potentially in the context of multiple values.
    - `format_data`: Context-independent representation of a single value.
      Used internally, e.g. for offset and scientific-notation strings.
    - `format_data_short`: Concise plain-text representation of a single value
      for the interactive mouseover tooltip.
    """
    # some classes want to see all the locs to help format
    # individual ones
    _locs = _tick_state.ticked([])
    locs = _api.deprecate_privatize_attribute("3.11")

    def __call__(self, x, pos=None):
        """
        Return the tick label strings for value *x* at tick index *pos*.

        This is the low-level formatting primitive for a single tick in
        the context of multiple ticks. Any context-dependent state
        (e.g. locs, offset, order of magnitude) must already be configured,
        typically by a prior call to ``format_ticks`` or ``set_locs``.

        *pos* defines the index into ``self.locs`` so that the format can
        depend on the location. ``pos=None`` indicates an unspecified
        location.

        The output may contain mathtext or LaTeX markup.

        Subclasses must override this method.
        """
        raise NotImplementedError('Derived must override')

    def format_ticks(self, values):
        """
        Return the tick label strings for all *values*.

        This is the public API for generating tick labels.  It calls
        ``set_locs`` to configure context-dependent formatting state before
        delegating to ``__call__`` for each individual value.

        The output may contain mathtext or LaTeX markup.

        Use this method (rather than ``__call__``) whenever formatting a
        complete set of tick values, so that formatters which need to see
        all tick locations (e.g. to determine precision, offsets, or which
        date components to display) can work correctly.
        """
        self.set_locs(values)
        return [self(value, i) for i, value in enumerate(values)]

    def format_data(self, value):
        """
        Return the context-independent string representation of a single *value*.

        This is used internally, e.g. for constructing offset and
        scientific-notation strings.  It always formats with ``pos=None``
        and should return a context-independent representation
        rather than a concise tick label.

        The output may contain mathtext or LaTeX markup.
        """
        return self.__call__(value)

    def format_data_short(self, value):
        """
        Return a short string representation of *value* for the mouseover
        tooltip (the coordinate display in the interactive figure window).

        This should return concise, plain text (no mathtext / LaTeX).
        The precision is typically adapted to the current axis resolution
        so that neighbouring pixels produce distinguishable labels.

        Defaults to `.Formatter.format_data`; subclasses should override
        this to provide a plain-text representation that is independent
        of the current tick locations.

        Note: The mouseover text can be customized by setting the
        ``Axes.fmt_xdata`` and ``Axes.fmt_ydata`` attributes.
        """
        return self.format_data(value)

    def get_offset(self):
        return ''

    def set_locs(self, locs):
        """
        Set the locations of the ticks.

        This method is called before computing the tick labels because some
        formatters need to know all tick locations to do so.
        """
        self._locs = locs

    @staticmethod
    def fix_minus(s):
        """
        Some classes may want to replace a hyphen for minus with the proper
        Unicode symbol (U+2212) for typographical correctness.  This is a
        helper method to perform such a replacement when it is enabled via
        :rc:`axes.unicode_minus`.
        """
        return (s.replace('-', '\N{MINUS SIGN}')
                if _rc('axes.unicode_minus')
                else s)

    def _set_locator(self, locator):
        """Subclasses may want to override this to set a locator."""
        pass


class NullFormatter(Formatter):
    """Always return the empty string."""

    def _spec(self):
        return {"kind": "null"}

    def __call__(self, x, pos=None):
        # docstring inherited
        return ''


class FixedFormatter(Formatter):
    """
    Return fixed strings for tick labels based only on position, not value.

    .. note::
        `.FixedFormatter` should only be used together with `.FixedLocator`.
        Otherwise, the labels may end up in unexpected positions.
    """

    def __init__(self, seq):
        """Set the sequence *seq* of strings that will be used for labels."""
        self.seq = seq
        self.offset_string = ''

    def __call__(self, x, pos=None):
        """
        Return the label that matches the position, regardless of the value.

        For positions ``pos < len(seq)``, return ``seq[i]`` regardless of
        *x*. Otherwise return empty string. ``seq`` is the sequence of
        strings that this object was initialized with.
        """
        if pos is None or pos >= len(self.seq):
            return ''
        else:
            return self.seq[pos]

    def get_offset(self):
        return self.offset_string

    def set_offset_string(self, ofs):
        self.offset_string = ofs


class FuncFormatter(Formatter):
    """
    Use a user-defined function for formatting.

    The function should take in two inputs (a tick value ``x`` and a
    position ``pos``), and return a string containing the corresponding
    tick label.
    """

    def __init__(self, func):
        self.func = func
        self.offset_string = ""

    def __call__(self, x, pos=None):
        """
        Return the value of the user defined function.

        *x* and *pos* are passed through as-is.
        """
        return self.func(x, pos)

    def get_offset(self):
        return self.offset_string

    def set_offset_string(self, ofs):
        self.offset_string = ofs


class FormatStrFormatter(Formatter):
    """
    Use an old-style ('%' operator) format string to format the tick.

    The format string should have a single variable format (%) in it.
    It will be applied to the value (not the position) of the tick.

    Negative numeric values (e.g., -1) will use a dash, not a Unicode minus;
    use mathtext to get a Unicode minus by wrapping the format specifier with $
    (e.g. "$%g$").
    """

    def __init__(self, fmt):
        self.fmt = fmt

    def __call__(self, x, pos=None):
        """
        Return the formatted label string.

        Only the value *x* is formatted. The position is ignored.
        """
        return self.fmt % x


class _UnicodeMinusFormat(string.Formatter):
    """
    A specialized string formatter so that `.StrMethodFormatter` respects
    :rc:`axes.unicode_minus`.  This implementation relies on the fact that the
    format string is only ever called with kwargs *x* and *pos*, so it blindly
    replaces dashes by unicode minuses without further checking.
    """

    def format_field(self, value, format_spec):
        return Formatter.fix_minus(super().format_field(value, format_spec))


class StrMethodFormatter(Formatter):
    """
    Use a new-style format string (as used by `str.format`) to format the tick.

    The field used for the tick value must be labeled *x* and the field used
    for the tick position must be labeled *pos*.

    The formatter will respect :rc:`axes.unicode_minus` when formatting
    negative numeric values.

    It is typically unnecessary to explicitly construct `.StrMethodFormatter`
    objects, as `~.Axis.set_major_formatter` directly accepts the format string
    itself.

    Examples
    --------
    >>> formatter = StrMethodFormatter("{x} km")
    >>> formatter(10)
    "10 km"
    """

    def __init__(self, fmt):
        self.fmt = fmt

    def __call__(self, x, pos=None):
        """
        Return the formatted label string.

        *x* and *pos* are passed to `str.format` as keyword arguments
        with those exact names.
        """
        return _UnicodeMinusFormat().format(self.fmt, x=x, pos=pos)


class _NativeFormatter(Formatter):
    """
    A formatter whose labels pltrs (Rust) writes: `set_locs` formats all the
    ticks at once (as pltrs (Rust) does when it draws an axis), and calling the
    formatter looks the label of a value up among them.
    """

    _labels = _tick_state.ticked(())
    _offset = _tick_state.ticked('')

    #: matplotlib's ``__call__`` of the class, with which a formatter whose
    #: parameters pltrs (Rust) does not hold (`_spec` raises `_Unheld`)
    #: labels its ticks (None: the class has none).
    _matplotlib_call = None

    def _spec(self):
        """The native description of the formatter (its kind and
        parameters)."""
        raise NotImplementedError('Derived must override')

    def _as_matplotlib(self, *values):
        """Whether to label *values* as matplotlib's formatter does: when
        the class can and a parameter or a value is no number pltrs (Rust)
        holds."""
        if self._matplotlib_call is None:
            return False
        if not _all_numbers(*values):
            return True
        try:
            self._spec()
        except _Unheld:
            return True
        return False

    @_qualnames.named_as("Formatter.set_locs")
    def set_locs(self, locs):
        # docstring inherited
        if self._as_matplotlib() or not np.iterable(locs):
            # matplotlib's keeps them, labelling each when it is called.
            self._locs = self._kept_locs(locs) if np.iterable(locs) else locs
            self._labels = ()
            return
        self._set_locs_on(locs, self._axis_info())

    @staticmethod
    def _kept_locs(locs):
        """The tick locations *locs* as the formatter keeps them: as given
        (an iterator's as a list, which can be read more than once)."""
        return list(locs) if iter(locs) is locs else locs

    def _set_locs_on(self, locs, info):
        """
        Take the tick locations *locs* and their labels on the axis *info*
        describes. The axis is described first: describing it may have
        pltrs (Rust) label its ticks with this very formatter. The
        locations are kept as given, as matplotlib's formatters keep them.
        """
        self._locs = self._kept_locs(locs)
        values = np.asarray(self._locs, dtype=float)
        self._labels, self._offset = _ticker.format_ticks(
            self._spec(), values, info)

    @_qualnames.named_as("Formatter.__call__")
    def __call__(self, x, pos=None):
        if self._as_matplotlib(x):
            return self._matplotlib_call(x, pos)
        locs = self._locs
        if isinstance(pos, Integral) and 0 <= pos < len(locs) and locs[pos] == x:
            return self._labels[pos]
        if x in locs:
            return self._labels[list(locs).index(x)]
        labels, _ = _ticker.format_ticks(
            self._spec(), np.asarray([*locs, x], dtype=float), self._axis_info())
        return labels[-1]

    @_qualnames.named_as("Formatter.format_ticks")
    def format_ticks(self, values):
        # docstring inherited
        self.set_locs(values)
        if type(self).__call__.__module__ != __name__ or self._as_matplotlib():
            # A subclass of the program's labels each tick itself, as
            # matplotlib's formatter does with parameters pltrs (Rust) does
            # not hold.
            return [self(value, i) for i, value in enumerate(values)]
        return list(self._labels)

    @_qualnames.named_as("Formatter.get_offset")
    def get_offset(self):
        # docstring inherited
        return self._offset

    @_qualnames.named_as("Formatter.format_data")
    def format_data(self, value):
        # docstring inherited
        return _ticker.format_data(self._spec(), float(value), False,
                                   self._axis_info())

    @_qualnames.named_as("Formatter.format_data_short")
    def format_data_short(self, value):
        # docstring inherited
        return _ticker.format_data(self._spec(), float(value), True,
                                   self._axis_info())


class ScalarFormatter(_NativeFormatter):
    """
    Format tick values as a number.

    Parameters
    ----------
    useOffset : bool or float, default: :rc:`axes.formatter.useoffset`
        Whether to use offset notation. See `.set_useOffset`.
    useMathText : bool, default: :rc:`axes.formatter.use_mathtext`
        Whether to use fancy math formatting. See `.set_useMathText`.
    useLocale : bool, default: :rc:`axes.formatter.use_locale`.
        Whether to use locale settings for decimal sign and positive sign.
        See `.set_useLocale`.
    usetex : bool, default: :rc:`text.usetex`
        To enable/disable the use of TeX's math mode for rendering the
        numbers in the formatter.

        .. versionadded:: 3.10

    Notes
    -----
    In addition to the parameters above, the formatting of scientific vs.
    floating point representation can be configured via `.set_scientific`
    and `.set_powerlimits`).

    **Offset notation and scientific notation**

    Offset notation and scientific notation look quite similar at first sight.
    Both split some information from the formatted tick values and display it
    at the end of the axis.

    - The scientific notation splits up the order of magnitude, i.e. a
      multiplicative scaling factor, e.g. ``1e6``.

    - The offset notation separates an additive constant, e.g. ``+1e6``. The
      offset notation label is always prefixed with a ``+`` or ``-`` sign
      and is thus distinguishable from the order of magnitude label.

    The following plot with x limits ``1_000_000`` to ``1_000_010`` illustrates
    the different formatting. Note the labels at the right edge of the x axis.

    .. plot::

        lim = (1_000_000, 1_000_010)

        fig, (ax1, ax2, ax3) = plt.subplots(3, 1, gridspec_kw={'hspace': 2})
        ax1.set(title='offset notation', xlim=lim)
        ax2.set(title='scientific notation', xlim=lim)
        ax2.xaxis.get_major_formatter().set_useOffset(False)
        ax3.set(title='floating-point notation', xlim=lim)
        ax3.xaxis.get_major_formatter().set_useOffset(False)
        ax3.xaxis.get_major_formatter().set_scientific(False)

    """

    orderOfMagnitude = _api.deprecate_privatize_attribute("3.11")
    format = _api.deprecate_privatize_attribute("3.11")
    offset = _tick_state.ticked(0)
    _orderOfMagnitude = _tick_state.ticked(0)
    _format = _tick_state.ticked('')

    def __init__(self, useOffset=None, useMathText=None, useLocale=None, *,
                 usetex=None):
        useOffset = _val_or_rc(useOffset, 'axes.formatter.useoffset')
        self._offset_threshold = _rc('axes.formatter.offset_threshold')
        self._orderOfMagnitude = 0
        self._format = ''
        self.set_useOffset(useOffset)
        self.set_usetex(usetex)
        self.set_useMathText(useMathText)
        self._scientific = True
        self._powerlimits = _rc('axes.formatter.limits')
        self.set_useLocale(useLocale)

    def _spec(self):
        # An offset that is no number (which `set_locs` fails on as
        # matplotlib's does) offsets nothing; numpy's booleans are numbers
        # its arithmetic takes.
        use_offset = True if self._useOffset else (
            float(self.offset)
            if isinstance(self.offset, (numbers.Real, np.bool_)) and self.offset else False)
        # Power limits pltrs (Rust) does not hold leave the labels to
        # matplotlib's arithmetic (see `set_locs`): any it holds will do.
        powerlimits = self._held_powerlimits() or (0, 0)
        return {"kind": "scalar", "use_offset": use_offset,
                "use_mathtext": bool(self._useMathText), "usetex": self._usetex,
                "scientific": self._scientific,
                "powerlimits": powerlimits,
                "locale": _numeric_locale() if self._useLocale else None}

    def _held_powerlimits(self):
        """The power limits as pltrs (Rust) holds them, integers; None for
        limits that are not (NaN, fractions, text), which matplotlib
        compares as they are."""
        limits = tuple(self._powerlimits)
        if all(isinstance(limit, numbers.Integral) for limit in limits):
            return tuple(map(int, limits))
        return None

    def _orders_by_view(self):
        """Whether matplotlib's order of magnitude is computed from the
        view: scientific notation not fixed to an exponent."""
        return self._scientific and not (
            self._powerlimits[0] == self._powerlimits[1] != 0)

    def _compares_with_view(self):
        """Whether matplotlib's formatter compares the locations with the
        view: for an offset, and for an order of magnitude."""
        return self._useOffset or self._orders_by_view()

    def _first_steps_with(self, locs):
        """
        Fail as matplotlib's `set_locs` fails to compute with *locs*, no
        numbers (an array of another kind, of text or None): comparing them
        with the view, then subtracting the offset to choose their format.
        """
        values = np.asarray(locs)
        if values.dtype.kind in "biuf":
            return
        if self._compares_with_view():
            vmin, vmax = sorted(self.axis.get_view_interval())
            _first_steps(values, lambda values: (vmin <= values) & (values <= vmax))
        _first_steps(values, lambda values: values - self.offset)

    def get_usetex(self):
        """Return whether TeX's math mode is enabled for rendering."""
        return self._usetex

    def set_usetex(self, val):
        """Set whether to use TeX's math mode for rendering numbers in the formatter."""
        self._usetex = _val_or_rc(val, 'text.usetex')

    usetex = property(fget=get_usetex, fset=set_usetex)

    def get_useOffset(self):
        """
        Return whether automatic mode for offset notation is active.

        This returns True if ``set_useOffset(True)``; it returns False if an
        explicit offset was set, e.g. ``set_useOffset(1000)``.

        See Also
        --------
        ScalarFormatter.set_useOffset
        """
        return self._useOffset

    def set_useOffset(self, val):
        """
        Set whether to use offset notation.

        When formatting a set numbers whose value is large compared to their
        range, the formatter can separate an additive constant. This can
        shorten the formatted numbers so that they are less likely to overlap
        when drawn on an axis.

        Parameters
        ----------
        val : bool or float
            - If False, do not use offset notation.
            - If True (=automatic mode), use offset notation if it can make
              the residual numbers significantly shorter. The exact behavior
              is controlled by :rc:`axes.formatter.offset_threshold`.
            - If a number, force an offset of the given value.

        Examples
        --------
        With active offset notation, the values

        ``100_000, 100_002, 100_004, 100_006, 100_008``

        will be formatted as ``0, 2, 4, 6, 8`` plus an offset ``+1e5``, which
        is written to the edge of the axis.
        """
        if isinstance(val, bool):
            self.offset = 0
            self._useOffset = val
        else:
            self._useOffset = False
            self.offset = val

    useOffset = property(fget=get_useOffset, fset=set_useOffset)

    def get_useLocale(self):
        """
        Return whether locale settings are used for formatting.

        See Also
        --------
        ScalarFormatter.set_useLocale
        """
        return self._useLocale

    def set_useLocale(self, val):
        """
        Set whether to use locale settings for decimal sign and positive sign.

        Parameters
        ----------
        val : bool or None
            *None* resets to :rc:`axes.formatter.use_locale`.
        """
        # (Kept as given, as matplotlib's; its truth tells.)
        self._useLocale = _val_or_rc(val, 'axes.formatter.use_locale')

    useLocale = property(fget=get_useLocale, fset=set_useLocale)

    def _format_maybe_minus_and_locale(self, fmt, arg):
        """
        Format *arg* with *fmt*, applying Unicode minus and locale if desired.
        """
        return self.fix_minus(
                # Escape commas introduced by locale.format_string if using math text,
                # but not those present from the beginning in fmt.
                (",".join(locale.format_string(part, (arg,), True).replace(",", "{,}")
                          for part in fmt.split(",")) if self._useMathText
                 else locale.format_string(fmt, (arg,), True))
                if self._useLocale
                else fmt % arg)

    def get_useMathText(self):
        """
        Return whether to use fancy math formatting.

        See Also
        --------
        ScalarFormatter.set_useMathText
        """
        return self._useMathText

    def set_useMathText(self, val):
        r"""
        Set whether to use fancy math formatting.

        If active, scientific notation is formatted as :math:`1.2 \times 10^3`.

        Parameters
        ----------
        val : bool or None
            *None* resets to :rc:`axes.formatter.use_mathtext`.
        """
        # (Kept as given, as matplotlib's; its truth tells.)
        self._useMathText = _val_or_rc(val, 'axes.formatter.use_mathtext')
        if val is None:
            _check_text_font()

    useMathText = property(fget=get_useMathText, fset=set_useMathText)

    def set_scientific(self, b):
        """
        Turn scientific notation on or off.

        See Also
        --------
        ScalarFormatter.set_powerlimits
        """
        self._scientific = bool(b)

    def set_powerlimits(self, lims):
        r"""
        Set size thresholds for scientific notation.

        Parameters
        ----------
        lims : (int, int)
            A tuple *(min_exp, max_exp)* containing the powers of 10 that
            determine the switchover threshold. For a number representable as
            :math:`a \times 10^\mathrm{exp}` with :math:`1 <= |a| < 10`,
            scientific notation will be used if ``exp <= min_exp`` or
            ``exp >= max_exp``.

            The default limits are controlled by :rc:`axes.formatter.limits`.

            In particular numbers with *exp* equal to the thresholds are
            written in scientific notation.

            Typically, *min_exp* will be negative and *max_exp* will be
            positive.

            For example, ``formatter.set_powerlimits((-3, 4))`` will provide
            the following formatting:
            :math:`1 \times 10^{-3}, 9.9 \times 10^{-3}, 0.01,`
            :math:`9999, 1 \times 10^4`.

        See Also
        --------
        ScalarFormatter.set_scientific
        """
        if len(lims) != 2:
            raise ValueError("'lims' must be a sequence of length 2")
        self._powerlimits = lims

    def __call__(self, x, pos=None):
        """
        Return the format for tick value *x* at position *pos*.
        """
        if len(self._locs) == 0:
            return ''
        else:
            xp = (x - self.offset) / (10. ** self._orderOfMagnitude)
            if abs(xp) < 1e-8:
                xp = 0
            return self._format_maybe_minus_and_locale(self._format, xp)

    def format_data(self, value):
        # docstring inherited
        # matplotlib's first step, which refuses zero, NaN and infinities
        # in the words of the running Python.
        e = math.floor(math.log10(abs(value)))
        if isinstance(value, np.floating) and value.dtype != np.float64:
            # Its significand computed in its own precision.
            return self._format_in_precision(value, e)
        # The value alone, as matplotlib's formats it: no tick of the axis
        # is computed (which would set the order of magnitude).
        return _ticker.format_data(self._spec(), float(value), False, None)

    def _format_in_precision(self, value, e):
        """`format_data` of *value*, a numpy float of another precision
        than a double's, of decimal exponent *e*: matplotlib's arithmetic,
        in that precision."""
        s = round(value / 10**e, 10)
        significand = self._format_maybe_minus_and_locale(
            "%d" if s % 1 == 0 else "%1.10g", s)
        if e == 0:
            return significand
        exponent = self._format_maybe_minus_and_locale("%d", e)
        if self._useMathText or self._usetex:
            exponent = "10^{%s}" % exponent
            return (exponent if s == 1  # reformat 1x10^y as 10^y
                    else rf"{significand} \times {exponent}")
        return f"{significand}e{exponent}"

    def get_offset(self):
        """
        Return scientific notation, plus offset.
        """
        # Composed from the settings as they are now, as matplotlib's is.
        if len(self._locs) == 0:
            return ''
        if self._orderOfMagnitude or self.offset:
            offsetStr = ''
            sciNotStr = ''
            if self.offset:
                offsetStr = self.format_data(self.offset)
                if self.offset > 0:
                    offsetStr = '+' + offsetStr
            if self._orderOfMagnitude:
                if self._usetex or self._useMathText:
                    sciNotStr = self.format_data(10 ** self._orderOfMagnitude)
                else:
                    sciNotStr = '1e%d' % self._orderOfMagnitude
            if self._useMathText or self._usetex:
                if sciNotStr != '':
                    sciNotStr = r'\times\mathdefault{%s}' % sciNotStr
                s = fr'${sciNotStr}\mathdefault{{{offsetStr}}}$'
            else:
                s = ''.join((sciNotStr, offsetStr))
            return self.fix_minus(s)
        return ''

    def set_locs(self, locs):
        # docstring inherited
        # matplotlib's first steps: the number of locations, then, to
        # compute an offset or the order of magnitude, their comparison
        # with the view, which fails for what is no number.
        _first_steps(locs, len)
        locs = self._kept_locs(locs)
        if len(locs):
            self._first_steps_with(locs)
        info = self._program_view_info(self._axis_info(), locs)
        self._set_locs_on(locs, info)
        if len(self._locs) and not isinstance(self.offset, numbers.Real):
            # matplotlib keeps the offset, and subtracts it from the
            # locations to choose their format.
            np.asarray(self._locs) - self.offset
        locs = np.asarray(self._locs, dtype=float)
        if len(locs) == 0:
            return
        offset, self._orderOfMagnitude = _ticker.scalar_offset(
            self._spec(), locs, info)
        if self._useOffset:
            # matplotlib's offset: 0, else a numpy float.
            self.offset = np.float64(offset) if offset else 0
        self._format = _ticker.scalar_format(self._spec(), locs, info)
        if self._held_powerlimits() is None:
            # Power limits compared as given, as matplotlib's are.
            self._orderOfMagnitude = _scalar_reference.order_of_magnitude(self, self._locs)
            self._format = _scalar_reference.label_format(self, self._locs)
            self._labels = tuple(self(x, i) for i, x in enumerate(self._locs))

    def _program_view_info(self, info, locs):
        """*info* with the view interval the program's class of the axis
        answers, asked as often as matplotlib's `set_locs` asks it (in
        ``_compute_offset``, ``_set_order_of_magnitude`` and, for fewer
        than two locations, ``_set_format``); *info* itself for the
        package's axes. A formatter of no axis fails to ask, as
        matplotlib's does."""
        if len(locs) == 0 or not (self.axis is None or _asks_program_view(self.axis)):
            return info
        view = None
        if self._useOffset:
            view = self.axis.get_view_interval()
        if self._orders_by_view():
            view = self.axis.get_view_interval()
        if len(locs) < 2:
            view = self.axis.get_view_interval()
        return info if view is None else _with_view(info, sorted(view))

    def format_data_short(self, value):
        # docstring inherited
        if value is np.ma.masked:
            return ""
        if isinstance(value, Integral):
            return self.fix_minus("%d" % value)
        if getattr(self.axis, "__name__", "") in ("xaxis", "yaxis"):
            if not isinstance(value, numbers.Real):
                # matplotlib measures the value's distance to the values of
                # its neighbouring pixels, as given.
                abs(np.zeros(2) - value).max()
            text = _axis.format_data_short(*self.axis._args(), float(value))
            if text is not None:
                return text
        return super().format_data_short(value)


class LogFormatter(Formatter):
    """
    Base class for formatting ticks on a log or symlog scale.

    It may be instantiated directly, or subclassed.

    Parameters
    ----------
    base : float, default: 10.
        Base of the logarithm used in all calculations.

    labelOnlyBase : bool, default: False
        If True, label ticks only at integer powers of base.
        This is normally True for major ticks and False for minor ticks.

    minor_thresholds : (subset, all), default: (1, 0.4)
        If labelOnlyBase is False, these two numbers control
        the labeling of ticks that are not at integer powers of
        base; normally these are the minor ticks.

        The first number (*subset*) is the largest number of major ticks for
        which minor ticks are labeled; e.g., the default, 1, means that minor
        ticks are labeled as long as there is no more than 1 major tick.  (It
        is assumed that major ticks are at integer powers of *base*.)

        The second number (*all*) is a threshold, in log-units of the axis
        limit range, over which only a subset of the minor ticks are labeled,
        so as to avoid crowding; e.g., with the default value (0.4) and the
        usual ``base=10``, all minor ticks are shown only if the axis limit
        range spans less than 0.4 decades.

    linthresh : None or float, default: None
        If a symmetric log scale is in use, its ``linthresh``
        parameter must be supplied here.

    Notes
    -----
    The `set_locs` method must be called to enable the subsetting
    logic controlled by the ``minor_thresholds`` parameter.

    In some cases such as the colorbar, there is no distinction between
    major and minor ticks; the tick locations might be set manually,
    or by a locator that puts ticks at integer powers of base and
    at intermediate locations.  For this situation, disable the
    minor_thresholds logic by using ``minor_thresholds=(np.inf, np.inf)``,
    so that all ticks will be labeled.

    To disable labeling of minor ticks when 'labelOnlyBase' is False,
    use ``minor_thresholds=(0, 0)``.  This is the default for the
    "classic" style.

    Examples
    --------
    To label a subset of minor ticks when there are up to 2 major ticks,
    and all of the ticks when zoomed in to 0.5 decades or less, use
    ``minor_thresholds=(2, 0.5)``.
    """

    _kind = "log"
    _sublabels = _tick_state.ticked()

    def __init__(self, base=10.0, labelOnlyBase=False,
                 minor_thresholds=None,
                 linthresh=None):
        self.set_base(base)
        self.set_label_minor(labelOnlyBase)
        if minor_thresholds is None:
            if _rc('_internal.classic_mode'):
                minor_thresholds = (0, 0)
            else:
                minor_thresholds = (1, 0.4)
        self.minor_thresholds = minor_thresholds
        self._sublabels = None
        self._linthresh = linthresh

    def _spec(self):
        return {"kind": self._kind, "base": float(self._base),
                "label_only_base": bool(self.labelOnlyBase),
                "minor_thresholds": tuple(map(float, self.minor_thresholds)),
                "linthresh": self._linthresh}

    def set_base(self, base):
        """
        Change the *base* for labeling.

        .. warning::
           Should always match the base used for :class:`LogLocator`
        """
        self._base = float(base)

    def set_label_minor(self, labelOnlyBase):
        """
        Switch minor tick labeling on or off.

        Parameters
        ----------
        labelOnlyBase : bool
            If True, label ticks only at integer powers of base.
        """
        self.labelOnlyBase = labelOnlyBase

    def set_locs(self, locs=None):
        """
        Use axis view limits to control which ticks are labeled.

        The *locs* parameter is ignored in the present algorithm.
        """
        sublabels = _ticker.log_sublabels(self._spec(), self._axis_info())
        # As matplotlib's sets: {1} for the powers of the base alone, else
        # numpy's numbers.
        self._sublabels = (None if sublabels is None
                           else {1} if sublabels == [1.0]
                           else set(np.asarray(sublabels)))

    #: matplotlib's first steps with the value (see `_first_steps`): it
    #: tests for zero (``x == 0.0``), then takes its absolute value.
    _value_steps = (lambda x: bool(x == 0.0), abs)

    def __call__(self, x, pos=None):
        # docstring inherited
        # A mathtext formatter labels as this class does when the program
        # calls this method on it (its exponents label as their class).
        kind = None if self._kind == LogFormatterExponent._kind else LogFormatter._kind
        return self._log_label(x, LogFormatter._value_steps, kind)

    def _log_label(self, x, steps, kind=None):
        """The label of *x*, after matplotlib's first *steps* with it (its
        class' ``__call__``'s, see `_first_steps`), written as the formatters
        of *kind* write it (by default, as this one's)."""
        _first_steps(x, *steps)
        if isinstance(x, numbers.Real) and math.isinf(x):
            # matplotlib rounds the exponent of an infinite value first.
            _ = round(x)
        spec = self._spec()
        if kind is not None:
            spec["kind"] = kind
        return _ticker.log_label(spec, float(x), self._sublabels, self._axis_info())

    def format_data(self, value):
        # docstring inherited
        with cbook._setattr_cm(self, labelOnlyBase=False):
            return cbook.strip_math(self.__call__(value))

    def format_data_short(self, value):
        # docstring inherited
        return ('%-12g' % value).rstrip()

    def _pprint_val(self, x, d):
        """*x* written for a view spanning *d*."""
        return _ticker.pprint_val(float(x), float(d))


class LogFormatterExponent(LogFormatter):
    """
    Format values for log axis using ``exponent = log_base(value)``.
    """

    _kind = "log_exponent"


class LogFormatterMathtext(LogFormatter):
    """
    Format values for log axis using ``exponent = log_base(value)``.
    """

    _kind = "log_mathtext"
    # It tests for zero, then for the sign (``x < 0``).
    _value_steps = (lambda x: bool(x == 0), lambda x: bool(x < 0))

    def __call__(self, x, pos=None):
        # docstring inherited
        return self._log_label(x, LogFormatterMathtext._value_steps)


class LogFormatterSciNotation(LogFormatterMathtext):
    """
    Format values following scientific notation in a logarithmic axis.
    """

    _kind = "log_scinotation"


class LogitFormatter(_NativeFormatter):
    """
    Probability formatter (using Math text).
    """

    _labelled = _tick_state.ticked()

    def __init__(
        self,
        *,
        use_overline=False,
        one_half=r"\frac{1}{2}",
        minor=False,
        minor_threshold=25,
        minor_number=6,
    ):
        r"""
        Parameters
        ----------
        use_overline : bool, default: False
            If x > 1/2, with x = 1 - v, indicate if x should be displayed as
            $\overline{v}$. The default is to display $1 - v$.

        one_half : str, default: r"\\frac{1}{2}"
            The string used to represent 1/2.

        minor : bool, default: False
            Indicate if the formatter is formatting minor ticks or not.
            Basically minor ticks are not labelled, except when only few ticks
            are provided, ticks with most space with neighbor ticks are
            labelled. See other parameters to change the default behavior.

        minor_threshold : int, default: 25
            Maximum number of locs for labelling some minor ticks. This
            parameter have no effect if minor is False.

        minor_number : int, default: 6
            Number of ticks which are labelled when the number of ticks is
            below the threshold.
        """
        self._use_overline = use_overline
        self._one_half = one_half
        self._minor = minor
        self._labelled = set()
        self._minor_threshold = minor_threshold
        self._minor_number = minor_number

    def _spec(self):
        return {"kind": "logit", "use_overline": bool(self._use_overline),
                "one_half": self._one_half, "minor": bool(self._minor),
                "minor_threshold": int(self._minor_threshold),
                "minor_number": int(self._minor_number)}

    @staticmethod
    def _kept_locs(locs):
        # matplotlib's logit formatter keeps its locations as an array.
        return np.array(locs)

    @_qualnames.named_as("Formatter.format_data")
    def format_data(self, value):
        # docstring inherited
        # matplotlib's formats it through `__call__`, which first tests it
        # against the interval (``x <= 0 or x >= 1``).
        _first_steps(value, lambda x: bool(x <= 0 or x >= 1))
        return super().format_data(value)

    def use_overline(self, use_overline):
        r"""
        Switch display mode with overline for labelling p>1/2.

        Parameters
        ----------
        use_overline : bool
            If x > 1/2, with x = 1 - v, indicate if x should be displayed as
            $\overline{v}$. The default is to display $1 - v$.
        """
        self._use_overline = use_overline

    def set_one_half(self, one_half):
        r"""
        Set the way one half is displayed.

        one_half : str
            The string used to represent 1/2.
        """
        self._one_half = one_half

    def set_minor_threshold(self, minor_threshold):
        """
        Set the threshold for labelling minors ticks.

        Parameters
        ----------
        minor_threshold : int
            Maximum number of locations for labelling some minor ticks. This
            parameter have no effect if minor is False.
        """
        self._minor_threshold = minor_threshold

    def set_minor_number(self, minor_number):
        """
        Set the number of minor ticks to label when some minor ticks are
        labelled.

        Parameters
        ----------
        minor_number : int
            Number of ticks which are labelled when the number of ticks is
            below the threshold.
        """
        self._minor_number = minor_number

    #: matplotlib's labelling, for locations that are no finite numbers.
    _matplotlib_call = _logit_reference.call

    def _format_value(self, x, locs, sci_notation=True):
        # matplotlib's label of *x* among *locs*, which subclasses call
        # (cartopy's latitude formatter).
        return _logit_reference._format_value(x, locs, sci_notation)

    def _as_matplotlib(self, *values):
        # docstring inherited; also while the locations are no finite
        # numbers, which matplotlib's formatter keeps and labels as given.
        return (super()._as_matplotlib(*values)
                or self.__dict__.get("_locs_as_given", False))

    def set_locs(self, locs):
        # docstring inherited
        values = np.array(locs)
        self._locs_as_given = not (values.dtype.kind in "biuf"
                                   and np.isfinite(values).all())
        if self._locs_as_given:
            self._labels = ()
            _logit_reference.set_locs(self, locs)
            return
        super().set_locs(locs)
        # The minor ticks that get labels.
        self._labelled = set(_ticker.logit_labelled(
            self._spec(), np.asarray(self._locs, dtype=float)))

    def __call__(self, x, pos=None):
        # docstring inherited
        # matplotlib's first steps: an unlabelled minor tick, then the
        # interval (``x <= 0 or x >= 1``).
        if not (self._minor and x not in self._labelled):
            _first_steps(x, lambda v: v <= 0 or v >= 1)
        return super().__call__(x, pos)

    def format_data_short(self, value):
        # docstring inherited
        # matplotlib's first step: its threshold of scientific notation.
        _first_steps(value, lambda v: v < 0.1)
        return super().format_data_short(value)


class EngFormatter(ScalarFormatter):
    """
    Format axis values using engineering prefixes to represent powers
    of 1000, plus a specified unit, e.g., 10 MHz instead of 1e7.
    """

    # The offset pltrs (Rust) gives its labels, as matplotlib's formatter
    # gives its own.
    get_offset = _NativeFormatter.get_offset

    # The SI engineering prefixes
    ENG_PREFIXES = {
        -30: "q",
        -27: "r",
        -24: "y",
        -21: "z",
        -18: "a",
        -15: "f",
        -12: "p",
         -9: "n",
         -6: "\N{MICRO SIGN}",
         -3: "m",
          0: "",
          3: "k",
          6: "M",
          9: "G",
         12: "T",
         15: "P",
         18: "E",
         21: "Z",
         24: "Y",
         27: "R",
         30: "Q"
    }

    def __init__(self, unit="", places=None, sep=" ", *, usetex=None,
                 useMathText=None, useOffset=False):
        r"""
        Parameters
        ----------
        unit : str, default: ""
            Unit symbol to use, suitable for use with single-letter
            representations of powers of 1000. For example, 'Hz' or 'm'.

        places : int, default: None
            Precision with which to display the number, specified in
            digits after the decimal point (there will be between one
            and three digits before the decimal point). If it is None,
            the formatting falls back to the floating point format '%g',
            which displays up to 6 *significant* digits, i.e. the equivalent
            value for *places* varies between 0 and 5 (inclusive).

        sep : str, default: " "
            Separator used between the value and the prefix/unit. For
            example, one get '3.14 mV' if ``sep`` is " " (default) and
            '3.14mV' if ``sep`` is "". Besides the default behavior, some
            other useful options may be:

            * ``sep=""`` to append directly the prefix/unit to the value;
            * ``sep="\N{THIN SPACE}"`` (``U+2009``);
            * ``sep="\N{NARROW NO-BREAK SPACE}"`` (``U+202F``);
            * ``sep="\N{NO-BREAK SPACE}"`` (``U+00A0``).

        usetex : bool, default: :rc:`text.usetex`
            To enable/disable the use of TeX's math mode for rendering the
            numbers in the formatter.

        useMathText : bool, default: :rc:`axes.formatter.use_mathtext`
            To enable/disable the use mathtext for rendering the numbers in
            the formatter.
        useOffset : bool or float, default: False
            Whether to use offset notation with :math:`10^{3*N}` based prefixes.
            This features allows showing an offset with standard SI order of
            magnitude prefix near the axis. Offset is computed similarly to
            how `ScalarFormatter` computes it internally, but here you are
            guaranteed to get an offset which will make the tick labels exceed
            3 digits. See also `.set_useOffset`.

            .. versionadded:: 3.10
        """
        self.unit = unit
        self.places = places
        self.sep = sep
        super().__init__(
            useOffset=useOffset,
            useMathText=useMathText,
            useLocale=False,
            usetex=usetex,
        )

    def _spec(self):
        return {"kind": "eng", "unit": self.unit, "places": self.places,
                "sep": self.sep, "usetex": bool(self._usetex),
                "use_mathtext": bool(self._useMathText),
                "use_offset": bool(self._useOffset),
                "prefixes": list(self.ENG_PREFIXES.items())}

    def __call__(self, x, pos=None):
        """
        Return the format for tick value *x* at position *pos*.

        If there is no currently offset in the data, it returns the best
        engineering formatting that fits the given argument, independently.
        """
        if len(self._locs) == 0 or self._offset == '':
            if not isinstance(x, numbers.Real):
                # matplotlib's arithmetic, on what is no number.
                return self.fix_minus(self.format_data(x))
            labels, _ = _ticker.format_ticks(
                {**self._spec(), "use_offset": False},
                np.asarray([x], dtype=float), self._axis_info())
            return labels[0]
        return _NativeFormatter.__call__(self, x, pos)

    def format_eng(self, num):
        """Alias to EngFormatter.format_data"""
        return self.format_data(num)

    def format_data(self, value):
        """
        Format a number in engineering notation, appending a letter
        representing the power of 1000 of the original number.
        Some examples:

        >>> format_data(0)        # for self.places = 0
        '0'

        >>> format_data(1000000)  # for self.places = 1
        '1.0 M'

        >>> format_data(-1e-6)  # for self.places = 2
        '-1.00 \N{MICRO SIGN}'
        """
        # matplotlib 3.11.2's method, as it is.
        sign = 1
        fmt = "g" if self.places is None else f".{self.places:d}f"

        if value < 0:
            sign = -1
            value = -value

        if value != 0:
            pow10 = int(math.floor(math.log10(value) / 3) * 3)
        else:
            pow10 = 0
            # Force value to zero, to avoid inconsistencies like
            # format_eng(-0) = "0" and format_eng(0.0) = "0"
            # but format_eng(-0.0) = "-0.0"
            value = 0.0

        pow10 = np.clip(pow10, min(self.ENG_PREFIXES), max(self.ENG_PREFIXES))

        mant = sign * value / (10.0 ** pow10)
        # Taking care of the cases like 999.9..., which may be rounded to 1000
        # instead of 1 k.  Beware of the corner case of values that are beyond
        # the range of SI prefixes (i.e. > 'Y').
        if (abs(float(format(mant, fmt))) >= 1000
                and pow10 < max(self.ENG_PREFIXES)):
            mant /= 1000
            pow10 += 3

        unit_prefix = self.ENG_PREFIXES[int(pow10)]
        if self.unit or unit_prefix:
            suffix = f"{self.sep}{unit_prefix}{self.unit}"
        else:
            suffix = ""
        if self._usetex or self._useMathText:
            return f"${mant:{fmt}}${suffix}"
        else:
            return f"{mant:{fmt}}{suffix}"

    def set_locs(self, locs):
        # docstring inherited
        super().set_locs(locs)

    def _orders_by_view(self):
        # docstring inherited: matplotlib's engineering formatter takes the
        # order of magnitude of the view's span,
        return True

    def _compares_with_view(self):
        # docstring inherited: comparing the locations with the view only
        # for an offset.
        return self._useOffset

    def get_offset(self):
        # docstring inherited; the offset pltrs (Rust) gives its labels, as
        # matplotlib's formatter gives its own.
        return _NativeFormatter.get_offset(self)


class PercentFormatter(_NativeFormatter):
    """
    Format numbers as a percentage.

    Parameters
    ----------
    xmax : float
        Determines how the number is converted into a percentage.
        *xmax* is the data value that corresponds to 100%.
        Percentages are computed as ``x / xmax * 100``. So if the data is
        already scaled to be percentages, *xmax* will be 100. Another common
        situation is where *xmax* is 1.0.

    decimals : None or int
        The number of decimal places to place after the point.
        If *None* (the default), the number will be computed automatically.

    symbol : str or None
        A string that will be appended to the label. It may be
        *None* or empty to indicate that no symbol should be used. LaTeX
        special characters are escaped in *symbol* whenever latex mode is
        enabled, unless *is_latex* is *True*.

    is_latex : bool
        If *False*, reserved LaTeX characters in *symbol* will be escaped.
    """

    def __init__(self, xmax=100, decimals=None, symbol='%', is_latex=False):
        self.xmax = xmax + 0.0
        self.decimals = decimals
        self._symbol = symbol
        self._is_latex = is_latex

    def _spec(self):
        if not (isinstance(self._symbol, str) or self._symbol is None):
            raise _Unheld(self._symbol)
        decimals = self.decimals
        return {"kind": "percent", "xmax": _held(self.xmax),
                "decimals": None if decimals is None else _held(decimals, int),
                "symbol": self._symbol, "is_latex": bool(self._is_latex)}

    def _matplotlib_call(self, x, pos=None):
        """matplotlib's ``__call__``: *x* as a percentage, with the
        decimals the axis' view asks for."""
        ax_min, ax_max = self.axis.get_view_interval()
        display_range = abs(ax_max - ax_min)
        return self.fix_minus(self.format_pct(x, display_range))

    @_qualnames.named_as("Formatter.format_data_short")
    def format_data_short(self, value):
        # docstring inherited
        if self._as_matplotlib(value):
            return self(value)
        # matplotlib's formats it through `__call__`, its first steps the
        # percentage (``x / self.xmax``) written with decimals.
        _first_steps(value, lambda x: f'{self.convert_to_pct(x):0.1f}')
        return super().format_data_short(value)

    def __call__(self, x, pos=None):
        """Format the tick as a percentage with the appropriate scaling."""
        if self._as_matplotlib(x):
            return self._matplotlib_call(x, pos)
        if not np.iterable(self._locs) or len(self._locs) == 0:
            labels, _ = _ticker.format_ticks(
                self._spec(), np.asarray([x], dtype=float), self._axis_info())
            return labels[0]
        return super().__call__(x, pos)

    def convert_to_pct(self, x):
        return 100.0 * (x / self.xmax)

    def format_pct(self, x, display_range):
        """
        Format the number as a percentage number with the correct
        number of decimals and adds the percent symbol, if any.

        If ``self.decimals`` is `None`, the number of digits after the
        decimal point is set based on the *display_range* of the axis
        as follows:

        ============= ======== =======================
        display_range decimals sample
        ============= ======== =======================
        >50           0        ``x = 34.5`` => 35%
        >5            1        ``x = 34.5`` => 34.5%
        >0.5          2        ``x = 34.5`` => 34.50%
        ...           ...      ...
        ============= ======== =======================

        This method will not be very good for tiny axis ranges or
        extremely large ones. It assumes that the values on the chart
        are percentages displayed on a reasonable scale.
        """
        if not self._as_matplotlib(x, display_range):
            return _ticker.format_pct(self._spec(), float(x), float(display_range))
        # matplotlib's, for values or parameters that are no numbers.
        x = self.convert_to_pct(x)
        if self.decimals is None:
            # conversion works because display_range is a difference
            scaled_range = self.convert_to_pct(display_range)
            if scaled_range <= 0:
                decimals = 0
            else:
                decimals = math.ceil(2.0 - math.log10(2.0 * scaled_range))
                if decimals > 5:
                    decimals = 5
                elif decimals < 0:
                    decimals = 0
        else:
            decimals = self.decimals
        s = f'{x:0.{int(decimals)}f}'
        return s + self.symbol

    @property
    def symbol(self):
        r"""
        The configured percent symbol as a string.

        If LaTeX is enabled via :rc:`text.usetex`, the special characters
        ``{'#', '$', '%', '&', '~', '_', '^', '\', '{', '}'}`` are
        automatically escaped in the string.
        """
        symbol = self._symbol
        if not symbol:
            symbol = ''
        elif not self._is_latex and _rc('text.usetex'):
            # Source: http://www.personal.ceu.hu/tex/specchar.htm
            # Backslash must be first for this to work correctly since
            # it keeps getting added in
            for spec in r'\#$%&~_^{}':
                symbol = symbol.replace(spec, '\\' + spec)
        return symbol

    @symbol.setter
    def symbol(self, symbol):
        self._symbol = symbol


class Locator(TickHelper):
    """
    Determine tick locations.

    Note that the same locator should not be used across multiple
    `~matplotlib.axis.Axis` because the locator stores references to the Axis
    data and view limits.
    """

    # Some automatic tick locators can generate so many ticks they
    # kill the machine when you try and render them.
    # This parameter is set to cause locators to raise an error if too
    # many ticks are generated.
    MAXTICKS = 1000

    def tick_values(self, vmin, vmax):
        """
        Return the values of the located ticks given **vmin** and **vmax**.

        .. note::
            To get tick locations with the vmin and vmax values defined
            automatically for the associated ``axis`` simply call
            the Locator instance::

                >>> print(type(loc))
                <type 'Locator'>
                >>> print(loc())
                [1, 2, 3, 4]

        """
        raise NotImplementedError('Derived must override')

    def set_params(self, **kwargs):
        """
        Do nothing, and raise a warning. Any locator class not supporting the
        set_params() function will call this.
        """
        _api.warn_external(
            "'set_params()' not defined for locator of type " +
            str(type(self)))

    def __call__(self):
        """Return the locations of the ticks."""
        raise NotImplementedError('Derived must override')

    def _held_spec(self):
        """The native description pltrs (Rust) places the locator's ticks
        from (its ``_spec``), None for a locator that has none."""
        spec = getattr(self, "_spec", None)
        return None if spec is None else spec()

    def raise_if_exceeds(self, locs):
        """
        Log at WARNING level if *locs* is longer than `Locator.MAXTICKS`.

        This is intended to be called immediately before returning *locs* from
        ``__call__`` to inform users in case their Locator returns a huge
        number of ticks, causing Matplotlib to run out of memory.

        The "strange" name of this method dates back to when it would raise an
        exception instead of emitting a log.
        """
        if len(locs) >= self.MAXTICKS:
            _log.warning(
                "Locator attempting to generate %s ticks ([%s, ..., %s]), "
                "which exceeds Locator.MAXTICKS (%s).",
                len(locs), locs[0], locs[-1], self.MAXTICKS)
        return locs

    def nonsingular(self, v0, v1):
        """
        Adjust a range as needed to avoid singularities.

        This method gets called during autoscaling, with ``(v0, v1)`` set to
        the data limits on the Axes if the Axes contains any data, or
        ``(-inf, +inf)`` if not.

        - If ``v0 == v1`` (possibly up to some floating point slop), this
          method returns an expanded interval around this value.
        - If ``(v0, v1) == (-inf, +inf)``, this method returns appropriate
          default view limits.
        - Otherwise, ``(v0, v1)`` is returned without modification.
        """
        from .transforms import _nonsingular
        return _nonsingular(v0, v1, expander=.05)

    def view_limits(self, vmin, vmax):
        """
        Select a scale for the range from vmin to vmax.

        Subclasses should override this method to change locator behaviour.
        """
        from .transforms import _nonsingular
        return _nonsingular(vmin, vmax)


def _asks_program_view(axis):
    """Whether matplotlib's helpers ask *axis* for its view interval where
    pltrs (Rust) describes the axis drawn: when it is of a class of the
    program's answering ``get_view_interval`` itself, or no axis at all
    (an object the program set, which fails to answer as in matplotlib)."""
    if axis is None or isinstance(axis, _DummyAxis):
        return False
    if not hasattr(axis, "_native_info"):
        return True
    return _program_classes.takes_from_program(type(axis), "get_view_interval")


def _with_view(info, view):
    """The axis description *info* with the view interval *view*, as the
    program's class of the axis answers it."""
    low, high = map(float, view)
    if isinstance(info, dict):
        return {**info, "view_interval": (low, high)}
    return _axis.with_view_interval(info, low, high)


class _NativeLocator(Locator):
    """
    A locator whose ticks pltrs (Rust) computes from its parameters.

    While a parameter is no number pltrs (Rust) holds (`_spec` raises
    `_Unheld`), or for limits that are no numbers, the locator computes as
    matplotlib's does, with its class' functions of
    `._locators_as_matplotlib` (the ``_matplotlib_*`` attributes; a class
    without one computes natively all the same).
    """

    #: Whether matplotlib's ``__call__`` of the class ticks the view
    #: interval it asks the axis for (``tick_values(*get_view_interval())``),
    #: which an axis of the program's class answers itself.
    _ticks_the_view = False

    #: matplotlib's computations of the class (see the class).
    _matplotlib_call = _locators_as_matplotlib.view_call
    _matplotlib_tick_values = None
    _matplotlib_view_limits = None

    def _spec(self):
        """The native description of the locator (its kind and
        parameters)."""
        raise NotImplementedError('Derived must override')

    def _held_spec(self):
        # docstring inherited
        # None while a parameter is no number pltrs (Rust) holds.
        try:
            return self._spec()
        except _Unheld:
            return None

    def _as_matplotlib(self, *values):
        """Whether to compute as matplotlib's locator does, for limits
        *values*: when the class can and a parameter or a limit is no
        number pltrs (Rust) holds."""
        return (self._matplotlib_tick_values is not None
                and (not _all_numbers(*values) or self._held_spec() is None))

    def tick_values(self, vmin, vmax):
        # docstring inherited
        if self._as_matplotlib(vmin, vmax):
            return self._matplotlib_tick_values(vmin, vmax)
        locs = _ticker.tick_values(self._spec(), float(vmin), float(vmax),
                                   self._axis_info())
        return self.raise_if_exceeds(locs)

    def __call__(self):
        """Return the locations of the ticks."""
        if self._as_matplotlib():
            return self._matplotlib_call()
        if self._ticks_the_view and (_asks_program_view(self.axis) or self._gives_tick_values()):
            vmin, vmax = self.axis.get_view_interval()
            return self.tick_values(vmin, vmax)
        return self.raise_if_exceeds(
            _ticker.locate(self._spec(), self._axis_info()))

    def _gives_tick_values(self):
        """Whether a class of the program's computes the ticks itself (its
        ``tick_values``), which matplotlib's ``__call__`` asks for the view
        interval."""
        return _program_classes.takes_from_program(type(self), "tick_values")

    def view_limits(self, vmin, vmax):
        # docstring inherited
        if self._as_matplotlib(vmin, vmax):
            if self._matplotlib_view_limits is None:
                return super().view_limits(vmin, vmax)
            return self._matplotlib_view_limits(vmin, vmax)
        return _ticker.view_limits(self._spec(), float(vmin), float(vmax),
                                   self._axis_info())

    def nonsingular(self, v0, v1):
        # docstring inherited
        if self._as_matplotlib(v0, v1):
            return super().nonsingular(v0, v1)
        # matplotlib's locators give their default range without reading
        # the axis (whose view pltrs (Rust) would autoscale first).
        bounded = np.isfinite(float(v0)) and np.isfinite(float(v1))
        return _ticker.nonsingular(self._spec(), float(v0), float(v1),
                                   self._axis_info() if bounded else None)


class IndexLocator(_NativeLocator):
    """
    Place ticks at every nth point plotted.

    IndexLocator assumes index plotting; i.e., that the ticks are placed at integer
    values in the range between 0 and len(data) inclusive.
    """

    def __init__(self, base, offset):
        """Place ticks every *base* data point, starting at *offset*."""
        self._base = base
        self.offset = offset

    _matplotlib_call = _locators_as_matplotlib.index_call
    _matplotlib_tick_values = _locators_as_matplotlib.index_tick_values

    def _spec(self):
        return {"kind": "index", "base": _held(self._base),
                "offset": _held(self.offset)}

    def set_params(self, base=None, offset=None):
        """Set parameters within this locator"""
        if base is not None:
            self._base = base
        if offset is not None:
            self.offset = offset

    def __call__(self):
        """Return the locations of the ticks"""
        dmin, dmax = self.axis.get_data_interval()
        return self.tick_values(dmin, dmax)

    def tick_values(self, vmin, vmax):
        return super().tick_values(vmin, vmax)


class FixedLocator(_NativeLocator):
    r"""
    Place ticks at a set of fixed values.

    If *nbins* is None ticks are placed at all values. Otherwise, the *locs* array of
    possible positions will be subsampled to keep the number of ticks
    :math:`\leq nbins + 1`. The subsampling will be done to include the smallest
    absolute value; for example, if zero is included in the array of possibilities, then
    it will be included in the chosen ticks.
    """

    def __init__(self, locs, nbins=None):
        self._locs = np.asarray(locs)
        _api.check_shape((None,), locs=self._locs)
        self.nbins = max(nbins, 2) if nbins is not None else None
        if self._locs is locs:
            # An array the program has, which it may change.
            self._follow_locs()

    @property
    def locs(self):
        """The locations: the locator's own array, which matplotlib reads
        at every draw, so that what a program changes in it moves the
        ticks (the figure asks the locator from now on)."""
        if "_locs_followed" not in self.__dict__:
            self._follow_locs()
        return self._locs

    @locs.setter
    def locs(self, locs):
        self._locs = locs

    def _follow_locs(self):
        """Have the figure of the axis the locator ticks, once it ticks
        one, ask it before each draw whether its array changed."""
        object.__setattr__(self, "_locs_followed", True)
        _tick_helpers.follow_in_place(self)

    def _take_changed_in_place(self):
        """Install the locator again if the program changed its array
        (asked by the figure: `._own_state.follow`)."""
        _tick_helpers.check_native(self)

    def _spec(self):
        return {"kind": "fixed", "locs": [float(v) for v in self._locs],
                "nbins": self.nbins}

    def set_params(self, nbins=None):
        """Set parameters within this locator."""
        if nbins is not None:
            self.nbins = nbins

    def __call__(self):
        return self.tick_values(None, None)

    def tick_values(self, vmin, vmax):
        """
        Return the locations of the ticks.

        .. note::

            Because the values are fixed, *vmin* and *vmax* are not used.
        """
        if self.nbins is None:
            return self._locs
        # A subset of the locations, as matplotlib slices them.
        ticks = super().tick_values(0.0, 0.0)
        if self._locs.dtype.kind in "iu":
            return ticks.astype(self._locs.dtype)
        return ticks


class NullLocator(Locator):
    """
    Place no ticks.
    """

    def _spec(self):
        return {"kind": "null"}

    def __call__(self):
        return self.tick_values(None, None)

    def tick_values(self, vmin, vmax):
        """
        Return the locations of the ticks.

        .. note::

            Because there are no ticks, *vmin* and *vmax* are not used.
        """
        return []


class LinearLocator(_NativeLocator):
    """
    Place ticks at evenly spaced values.

    The first time this function is called, it will try to set the number of
    ticks to make a nice tick partitioning.  Thereafter, the number of ticks
    will be fixed to avoid jumping during interactive navigation.
    """

    _ticks_the_view = True

    def __init__(self, numticks=None, presets=None):
        """
        Parameters
        ----------
        numticks : int or None, default None
            Number of ticks. If None, *numticks* = 11.
        presets : dict or None, default: None
            Dictionary mapping ``(vmin, vmax)`` to an array of locations.
            Overrides *numticks* if there is an entry for the current
            ``(vmin, vmax)``.
        """
        self.numticks = numticks
        if presets is None:
            self.presets = {}
        else:
            self.presets = presets

    @property
    def numticks(self):
        # Old hard-coded default.
        return self._numticks if self._numticks is not None else 11

    @numticks.setter
    def numticks(self, numticks):
        self._numticks = numticks

    def _spec(self):
        # Presets that are no mapping, which matplotlib keeps, preset no
        # ticks of the axis (`tick_values` looks them up as matplotlib does).
        presets = self.presets.items() if isinstance(self.presets, Mapping) else ()
        return {"kind": "linear", "numticks": self.numticks,
                "presets": [((float(a), float(b)), [float(v) for v in locs])
                            for (a, b), locs in presets]}

    def set_params(self, numticks=None, presets=None):
        """Set parameters within this locator."""
        if presets is not None:
            self.presets = presets
        if numticks is not None:
            self.numticks = numticks

    def __call__(self):
        """Return the locations of the ticks."""
        vmin, vmax = self.axis.get_view_interval()
        return self.tick_values(vmin, vmax)

    def tick_values(self, vmin, vmax):
        # docstring inherited
        from .transforms import _nonsingular
        bounds = _nonsingular(vmin, vmax, expander=0.05)
        if bounds in self.presets:
            return self.presets[bounds]
        return super().tick_values(vmin, vmax)

    def view_limits(self, vmin, vmax):
        'Try to choose the view limits intelligently.'

        if vmax < vmin:
            vmin, vmax = vmax, vmin

        if vmin == vmax:
            vmin -= 1
            vmax += 1

        if _rc('axes.autolimit_mode') == 'round_numbers':
            exponent, remainder = divmod(
                math.log10(vmax - vmin), math.log10(max(self.numticks - 1, 1)))
            exponent -= (remainder < .5)
            scale = max(self.numticks - 1, 1) ** (-exponent)
            vmin = math.floor(scale * vmin) / scale
            vmax = math.ceil(scale * vmax) / scale

        from .transforms import _nonsingular
        return _nonsingular(vmin, vmax)


class _Edge_integer:
    """
    Helper for `.MaxNLocator`, `.MultipleLocator`, etc.

    Take floating-point precision limitations into account when calculating
    tick locations as integer multiples of a step.
    """

    def __init__(self, step, offset):
        """
        Parameters
        ----------
        step : float > 0
            Interval between ticks.
        offset : float
            Offset subtracted from the data limits prior to calculating tick
            locations.
        """
        if step <= 0:
            raise ValueError("'step' must be positive")
        self.step = step
        self._offset = abs(offset)

    def closeto(self, ms, edge):
        # Allow more slop when the offset is large compared to the step.
        if self._offset > 0:
            digits = np.log10(self._offset / self.step)
            tol = max(1e-10, 10 ** (digits - 12))
            tol = min(0.4999, tol)
        else:
            tol = 1e-10
        return abs(ms - edge) < tol

    def le(self, x):
        """Return the largest n: n*step <= x."""
        d, m = divmod(x, self.step)
        if self.closeto(m / self.step, 1):
            return d + 1
        return d

    def ge(self, x):
        """Return the smallest n: n*step >= x."""
        d, m = divmod(x, self.step)
        if self.closeto(m / self.step, 0):
            return d
        return d + 1


class MultipleLocator(_NativeLocator):
    """
    Place ticks at every integer multiple of a base plus an offset.
    """

    _ticks_the_view = True

    def __init__(self, base=1.0, offset=0.0):
        """
        Parameters
        ----------
        base : float > 0, default: 1.0
            Interval between ticks.
        offset : float, default: 0.0
            Value added to each multiple of *base*.

            .. versionadded:: 3.8
        """
        self._edge = _Edge_integer(base, 0)
        self._offset = offset

    _matplotlib_tick_values = _locators_as_matplotlib.multiple_tick_values
    _matplotlib_view_limits = _locators_as_matplotlib.multiple_view_limits

    def _spec(self):
        return {"kind": "multiple", "base": _held(self._edge.step),
                "offset": _held(self._offset)}

    def set_params(self, base=None, offset=None):
        """
        Set parameters within this locator.

        Parameters
        ----------
        base : float > 0, optional
            Interval between ticks.
        offset : float, optional
            Value added to each multiple of *base*.

            .. versionadded:: 3.8
        """
        if base is not None:
            self._edge = _Edge_integer(base, 0)
        if offset is not None:
            self._offset = offset

    def view_limits(self, dmin, dmax):
        """
        Set the view limits to the nearest tick values that contain the data.
        """
        return super().view_limits(dmin, dmax)

    def __call__(self):
        """Return the locations of the ticks."""
        return super().__call__()

    def tick_values(self, vmin, vmax):
        return super().tick_values(vmin, vmax)


def scale_range(vmin, vmax, n=1, threshold=100):
    # The power of ten near |vmax - vmin| / n, and an offset (a power of
    # ten) when the mean is large compared to the range: matplotlib's
    # arithmetic on the numbers as given, whose types decide how a zero
    # range fails.
    dv = abs(vmax - vmin)  # > 0 as nonsingular is called before.
    meanv = (vmax + vmin) / 2
    if abs(meanv) / dv < threshold:
        offset = 0
    else:
        offset = math.copysign(10 ** (math.log10(abs(meanv)) // 1), meanv)
    scale = 10 ** (math.log10(dv / n) // 1)
    return scale, offset


class MaxNLocator(_NativeLocator):
    """
    Place evenly spaced ticks, with a cap on the total number of ticks.

    Finds nice tick locations with no more than :math:`nbins + 1` ticks being within the
    view limits. Locations beyond the limits are added to support autoscaling.
    """

    _ticks_the_view = True

    default_params = dict(nbins=10,
                          steps=None,
                          integer=False,
                          symmetric=False,
                          prune=None,
                          min_n_ticks=2)

    def __init__(self, nbins=None, **kwargs):
        """
        Parameters
        ----------
        nbins : int or 'auto', default: 10
            Maximum number of intervals; one less than max number of
            ticks.  If the string 'auto', the number of bins will be
            automatically determined based on the length of the axis.

        steps : array-like, optional
            Sequence of acceptable tick multiples, starting with 1 and
            ending with 10. For example, if ``steps=[1, 2, 4, 5, 10]``,
            ``20, 40, 60`` or ``0.4, 0.6, 0.8`` would be possible
            sets of ticks because they are multiples of 2.
            ``30, 60, 90`` would not be generated because 3 does not
            appear in this example list of steps.

        integer : bool, default: False
            If True, ticks will take only integer values, provided at least
            *min_n_ticks* integers are found within the view limits.

        symmetric : bool, default: False
            If True, autoscaling will result in a range symmetric about zero.

        prune : {'lower', 'upper', 'both', None}, default: None
            Remove the 'lower' tick, the 'upper' tick, or ticks on 'both' sides
            *if they fall exactly on an axis' edge* (this typically occurs when
            :rc:`axes.autolimit_mode` is 'round_numbers').  Removing such ticks
            is mostly useful for stacked or ganged plots, where the upper tick
            of an Axes overlaps with the lower tick of the axes above it.

        min_n_ticks : int, default: 2
            Relax *nbins* and *integer* constraints if necessary to obtain
            this minimum number of ticks.
        """
        if nbins is not None:
            kwargs['nbins'] = nbins
        self.set_params(**{**self.default_params, **kwargs})

    @staticmethod
    def _validate_steps(steps):
        if not np.iterable(steps):
            raise ValueError('steps argument must be an increasing sequence '
                             'of numbers between 1 and 10 inclusive')
        steps = np.asarray(steps)
        if np.any(np.diff(steps) <= 0) or steps[-1] > 10 or steps[0] < 1:
            raise ValueError('steps argument must be an increasing sequence '
                             'of numbers between 1 and 10 inclusive')
        if steps[0] != 1:
            steps = np.concatenate([[1], steps])
        if steps[-1] != 10:
            steps = np.concatenate([steps, [10]])
        return steps

    @staticmethod
    def _staircase(steps):
        # Make an extended staircase within which the needed step will be
        # found.  This is probably much larger than necessary.
        return np.concatenate([0.1 * steps[:-1], steps, [10 * steps[1]]])

    def set_params(self, **kwargs):
        """
        Set parameters for this locator.

        Parameters
        ----------
        nbins : int or 'auto', optional
            see `.MaxNLocator`
        steps : array-like, optional
            see `.MaxNLocator`
        integer : bool, optional
            see `.MaxNLocator`
        symmetric : bool, optional
            see `.MaxNLocator`
        prune : {'lower', 'upper', 'both', None}, optional
            see `.MaxNLocator`
        min_n_ticks : int, optional
            see `.MaxNLocator`
        """
        if 'nbins' in kwargs:
            self._nbins = kwargs.pop('nbins')
            if self._nbins != 'auto':
                self._nbins = int(self._nbins)
        if 'symmetric' in kwargs:
            self._symmetric = kwargs.pop('symmetric')
        if 'prune' in kwargs:
            prune = kwargs.pop('prune')
            _api.check_in_list(['upper', 'lower', 'both', None], prune=prune)
            self._prune = prune
        if 'min_n_ticks' in kwargs:
            self._min_n_ticks = max(1, kwargs.pop('min_n_ticks'))
        if 'steps' in kwargs:
            steps = kwargs.pop('steps')
            if steps is None:
                self._steps = np.array([1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10])
            else:
                self._steps = self._validate_steps(steps)
            self._extended_steps = self._staircase(self._steps)
        if 'integer' in kwargs:
            self._integer = kwargs.pop('integer')
        if kwargs:
            raise _api.kwarg_error("set_params", kwargs)

    _matplotlib_tick_values = _locators_as_matplotlib.maxn_tick_values
    _matplotlib_view_limits = _locators_as_matplotlib.maxn_view_limits

    def _spec(self):
        # Fewer than one bin leaves the ticks to matplotlib's arithmetic.
        nbins = self._nbins if self._nbins == 'auto' else _held(self._nbins, int, least=1)
        return {"kind": "maxn", "nbins": nbins,
                "steps": [float(s) for s in self._steps],
                "integer": bool(self._integer),
                "symmetric": bool(self._symmetric), "prune": self._prune,
                "min_n_ticks": int(self._min_n_ticks)}

    def view_limits(self, dmin, dmax):
        # docstring inherited
        return super().view_limits(dmin, dmax)

    def _gives_raw_ticks(self):
        """Whether a class of the program's places the ticks itself before
        they are pruned (its ``_raw_ticks``, as cartopy's
        ``LongitudeLocator`` chooses its steps): pltrs (Rust) then ticks
        through it, as matplotlib's `tick_values` does."""
        return _program_classes.takes_from_program(type(self), "_raw_ticks")

    def __call__(self):
        # docstring inherited
        if self._gives_raw_ticks() or self._gives_tick_values():
            vmin, vmax = self.axis.get_view_interval()
            return self.tick_values(vmin, vmax)
        return super().__call__()

    def tick_values(self, vmin, vmax):
        # docstring inherited
        if not self._gives_raw_ticks():
            return super().tick_values(vmin, vmax)
        # matplotlib's, around the class' own `_raw_ticks`.
        return _locators_as_matplotlib.maxn_tick_values(self, vmin, vmax)

    def _raw_ticks(self, vmin, vmax):
        """
        Generate a list of tick locations including the range *vmin* to
        *vmax*.  In some applications, one or both of the end locations
        will not be needed, in which case they are trimmed off
        elsewhere.
        """
        # pltrs: the ticks pltrs (Rust) places over *vmin* to *vmax*, made
        # nonsingular, neither symmetric nor pruned; matplotlib's scale of
        # the range by fewer than one bin fails.
        if self._nbins != 'auto' and self._nbins < 1:
            scale_range(vmin, vmax, self._nbins)
        spec = {**self._spec(), "symmetric": False, "prune": None}
        return _ticker.tick_values(spec, float(vmin), float(vmax), self._axis_info())


class LogLocator(_NativeLocator):
    """
    Place logarithmically spaced ticks.

    Places ticks at the values ``subs[j] * base**i``.
    """

    _ticks_the_view = True

    def __init__(self, base=10.0, subs=(1.0,), *, numticks=None):
        """
        Parameters
        ----------
        base : float, default: 10.0
            The base of the log used, so major ticks are placed at ``base**n``, where
            ``n`` is an integer.
        subs : None or {'auto', 'all'} or sequence of float, default: (1.0,)
            Gives the multiples of integer powers of the base at which to place ticks.
            The default of ``(1.0, )`` places ticks only at integer powers of the base.
            Permitted string values are ``'auto'`` and ``'all'``. Both of these use an
            algorithm based on the axis view limits to determine whether and how to put
            ticks between integer powers of the base:
            - ``'auto'``: Ticks are placed only between integer powers.
            - ``'all'``: Ticks are placed between *and* at integer powers.
            - ``None``: Equivalent to ``'auto'``.
        numticks : None or int, default: None
            The maximum number of ticks to allow on a given axis. The default of
            ``None`` will try to choose intelligently as long as this Locator has
            already been assigned to an axis using `~.axis.Axis.get_tick_space`, but
            otherwise falls back to 9.
        """
        if numticks is None:
            if _rc('_internal.classic_mode'):
                numticks = 15
            else:
                numticks = 'auto'
        self._base = float(base)
        self._set_subs(subs)
        self.numticks = numticks

    def set_params(self, base=None, subs=None, *, numticks=None):
        """Set parameters within this locator."""
        if base is not None:
            self._base = float(base)
        if subs is not None:
            self._set_subs(subs)
        if numticks is not None:
            self.numticks = numticks

    def _set_subs(self, subs):
        """
        Set the minor ticks for the log scaling every ``base**i*subs[j]``.
        """
        if subs is None:  # consistency with previous bad API
            self._subs = 'auto'
        elif isinstance(subs, str):
            _api.check_in_list(('all', 'auto'), subs=subs)
            self._subs = subs
        else:
            try:
                self._subs = np.asarray(subs, dtype=float)
            except ValueError as e:
                raise ValueError("subs must be None, 'all', 'auto' or "
                                 "a sequence of floats, not "
                                 f"{subs}.") from e
            if self._subs.ndim != 1:
                raise ValueError("A sequence passed to subs must be "
                                 "1-dimensional, not "
                                 f"{self._subs.ndim}-dimensional.")

    _matplotlib_tick_values = _locators_as_matplotlib.log_tick_values

    def _spec(self):
        subs = self._subs if isinstance(self._subs, str) else [
            float(s) for s in self._subs]
        numticks = self.numticks if self.numticks == "auto" else _held(self.numticks, int)
        return {"kind": "log", "base": self._base, "subs": subs,
                "numticks": numticks}

    def view_limits(self, vmin, vmax):
        """Try to choose the view limits intelligently."""
        vmin, vmax = self.nonsingular(vmin, vmax)
        if _rc('axes.autolimit_mode') == 'round_numbers':
            # The decades around them, numpy floats as matplotlib's.
            return tuple(map(np.float64, super().view_limits(vmin, vmax)))
        return vmin, vmax

    def nonsingular(self, vmin, vmax):
        # docstring inherited
        # matplotlib's, the limits typed as it computes them: its initial
        # range as ints, decades as numpy floats, the limits kept as given.
        if vmin > vmax:
            vmin, vmax = vmax, vmin
        if not np.isfinite(vmin) or not np.isfinite(vmax):
            return 1, 10  # Initial range, no data plotted yet.
        if vmax <= 0:
            _api.warn_external(
                "Data has no positive values, and therefore cannot be "
                "log-scaled.")
            return 1, 10
        if vmin <= 0:
            # Consider shared axises
            minpos = min(axis.get_minpos() for axis in self.axis._get_shared_axis())
            # This should never take effect.
            vmin = minpos if np.isfinite(minpos) else 1e-300
        if vmin == vmax:
            lower, upper = super().nonsingular(vmin, vmax)
            return np.float64(lower), np.float64(upper)
        return vmin, vmax

    def __call__(self):
        """Return the locations of the ticks."""
        return super().__call__()

    def tick_values(self, vmin, vmax):
        return super().tick_values(vmin, vmax)


class SymmetricalLogLocator(_NativeLocator):
    """
    Place ticks spaced linearly near zero and spaced logarithmically beyond a threshold.
    """

    _ticks_the_view = True

    def __init__(self, transform=None, subs=None, linthresh=None, base=None):
        """
        Parameters
        ----------
        transform : `~.scale.SymmetricalLogTransform`, optional
            If set, defines the *base* and *linthresh* of the symlog transform.
        base, linthresh : float, optional
            The *base* and *linthresh* of the symlog transform, as documented
            for `.SymmetricalLogScale`.  These parameters are only used if
            *transform* is not set.
        subs : sequence of float, default: [1]
            The multiples of integer powers of the base where ticks are placed,
            i.e., ticks are placed at
            ``[sub * base**i for i in ... for sub in subs]``.

        Notes
        -----
        Either *transform*, or both *base* and *linthresh*, must be given.
        """
        if transform is not None:
            self._base = transform.base
            self._linthresh = transform.linthresh
        elif linthresh is not None and base is not None:
            self._base = base
            self._linthresh = linthresh
        else:
            raise ValueError("Either transform, or both linthresh "
                             "and base, must be provided.")
        if subs is None:
            self._subs = [1.0]
        else:
            self._subs = subs
        self.numticks = 15

    def set_params(self, subs=None, numticks=None):
        """Set parameters within this locator."""
        if numticks is not None:
            self.numticks = numticks
        if subs is not None:
            self._subs = subs

    _matplotlib_tick_values = _locators_as_matplotlib.symlog_tick_values
    _matplotlib_view_limits = _locators_as_matplotlib.symlog_view_limits

    def _spec(self):
        return {"kind": "symlog", "base": _held(self._base),
                "linthresh": _held(self._linthresh),
                "subs": _held_all(self._subs),
                "numticks": _held(self.numticks, int, least=2)}

    def __call__(self):
        """Return the locations of the ticks."""
        # Note, these are untransformed coordinates
        vmin, vmax = self.axis.get_view_interval()
        return self.tick_values(vmin, vmax)

    def tick_values(self, vmin, vmax):
        # docstring inherited
        if vmax < vmin:
            vmin, vmax = vmax, vmin
        if -self._linthresh <= vmin < vmax <= self._linthresh:
            # Only the linear range is shown: matplotlib's ticks are the
            # limits and 0, the values as given.
            return sorted({vmin, 0, vmax})
        return super().tick_values(vmin, vmax)

    def view_limits(self, vmin, vmax):
        """Try to choose the view limits intelligently."""
        return super().view_limits(vmin, vmax)


class AsinhLocator(_NativeLocator):
    """
    Place ticks spaced evenly on an inverse-sinh scale.

    Generally used with the `~.scale.AsinhScale` class.

    .. note::

       This API is provisional and may be revised in the future
       based on early user feedback.
    """

    def __init__(self, linear_width, numticks=11, symthresh=0.2,
                 base=10, subs=None):
        """
        Parameters
        ----------
        linear_width : float
            The scale parameter defining the extent
            of the quasi-linear region.
        numticks : int, default: 11
            The approximate number of major ticks that will fit
            along the entire axis
        symthresh : float, default: 0.2
            The fractional threshold beneath which data which covers
            a range that is approximately symmetric about zero
            will have ticks that are exactly symmetric.
        base : int, default: 10
            The number base used for rounding tick locations
            on a logarithmic scale. If this is less than one,
            then rounding is to the nearest integer multiple
            of powers of ten.
        subs : tuple, default: None
            Multiples of the number base, typically used
            for the minor ticks, e.g. (2, 5) when base=10.
        """
        super().__init__()
        self.linear_width = linear_width
        self.numticks = numticks
        self.symthresh = symthresh
        self.base = base
        self.subs = subs

    def set_params(self, numticks=None, symthresh=None,
                   base=None, subs=None):
        """Set parameters within this locator."""
        if numticks is not None:
            self.numticks = numticks
        if symthresh is not None:
            self.symthresh = symthresh
        if base is not None:
            self.base = base
        if subs is not None:
            self.subs = subs if len(subs) > 0 else None

    _matplotlib_call = _locators_as_matplotlib.asinh_call
    _matplotlib_tick_values = _locators_as_matplotlib.asinh_tick_values

    def _spec(self):
        return {"kind": "asinh", "linear_width": _held(self.linear_width),
                "numticks": _held(self.numticks, int),
                "symthresh": _held(self.symthresh), "base": _held(self.base),
                "subs": None if self.subs is None else _held_all(self.subs)}

    def __call__(self):
        return super().__call__()

    def tick_values(self, vmin, vmax):
        return super().tick_values(vmin, vmax)


class LogitLocator(MaxNLocator):
    """
    Place ticks spaced evenly on a logit scale.
    """

    def __init__(self, minor=False, *, nbins="auto"):
        """
        Parameters
        ----------
        nbins : int or 'auto', optional
            Number of ticks. Only used if minor is False.
        minor : bool, default: False
            Indicate if this locator is for minor ticks or not.
        """

        self._minor = minor
        super().__init__(nbins=nbins, steps=[1, 2, 5, 10])

    def set_params(self, minor=None, **kwargs):
        """Set parameters within this locator."""
        if minor is not None:
            self._minor = minor
        super().set_params(**kwargs)

    @property
    def minor(self):
        return self._minor

    @minor.setter
    def minor(self, value):
        self.set_params(minor=value)

    _matplotlib_tick_values = _locators_as_matplotlib.logit_tick_values

    def _spec(self):
        return {"kind": "logit", "minor": bool(self._minor),
                "nbins": self._nbins}

    def _as_matplotlib(self, *values):
        # docstring inherited
        # matplotlib takes the logarithms of limits of another precision
        # (a numpy.float32) in that precision, with numpy's log10 of the
        # platform (Intel's SVML where numpy runs with AVX-512, the C
        # library's elsewhere, which round 0.01 to either side of a
        # decade): its code computes them, with that numpy.
        return super()._as_matplotlib(*values) or _other_precision(*values)

    def tick_values(self, vmin, vmax):
        # docstring inherited
        self._refuse_polar()
        _locators_as_matplotlib.warn_outside_logit(vmin, vmax)
        ticks = super().tick_values(vmin, vmax)
        if (self._minor and not len(ticks)
                and _logit_zoomed(*MaxNLocator.nonsingular(self, vmin, vmax))):
            # matplotlib's minor ticks of a view zoomed in so far that it
            # is ticked linearly: a list.
            return []
        return ticks

    @_qualnames.named_as("MaxNLocator.__call__")
    def __call__(self):
        # matplotlib's call (`MaxNLocator`'s) asks `tick_values`, which
        # refuses a polar axis before anything is computed.
        self._refuse_polar()
        return super().__call__()

    def _refuse_polar(self):
        """Refuse to tick a polar axis, as matplotlib's locator does."""
        # dummy axis has no axes attribute
        if hasattr(self.axis, "axes") and self.axis.axes.name == "polar":
            raise NotImplementedError("Polar axis cannot be logit scaled yet")

    def nonsingular(self, vmin, vmax):
        # docstring inherited
        _locators_as_matplotlib.warn_outside_logit(vmin, vmax)
        if _other_precision(vmin, vmax):
            return _locators_as_matplotlib.logit_nonsingular(self, vmin, vmax)
        return super().nonsingular(vmin, vmax)


def _other_precision(*values):
    """Whether one of *values* is a numpy float that is no Python float (a
    numpy.float32, say), which matplotlib computes with in its precision."""
    return any(isinstance(value, np.floating) and not isinstance(value, float)
               for value in values)


def _logit_zoomed(vmin, vmax):
    """Whether the logit view *vmin*, *vmax* holds fewer than two of the
    ideal logit ticks (..., 1e-1, 1/2, 1-1e-1, ...), which matplotlib's
    `LogitLocator` then ticks as a linear view."""
    binf = int(np.floor(np.log10(vmin)) if vmin < 0.5
               else 0 if vmin < 0.9 else -np.ceil(np.log10(1 - vmin)))
    bsup = int(np.ceil(np.log10(vmax)) if vmax <= 0.5
               else 1 if vmax <= 0.9 else -np.floor(np.log10(1 - vmax)))
    return bsup - binf - 1 < 2


class AutoLocator(MaxNLocator):
    """
    Place evenly spaced ticks, with the step size and maximum number of ticks chosen
    automatically.

    This is a subclass of `~matplotlib.ticker.MaxNLocator`, with parameters
    *nbins = 'auto'* and *steps = [1, 2, 2.5, 5, 10]*.
    """

    def __init__(self):
        """
        To know the values of the non-public parameters, please have a
        look to the defaults of `~matplotlib.ticker.MaxNLocator`.
        """
        if _rc('_internal.classic_mode'):
            nbins = 9
            steps = [1, 2, 5, 10]
        else:
            nbins = 'auto'
            steps = [1, 2, 2.5, 5, 10]
        super().__init__(nbins=nbins, steps=steps)
        self._defaults = (nbins, steps)

    def _spec(self):
        # An AutoLocator left as made is pltrs (Rust)'s, which follows
        # the classic mode it is used in.
        spec = super()._spec()
        nbins, steps = self._defaults
        if (spec["nbins"] == nbins and spec["steps"] == steps
                and not spec["integer"] and not spec["symmetric"]
                and spec["prune"] is None and spec["min_n_ticks"] == 2):
            return {"kind": "auto"}
        return spec


class AutoMinorLocator(_NativeLocator):
    """
    Place evenly spaced minor ticks, with the step size and maximum number of ticks
    chosen automatically.

    The Axis must use a linear scale and have evenly spaced major ticks.
    """

    _reads_major_ticks = True
    ndivs = _tick_state.ticked()

    def __init__(self, n=None):
        """
        Parameters
        ----------
        n : int or 'auto', default: :rc:`xtick.minor.ndivs` or :rc:`ytick.minor.ndivs`
            The number of subdivisions of the interval between major ticks;
            e.g., n=2 will place a single minor tick midway between major ticks.

            If *n* is 'auto', it will be set to 4 or 5: if the distance
            between the major ticks equals 1, 2.5, 5 or 10 it can be perfectly
            divided in 5 equidistant sub-intervals with a length multiple of
            0.05; otherwise, it is divided in 4 sub-intervals.
        """
        self.ndivs = n

    def _spec(self):
        return {"kind": "autominor", "n": self.ndivs}

    def __call__(self):
        # docstring inherited
        info = self._axis_info()
        if self._drawn_axis is not None and self.axis is not None:
            # Ticking from a description of its axis (`_tick_helpers.drawing`),
            # the locator reads the major ticks from the axis, as
            # matplotlib's does in its call (`get_majorticklocs`).
            majors = np.asarray(self.axis.get_majorticklocs(), dtype=float).ravel()
            info = _axis.with_major_ticks(info, [float(v) for v in majors])
        locs = _ticker.locate(self._spec(), info)
        if self.ndivs is None:
            self._settle_ndivs(info)
        return locs

    def _settle_ndivs(self, info):
        """Take the rcParam's number of divisions, as matplotlib's locator
        does the first time it places ticks between major ones (at least
        two, those of *info*, the description of its axis), on an axis not
        of a log scale."""
        axis = self.axis
        get_scale = getattr(axis, "get_scale", None)
        if info is None or get_scale is None or get_scale() == 'log':
            return
        major = (info["major_ticks"] if isinstance(info, dict)
                 else _axis.info_fields(info)[3])
        if len(np.unique(major)) < 2:
            return
        self.ndivs = _rc('ytick.minor.ndivs'
                         if getattr(axis, "axis_name", None) == 'y'
                         else 'xtick.minor.ndivs')  # for x and z axis

    def tick_values(self, vmin, vmax):
        raise NotImplementedError(
            f"Cannot get tick locations for a {type(self).__name__}")


class _NativeLocatorView(Locator):
    """
    A locator pltrs (Rust) installed on an axis that has no class of its
    own here: it locates ticks through its token (see `._tick_helpers`).
    """

    def __init__(self, token):
        self._token = token

    def __call__(self):
        return _axis.helper_locate(self._token, self._axis_info())

    def tick_values(self, vmin, vmax):
        return _axis.helper_tick_values(self._token, float(vmin), float(vmax),
                                        self._axis_info())


class _NativeFormatterView(Formatter):
    """
    A formatter pltrs (Rust) installed on an axis that has no class of its
    own here: it labels ticks through its token (see `._tick_helpers`).
    """

    _offset = _tick_state.ticked('')

    def __init__(self, token):
        self._token = token

    def format_ticks(self, values):
        # docstring inherited
        labels, self._offset = _axis.helper_format_ticks(
            self._token, np.asarray(values, dtype=float), self._axis_info())
        return labels

    def __call__(self, x, pos=None):
        return self.format_ticks([x])[0]

    def get_offset(self):
        # docstring inherited
        return self._offset

    def format_data(self, value):
        # docstring inherited
        return _axis.helper_format_data(self._token, float(value), False,
                                        self._axis_info())

    def format_data_short(self, value):
        # docstring inherited
        return _axis.helper_format_data(self._token, float(value), True,
                                        self._axis_info())


# What matplotlib's module also has from its imports, which programs
# import from it.
import itertools  # noqa: E402, F401
from . import transforms as mtransforms  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
