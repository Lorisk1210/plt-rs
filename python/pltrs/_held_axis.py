"""
matplotlib's `.Axis`, `.XAxis` and `.YAxis`, for the axes pltrs (Rust) does
not hold (see "Projections of programs" in ``python/DESIGN.md``).


pltrs (Rust) keeps the x and y axis of an Axes (and the z axis of a 3-D
one) and draws them from its state. An axis of a program's class under
another name (``axis_name = 't'``, mpltern's ``TAxis``) has a geometry
only its class knows: where its ticks go is what its tick class'
``update_position`` and transforms say, where its label goes what its
``_update_label_position`` says. Such an axis is *held*: it is matplotlib's
axis, the code below, over the package's objects (standalone `.Tick`
objects, loose `.Text` labels, the tick helpers), and pltrs (Rust) holds
its place only: a plain artist of the Axes, painted through the axis'
``draw`` in the place the Axes draws its axes and measured through its
``get_tightbbox`` (``_native.axes.add_program_axis``).


The classes here are namespaces of functions, never instantiated: the
methods of the package's `.Axis`, `.XAxis` and `.YAxis` hand a held axis'
calls over to the function of the same name here (`.axis._serving_held`),
so that a program's class deriving from `.XAxis` reaches them through its
own methods and its ``super()`` calls. What they have as attributes
(``label``, ``majorTicks``, ``units``...) lives in the axis' ``__dict__``
(`read`, `write`, `delete`).


Kept close to matplotlib 3.11.2's ``axis.py`` (Matplotlib License), its
docstrings left to the methods of `.axis` that carry them.
"""

import datetime
import functools
from numbers import Real
import warnings

import numpy as np

from . import (_api, _handles, _python_drawing, axis as _axis_module, cbook,
               scale as mscale, text as mtext, ticker as mticker,
               transforms as mtransforms, units as munits)
from ._axis_artists import XTick, YTick
from ._native import axes as _axes
from .artist import Artist, allow_rasterization
from .ticker import NullLocator

mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package


#: The group matplotlib's renderers open around an axis.
_GROUP = "matplotlib.axis"


def _namespace(axis):
    """The class here whose functions serve *axis*."""
    if isinstance(axis, _axis_module.XAxis):
        return XAxis
    if isinstance(axis, _axis_module.YAxis):
        return YAxis
    return Axis


def function(axis, name):
    """matplotlib's method *name* of the held *axis*, as a function of the
    axis; None where matplotlib's axis has no such method (a helper of the
    package's own)."""
    found = getattr(_namespace(axis), name, None)
    return found if callable(found) else None


def _descriptor(axis, name):
    """What matplotlib's axis class keeps under *name* for its instances
    (a property, the lazy tick lists), or None."""
    for klass in _namespace(axis).__mro__:
        if name in vars(klass):
            found = vars(klass)[name]
            return found if hasattr(found, "__get__") and not callable(found) else None
    return None


def _beyond_axis(axis, name):
    """What the classes after the package's `.Axis` in the class of *axis*
    keep under *name* (`.Artist`'s ``zorder``), or None."""
    mro = type(axis).__mro__
    for klass in mro[mro.index(_axis_module.Axis) + 1:]:
        if name in vars(klass):
            return vars(klass)[name]
    return None


def read(axis, name):
    """The attribute *name* of the held *axis*, which the package's class
    keeps as a property of the native axis: what the axis holds, else what
    matplotlib's class gives (a lazy tick list), else `.Artist`'s."""
    try:
        return axis.__dict__[name]
    except KeyError:
        pass
    descriptor = _descriptor(axis, name) or _beyond_axis(axis, name)
    if not hasattr(descriptor, "__get__"):
        raise AttributeError(
            f"{type(axis).__name__!r} object has no attribute {name!r}") from None
    return descriptor.__get__(axis, type(axis))


def write(axis, name, value):
    """Set the attribute *name* of the held *axis*."""
    descriptor = _descriptor(axis, name)
    if not hasattr(descriptor, "__set__"):
        descriptor = _beyond_axis(axis, name)
    if hasattr(descriptor, "__set__"):
        descriptor.__set__(axis, value)
    else:
        axis.__dict__[name] = value


def delete(axis, name):
    """Delete the attribute *name* of the held *axis*."""
    try:
        del axis.__dict__[name]
    except KeyError:
        raise AttributeError(
            f"{type(axis).__name__!r} object has no attribute {name!r}") from None


def _tick_group_visible(kw):
    """Whether the ticks of the parameters *kw* show any of their parts
    (matplotlib's ``Axis._tick_group_visible``)."""
    return (kw.get('tick1On') is not False or
            kw.get('tick2On') is not False or
            kw.get('label1On') is not False or
            kw.get('label2On') is not False or
            kw.get('gridOn') is not False)


def _validate_converted_limits(limit, convert):
    """*limit* converted with *convert*, refused when it is no finite
    number (matplotlib's ``_AxesBase._validate_converted_limits``)."""
    if limit is not None:
        converted_limit = convert(limit)
        if isinstance(converted_limit, np.ndarray):
            converted_limit = converted_limit.squeeze()
        if (isinstance(converted_limit, Real)
                and not np.isfinite(converted_limit)):
            raise ValueError("Axis limits cannot be NaN or Inf")
        return converted_limit


def _ticker():
    """A ticker holding its own helpers, as matplotlib's `.Ticker` is."""
    ticker = _axis_module.Ticker()
    ticker._locator_is_default = True
    ticker._formatter_is_default = True
    return ticker


def _hold(axis, axes):
    """
    Give the held *axis* of *axes* its artist: one the Axes draws and
    measures in the place of its axes when the Axes lists the axis
    (``_axis_names``), else a plain artist no one draws. Either is painted
    through the axis' ``draw`` and measured through its ``get_tightbbox``.
    """
    # What matplotlib's constructor reads first of *axes*.
    axes.get_figure(root=False)
    name = axis.axis_name
    listed = name in axes._axis_names
    if listed:
        axis._native = _axes.add_program_axis(axes._native, name)
    Artist.__init__(axis)
    axis._axes = axes
    if listed:
        _handles.adopt(axis, axis._native, axes.figure)
    _python_drawing.draw_through_python(axis, measures=True)


class _LazyTickList:
    # A descriptor making a tick list of a held axis when it is first read.

    def __init__(self, major):
        self._major = major

    def __get__(self, instance, owner):
        if instance is None:
            return self
        else:
            # instance._get_tick() can itself try to access the majorTicks
            # attribute (e.g. in certain projection classes which override
            # e.g. get_xaxis_text1_transform).  In order to avoid infinite
            # recursion, first set the majorTicks on the instance temporarily
            # to an empty list. Then create the tick; note that _get_tick()
            # may call reset_ticks(). Therefore, the final tick list is
            # created and assigned afterwards.
            if self._major:
                instance.majorTicks = []
                tick = instance._get_tick(major=True)
                instance.majorTicks = [tick]
                return instance.majorTicks
            else:
                instance.minorTicks = []
                tick = instance._get_tick(major=False)
                instance.minorTicks = [tick]
                return instance.minorTicks


