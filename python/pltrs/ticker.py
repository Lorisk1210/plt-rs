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
one of its attributes changes the spec (`TickHelper.__setattr__`).
"""

import locale
import logging
from numbers import Integral
import string

import numpy as np

from . import _api, _tick_helpers, cbook
from ._native import axis as _axis, ticker as _ticker

_log = logging.getLogger(__name__)

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
    return rcParams[key]


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
    """The base of locators and formatters: the axis they serve."""

    axis = None

    def __setattr__(self, name, value):
        super().__setattr__(name, value)
        _tick_helpers.check_native(self)

    def set_axis(self, axis):
        """Attach the helper to *axis*."""
        self.axis = axis

    def create_dummy_axis(self, **kwargs):
        """Attach the helper to a `_DummyAxis` unless it has an axis."""
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
        native_info = getattr(axis, "_native_info", None)
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


class Formatter(TickHelper):
    """
    Create a string based on a tick value and location.
    """
    # some classes want to see all the locs to help format
    # individual ones
    _locs = []
    locs = _api.deprecate_privatize_attribute("3.11")

    def __call__(self, x, pos=None):
        """
        Return the format for tick value *x* at position pos.
        ``pos=None`` indicates an unspecified location.
        """
        raise NotImplementedError('Derived must override')

    def format_ticks(self, values):
        """Return the tick labels for all the ticks at once."""
        self.set_locs(values)
        return [self(value, i) for i, value in enumerate(values)]

    def format_data(self, value):
        """
        Return the full string representation of the value with the
        position unspecified.
        """
        return self.__call__(value)

    def format_data_short(self, value):
        """
        Return a short string version of the tick value.

        Defaults to the position-independent long value.
        """
        return self.format_data(value)

    def get_offset(self):
        """Return the offset (or scale) text shown at the end of the axis."""
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
        """Set the offset text."""
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
        """Set the offset text."""
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

    _labels = ()
    _offset = ''

    def _spec(self):
        """The native description of the formatter (its kind and
        parameters)."""
        raise NotImplementedError('Derived must override')

    def set_locs(self, locs):
        # docstring inherited
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

    def __call__(self, x, pos=None):
        """Return the label of tick value *x*, formatted with the others set
        by `set_locs`."""
        locs = self._locs
        if pos is not None and 0 <= pos < len(locs) and locs[pos] == x:
            return self._labels[pos]
        if x in locs:
            return self._labels[list(locs).index(x)]
        labels, _ = _ticker.format_ticks(
            self._spec(), np.asarray([*locs, x], dtype=float), self._axis_info())
        return labels[-1]

    def format_ticks(self, values):
        # docstring inherited
        self.set_locs(values)
        if type(self).__call__.__module__ != __name__:
            # A subclass of the program's labels each tick itself.
            return [self(value, i) for i, value in enumerate(values)]
        return list(self._labels)

    def get_offset(self):
        # docstring inherited
        return self._offset

    def format_data(self, value):
        # docstring inherited
        return _ticker.format_data(self._spec(), float(value), False,
                                   self._axis_info())

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
    """

    orderOfMagnitude = _api.deprecate_privatize_attribute("3.11")
    format = _api.deprecate_privatize_attribute("3.11")

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
        use_offset = True if self._useOffset else (
            float(self.offset) if self.offset else False)
        return {"kind": "scalar", "use_offset": use_offset,
                "use_mathtext": self._useMathText, "usetex": self._usetex,
                "scientific": self._scientific,
                "powerlimits": tuple(self._powerlimits),
                "locale": _numeric_locale() if self._useLocale else None}

    def get_usetex(self):
        """Return whether TeX's math mode is enabled for rendering."""
        return self._usetex

    def set_usetex(self, val):
        """Set whether to use TeX's math mode for rendering numbers in the
        formatter."""
        self._usetex = _val_or_rc(val, 'text.usetex')

    usetex = property(fget=get_usetex, fset=set_usetex)

    def get_useOffset(self):
        """
        Return whether automatic mode for offset notation is active.

        This returns True if ``set_useOffset(True)``; it returns False if an
        explicit offset was set, e.g. ``set_useOffset(1000)``.
        """
        return self._useOffset

    def set_useOffset(self, val):
        """
        Set whether to use offset notation.

        Parameters
        ----------
        val : bool or float
            - If False, do not use offset notation.
            - If True (=automatic mode), use offset notation if it can make
              the residual numbers significantly shorter. The exact behavior
              is controlled by :rc:`axes.formatter.offset_threshold`.
            - If a number, force an offset of the given value.
        """
        if isinstance(val, bool):
            self.offset = 0
            self._useOffset = val
        else:
            self._useOffset = False
            self.offset = val

    useOffset = property(fget=get_useOffset, fset=set_useOffset)

    def get_useLocale(self):
        """Return whether locale settings are used for formatting."""
        return self._useLocale

    def set_useLocale(self, val):
        """
        Set whether to use locale settings for decimal sign and positive sign.

        Parameters
        ----------
        val : bool or None
            *None* resets to :rc:`axes.formatter.use_locale`.
        """
        self._useLocale = bool(_val_or_rc(val, 'axes.formatter.use_locale'))

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
        """Return whether to use fancy math formatting."""
        return self._useMathText

    def set_useMathText(self, val):
        """
        Set whether to use fancy math formatting.

        If active, scientific notation is formatted as :math:`1.2 \\times 10^3`.
        """
        self._useMathText = bool(_val_or_rc(val, 'axes.formatter.use_mathtext'))

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
        return super().__call__(x, pos)

    def set_locs(self, locs):
        # docstring inherited
        info = self._axis_info()
        self._set_locs_on(locs, info)
        locs = np.asarray(self._locs, dtype=float)
        offset, self._orderOfMagnitude = _ticker.scalar_offset(
            self._spec(), locs, info)
        if self._useOffset:
            self.offset = offset
        if len(locs) and self._spec()["kind"] == "scalar":
            self._format = _ticker.scalar_format(self._spec(), locs, info)

    def format_data_short(self, value):
        # docstring inherited
        if value is np.ma.masked:
            return ""
        if isinstance(value, Integral):
            return self.fix_minus("%d" % value)
        if getattr(self.axis, "__name__", "") in ("xaxis", "yaxis"):
            text = _axis.format_data_short(*self.axis._args(), float(value))
            if text is not None:
                return text
        return super().format_data_short(value)


