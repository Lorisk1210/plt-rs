"""
`SecondaryAxis` (matplotlib's ``axes._secondary_axes``): an extra axis on
an Axes, showing the parent's range converted by a pair of functions.
pltrs (Rust) makes it as a child axes of its parent and keeps its scale,
limits and place in step with the parent's (``_native.insets``).
"""

import functools
import numbers

from .. import _api, _handles, scale as mscale, transforms
from .._native import artist as _artist, insets as _insets
from . import _program_locators
from ._axes import Axes
from ._insets import _TransformedBoundsLocator


class SecondaryAxis(Axes):
    """
    General class to hold a Secondary_X/Yaxis.
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True
    #: Where matplotlib makes a secondary axes of each orientation, as
    #: ``[left, bottom, width, height]`` in figure fractions.
    _MADE_AT = {"x": (0, 1., 1, 0.0001), "y": (0, 1., 0.0001, 1)}

    def get_position(self, original=False):
        """
        Return the position of the Axes within the figure as a `.Bbox`.

        pltrs: as in matplotlib, whose figure places a secondary axes by
        its locator only when it is drawn (``Figure._get_draw_artists``,
        animated axes too), the original position stays where it was
        made, and so does the active one until the figure is first drawn.
        """
        figure = self.get_figure(root=True)
        if original or figure is None or not _artist.draw_count(figure._native):
            return transforms.Bbox.from_bounds(*self._MADE_AT[self._orientation])
        return super().get_position(original)

    def _active_bounds(self):
        # Where the axis was made until the figure's first draw places it
        # (see `get_position`).
        figure = self.get_figure(root=True)
        if figure is None or not _artist.draw_count(figure._native):
            return self._MADE_AT[self._orientation]
        return super()._active_bounds()

    def __init__(self, parent, orientation, location, functions,
                 transform=None, **kwargs):
        """
        See `.secondary_xaxis` and `.secondary_yaxis` for the doc string.
        While there is no need for this to be private, it should really be
        called by those higher level functions.
        """
        _api.check_in_list(["x", "y"], orientation=orientation)
        self._parent = parent
        self._orientation = orientation
        self._ticks_set = False
        if orientation == 'x':
            self._locstrings = ['top', 'bottom']
            self._otherstrings = ['left', 'right']
        else:
            self._locstrings = ['right', 'left']
            self._otherstrings = ['top', 'bottom']
        self._functions = self._checked_functions(functions)
        native_location = self._native_location(location, transform)
        _handles.adopt(self, _insets.secondary_axis(
            parent._native, orientation, native_location,
            self._native_functions()), parent.figure)
        self._child_of = parent
        self._init_axes_state()
        self._axis = self.xaxis if orientation == 'x' else self.yaxis
        if self._pos < 0.5:
            # flip the location strings...
            self._locstrings = self._locstrings[::-1]
        self._internal_update(kwargs)

    def __repr__(self):
        fields = []
        if self.get_label():
            fields += [f"label={self.get_label()!r}"]
        for name, label in (("x", self.get_xlabel()), ("y", self.get_ylabel())):
            if label:
                fields += [f"{name}label={label!r}"]
        return f"<{self.__class__.__name__}: " + ", ".join(fields) + ">"

    def set_alignment(self, align):
        """
        Set if axes spine and labels are drawn at top or bottom (or left/right)
        of the Axes.

        Parameters
        ----------
        align : {'top', 'bottom', 'left', 'right'}
            Either 'top' or 'bottom' for orientation='x' or
            'left' or 'right' for orientation='y' axis.
        """
        _api.check_in_list(self._locstrings, align=align)
        if align == self._locstrings[1]:  # Need to change the orientation.
            self._locstrings = self._locstrings[::-1]
        self.spines[self._locstrings[0]].set_visible(True)
        self.spines[self._locstrings[1]].set_visible(False)
        self._axis.set_ticks_position(align)
        self._axis.set_label_position(align)

    def set_location(self, location, transform=None):
        """
        Set the vertical or horizontal location of the axes in
        parent-normalized coordinates.

        Parameters
        ----------
        location : {'top', 'bottom', 'left', 'right'} or float
            The position to put the secondary axis.  Strings can be 'top' or
            'bottom' for orientation='x' and 'right' or 'left' for
            orientation='y'. A float indicates the relative position on the
            parent Axes to put the new Axes, 0.0 being the bottom (or left)
            and 1.0 being the top (or right).

        transform : `.Transform`, optional
            Transform for the location to use. Defaults to
            the parent's ``transAxes``, so locations are normally relative to
            the parent axes.
        """
        _insets.set_secondary_location(
            self._native, self._native_location(location, transform))

    def _native_location(self, location, transform):
        """The native location of *location* in the coordinates of
        *transform*, checked as matplotlib's `set_location` checks it."""
        _api.check_isinstance((transforms.Transform, None), transform=transform)
        if isinstance(location, str):
            _api.check_in_list(self._locstrings, location=location)
            self._pos = 1. if location in ('top', 'right') else 0.
        elif isinstance(location, numbers.Real):
            self._pos = location
        else:
            raise ValueError(
                f"location must be {self._locstrings[0]!r}, "
                f"{self._locstrings[1]!r}, or a float, not {location!r}")
        self._loc = location
        if self._orientation == 'x':
            bounds = [0, self._pos, 1., 1e-10]
        else:
            bounds = [self._pos, 0, 1e-10, 1]
        parent = self._parent
        _program_locators.forget(self, parent._figure)
        if transform is None or transform is parent.transAxes:
            self._axes_locator = _TransformedBoundsLocator(
                bounds, parent.transAxes)
            return location if isinstance(location, str) else float(location)
        self._axes_locator = _TransformedBoundsLocator(
            bounds, self._located_blend(transform))
        if transform is parent.transData:
            return ("data", float(location))
        # Any other transform is followed at every draw, where it is
        # evaluated, as the parent's fraction it puts the axis at.
        self._location_transform = transform
        _program_locators.follow(self, parent._figure)
        return self._parent_fraction(transform)

    def _locate_by_program(self, renderer):
        """Put the axis where its locating transform says now (see
        `._program_locators`)."""
        _insets.set_secondary_location(
            self._native, self._parent_fraction(self._location_transform))

    def _located_blend(self, transform):
        """The parent's axes fractions across the axis, *transform* along
        it (matplotlib's blend of the locating transform)."""
        if self._orientation == 'x':
            return transforms.blended_transform_factory(
                self._parent.transAxes, transform)
        return transforms.blended_transform_factory(
            transform, self._parent.transAxes)

    def _parent_fraction(self, transform):
        """Where *transform* puts the axis now, as a fraction of the
        parent (the blend is made anew: the figure's live transforms do
        not invalidate the matrices transforms keep)."""
        point = (0, self._pos) if self._orientation == 'x' else (self._pos, 0)
        fraction = self._parent.transAxes.inverted().transform(
            self._located_blend(transform).transform(point))
        return float(fraction[1] if self._orientation == 'x' else fraction[0])

    @functools.wraps(Axes.set_xticks)
    def set_xticks(self, *args, **kwargs):
        if self._orientation == "y":
            raise TypeError("Cannot set xticks on a secondary y-axis")
        ret = super().set_xticks(*args, **kwargs)
        self._ticks_set = True
        return ret

    @functools.wraps(Axes.set_yticks)
    def set_yticks(self, *args, **kwargs):
        if self._orientation == "x":
            raise TypeError("Cannot set yticks on a secondary x-axis")
        ret = super().set_yticks(*args, **kwargs)
        self._ticks_set = True
        return ret

    def set_ticks(self, *args, **kwargs):
        """Set the tick locations of the secondary axis (`.Axis.set_ticks`)."""
        ret = self._axis.set_ticks(*args, **kwargs)
        self._ticks_set = True
        return ret

    @staticmethod
    def _checked_functions(functions):
        """The pair of functions *functions* stands for, checked as
        matplotlib's `set_functions` checks them."""
        if (isinstance(functions, tuple) and len(functions) == 2 and
                callable(functions[0]) and callable(functions[1])):
            return functions
        if isinstance(functions, transforms.Transform):
            return (functions.transform,
                    lambda x: functions.inverted().transform(x))
        if functions is None:
            return (lambda x: x, lambda x: x)
        raise ValueError('functions argument of secondary Axes '
                         'must be a two-tuple of callable functions '
                         'with the first function being the transform '
                         'and the second being the inverse')

    def _default_axis_scale(self, axis, name):
        """The scale of the secondary axis, which pltrs (Rust) sets from
        the parent's scale: matplotlib's ``_set_scale`` sets the function
        scale of the inverted functions ('functionlog' over a log axis)."""
        if axis is self.__dict__.get("_axis") and name in ("function", "functionlog"):
            scale = mscale.FuncScaleLog if name == "functionlog" else mscale.FuncScale
            return scale(axis, self._functions[::-1])
        return super()._default_axis_scale(axis, name)

    def _native_functions(self):
        """The functions, which pltrs (Rust) calls with arrays, as
        matplotlib does."""
        return self._functions

    def set_functions(self, functions):
        """
        Set how the secondary axis converts limits from the parent Axes.

        Parameters
        ----------
        functions : 2-tuple of func, or `Transform` with an inverse.
            Transform between the parent axis values and the secondary axis
            values.

            If supplied as a 2-tuple of functions, the first function is
            the forward transform function and the second is the inverse
            transform.

            If a transform is supplied, then the transform must have an
            inverse.
        """
        self._functions = self._checked_functions(functions)
        _insets.set_secondary_functions(self._native, self._native_functions())

    def set_aspect(self, *args, **kwargs):
        """
        Secondary Axes cannot set the aspect ratio, so calling this just
        sets a warning.
        """
        _api.warn_external("Secondary Axes can't set the aspect ratio")

    def set_color(self, color):
        """
        Change the color of the secondary Axes and all decorators.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        axis = self._axis
        axis.set_tick_params(colors=color)
        for spine in self.spines.values():
            if spine.axis is axis:
                spine.set_color(color)
        axis.label.set_color(color)