class Axis:
    def __init__(self, axes, *, pickradius=15, clear=True):
        _hold(self, axes)
        self._remove_overlapping_locs = True

        self.isDefault_label = True

        self.major = _ticker()
        self.minor = _ticker()
        self.callbacks = cbook.CallbackRegistry(signals=["units"])

        self._autolabelpos = True

        self.label = mtext.Text(
            np.nan, np.nan,
            fontsize=mpl.rcParams['axes.labelsize'],
            fontweight=mpl.rcParams['axes.labelweight'],
            color=mpl.rcParams['axes.labelcolor'],
        )  #: The `.Text` object of the axis label.

        self._set_artist_props(self.label)
        self.offsetText = mtext.Text(np.nan, np.nan)
        self._set_artist_props(self.offsetText)

        self.labelpad = mpl.rcParams['axes.labelpad']

        self.pickradius = pickradius

        # Initialize here for testing; later add API
        self._major_tick_kw = dict()
        self._minor_tick_kw = dict()

        if clear:
            self.clear()
        else:
            self._converter = None
            self._converter_is_explicit = False
            self.units = None

        self._autoscale_on = True

    @property
    def isDefault_majloc(self):
        return self.major._locator_is_default

    @isDefault_majloc.setter
    def isDefault_majloc(self, value):
        self.major._locator_is_default = value

    @property
    def isDefault_majfmt(self):
        return self.major._formatter_is_default

    @isDefault_majfmt.setter
    def isDefault_majfmt(self, value):
        self.major._formatter_is_default = value

    @property
    def isDefault_minloc(self):
        return self.minor._locator_is_default

    @isDefault_minloc.setter
    def isDefault_minloc(self, value):
        self.minor._locator_is_default = value

    @property
    def isDefault_minfmt(self):
        return self.minor._formatter_is_default

    @isDefault_minfmt.setter
    def isDefault_minfmt(self, value):
        self.minor._formatter_is_default = value

    def _get_shared_axes(self):
        return self.axes._shared_axes[
            self._get_axis_name()].get_siblings(self.axes)

    def _get_shared_axis(self):
        name = self._get_axis_name()
        return [ax._axis_map[name] for ax in self._get_shared_axes()]

    def _get_axis_name(self):
        return next(name for name, axis in self.axes._axis_map.items()
                    if axis is self)

    # During initialization, Axis objects often create ticks that are later
    # unused; this turns out to be a very slow step.  Instead, use a custom
    # descriptor to make the tick lists lazy and instantiate them as needed.
    majorTicks = _LazyTickList(major=True)

    minorTicks = _LazyTickList(major=False)

    def get_remove_overlapping_locs(self):
        return self._remove_overlapping_locs

    def set_remove_overlapping_locs(self, val):
        self._remove_overlapping_locs = bool(val)

    remove_overlapping_locs = property(
        get_remove_overlapping_locs, set_remove_overlapping_locs,
        doc=('If minor ticker locations that overlap with major '
             'ticker locations should be trimmed.'))

    def set_label_coords(self, x, y, transform=None):
        self._autolabelpos = False
        if transform is None:
            transform = self.axes.transAxes

        self.label.set_transform(transform)
        self.label.set_position((x, y))
        self.stale = True

    def get_transform(self):
        return self._scale.get_transform()

    def get_scale(self):
        return self._scale.name

    def _set_scale(self, value, **kwargs):
        if not isinstance(value, mscale.ScaleBase):
            self._scale = mscale.scale_factory(value, self, **kwargs)
        else:
            self._scale = value
        self._scale.set_default_locators_and_formatters(self)

        self.isDefault_majloc = True
        self.isDefault_minloc = True
        self.isDefault_majfmt = True
        self.isDefault_minfmt = True

    # This method is directly wrapped by Axes.set_{x,y}scale.
    def _set_axes_scale(self, value, **kwargs):
        name = self._get_axis_name()
        old_default_lims = (self.get_major_locator()
                            .nonsingular(-np.inf, np.inf))
        for ax in self._get_shared_axes():
            ax._axis_map[name]._set_scale(value, **kwargs)
            ax._update_transScale()
            ax.stale = True
        new_default_lims = (self.get_major_locator()
                            .nonsingular(-np.inf, np.inf))
        if old_default_lims != new_default_lims:
            # Force autoscaling now, to take advantage of the scale locator's
            # nonsingular() before it possibly gets swapped out by the user.
            self.axes.autoscale_view(
                **{f"scale{k}": k == name for k in self.axes._axis_names})

    def limit_range_for_scale(self, vmin, vmax):
        return self._scale.limit_range_for_scale(vmin, vmax, self.get_minpos())

    def _nan_out_of_scale_range(self, data):
        data = np.asanyarray(data, dtype=float)
        valid = self._scale.val_in_range(data)
        return data if np.all(valid) else np.where(valid, data, np.nan)

    def _get_autoscale_on(self):
        return self._autoscale_on

    def _set_autoscale_on(self, b):
        if b is not None:
            self._autoscale_on = b

    def get_children(self):
        return [self.label, self.offsetText,
                *self.get_major_ticks(), *self.get_minor_ticks()]

    def _reset_major_tick_kw(self):
        self._major_tick_kw.clear()
        self._major_tick_kw['gridOn'] = (
                mpl.rcParams['axes.grid'] and
                mpl.rcParams['axes.grid.which'] in ('both', 'major'))

    def _reset_minor_tick_kw(self):
        self._minor_tick_kw.clear()
        self._minor_tick_kw['gridOn'] = (
                mpl.rcParams['axes.grid'] and
                mpl.rcParams['axes.grid.which'] in ('both', 'minor'))

    def clear(self):
        self.label._reset_visual_defaults()
        # The above resets the label formatting using text rcParams,
        # so we then update the formatting using axes rcParams
        self.label.set_color(mpl.rcParams['axes.labelcolor'])
        self.label.set_fontsize(mpl.rcParams['axes.labelsize'])
        self.label.set_fontweight(mpl.rcParams['axes.labelweight'])
        self.offsetText._reset_visual_defaults()
        self.labelpad = mpl.rcParams['axes.labelpad']

        self._init()

        self._set_scale('linear')

        # Clear the callback registry for this axis, or it may "leak"
        self.callbacks = cbook.CallbackRegistry(signals=["units"])

        # whether the grids are on
        self._major_tick_kw['gridOn'] = (
                mpl.rcParams['axes.grid'] and
                mpl.rcParams['axes.grid.which'] in ('both', 'major'))
        self._minor_tick_kw['gridOn'] = (
                mpl.rcParams['axes.grid'] and
                mpl.rcParams['axes.grid.which'] in ('both', 'minor'))
        self.reset_ticks()

        self._converter = None
        self._converter_is_explicit = False
        self.units = None
        self.stale = True

    def reset_ticks(self):
        # Restore the lazy tick lists.
        try:
            del self.majorTicks
        except AttributeError:
            pass
        try:
            del self.minorTicks
        except AttributeError:
            pass
        # (matplotlib's Axes has no patch until its first clear made one,
        # after it cleared its axes: an axis made with its Axes is clipped
        # to the patch only by a later clear.)
        if self.axes.__dict__.get("_cleared_once"):
            self.set_clip_path(self.axes.patch)

    def minorticks_on(self):
        scale = self.get_scale()
        if scale == 'log':
            s = self._scale
            self.set_minor_locator(mticker.LogLocator(s.base, s.subs))
        elif scale == 'symlog':
            s = self._scale
            self.set_minor_locator(
                mticker.SymmetricalLogLocator(s._transform, s.subs))
        elif scale == 'asinh':
            s = self._scale
            self.set_minor_locator(
                    mticker.AsinhLocator(s.linear_width, base=s._base,
                                         subs=s._subs))
        elif scale == 'logit':
            self.set_minor_locator(mticker.LogitLocator(minor=True))
        else:
            self.set_minor_locator(mticker.AutoMinorLocator())

    def minorticks_off(self):
        self.set_minor_locator(mticker.NullLocator())

    def set_tick_params(self, which='major', reset=False, **kwargs):
        _api.check_in_list(['major', 'minor', 'both'], which=which)
        kwtrans = self._translate_tick_params(kwargs)

        # the kwargs are stored in self._major/minor_tick_kw so that any
        # future new ticks will automatically get them
        if reset:
            if which in ['major', 'both']:
                self._reset_major_tick_kw()
                self._major_tick_kw.update(kwtrans)
            if which in ['minor', 'both']:
                self._reset_minor_tick_kw()
                self._minor_tick_kw.update(kwtrans)
            self.reset_ticks()
        else:
            if which in ['major', 'both']:
                self._major_tick_kw.update(kwtrans)
                for tick in self.majorTicks:
                    tick._apply_params(**kwtrans)
            if which in ['minor', 'both']:
                self._minor_tick_kw.update(kwtrans)
                for tick in self.minorTicks:
                    tick._apply_params(**kwtrans)
            # labelOn and labelcolor also apply to the offset text.
            if 'label1On' in kwtrans or 'label2On' in kwtrans:
                self.offsetText.set_visible(
                    self._major_tick_kw.get('label1On', False)
                    or self._major_tick_kw.get('label2On', False))
            if 'labelcolor' in kwtrans:
                self.offsetText.set_color(kwtrans['labelcolor'])

        self.stale = True

    def get_tick_params(self, which='major'):
        _api.check_in_list(['major', 'minor'], which=which)
        if which == 'major':
            return self._translate_tick_params(
                self._major_tick_kw, reverse=True
            )
        return self._translate_tick_params(self._minor_tick_kw, reverse=True)

    def set_clip_path(self, path, transform=None):
        Artist.set_clip_path(self, path, transform)
        for child in self.majorTicks + self.minorTicks:
            child.set_clip_path(path, transform)
        self.stale = True

    def get_view_interval(self):
        raise NotImplementedError('Derived must override')

    def set_view_interval(self, vmin, vmax, ignore=False):
        raise NotImplementedError('Derived must override')

    def get_data_interval(self):
        raise NotImplementedError('Derived must override')

    def set_data_interval(self, vmin, vmax, ignore=False):
        raise NotImplementedError('Derived must override')

    def get_inverted(self):
        low, high = self.get_view_interval()
        return high < low

    def set_inverted(self, inverted):
        a, b = self.get_view_interval()
        # cast to bool to avoid bad interaction between python 3.8 and np.bool_
        self._set_lim(*sorted((a, b), reverse=bool(inverted)), auto=None)

    def set_default_intervals(self):
        pass

    def _set_lim(self, v0, v1, *, emit=True, auto):
        name = self._get_axis_name()

        self.axes._process_unit_info([(name, (v0, v1))], convert=False)
        v0 = _validate_converted_limits(v0, self.convert_units)
        v1 = _validate_converted_limits(v1, self.convert_units)

        if v0 is None or v1 is None:
            # Axes init calls set_xlim(0, 1) before get_xlim() can be called,
            # so only grab the limits if we really need them.
            old0, old1 = self.get_view_interval()
            if v0 is None:
                v0 = old0
            if v1 is None:
                v1 = old1

        if self.get_scale() == 'log' and (v0 <= 0 or v1 <= 0):
            # Axes init calls set_xlim(0, 1) before get_xlim() can be called,
            # so only grab the limits if we really need them.
            old0, old1 = self.get_view_interval()
            if v0 <= 0:
                _api.warn_external(f"Attempt to set non-positive {name}lim on "
                                   f"a log-scaled axis will be ignored.")
                v0 = old0
            if v1 <= 0:
                _api.warn_external(f"Attempt to set non-positive {name}lim on "
                                   f"a log-scaled axis will be ignored.")
                v1 = old1
        if v0 == v1:
            _api.warn_external(
                f"Attempting to set identical low and high {name}lims "
                f"makes transformation singular; automatically expanding.")
        reverse = bool(v0 > v1)  # explicit cast needed for python3.8+np.bool_.
        v0, v1 = self.get_major_locator().nonsingular(v0, v1)
        v0, v1 = self.limit_range_for_scale(v0, v1)
        v0, v1 = sorted([v0, v1], reverse=bool(reverse))

        self.set_view_interval(v0, v1, ignore=True)
        # (matplotlib marks the view limits of the Axes sharing the axis as
        # no longer stale here; pltrs (Rust) autoscales the x and y views
        # only, which a held axis is none of.)
        self._set_autoscale_on(auto)

        if emit:
            self.axes.callbacks.process(f"{name}lim_changed", self.axes)
            # Call all of the other Axes that are shared with this one
            for other in self._get_shared_axes():
                if other is self.axes:
                    continue
                other._axis_map[name]._set_lim(v0, v1, emit=False, auto=auto)
                if emit:
                    other.callbacks.process(f"{name}lim_changed", other)
                if ((other_fig := other.get_figure(root=False)) !=
                        self.get_figure(root=False)):
                    other_fig.canvas.draw_idle()

        self.stale = True
        return v0, v1

    def _set_artist_props(self, a):
        if a is None:
            return
        a.set_figure(self.get_figure(root=False))

    def _update_ticks(self):
        # Check if major ticks should be computed.
        # Skip if using NullLocator or if all visible components are off.
        if (_tick_group_visible(self._major_tick_kw)
                and not isinstance(self.get_major_locator(), NullLocator)):
            major_locs = self.get_majorticklocs()
            major_labels = self.major.formatter.format_ticks(major_locs)
            major_ticks = self.get_major_ticks(len(major_locs))
            for tick, loc, label in zip(major_ticks, major_locs, major_labels):
                tick.update_position(loc)
                tick.label1.set_text(label)
                tick.label2.set_text(label)
        else:
            major_ticks = []

        # Check if minor ticks should be computed.
        if (_tick_group_visible(self._minor_tick_kw)
                and not isinstance(self.get_minor_locator(), NullLocator)):
            minor_locs = self.get_minorticklocs()
            minor_labels = self.minor.formatter.format_ticks(minor_locs)
            minor_ticks = self.get_minor_ticks(len(minor_locs))
            for tick, loc, label in zip(minor_ticks, minor_locs, minor_labels):
                tick.update_position(loc)
                tick.label1.set_text(label)
                tick.label2.set_text(label)
        else:
            minor_ticks = []

        ticks = [*major_ticks, *minor_ticks]
        if not ticks:
            return []

        view_low, view_high = self.get_view_interval()
        if view_low > view_high:
            view_low, view_high = view_high, view_low

        if (hasattr(self, "axes") and self.axes.name == '3d'
                and mpl.rcParams['axes3d.automargin']):
            # In mpl3.8, the margin was 1/48. Due to the change in automargin
            # behavior in mpl3.9, we need to adjust this to compensate for a
            # zoom factor of 2/48, giving us a 23/24 modifier. So the new
            # margin is 0.019965277777777776 = 1/48*23/24.
            margin = 0.019965277777777776
            delta = view_high - view_low
            view_high = view_high - delta * margin
            view_low = view_low + delta * margin

        interval_t = self.get_transform().transform([view_low, view_high])

        ticks_to_draw = []
        for tick in ticks:
            try:
                loc_t = self.get_transform().transform(tick.get_loc())
            except AssertionError:
                # transforms.transform doesn't allow masked values but
                # some scales might make them, so we need this try/except.
                pass
            else:
                if mtransforms._interval_contains_close(interval_t, loc_t):
                    ticks_to_draw.append(tick)

        return ticks_to_draw

    def _get_ticklabel_bboxes(self, ticks, renderer):
        return ([tick.label1.get_window_extent(renderer)
                 for tick in ticks
                 if tick.label1.get_visible() and tick.label1.get_in_layout()],
                [tick.label2.get_window_extent(renderer)
                 for tick in ticks
                 if tick.label2.get_visible() and tick.label2.get_in_layout()])

    def get_tightbbox(self, renderer=None, *, for_layout_only=False):
        if not self.get_visible() or for_layout_only and not self.get_in_layout():
            return
        if renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()
        ticks_to_draw = self._update_ticks()

        self._update_label_position(renderer)

        # go back to just this axis's tick labels
        tlb1, tlb2 = self._get_ticklabel_bboxes(ticks_to_draw, renderer)

        self._update_offset_text_position(tlb1, tlb2)
        self.offsetText.set_text(self.major.formatter.get_offset())

        bboxes = [
            *(a.get_window_extent(renderer)
              for a in [self.offsetText]
              if a.get_visible()),
            *tlb1, *tlb2,
        ]
        # take care of label
        if self.label.get_visible():
            bb = self.label.get_window_extent(renderer)
            # for constrained/tight_layout, we want to ignore the label's
            # width/height because the adjustments they make can't be improved.
            # this code collapses the relevant direction
            if for_layout_only:
                if self.axis_name == "x" and bb.width > 0:
                    bb.x0 = (bb.x0 + bb.x1) / 2 - 0.5
                    bb.x1 = bb.x0 + 1.0
                if self.axis_name == "y" and bb.height > 0:
                    bb.y0 = (bb.y0 + bb.y1) / 2 - 0.5
                    bb.y1 = bb.y0 + 1.0
            bboxes.append(bb)
        bboxes = [b for b in bboxes if b._is_finite()]
        if bboxes:
            return mtransforms.Bbox.union(bboxes)
        else:
            return None

    def get_tick_padding(self):
        values = []
        if len(self.majorTicks):
            values.append(self.majorTicks[0].get_tick_padding())
        if len(self.minorTicks):
            values.append(self.minorTicks[0].get_tick_padding())
        return max(values, default=0)

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited

        if not self.get_visible():
            return
        renderer.open_group(_GROUP, gid=self.get_gid())

        ticks_to_draw = self._update_ticks()
        tlb1, tlb2 = self._get_ticklabel_bboxes(ticks_to_draw, renderer)

        for tick in ticks_to_draw:
            tick.draw(renderer)

        # Shift label away from axes to avoid overlapping ticklabels.
        self._update_label_position(renderer)
        self.label.draw(renderer)

        self._update_offset_text_position(tlb1, tlb2)
        self.offsetText.set_text(self.major.formatter.get_offset())
        self.offsetText.draw(renderer)

        renderer.close_group(_GROUP)
        self.stale = False

    def get_gridlines(self):
        ticks = self.get_major_ticks()
        return cbook.silent_list('Line2D gridline',
                                 [tick.gridline for tick in ticks])

    def set_label(self, s):
        raise RuntimeError(
            "A legend label cannot be assigned to an Axis. Did you mean to "
            "set the axis label via set_label_text()?")

    def get_label(self):
        return self.label

    def get_offset_text(self):
        return self.offsetText

    def get_pickradius(self):
        return self._pickradius

    def get_majorticklabels(self):
        self._update_ticks()
        ticks = self.get_major_ticks()
        labels1 = [tick.label1 for tick in ticks if tick.label1.get_visible()]
        labels2 = [tick.label2 for tick in ticks if tick.label2.get_visible()]
        return labels1 + labels2

    def get_minorticklabels(self):
        self._update_ticks()
        ticks = self.get_minor_ticks()
        labels1 = [tick.label1 for tick in ticks if tick.label1.get_visible()]
        labels2 = [tick.label2 for tick in ticks if tick.label2.get_visible()]
        return labels1 + labels2

    def get_ticklabels(self, minor=False, which=None):
        if which is not None:
            if which == 'minor':
                return self.get_minorticklabels()
            elif which == 'major':
                return self.get_majorticklabels()
            elif which == 'both':
                return self.get_majorticklabels() + self.get_minorticklabels()
            else:
                _api.check_in_list(['major', 'minor', 'both'], which=which)
        if minor:
            return self.get_minorticklabels()
        return self.get_majorticklabels()

    def get_majorticklines(self):
        lines = []
        ticks = self.get_major_ticks()
        for tick in ticks:
            lines.append(tick.tick1line)
            lines.append(tick.tick2line)
        return cbook.silent_list('Line2D ticklines', lines)

    def get_minorticklines(self):
        lines = []
        ticks = self.get_minor_ticks()
        for tick in ticks:
            lines.append(tick.tick1line)
            lines.append(tick.tick2line)
        return cbook.silent_list('Line2D ticklines', lines)

    def get_ticklines(self, minor=False):
        if minor:
            return self.get_minorticklines()
        return self.get_majorticklines()

    def get_majorticklocs(self):
        return self.major.locator()

    def get_minorticklocs(self):
        # Remove minor ticks duplicating major ticks.
        minor_locs = np.asarray(self.minor.locator())
        if self.remove_overlapping_locs:
            major_locs = self.major.locator()
            transform = self._scale.get_transform()
            tr_minor_locs = transform.transform(minor_locs)
            tr_major_locs = transform.transform(major_locs)
            lo, hi = sorted(transform.transform(self.get_view_interval()))
            # Use the transformed view limits as scale.  1e-5 is the default
            # rtol for np.isclose.
            tol = (hi - lo) * 1e-5
            mask = np.isclose(tr_minor_locs[:, None], tr_major_locs[None, :],
                              atol=tol, rtol=0).any(axis=1)
            minor_locs = minor_locs[~mask]
        return minor_locs

    def get_ticklocs(self, *, minor=False):
        return self.get_minorticklocs() if minor else self.get_majorticklocs()

    def get_ticks_direction(self, minor=False):
        if minor:
            return np.array(
                [tick._tickdir for tick in self.get_minor_ticks()])
        else:
            return np.array(
                [tick._tickdir for tick in self.get_major_ticks()])

    def _get_tick(self, major):
        if self._tick_class is None:
            raise NotImplementedError(
                f"The Axis subclass {self.__class__.__name__} must define "
                "_tick_class or reimplement _get_tick()")
        tick_kw = self._major_tick_kw if major else self._minor_tick_kw
        return self._tick_class(self.axes, 0, major=major, **tick_kw)

    def _get_tick_label_size(self, axis_name):
        tick_kw = self._major_tick_kw
        size = tick_kw.get('labelsize',
                           mpl.rcParams[f'{axis_name}tick.labelsize'])
        return mtext.FontProperties(size=size).get_size_in_points()

    def _copy_tick_props(self, src, dest):
        if src is None or dest is None:
            return
        dest.label1.update_from(src.label1)
        dest.label2.update_from(src.label2)
        dest.tick1line.update_from(src.tick1line)
        dest.tick2line.update_from(src.tick2line)
        dest.gridline.update_from(src.gridline)
        dest.update_from(src)
        dest._loc = src._loc
        dest._size = src._size
        dest._width = src._width
        dest._base_pad = src._base_pad
        dest._labelrotation = src._labelrotation
        dest._zorder = src._zorder
        dest._tickdir = src._tickdir

    def get_label_text(self):
        return self.label.get_text()

    def get_major_locator(self):
        return self.major.locator

    def get_minor_locator(self):
        return self.minor.locator

    def get_major_formatter(self):
        return self.major.formatter

    def get_minor_formatter(self):
        return self.minor.formatter

    def get_major_ticks(self, numticks=None):
        if numticks is None:
            numticks = len(self.get_majorticklocs())

        while len(self.majorTicks) < numticks:
            # Update the new tick label properties from the old.
            tick = self._get_tick(major=True)
            self.majorTicks.append(tick)
            self._copy_tick_props(self.majorTicks[0], tick)

        return self.majorTicks[:numticks]

    def get_minor_ticks(self, numticks=None):
        if numticks is None:
            numticks = len(self.get_minorticklocs())

        while len(self.minorTicks) < numticks:
            # Update the new tick label properties from the old.
            tick = self._get_tick(major=False)
            self.minorTicks.append(tick)
            self._copy_tick_props(self.minorTicks[0], tick)

        return self.minorTicks[:numticks]

    def grid(self, visible=None, which='major', **kwargs):
        if kwargs:
            if visible is None:
                visible = True
            elif not visible:  # something false-like but not None
                _api.warn_external('First parameter to grid() is false, '
                                   'but line properties are supplied. The '
                                   'grid will be enabled.')
                visible = True
        which = which.lower()
        _api.check_in_list(['major', 'minor', 'both'], which=which)
        gridkw = {f'grid_{name}': value for name, value in kwargs.items()}
        if which in ['minor', 'both']:
            gridkw['gridOn'] = (not self._minor_tick_kw['gridOn']
                                if visible is None else visible)
            self.set_tick_params(which='minor', **gridkw)
        if which in ['major', 'both']:
            gridkw['gridOn'] = (not self._major_tick_kw['gridOn']
                                if visible is None else visible)
            self.set_tick_params(which='major', **gridkw)
        self.stale = True

    def update_units(self, data):
        if not self._converter_is_explicit:
            converter = munits.registry.get_converter(data)
        else:
            converter = self._converter

        if converter is None:
            return False

        neednew = self._converter != converter
        self._set_converter(converter)
        default = self._converter.default_units(data, self)
        if default is not None and self.units is None:
            self.set_units(default)

        elif neednew:
            self._update_axisinfo()
        self.stale = True
        return True

    def _update_axisinfo(self):
        if self._converter is None:
            return

        info = self._converter.axisinfo(self.units, self)

        if info is None:
            return
        if info.majloc is not None and \
           self.major.locator != info.majloc and self.isDefault_majloc:
            self.set_major_locator(info.majloc)
            self.isDefault_majloc = True
        if info.minloc is not None and \
           self.minor.locator != info.minloc and self.isDefault_minloc:
            self.set_minor_locator(info.minloc)
            self.isDefault_minloc = True
        if info.majfmt is not None and \
           self.major.formatter != info.majfmt and self.isDefault_majfmt:
            self.set_major_formatter(info.majfmt)
            self.isDefault_majfmt = True
        if info.minfmt is not None and \
           self.minor.formatter != info.minfmt and self.isDefault_minfmt:
            self.set_minor_formatter(info.minfmt)
            self.isDefault_minfmt = True
        if info.label is not None and self.isDefault_label:
            self.set_label_text(info.label)
            self.isDefault_label = True

        self.set_default_intervals()

    def have_units(self):
        return self._converter is not None or self.units is not None

    def convert_units(self, x):
        # If x is natively supported by Matplotlib, doesn't need converting
        if munits._is_natively_supported(x):
            return x

        if self._converter is None:
            self._set_converter(munits.registry.get_converter(x))

        if self._converter is None:
            return x
        try:
            ret = self._converter.convert(x, self.units, self)
        except Exception as e:
            raise munits.ConversionError('Failed to convert value(s) to axis '
                                         f'units: {x!r}') from e
        return ret

    def get_converter(self):
        return self._converter

    def set_converter(self, converter):
        self._set_converter(converter)
        self._converter_is_explicit = True

    def _set_converter(self, converter):
        if self._converter is converter or self._converter == converter:
            return
        if self._converter_is_explicit:
            raise RuntimeError("Axis already has an explicit converter set")
        elif (
            self._converter is not None and
            not isinstance(converter, type(self._converter)) and
            not isinstance(self._converter, type(converter))
        ):
            _api.warn_external(
                "This axis already has a converter set and "
                "is updating to a potentially incompatible converter"
            )
        self._converter = converter

    def set_units(self, u):
        if u == self.units:
            return
        for axis in self._get_shared_axis():
            axis.units = u
            axis._update_axisinfo()
            axis.callbacks.process('units')
            axis.stale = True

    def get_units(self):
        return self.units

    def set_label_text(self, label, fontdict=None, **kwargs):
        self.isDefault_label = False
        self.label.set_text(label)
        if fontdict is not None:
            self.label.update(fontdict)
        self.label.update(kwargs)
        self.stale = True
        return self.label

    def set_major_formatter(self, formatter):
        self._set_formatter(formatter, self.major)

    def set_minor_formatter(self, formatter):
        self._set_formatter(formatter, self.minor)

    def _set_formatter(self, formatter, level):
        if isinstance(formatter, str):
            formatter = mticker.StrMethodFormatter(formatter)
        # Don't allow any other TickHelper to avoid easy-to-make errors,
        # like using a Locator instead of a Formatter.
        elif (callable(formatter) and
              not isinstance(formatter, mticker.TickHelper)):
            formatter = mticker.FuncFormatter(formatter)
        else:
            _api.check_isinstance(mticker.Formatter, formatter=formatter)

        if (isinstance(formatter, mticker.FixedFormatter)
                and len(formatter.seq) > 0
                and not isinstance(level.locator, mticker.FixedLocator)
                and not (hasattr(level.locator, 'base') and
                         isinstance(level.locator.base, mticker.FixedLocator))):
            _api.warn_external('FixedFormatter should only be used together '
                               'with FixedLocator')

        if level == self.major:
            self.isDefault_majfmt = False
        else:
            self.isDefault_minfmt = False

        level.formatter = formatter
        formatter.set_axis(self)
        self.stale = True

    def set_major_locator(self, locator):
        _api.check_isinstance(mticker.Locator, locator=locator)
        self.isDefault_majloc = False
        self.major.locator = locator
        if self.major.formatter:
            self.major.formatter._set_locator(locator)
        locator.set_axis(self)
        self.stale = True

    def set_minor_locator(self, locator):
        _api.check_isinstance(mticker.Locator, locator=locator)
        self.isDefault_minloc = False
        self.minor.locator = locator
        if self.minor.formatter:
            self.minor.formatter._set_locator(locator)
        locator.set_axis(self)
        self.stale = True

    def set_pickradius(self, pickradius):
        if not isinstance(pickradius, Real) or pickradius < 0:
            raise ValueError("pick radius should be a distance")
        self._pickradius = pickradius

    pickradius = property(
        get_pickradius, set_pickradius, doc="The acceptance radius for "
        "containment tests. See also `.Axis.contains`.")

    def set_ticklabels(self, labels, *, minor=False, fontdict=None, **kwargs):
        try:
            labels = [t.get_text() if hasattr(t, 'get_text') else t
                      for t in labels]
        except TypeError:
            raise TypeError(f"{labels=!r} must be a sequence") from None
        locator = (self.get_minor_locator() if minor
                   else self.get_major_locator())
        if not labels:
            formatter = mticker.NullFormatter()
        elif (isinstance(locator, mticker.FixedLocator) or
              (hasattr(locator, 'base') and
               isinstance(locator.base, mticker.FixedLocator))):
            # Also handles locators that wrap a FixedLocator (e.g. RadialLocator).
            fixed_locator = (locator if isinstance(locator, mticker.FixedLocator)
                             else locator.base)
            # Passing [] as a list of labels is often used as a way to
            # remove all tick labels, so only error for > 0 labels
            if len(fixed_locator.locs) != len(labels) and len(labels) != 0:
                raise ValueError(
                    "The number of FixedLocator locations"
                    f" ({len(fixed_locator.locs)}), usually from a call to"
                    " set_ticks, does not match"
                    f" the number of labels ({len(labels)}).")
            tickd = {loc: lab for loc, lab in zip(fixed_locator.locs, labels)}
            func = functools.partial(self._format_with_dict, tickd)
            formatter = mticker.FuncFormatter(func)
        else:
            _api.warn_external(
                 "set_ticklabels() should only be used with a fixed number of "
                 "ticks, i.e. after set_ticks() or using a FixedLocator. "
                 "Otherwise, ticks may be mislabeled.")
            formatter = mticker.FixedFormatter(labels)

        with warnings.catch_warnings():
            warnings.filterwarnings(
                "ignore",
                message="FixedFormatter should only be used together with FixedLocator")
            if minor:
                self.set_minor_formatter(formatter)
                locs = self.get_minorticklocs()
                ticks = self.get_minor_ticks(len(locs))
            else:
                self.set_major_formatter(formatter)
                locs = self.get_majorticklocs()
                ticks = self.get_major_ticks(len(locs))

        ret = []
        if fontdict is not None:
            kwargs.update(fontdict)
        for pos, (loc, tick) in enumerate(zip(locs, ticks)):
            tick.update_position(loc)
            tick_label = formatter(loc, pos)
            # deal with label1
            tick.label1.set_text(tick_label)
            tick.label1._internal_update(kwargs)
            # deal with label2
            tick.label2.set_text(tick_label)
            tick.label2._internal_update(kwargs)
            # only return visible tick labels
            if tick.label1.get_visible():
                ret.append(tick.label1)
            if tick.label2.get_visible():
                ret.append(tick.label2)

        self.stale = True
        return ret

    def _set_tick_locations(self, ticks, *, minor=False):
        # see docstring of set_ticks

        # XXX if the user changes units, the information will be lost here
        ticks = self.convert_units(ticks)
        locator = mticker.FixedLocator(ticks)  # validate ticks early.
        if len(ticks):
            old_vmin, old_vmax = self.get_view_interval()
            for axis in self._get_shared_axis():
                # set_view_interval maintains any preexisting inversion.
                axis.set_view_interval(min(ticks), max(ticks))
            new_vmin, new_vmax = self.get_view_interval()
            if old_vmin != new_vmin or old_vmax != new_vmax:
                self.axes.callbacks.process(
                    f"{self._get_axis_name()}lim_changed",
                    self.axes,
                )
        self.axes.stale = True
        if minor:
            self.set_minor_locator(locator)
            return self.get_minor_ticks(len(ticks))
        else:
            self.set_major_locator(locator)
            return self.get_major_ticks(len(ticks))

    def set_ticks(self, ticks, labels=None, *, minor=False, **kwargs):
        if labels is None and kwargs:
            first_key = next(iter(kwargs))
            raise ValueError(
                f"Incorrect use of keyword argument {first_key!r}. Keyword arguments "
                "other than 'minor' modify the text labels and can only be used if "
                "'labels' are passed as well.")
        result = self._set_tick_locations(ticks, minor=minor)
        if labels is not None:
            self.set_ticklabels(labels, minor=minor, **kwargs)
        return result

    def _get_tick_boxes_siblings(self, renderer):
        # Get the Grouper keeping track of x or y label groups for this figure.
        name = self._get_axis_name()
        # (pltrs (Rust) aligns the labels of the x and y axes, and keeps
        # their groups: a figure has none for the names of held axes.)
        groups = getattr(self.get_figure(root=False), "_align_label_groups", {})
        if name not in groups:
            return [], []
        grouper = groups[name]
        bboxes = []
        bboxes2 = []
        # If we want to align labels from other Axes:
        for ax in grouper.get_siblings(self.axes):
            axis = ax._axis_map[name]
            ticks_to_draw = axis._update_ticks()
            tlb, tlb2 = axis._get_ticklabel_bboxes(ticks_to_draw, renderer)
            bboxes.extend(tlb)
            bboxes2.extend(tlb2)
        return bboxes, bboxes2

    def _update_label_position(self, renderer):
        raise NotImplementedError('Derived must override')

    def _update_offset_text_position(self, bboxes, bboxes2):
        raise NotImplementedError('Derived must override')

    def axis_date(self, tz=None):
        # By providing a sample datetime instance with the desired timezone,
        # the registered converter can be selected, and the "units" attribute,
        # which is the timezone, can be set.
        if isinstance(tz, str):
            import dateutil.tz
            tz = dateutil.tz.gettz(tz)
        self.update_units(datetime.datetime(2009, 1, 1, 0, 0, 0, 0, tz))

    def get_tick_space(self):
        # Must be overridden in the subclass
        raise NotImplementedError()

    def _get_ticks_position(self):
        representative_ticks = []
        if not isinstance(self.get_major_locator(), NullLocator):
            representative_ticks.append(self.majorTicks[0])
        if not isinstance(self.get_minor_locator(), NullLocator):
            representative_ticks.append(self.minorTicks[0])

        if all(tick.tick1line.get_visible()
               and not tick.tick2line.get_visible()
               and tick.label1.get_visible()
               and not tick.label2.get_visible()
               for tick in representative_ticks):
            return 1
        elif all(tick.tick2line.get_visible()
                 and not tick.tick1line.get_visible()
                 and tick.label2.get_visible()
                 and not tick.label1.get_visible()
                 for tick in representative_ticks):
            return 2
        elif all(tick.tick1line.get_visible()
                 and tick.tick2line.get_visible()
                 and tick.label1.get_visible()
                 and not tick.label2.get_visible()
                 for tick in representative_ticks):
            return "default"
        else:
            return "unknown"

    def get_label_position(self):
        return self.label_position

    def set_label_position(self, position):
        raise NotImplementedError()

    def get_minpos(self):
        raise NotImplementedError()