class LogFormatter(Formatter):
    """
    Base class for formatting ticks on a log or symlog scale.

    Parameters
    ----------
    base : float, default: 10.
        Base of the logarithm used in all calculations.
    labelOnlyBase : bool, default: False
        If True, label ticks only at integer powers of base.
    minor_thresholds : (subset, all), default: (1, 0.4)
        If labelOnlyBase is False, these two numbers control
        the labeling of ticks that are not at integer powers of
        base; normally these are the minor ticks.
    linthresh : None or float, default: None
        If a symmetric log scale is in use, its ``linthresh``
        parameter must be supplied here.
    """

    _kind = "log"

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
        self._sublabels = _ticker.log_sublabels(self._spec(), self._axis_info())

    def __call__(self, x, pos=None):
        # docstring inherited
        return _ticker.log_label(self._spec(), float(x), self._sublabels,
                                 self._axis_info())

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


class LogFormatterSciNotation(LogFormatterMathtext):
    """
    Format values following scientific notation in a logarithmic axis.
    """

    _kind = "log_scinotation"


class LogitFormatter(_NativeFormatter):
    """
    Probability formatter (using Math text).
    """

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


class EngFormatter(ScalarFormatter):
    """
    Format axis values using engineering prefixes to represent powers
    of 1000, plus a specified unit, e.g., 10 MHz instead of 1e7.
    """

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
            representations of powers of 1000.
        places : int, default: None
            Precision with which to display the number, specified in
            digits after the decimal point (there will be between one
            and three digits before the decimal point). If it is None,
            the formatting falls back to the floating point format '%g',
            which displays up to 6 *significant* digits.
        sep : str, default: " "
            Separator used between the value and the prefix/unit.
        usetex : bool, default: :rc:`text.usetex`
            To enable/disable the use of TeX's math mode for rendering the
            numbers in the formatter.
        useMathText : bool, default: :rc:`axes.formatter.use_mathtext`
            To enable/disable the use mathtext for rendering the numbers in
            the formatter.
        useOffset : bool or float, default: False
            Whether to use offset notation with :math:`10^{3*N}` based prefixes.
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
            labels, _ = _ticker.format_ticks(
                {**self._spec(), "use_offset": False},
                np.asarray([x], dtype=float), self._axis_info())
            return labels[0]
        return _NativeFormatter.__call__(self, x, pos)

    def format_eng(self, num):
        """
        Format a number in engineering notation, appending a letter
        representing the power of 1000 of the original number.
        """
        labels, _ = _ticker.format_ticks(
            {**self._spec(), "use_offset": False, "unit": ""},
            np.asarray([num], dtype=float), None)
        return labels[0].rstrip()


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
        return {"kind": "percent", "xmax": self.xmax, "decimals": self.decimals,
                "symbol": self._symbol, "is_latex": bool(self._is_latex)}

    def __call__(self, x, pos=None):
        """Format the tick as a percentage with the appropriate scaling."""
        if len(self._locs) == 0:
            labels, _ = _ticker.format_ticks(
                self._spec(), np.asarray([x], dtype=float), self._axis_info())
            return labels[0]
        return super().__call__(x, pos)

    def convert_to_pct(self, x):
        """*x* as a percentage of *xmax*."""
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
        return _ticker.format_pct(self._spec(), float(x), float(display_range))

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

    def raise_if_exceeds(self, locs):
        """
        Log at WARNING level if *locs* is longer than `Locator.MAXTICKS`.

        This is intended to be called immediately before returning *locs* from
        ``__call__`` to inform users in case their Locator returns a huge
        number of ticks, causing Matplotlib to run out of memory.
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


class _NativeLocator(Locator):
    """A locator whose ticks pltrs (Rust) computes from its parameters."""

    def _spec(self):
        """The native description of the locator (its kind and
        parameters)."""
        raise NotImplementedError('Derived must override')

    def tick_values(self, vmin, vmax):
        # docstring inherited
        locs = _ticker.tick_values(self._spec(), float(vmin), float(vmax),
                                   self._axis_info())
        return self.raise_if_exceeds(locs)

    def __call__(self):
        """Return the locations of the ticks."""
        return self.raise_if_exceeds(
            _ticker.locate(self._spec(), self._axis_info()))

    def view_limits(self, vmin, vmax):
        # docstring inherited
        return _ticker.view_limits(self._spec(), float(vmin), float(vmax),
                                   self._axis_info())

    def nonsingular(self, v0, v1):
        # docstring inherited
        return _ticker.nonsingular(self._spec(), float(v0), float(v1),
                                   self._axis_info())


class IndexLocator(_NativeLocator):
    """
    Place ticks at every nth point plotted.

    IndexLocator assumes index plotting; i.e., that the ticks are placed at
    integer values in the range between 0 and len(data) inclusive.
    """

    def __init__(self, base, offset):
        """Place ticks every *base* data point, starting at *offset*."""
        self._base = base
        self.offset = offset

    def _spec(self):
        return {"kind": "index", "base": float(self._base),
                "offset": float(self.offset)}

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


class FixedLocator(_NativeLocator):
    r"""
    Place ticks at a set of fixed values.

    If *nbins* is None ticks are placed at all values. Otherwise, the *locs*
    array of possible positions will be subsampled to keep the number of ticks
    :math:`\leq nbins + 1`. The subsampling will be done to include the
    smallest absolute value; for example, if zero is included in the array of
    possibilities, then it will be included in the chosen ticks.
    """

    def __init__(self, locs, nbins=None):
        self.locs = np.asarray(locs)
        _api.check_shape((None,), locs=self.locs)
        self.nbins = max(nbins, 2) if nbins is not None else None

    def _spec(self):
        return {"kind": "fixed", "locs": [float(v) for v in self.locs],
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

            Because the values are fixed, vmin and vmax are not used in this
            method.

        """
        if self.nbins is None:
            return self.locs
        # A subset of the locations, as matplotlib slices them.
        ticks = super().tick_values(0.0, 0.0)
        if self.locs.dtype.kind in "iu":
            return ticks.astype(self.locs.dtype)
        return ticks


