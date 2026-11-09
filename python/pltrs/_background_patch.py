"""
The backgrounds of axes and figures (matplotlib's ``Axes.patch`` and
``Figure.patch``).

pltrs (Rust) draws the background of an axes or a (sub)figure as part of
it. Its ``patch`` is a *mirror* (`._mirrors`): a loose patch of the class
matplotlib makes it of (the `.Rectangle` of the unit square, or what the
Axes' ``_gen_axes_patch`` makes: a `.Wedge` for polar axes), in the
coordinates matplotlib places it in, which takes the background's style
(colours, edge width and style, hatch, caps, joins, anti-aliasing, alpha,
visibility, z-order and group id) whenever it is handed out, and whose
setters give the background their changes. Its extent, its figure,
whether it contains a point and the region clipping to it clips to are
those of the background. A patch the program makes an Axes' patch
(`set_axes_patch`) becomes such a mirror too, its outline the
background's shape.
"""

from numbers import Real

import numpy as np

from . import _live_transforms, _mirrors
from ._native import axisartist as _axisartist, patches as _patches
from ._native import projections as _projections


class _Background(_mirrors.Mirroring):
    """The background of *owner*, an Axes or a (sub)figure."""

    def __init__(self, owner):
        self.owner = owner

    def push(self, patch):
        _patches.push_background(self.owner._native, patch._native)

    def pull(self, patch):
        """Give *patch* the background's style."""
        _patches.pull_background(self.owner._native, patch._native)

    def window_extent(self, patch, renderer=None, dpi=None):
        return patch.get_path().get_extents(patch.get_transform())

    def contains(self, patch, mouseevent, radius=None):
        return self.owner.contains(mouseevent)


class _AxesBackground(_Background):
    """The background of an Axes."""

    def figure(self, patch, root=False):
        return self.owner.get_figure(root=root)

    def contains_point(self, patch, point, radius=None):
        if not all(isinstance(value, Real) for value in point):
            # What is no pair of numbers is hit-tested with the patch's path
            # as matplotlib's patch hit-tests any point (its errors).
            from .patches import Patch
            return Patch.contains_point(patch, point, radius)
        return self.owner.contains_point(point)

    def clip_spec(self, patch):
        # The outline of the Axes, followed as it changes.
        return ("artist", self.owner._native)

    def refresh(self, patch):
        # What the Axes' draws gave the patch (a polar Axes' `.Wedge`).
        self.owner._fit_patch(patch)


class _ShapedAxesBackground(_AxesBackground):
    """The background of an Axes whose patch the program makes (its class'
    ``_gen_axes_patch``, or `set_axes_patch`), whose outline is the
    background's shape and what the Axes clips to, whenever the program
    changes the patch (cartopy's ``GeoAxes`` gives its patch the map's
    boundary as it draws, clipped to the view; astropy's frames move the
    vertices of their patch's path)."""

    def push(self, patch):
        super().push(patch)
        push_outline(self.owner, patch)


class _FigureBackground(_Background):
    """The background of a (sub)figure."""

    def figure(self, patch, root=False):
        return self.owner.get_figure(root=True) if root else self.owner

    def clip_spec(self, patch):
        # The box of the (sub)figure, as matplotlib clips to a Rectangle.
        from ._artist_clipping import _rectangle
        from .transforms import Bbox
        return ("bbox", _rectangle(Bbox.unit()),
                _live_transforms.to_native(self.owner.transSubfigure))


def _attach(patch, mirroring):
    """Make *patch* the mirror of a background (*mirroring*). A patch of any
    class may be the background (the one an Axes class of the program's
    makes, a `.RegularPolygon` say): its class is made
    `~._mirrors.mirrorable` when one first is."""
    if not getattr(type(patch).set_facecolor, "_pushes", False):
        _mirrors.mirrorable(type(patch))
    _mirrors.attach(patch, mirroring)


def _made(owner, patch, mirroring, transform):
    """*patch* made the mirror of the background of *owner*, placed by
    *transform* unless it was made with a transform of its own, as
    matplotlib places it."""
    _attach(patch, mirroring)
    if not patch.is_transform_set():
        with _mirrors.detached(patch):
            patch.set_transform(transform)
    owner.__dict__["_patch"] = patch
    return patch


