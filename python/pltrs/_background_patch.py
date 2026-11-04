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
those of the background.
"""

from . import _live_transforms, _mirrors
from ._native import axisartist as _axisartist, patches as _patches


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
        return self.owner.contains_point(point)

    def clip_spec(self, patch):
        # The outline of the Axes, followed as it changes.
        return ("artist", self.owner._native)


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


def _made(owner, patch, mirroring, transform):
    """*patch* made the mirror of the background of *owner*, placed by
    *transform* unless it was made with a transform of its own, as
    matplotlib places it. A patch of any class may be the background (the
    one an Axes class of the program's makes, a `.RegularPolygon` say):
    its class is made `~._mirrors.mirrorable` when one first is."""
    if not getattr(type(patch).set_facecolor, "_pushes", False):
        _mirrors.mirrorable(type(patch))
    _mirrors.attach(patch, mirroring)
    if not patch.is_transform_set():
        with _mirrors.detached(patch):
            patch.set_transform(transform)
    owner.__dict__["_patch"] = patch
    return patch


def axes_patch(axes):
    """The patch of the background of *axes* (the same object while the
    Axes keeps it), with the background's style."""
    patch = axes.__dict__.get("_patch")
    if patch is None:
        patch = _made(axes, axes._gen_axes_patch(), _AxesBackground(axes),
                      axes.transAxes)
    patch._mirror.pull(patch)
    return patch


def share_axes_patch(axes, patch):
    """Make *patch*, the background of another Axes, that of *axes*
    (matplotlib's ``aux_ax.patch = ax.patch``): the parasite *axes* clips
    to its outline and draws it again among its children."""
    mirror = _mirrors.mirroring(patch)
    if not isinstance(mirror, _AxesBackground):
        raise TypeError("an Axes' patch must be the background patch of an Axes")
    if mirror.owner is not axes:
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
    patch._mirror.pull(patch)
    return patch