def _make_getset_interval(method_name, lim_name, attr_name):
    """
    Helper to generate ``get_{data,view}_interval`` and
    ``set_{data,view}_interval`` implementations.
    """

    def getter(self):
        # docstring inherited.
        return getattr(getattr(self.axes, lim_name), attr_name)

    def setter(self, vmin, vmax, ignore=False):
        # docstring inherited.
        if ignore:
            setattr(getattr(self.axes, lim_name), attr_name, (vmin, vmax))
        else:
            oldmin, oldmax = getter(self)
            if oldmin < oldmax:
                setter(self, min(vmin, vmax, oldmin), max(vmin, vmax, oldmax),
                       ignore=True)
            else:
                setter(self, max(vmin, vmax, oldmin), min(vmin, vmax, oldmax),
                       ignore=True)
        self.stale = True

    getter.__name__ = f"get_{method_name}_interval"
    setter.__name__ = f"set_{method_name}_interval"

    return getter, setter


class XAxis(Axis):
    def __init__(self, *args, **kwargs):
        Axis.__init__(self, *args, **kwargs)
        self._init()

    def _init(self):
        # x in axes coords, y in display coords (to be updated at draw time by
        # _update_label_positions and _update_offset_text_position).
        self.label.set(
            x=0.5, y=0,
            verticalalignment='top', horizontalalignment='center',
            transform=mtransforms.blended_transform_factory(
                self.axes.transAxes, mtransforms.IdentityTransform()),
        )
        self.label_position = 'bottom'

        if mpl.rcParams['xtick.labelcolor'] == 'inherit':
            tick_color = mpl.rcParams['xtick.color']
        else:
            tick_color = mpl.rcParams['xtick.labelcolor']

        self.offsetText.set(
            x=1, y=0,
            verticalalignment='top', horizontalalignment='right',
            transform=mtransforms.blended_transform_factory(
                self.axes.transAxes, mtransforms.IdentityTransform()),
            fontsize=mpl.rcParams['xtick.labelsize'],
            color=tick_color
        )
        self.offset_text_position = 'bottom'

    def contains(self, mouseevent):
        if self._different_canvas(mouseevent):
            return False, {}
        x, y = mouseevent.x, mouseevent.y
        try:
            trans = self.axes.transAxes.inverted()
            xaxes, yaxes = trans.transform((x, y))
        except ValueError:
            return False, {}
        (l, b), (r, t) = self.axes.transAxes.transform([(0, 0), (1, 1)])
        inaxis = 0 <= xaxes <= 1 and (
            b - self._pickradius < y < b or
            t < y < t + self._pickradius)
        return inaxis, {}

    def set_label_position(self, position):
        self.label.set_verticalalignment(_api.getitem_checked({
            'top': 'baseline', 'bottom': 'top',
        }, position=position))
        self.label_position = position
        self.stale = True

    def _update_label_position(self, renderer):
        if not self._autolabelpos:
            return

        # get bounding boxes for this axis and any siblings
        # that have been set by `fig.align_xlabels()`
        bboxes, bboxes2 = self._get_tick_boxes_siblings(renderer=renderer)
        x, y = self.label.get_position()

        if self.label_position == 'bottom':
            # Union with extents of the bottom spine if present, of the axes otherwise.
            bbox = mtransforms.Bbox.union([
                *bboxes, self.axes.spines.get("bottom", self.axes).get_window_extent()])
            self.label.set_position(
                (x, bbox.y0 - self.labelpad * self.get_figure(root=True).dpi / 72))
        else:
            # Union with extents of the top spine if present, of the axes otherwise.
            bbox = mtransforms.Bbox.union([
                *bboxes2, self.axes.spines.get("top", self.axes).get_window_extent()])
            self.label.set_position(
                (x, bbox.y1 + self.labelpad * self.get_figure(root=True).dpi / 72))

    def _update_offset_text_position(self, bboxes, bboxes2):
        x, y = self.offsetText.get_position()
        if not hasattr(self, '_tick_position'):
            self._tick_position = 'bottom'
        if self._tick_position == 'bottom':
            if not len(bboxes):
                bottom = self.axes.bbox.ymin
            else:
                bbox = mtransforms.Bbox.union(bboxes)
                bottom = bbox.y0
            y = bottom - self.OFFSETTEXTPAD * self.get_figure(root=True).dpi / 72
        else:
            if not len(bboxes2):
                top = self.axes.bbox.ymax
            else:
                bbox = mtransforms.Bbox.union(bboxes2)
                top = bbox.y1
            y = top + self.OFFSETTEXTPAD * self.get_figure(root=True).dpi / 72
        self.offsetText.set_position((x, y))

    def set_ticks_position(self, position):
        if position == 'top':
            self.set_tick_params(which='both', top=True, labeltop=True,
                                 bottom=False, labelbottom=False)
            self._tick_position = 'top'
            self.offsetText.set_verticalalignment('bottom')
        elif position == 'bottom':
            self.set_tick_params(which='both', top=False, labeltop=False,
                                 bottom=True, labelbottom=True)
            self._tick_position = 'bottom'
            self.offsetText.set_verticalalignment('top')
        elif position == 'both':
            self.set_tick_params(which='both', top=True,
                                 bottom=True)
        elif position == 'none':
            self.set_tick_params(which='both', top=False,
                                 bottom=False)
        elif position == 'default':
            self.set_tick_params(which='both', top=True, labeltop=False,
                                 bottom=True, labelbottom=True)
            self._tick_position = 'bottom'
            self.offsetText.set_verticalalignment('top')
        else:
            _api.check_in_list(['top', 'bottom', 'both', 'default', 'none'],
                               position=position)
        self.stale = True

    def tick_top(self):
        label = True
        if 'label1On' in self._major_tick_kw:
            label = (self._major_tick_kw['label1On']
                     or self._major_tick_kw['label2On'])
        self.set_ticks_position('top')
        # If labels were turned off before this was called, leave them off.
        self.set_tick_params(which='both', labeltop=label)

    def tick_bottom(self):
        label = True
        if 'label1On' in self._major_tick_kw:
            label = (self._major_tick_kw['label1On']
                     or self._major_tick_kw['label2On'])
        self.set_ticks_position('bottom')
        # If labels were turned off before this was called, leave them off.
        self.set_tick_params(which='both', labelbottom=label)

    def get_ticks_position(self):
        return {1: "bottom", 2: "top",
                "default": "default", "unknown": "unknown"}[
                    self._get_ticks_position()]

    get_view_interval, set_view_interval = _make_getset_interval(
        "view", "viewLim", "intervalx")

    get_data_interval, set_data_interval = _make_getset_interval(
        "data", "dataLim", "intervalx")

    def get_minpos(self):
        return self.axes.dataLim.minposx

    def set_default_intervals(self):
        # docstring inherited
        # only change view if dataLim has not changed and user has
        # not changed the view:
        if (not self.axes.dataLim.mutatedx() and
                not self.axes.viewLim.mutatedx()):
            if self._converter is not None:
                info = self._converter.axisinfo(self.units, self)
                if info.default_limits is not None:
                    xmin, xmax = self.convert_units(info.default_limits)
                    self.axes.viewLim.intervalx = xmin, xmax
        self.stale = True

    def get_tick_space(self):
        ends = mtransforms.Bbox.unit().transformed(
            self.axes.transAxes - self.get_figure(root=False).dpi_scale_trans)
        length = ends.width * 72
        # There is a heuristic here that the aspect ratio of tick text
        # is no more than 3:1
        size = self._get_tick_label_size('x') * 3
        if size > 0:
            return int(np.floor(length / size))
        else:
            return 2**31 - 1