def axes_patch(axes):
    """The patch of the background of *axes* (the same object while the
    Axes keeps it), with the background's style."""
    patch = axes_patch_shape(axes)
    patch._mirror.pull(patch)
    return patch


def axes_patch_shape(axes):
    """The patch of the background of *axes* as `axes_patch` gives it, for
    its shape and place alone (what clips to it), its style not read."""
    patch = axes.__dict__.get("_patch")
    if patch is None:
        from .projections import _program
        mirroring = (_ShapedAxesBackground(axes) if _program.shapes_background(type(axes))
                     else _AxesBackground(axes))
        patch = _made(axes, axes._gen_axes_patch(), mirroring, axes.transAxes)
        # The colours matplotlib's Axes give their patch.
        patch.__dict__["_rc_colors"] = {"facecolor": "axes.facecolor"}
    return patch


def push_outline(axes, patch):
    """
    Make the outline of *patch*, in the coordinates of *axes* as they are
    now, the shape the background of *axes* fills and what the Axes clips
    to (matplotlib's ``set_clip_path(self.patch)``: its box for a
    `.Rectangle`); the unit square is the Axes' own box.
    """
    from .artist import Artist
    from .patches import Rectangle
    placement = patch.get_patch_transform() + (Artist.get_transform(patch) - axes.transAxes)
    outline = placement.transform_path(patch.get_path())
    if isinstance(patch, Rectangle) and np.allclose(
            outline.get_extents().bounds, (0, 0, 1, 1)):
        _projections.set_patch_outline(axes._native, None, True)
    else:
        _projections.set_patch_outline(axes._native, outline, isinstance(patch, Rectangle))


def set_axes_patch(axes, patch):
    """
    Make *patch* the background of *axes* (``ax.patch = patch``):

    * the background of another Axes (matplotlib's ``aux_ax.patch =
      ax.patch``): the parasite *axes* clips to its outline and draws it
      again among its children;
    * a patch of the program's, in an Axes or not (astropy's ``WCSAxes``
      gives its frame's `.PathPatch`, in data coordinates): the patch
      becomes the mirror of the background, which takes its style, and
      its outline, in the Axes' coordinates as it is now, is the shape the
      background fills and what the Axes clips to.
    """
    mirror = _mirrors.mirroring(patch)
    if mirror is not None and not isinstance(mirror, _AxesBackground):
        raise TypeError("an Axes' patch cannot be the background of a figure")
    if mirror is None:
        _attach(patch, _ShapedAxesBackground(axes))
        patch._mirror.push(patch)
    elif mirror.owner is not axes:
        _axisartist.share_floating_patch(mirror.owner._native, axes._native)
    axes.__dict__["_patch"] = patch


def figure_patch(figure):
    """The patch of the background of *figure* (a figure or subfigure),
    with the background's style."""
    from .patches import Rectangle
    patch = figure.__dict__.get("_patch")
    if patch is None:
        patch = _made(figure, Rectangle((0, 0), 1, 1), _FigureBackground(figure),
                      figure.transSubfigure)
        with _mirrors.detached(patch):
            # Left out of the figure's tight box, as matplotlib makes it.
            patch.set_in_layout(False)
        # The colours matplotlib's figures give their patch.
        patch.__dict__["_rc_colors"] = {"facecolor": "figure.facecolor",
                                        "edgecolor": "figure.edgecolor"}
    patch._mirror.pull(patch)
    return patch


def checked_style(facecolor, edgecolor, linewidth):
    """
    The width of the background of a (sub)figure styled *facecolor*,
    *edgecolor* and *linewidth*, checked as the `.Rectangle` matplotlib
    makes its ``patch`` of checks them (the edge colour, then the face
    colour, `.colors.to_rgba`; then the width, ``float``, as it keeps it;
    None for the rc defaults of patches).
    """
    from . import colors, rcParams
    colors.to_rgba(rcParams['patch.edgecolor'] if edgecolor is None else edgecolor)
    colors.to_rgba(rcParams['patch.facecolor'] if facecolor is None else facecolor)
    return float(rcParams['patch.linewidth'] if linewidth is None else linewidth)
