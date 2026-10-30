"""
A collection of functions and objects for creating or placing inset axes
(matplotlib's ``mpl_toolkits.axes_grid1.inset_locator``): pltrs (Rust)
places the insets anew at every layout pass.
"""

from pltrs import _api, _handles, _live_transforms
from pltrs._native import axes_grid1 as _axes_grid1


def _size(value):
    """The native size of *value*: inches, or ``"30%"`` of the anchor box."""
    if isinstance(value, str):
        return float(value.rstrip('%')), True
    return float(value), False


def _placement(parent_axes, loc, bbox_to_anchor, bbox_transform, borderpad,
               relative):
    """The native placement of an inset (matplotlib's checks of the
    anchor box)."""
    figure = parent_axes.get_figure(root=False)
    if (bbox_transform in [parent_axes.transAxes, figure.transFigure]
            and bbox_to_anchor is None):
        _api.warn_external("Using the axes or figure transform requires a "
                           "bounding box in the respective coordinates. "
                           "Using bbox_to_anchor=(0, 0, 1, 1) now.")
        bbox_to_anchor = (0, 0, 1, 1)
    if relative and isinstance(bbox_to_anchor, tuple) and len(bbox_to_anchor) != 4:
        raise ValueError("Using relative units for width or height "
                         "requires to provide a 4-tuple or a "
                         "`Bbox` instance to `bbox_to_anchor.")
    if bbox_to_anchor is not None and not isinstance(bbox_to_anchor, tuple):
        bbox_to_anchor = tuple(bbox_to_anchor.bounds)
        bbox_transform = None
    if not isinstance(borderpad, tuple):
        borderpad = (borderpad, borderpad)
    return (loc,
            None if bbox_to_anchor is None else [float(v) for v in bbox_to_anchor],
            None if bbox_transform is None
            else _live_transforms.to_native(bbox_transform),
            tuple(map(float, borderpad)))


def _wrap_inset(parent_axes, native, axes_class, axes_kwargs):
    """The inset's object, of *axes_class* (the parent's class by default),
    styled by *axes_kwargs*."""
    inset = _handles.wrap(native, axes_class or type(parent_axes),
                          parent_axes.figure)
    inset._internal_update(axes_kwargs or {})
    return inset


def inset_axes(parent_axes, width, height, loc='upper right',
               bbox_to_anchor=None, bbox_transform=None,
               axes_class=None, axes_kwargs=None,
               borderpad=0.5):
    """
    Create an inset axes with a given width and height.

    Parameters
    ----------
    parent_axes : `matplotlib.axes.Axes`
        Axes to place the inset axes.
    width, height : float or str
        Size of the inset axes to create. If a float is provided, it is
        the size in inches, e.g. *width=1.3*. If a string is provided, it is
        the size in relative units, e.g. *width='40%'*.
    loc : str, default: 'upper right'
        Location to place the inset axes.
    bbox_to_anchor : tuple or `~matplotlib.transforms.BboxBase`, optional
        Bbox that the inset axes will be anchored to.
    bbox_transform : `~matplotlib.transforms.Transform`, optional
        Transformation for the bbox that contains the inset axes.
    axes_class : `~matplotlib.axes.Axes` type, default: the parent's class
        The type of the newly created inset axes.
    axes_kwargs : dict, optional
        Keyword arguments to pass to the constructor of the inset axes.
    borderpad : float or (float, float), default: 0.5
        Padding between inset axes and the bbox_to_anchor, in fractions of
        the font size.

    Returns
    -------
    inset_axes : *axes_class*
        Inset axes object created.
    """
    relative = isinstance(width, str) or isinstance(height, str)
    native = _axes_grid1.inset_axes(
        parent_axes._native, (_size(width), _size(height)),
        _placement(parent_axes, loc, bbox_to_anchor, bbox_transform,
                   borderpad, relative))
    return _wrap_inset(parent_axes, native, axes_class, axes_kwargs)


def zoomed_inset_axes(parent_axes, zoom, loc='upper right',
                      bbox_to_anchor=None, bbox_transform=None,
                      axes_class=None, axes_kwargs=None,
                      borderpad=0.5):
    """
    Create an anchored inset axes by scaling a parent axes.

    Parameters
    ----------
    parent_axes : `~matplotlib.axes.Axes`
        Axes to place the inset axes.
    zoom : float
        Scaling factor of the data axes. *zoom* > 1 will enlarge the
        coordinates (i.e., "zoomed in"), while *zoom* < 1 will shrink the
        coordinates (i.e., "zoomed out").
    loc : str, default: 'upper right'
        Location to place the inset axes.
    bbox_to_anchor, bbox_transform, axes_class, axes_kwargs, borderpad
        As `inset_axes`.

    Returns
    -------
    inset_axes : *axes_class*
        Inset axes object created.
    """
    native = _axes_grid1.zoomed_inset_axes(
        parent_axes._native, float(zoom),
        _placement(parent_axes, loc, bbox_to_anchor, bbox_transform,
                   borderpad, False))
    return _wrap_inset(parent_axes, native, axes_class, axes_kwargs)