class YAxis(Axis):
    def __init__(self, *args, **kwargs):
        Axis.__init__(self, *args, **kwargs)
        self._init()

    def _init(self):
        # x in display coords, y in axes coords (to be updated at draw time by
        # _update_label_positions and _update_offset_text_position).
        self.label.set(
            x=0, y=0.5,
            verticalalignment='bottom', horizontalalignment='center',
            rotation='vertical', rotation_mode='anchor',
            transform=mtransforms.blended_transform_factory(
                mtransforms.IdentityTransform(), self.axes.transAxes),
        )
        self.label_position = 'left'

        if mpl.rcParams['ytick.labelcolor'] == 'inherit':
            tick_color = mpl.rcParams['ytick.color']
        else:
            tick_color = mpl.rcParams['ytick.labelcolor']

        # x in axes coords, y in display coords(!).
        self.offsetText.set(
            x=0, y=0.5,
            verticalalignment='baseline', horizontalalignment='left',
            transform=mtransforms.blended_transform_factory(
                self.axes.transAxes, mtransforms.IdentityTransform()),
            fontsize=mpl.rcParams['ytick.labelsize'],
            color=tick_color
        )
        self.offset_text_position = 'left'

    def contains(self, mouseevent):
        # docstring inherited
        if self._different_canvas(mouseevent):
            return False, {}
        x, y = mouseevent.x, mouseevent.y
        try:
            trans = self.axes.transAxes.inverted()
            xaxes, yaxes = trans.transform((x, y))
        except ValueError:
            return False, {}
        (l, b), (r, t) = self.axes.transAxes.transform([(0, 0), (1, 1)])
        inaxis = 0 <= yaxes <= 1 and (
            l - self._pickradius < x < l or
            r < x < r + self._pickradius)
        return inaxis, {}

    def set_label_position(self, position):
        self.label.set_rotation_mode('anchor')
        self.label.set_verticalalignment(_api.getitem_checked({
            'left': 'bottom', 'right': 'top',
        }, position=position))
        self.label_position = position
        self.stale = True

    def _update_label_position(self, renderer):
        if not self._autolabelpos:
            return

        # get bounding boxes for this axis and any siblings
        # that have been set by `fig.align_ylabels()`
        bboxes, bboxes2 = self._get_tick_boxes_siblings(renderer=renderer)
        x, y = self.label.get_position()

        if self.label_position == 'left':
            # Union with extents of the left spine if present, of the axes otherwise.
            bbox = mtransforms.Bbox.union([
                *bboxes, self.axes.spines.get("left", self.axes).get_window_extent()])
            self.label.set_position(
                (bbox.x0 - self.labelpad * self.get_figure(root=True).dpi / 72, y))
        else:
            # Union with extents of the right spine if present, of the axes otherwise.
            bbox = mtransforms.Bbox.union([
                *bboxes2, self.axes.spines.get("right", self.axes).get_window_extent()])
            self.label.set_position(
                (bbox.x1 + self.labelpad * self.get_figure(root=True).dpi / 72, y))

    def _update_offset_text_position(self, bboxes, bboxes2):
        x, _ = self.offsetText.get_position()
        if 'outline' in self.axes.spines:
            # Special case for colorbars:
            bbox = self.axes.spines['outline'].get_window_extent()
        else:
            bbox = self.axes.bbox
        top = bbox.ymax
        self.offsetText.set_position(
            (x, top + self.OFFSETTEXTPAD * self.get_figure(root=True).dpi / 72)
        )

    def set_offset_position(self, position):
        x, y = self.offsetText.get_position()
        x = _api.getitem_checked({'left': 0, 'right': 1}, position=position)

        self.offsetText.set_ha(position)
        self.offsetText.set_position((x, y))
        self.stale = True

    def set_ticks_position(self, position):
        if position == 'right':
            self.set_tick_params(which='both', right=True, labelright=True,
                                 left=False, labelleft=False)
            self.set_offset_position(position)
        elif position == 'left':
            self.set_tick_params(which='both', right=False, labelright=False,
                                 left=True, labelleft=True)
            self.set_offset_position(position)
        elif position == 'both':
            self.set_tick_params(which='both', right=True,
                                 left=True)
        elif position == 'none':
            self.set_tick_params(which='both', right=False,
                                 left=False)
        elif position == 'default':
            self.set_tick_params(which='both', right=True, labelright=False,
                                 left=True, labelleft=True)
        else:
            _api.check_in_list(['left', 'right', 'both', 'default', 'none'],
                               position=position)
        self.stale = True

    def tick_right(self):
        label = True
        if 'label1On' in self._major_tick_kw:
            label = (self._major_tick_kw['label1On']
                     or self._major_tick_kw['label2On'])
        self.set_ticks_position('right')
        # if labels were turned off before this was called
        # leave them off
        self.set_tick_params(which='both', labelright=label)

    def tick_left(self):
        label = True
        if 'label1On' in self._major_tick_kw:
            label = (self._major_tick_kw['label1On']
                     or self._major_tick_kw['label2On'])
        self.set_ticks_position('left')
        # if labels were turned off before this was called
        # leave them off
        self.set_tick_params(which='both', labelleft=label)

    def get_ticks_position(self):
        return {1: "left", 2: "right",
                "default": "default", "unknown": "unknown"}[
                    self._get_ticks_position()]

    get_view_interval, set_view_interval = _make_getset_interval(
        "view", "viewLim", "intervaly")

    get_data_interval, set_data_interval = _make_getset_interval(
        "data", "dataLim", "intervaly")

    def get_minpos(self):
        return self.axes.dataLim.minposy

    def set_default_intervals(self):
        # docstring inherited
        # only change view if dataLim has not changed and user has
        # not changed the view:
        if (not self.axes.dataLim.mutatedy() and
                not self.axes.viewLim.mutatedy()):
            if self._converter is not None:
                info = self._converter.axisinfo(self.units, self)
                if info.default_limits is not None:
                    ymin, ymax = self.convert_units(info.default_limits)
                    self.axes.viewLim.intervaly = ymin, ymax
        self.stale = True

    def get_tick_space(self):
        ends = mtransforms.Bbox.unit().transformed(
            self.axes.transAxes - self.get_figure(root=False).dpi_scale_trans)
        length = ends.height * 72
        # Having a spacing of at least 2 just looks good.
        size = self._get_tick_label_size('y') * 2
        if size > 0:
            return int(np.floor(length / size))
        else:
            return 2**31 - 1