class NullLocator(Locator):
    """
    No ticks
    """

    def _spec(self):
        return {"kind": "null"}

    def __call__(self):
        return self.tick_values(None, None)

    def tick_values(self, vmin, vmax):
        """
        Return the locations of the ticks.

        .. note::

            Because the values are Null, vmin and vmax are not used in this
            method.
        """
        return []


class LinearLocator(_NativeLocator):
    """
    Place ticks at evenly spaced values.

    The first time this function is called, it will try to set the
    number of ticks to make a nice tick partitioning.  Thereafter, the
    number of ticks will be fixed to avoid jumping during interactive
    navigation.
    """

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
        """The number of ticks (11 unless it was set)."""
        # Old hard-coded default.
        return self._numticks if self._numticks is not None else 11

    @numticks.setter
    def numticks(self, numticks):
        self._numticks = numticks

    def _spec(self):
        return {"kind": "linear", "numticks": self.numticks,
                "presets": [((float(a), float(b)), [float(v) for v in locs])
                            for (a, b), locs in self.presets.items()]}

    def set_params(self, numticks=None, presets=None):
        """Set parameters within this locator."""
        if presets is not None:
            self.presets = presets
        if numticks is not None:
            self.numticks = numticks

    def tick_values(self, vmin, vmax):
        # docstring inherited
        from .transforms import _nonsingular
        bounds = _nonsingular(vmin, vmax, expander=0.05)
        if bounds in self.presets:
            return self.presets[bounds]
        return super().tick_values(vmin, vmax)


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


