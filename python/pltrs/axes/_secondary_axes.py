"""
`SecondaryAxis` (matplotlib's ``axes._secondary_axes``): an extra axis on
an Axes, showing the parent's range converted by a pair of functions.
pltrs (Rust) makes it as a child axes of its parent and keeps its scale,
limits and place in step with the parent's (``_native.insets``).
"""

import functools
import numbers

import numpy as np

from .. import _api, _handles, transforms
from .._native import insets as _insets
from ._axes import Axes
from ._insets import _TransformedBoundsLocator


def _scalar_function(function):
    """*function*, which takes arrays as matplotlib calls it, called on one
    value as pltrs (Rust) calls it."""
    def call(value):
        with np.errstate(all="ignore"):
            return float(np.asarray(function(np.float64(value))))
    return call


class SecondaryAxis(Axes):
    """
    General class to hold a Secondary_X/Yaxis.
    """

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
        if transform is None or transform is parent.transAxes:
            self._axes_locator = _TransformedBoundsLocator(
                bounds, parent.transAxes)
            return location if isinstance(location, str) else float(location)
        if transform is not parent.transData:
            raise NotImplementedError(
                "pltrs: secondary axes located by a transform other than "
                "the parent's transData or transAxes")
        blend = (transforms.blended_transform_factory(parent.transAxes, transform)
                 if self._orientation == 'x' else
                 transforms.blended_transform_factory(transform, parent.transAxes))
        self._axes_locator = _TransformedBoundsLocator(bounds, blend)
        return ("data", float(location))

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

    def _native_functions(self):
        """The functions as pltrs (Rust) calls them, value by value."""
        forward, inverse = self._functions
        return (_scalar_function(forward), _scalar_function(inverse))

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