class MultipleLocator(_NativeLocator):
    """
    Place ticks at every integer multiple of a base plus an offset.
    """

    def __init__(self, base=1.0, offset=0.0):
        """
        Parameters
        ----------
        base : float > 0, default: 1.0
            Interval between ticks.
        offset : float, default: 0.0
            Value added to each multiple of *base*.
        """
        self._edge = _Edge_integer(base, 0)
        self._offset = offset

    def _spec(self):
        return {"kind": "multiple", "base": float(self._edge.step),
                "offset": float(self._offset)}

    def set_params(self, base=None, offset=None):
        """
        Set parameters within this locator.

        Parameters
        ----------
        base : float > 0, optional
            Interval between ticks.
        offset : float, optional
            Value added to each multiple of *base*.
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


def scale_range(vmin, vmax, n=1, threshold=100):
    """The power of ten near ``|vmax - vmin| / n``, and an offset (a power
    of ten) when the mean is large compared to the range."""
    scale, offset = _ticker.scale_range(vmin, vmax, n, threshold)
    # matplotlib's offset of a range near zero is the int 0.
    return scale, (0 if offset == 0 else offset)


class MaxNLocator(_NativeLocator):
    """
    Place evenly spaced ticks, with a cap on the total number of ticks.

    Finds nice tick locations with no more than :math:`nbins + 1` ticks being
    within the view limits. Locations beyond the limits are added to support
    autoscaling.
    """
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

    def _spec(self):
        return {"kind": "maxn", "nbins": self._nbins,
                "steps": [float(s) for s in self._steps],
                "integer": bool(self._integer),
                "symmetric": bool(self._symmetric), "prune": self._prune,
                "min_n_ticks": int(self._min_n_ticks)}

    def view_limits(self, dmin, dmax):
        # docstring inherited
        return super().view_limits(dmin, dmax)


class LogLocator(_NativeLocator):
    """
    Place logarithmically spaced ticks.

    Places ticks at the values ``subs[j] * base**i``.
    """

    def __init__(self, base=10.0, subs=(1.0,), *, numticks=None):
        """
        Parameters
        ----------
        base : float, default: 10.0
            The base of the log used, so major ticks are placed at ``base**n``,
            where ``n`` is an integer.
        subs : None or {'auto', 'all'} or sequence of float, default: (1.0,)
            Gives the multiples of integer powers of the base at which to
            place ticks.  The default of ``(1.0, )`` places ticks only at
            integer powers of the base.
            Permitted string values are ``'auto'`` and ``'all'``.
            Both of these use an algorithm based on the axis view
            limits to determine whether and how to put ticks between
            integer powers of the base:
            - ``'auto'``: Ticks are placed only between integer powers.
            - ``'all'``: Ticks are placed between *and* at integer powers.
            - ``None``: Equivalent to ``'auto'``.
        numticks : None or int, default: None
            The maximum number of ticks to allow on a given axis. The default
            of ``None`` will try to choose intelligently as long as this
            Locator has already been assigned to an axis using
            `~.axis.Axis.get_tick_space`, but otherwise falls back to 9.
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

    def _spec(self):
        subs = self._subs if isinstance(self._subs, str) else [
            float(s) for s in self._subs]
        return {"kind": "log", "base": self._base, "subs": subs,
                "numticks": self.numticks}

    def nonsingular(self, vmin, vmax):
        # docstring inherited
        lower, upper = super().nonsingular(vmin, vmax)
        # The limits typed as matplotlib's computes them: its initial
        # range as ints, decades as numpy floats, the limits kept as given.
        if vmin > vmax:
            vmin, vmax = vmax, vmin
        if not np.isfinite(vmin) or not np.isfinite(vmax) or vmax <= 0:
            return 1, 10
        if vmin == vmax:
            return np.float64(lower), np.float64(upper)
        return (vmin if vmin > 0 else np.float64(lower)), vmax


class SymmetricalLogLocator(_NativeLocator):
    """
    Place ticks spaced linearly near zero and spaced logarithmically beyond a
    threshold.
    """

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

    def _spec(self):
        return {"kind": "symlog", "base": float(self._base),
                "linthresh": float(self._linthresh),
                "subs": [float(s) for s in self._subs],
                "numticks": int(self.numticks)}

    def tick_values(self, vmin, vmax):
        # docstring inherited
        if vmax < vmin:
            vmin, vmax = vmax, vmin
        if -self._linthresh <= vmin < vmax <= self._linthresh:
            # Only the linear range is shown: matplotlib's ticks are the
            # limits and 0, the values as given.
            return sorted({vmin, 0, vmax})
        return super().tick_values(vmin, vmax)


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

    def _spec(self):
        return {"kind": "asinh", "linear_width": float(self.linear_width),
                "numticks": int(self.numticks),
                "symthresh": float(self.symthresh), "base": float(self.base),
                "subs": None if self.subs is None else [
                    float(s) for s in self.subs]}


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
        """Whether the locator places minor ticks."""
        return self._minor

    @minor.setter
    def minor(self, value):
        self.set_params(minor=value)

    def _spec(self):
        return {"kind": "logit", "minor": bool(self._minor),
                "nbins": self._nbins}

    def tick_values(self, vmin, vmax):
        # docstring inherited
        # dummy axis has no axes attribute
        if hasattr(self.axis, "axes") and self.axis.axes.name == "polar":
            raise NotImplementedError("Polar axis cannot be logit scaled yet")
        _warn_outside_logit(vmin, vmax)
        return super().tick_values(vmin, vmax)

    def nonsingular(self, vmin, vmax):
        # docstring inherited
        _warn_outside_logit(vmin, vmax)
        return super().nonsingular(vmin, vmax)


def _warn_outside_logit(vmin, vmax):
    """matplotlib's warning of `LogitLocator.nonsingular` for finite limits
    holding no value between 0 and 1."""
    vmin, vmax = min(vmin, vmax), max(vmin, vmax)
    if np.isfinite(vmin) and np.isfinite(vmax) and (vmax <= 0 or vmin >= 1):
        _api.warn_external(
            "Data has no values between 0 and 1, and therefore cannot be "
            "logit-scaled.")


class AutoLocator(MaxNLocator):
    """
    Place evenly spaced ticks, with the step size and maximum number of ticks
    chosen automatically.

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
    Place evenly spaced minor ticks, with the step size and maximum number of
    ticks chosen automatically.

    The Axis must use a linear scale and have evenly spaced major ticks.
    """

    _reads_major_ticks = True

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
        return _ticker.locate(self._spec(), self._axis_info())

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

    _offset = ''

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
