"""
The clip regions of every artist (matplotlib's ``Artist.set_clip_box``,
``Artist.set_clip_path`` and their getters).

pltrs (Rust) clips an artist to the box of an axes or a figure, which it
follows as the layout changes, and to a clip path: a `.Path` in the
coordinates of a transform, the outline of a patch of the same figure
(followed as the patch changes), or the bounding box of a path. A patch
of another figure, or outside any, clips with its outline as it is when
it is given.
"""

from . import _api, _background_patch, _live_transforms, _mirrors
from ._native import artist as _artist
from ._native import transforms as _transforms
from .path import Path
from .transforms import (Affine2D, Bbox, BboxBase, BboxTransformTo,
                         Transform, TransformedBbox,
                         TransformedPatchPath, TransformedPath)


def _placed(patch):
    """The outline of *patch* in its coordinates, and the native transform
    of those (display coordinates for a patch without a transform)."""
    path = patch.get_patch_transform().transform_path(patch.get_path())
    native = _transforms.get_transform(patch._native)
    return path, (_live_transforms.to_native(None) if native is None
                  else native)


def _rectangle(bbox):
    """The closed outline of the box *bbox*."""
    (x0, y0), (x1, y1) = bbox.get_points()
    return Path([[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]],
                closed=True)


def _rectangle_clip_box(transform):
    """The clip box of an artist clipped to the unit square through
    *transform* (an Axes' or a (sub)figure's background): matplotlib's
    ``set_clip_path`` of a `.Rectangle` covering it, the unit box through
    the rectangle's transform (kept as ``_spanning``, by which the box
    crosses to pltrs (Rust))."""
    box = TransformedBbox(Bbox.unit(), BboxTransformTo(Bbox.unit()) + Affine2D()
                          + transform)
    box._spanning = transform
    return box


def clipped_as_added(artist, kwargs):
    """
    Leave *artist*, which a plotting method made in its Axes with the
    keyword arguments *kwargs* (applied already), clipped as matplotlib's
    add method leaves it after its constructor took them, when they give a
    clip box or a clip path: an artist without a clip path is clipped to
    the Axes' patch (``set_clip_path(axes.patch)``: a `.Rectangle` replaces
    the box given with its own, another shape adds its outline); one with
    a clip path keeps the box given, none by default, where pltrs (Rust)
    made it clipped to its Axes. Callers ask only when *kwargs* give
    either, so that calls without them do nothing more.
    """
    if artist.axes is None:
        return
    if artist.get_clip_path() is None:
        artist.set_clip_path(artist.axes.patch)
    else:
        artist.set_clip_box(kwargs.get("clip_box"))


def patch_clip_box(artist, axes):
    """
    The clip box of *artist* clipped to the patch of *axes*, as
    matplotlib's ``set_clip_path(axes.patch)`` makes it, kept for it: the
    unit box through the transform of a `.Rectangle` patch (the Axes box,
    followed as it is laid out, for the unit square); None for a patch of
    another shape, which clips with its path (`patch_clip_path`).
    """
    from .patches import Rectangle
    if _is_axes_box(axes):
        return _live_transforms.kept(artist, "clip box", None,
                                     lambda: _rectangle_clip_box(axes.transAxes))
    patch = _background_patch.axes_patch_shape(axes)
    if not isinstance(patch, Rectangle):
        return None
    return _live_transforms.kept(
        artist, "clip box", patch,
        lambda: TransformedBbox(Bbox.unit(), patch.get_transform()))


def _is_axes_box(axes):
    """Whether the patch of *axes* is the unit square of its box, which
    pltrs (Rust) knows without the patch being made."""
    return not _artist.clips_to_patch_shape(axes._native)


def patch_clip_path(artist, axes):
    """The clip path of *artist* clipped to the patch of *axes*, as
    matplotlib's ``set_clip_path(axes.patch)`` makes it, kept for it: the
    patch's `.TransformedPatchPath`, None for a `.Rectangle` (which clips
    as its box, `patch_clip_box`)."""
    from .patches import Rectangle
    if _is_axes_box(axes):
        return None
    patch = _background_patch.axes_patch_shape(axes)
    if isinstance(patch, Rectangle):
        return None
    return _live_transforms.kept(artist, "clip path", patch,
                                 lambda: TransformedPatchPath(patch))


def _spanned_axes(figure, spanning):
    """The Axes whose box the native transform *spanning* spans, None for
    a (sub)figure's (or an Axes without a Python object)."""
    kind, *args = _transforms.describe(spanning)
    return figure._artists.get(args[0]) if kind == "axes" else None


def take_parent_defaults(artist, parent, clip=True):
    """
    Give *artist*, added to *parent* (an Axes, or a (sub)figure) by its
    ``add_artist``, what matplotlib's gives it where pltrs (Rust) adopted
    it without (an offset box, an inset indicator, a quiver key, a table,
    which pltrs (Rust) places its own way): the parent's transform
    (``transData``, ``transSubfigure``) unless it had one, and with *clip*
    the clip box of the parent's patch unless it has a clip path.
    """
    if not artist.is_transform_set():
        artist._given_transform = getattr(parent, "transData", None) or parent.transSubfigure
    if (clip and "_clipbox" not in artist.__dict__ and artist.get_clip_path() is None
            and _artist.get_clip_box(artist._native) is None):
        if hasattr(parent, "transAxes"):
            box = patch_clip_box(artist, parent)
        else:
            box = _rectangle_clip_box(parent.transSubfigure)
        if box is not None:
            artist._clipbox = box


def clip_box_spec(clipbox):
    """
    The native description of the clip box *clipbox* as the clip path
    clipping to its bounding box (None for None): the unit box through the
    transform spanning the box of an Axes or a (sub)figure, followed as it
    is laid out again, the box of a `.TransformedBbox` through its
    transform, else the box in display coordinates.
    """
    if clipbox is None:
        return None
    spanning = getattr(clipbox, "_spanning", None)
    if spanning is not None:
        return ("bbox", _rectangle(Bbox.unit()), _live_transforms.to_native(spanning))
    if isinstance(clipbox, TransformedBbox):
        return ("bbox", _rectangle(clipbox._bbox),
                _live_transforms.to_native(clipbox._transform))
    return ("bbox", _rectangle(clipbox), _live_transforms.to_native(None))


def clip_path_spec(path, transform=None):
    """
    The native description of the clip path `.Artist.set_clip_path` takes
    as *path* and *transform* (None for none), a patch clipping with its
    outline as it is now: a `.Rectangle` with its bounding box, as
    matplotlib makes its clip box of it.
    """
    from .patches import Patch, Rectangle
    if transform is None:
        mirrored = _mirrors.ask(path, "clip_spec")
        if mirrored is not _mirrors.OWN:
            return mirrored
        if isinstance(path, Rectangle):
            return ("bbox", *_placed(path))
        if isinstance(path, Patch):
            return ("path", *_placed(path))
        if isinstance(path, tuple):
            path, transform = path
    if path is None:
        return None
    if isinstance(path, Path):
        _api.check_isinstance(Transform, transform=transform)
        return ("path", path, _live_transforms.to_native(transform))
    if isinstance(path, TransformedPatchPath):
        return ("path", *_placed(path._patch))
    if isinstance(path, TransformedPath):
        return ("path", path._path, _live_transforms.to_native(path._transform))
    raise TypeError(
        "Invalid arguments to set_clip_path, of type "
        f"{type(path).__name__} and {type(transform).__name__}")


class ClippingMixin:
    """The clipping methods of `.Artist`."""

    #: The clip path set (`get_clip_path`), matplotlib's attribute, which
    #: the program's drawing reads (`.offsetbox.DrawingArea.draw`): None
    #: until one is set.
    _clippath = None

    def set_clip_box(self, clipbox):
        """
        Set the artist's clip `.Bbox`.

        Parameters
        ----------
        clipbox : `~matplotlib.transforms.BboxBase` or None
            Will typically be created from a `.TransformedBbox`. For instance,
            ``TransformedBbox(Bbox([[0, 0], [1, 1]]), ax.transAxes)`` is the default
            clipping for an artist added to an Axes.

        """
        _api.check_isinstance((BboxBase, None), clipbox=clipbox)
        if clipbox is self.get_clip_box():
            # matplotlib changes nothing for the box the artist has.
            return
        self._keep_patch_path()
        spanning = getattr(clipbox, "_spanning", None)
        if clipbox is None or spanning is not None:
            _artist.set_clip_box(
                self._native,
                None if spanning is None else _live_transforms.to_native(spanning))
        else:
            _artist.set_clip_path(self._native, clip_box_spec(clipbox))
        self._clipbox = clipbox
        self.pchanged()

    @property
    def clipbox(self):
        """The clip box (matplotlib's attribute; see `get_clip_box`)."""
        return self.get_clip_box()

    @clipbox.setter
    def clipbox(self, clipbox):
        self.set_clip_box(clipbox)

    def _made_with(self, kwargs):
        """
        Give the artist, which a plotting method just made in its Axes, the
        properties *kwargs* it is made with, as matplotlib's constructor
        takes them before the Axes adds the artist, clipped then as the
        add leaves it (`clipped_as_added`).
        """
        ret = self._internal_update(kwargs)
        if "clip_box" in kwargs or "clip_path" in kwargs:
            clipped_as_added(self, kwargs)
        return ret

    def get_clip_box(self):
        """Return the clipbox."""
        if "_clipbox" in self.__dict__:
            return self._clipbox
        mirrored = _mirrors.ask(self, "clip_box")
        if mirrored is not _mirrors.OWN:
            return mirrored
        spanning = self._native_clip_box()
        if spanning is None:
            return None
        axes = _spanned_axes(self._figure, spanning)
        if axes is not None and not _is_axes_box(axes):
            return patch_clip_box(self, axes)
        return self._spanned_clip_box(spanning)

    def _native_clip_box(self):
        """The native transform spanning the box of the Axes or (sub)figure
        pltrs (Rust) clips the artist to, None for none."""
        return None if self._figure is None else _artist.get_clip_box(self._native)

    def _patch_clipping_axes(self):
        """The Axes whose patch, of another shape than a `.Rectangle`,
        clips the artist through the box pltrs (Rust) clips it to (as
        matplotlib's ``set_clip_path(axes.patch)``), with no clip path set
        since; None otherwise."""
        if "_clippath" in self.__dict__:
            return None
        spanning = self._native_clip_box()
        axes = None if spanning is None else _spanned_axes(self._figure, spanning)
        if axes is None or patch_clip_path(self, axes) is None:
            return None
        return axes

    def _keep_patch_path(self):
        """Before the clip box changes: keep clipping the artist to the
        patch of its Axes when that clips it with its path, as an explicit
        clip path (matplotlib's ``set_clip_box`` leaves the clip path)."""
        axes = self._patch_clipping_axes()
        if axes is not None:
            _artist.set_clip_path(self._native, ("artist", axes._native))
            self._clippath = patch_clip_path(self, axes)

    def _spanned_clip_box(self, spanning):
        """
        The clip box pltrs (Rust) gave the artist, the unit square through
        the native transform *spanning*: a `.TransformedBbox` made the first
        time, as matplotlib's ``add_*`` methods make one of the Axes' patch
        (kept as ``_spanning`` by which the box crosses to pltrs (Rust)).
        """
        def make():
            transform = _live_transforms.from_native(self._figure, spanning)
            return _rectangle_clip_box(transform)
        return _live_transforms.kept(
            self, "clip box", _transforms.describe(spanning), make)

    def set_clip_path(self, path, transform=None):
        """
        Set the artist's clip path.

        Parameters
        ----------
        path : `~matplotlib.patches.Patch` or `.Path` or `.TransformedPath` or None
            The clip path. If given a `.Path`, *transform* must be provided as
            well. If *None*, a previously set clip path is removed.
        transform : `~matplotlib.transforms.Transform`, optional
            Only used if *path* is a `.Path`, in which case the given `.Path`
            is converted to a `.TransformedPath` using *transform*.

        Notes
        -----
        For efficiency, if *path* is a `.Rectangle` this method will set the
        clipping box to the corresponding rectangle and set the clipping path
        to ``None``.

        For technical reasons (support of `~.Artist.set`), a tuple
        (*path*, *transform*) is also accepted as a single positional
        parameter.

        .. ACCEPTS: Patch or (Path, Transform) or None
        """
        from .patches import Patch, Rectangle

        spec = clip_path_spec(path, transform)
        if self._patch_clipping_axes() is not None:
            # The patch path clipping the artist is replaced.
            _artist.set_clip_box(self._native, None)
        if transform is None and isinstance(path, tuple):
            path, transform = path
        if transform is None and isinstance(path, Rectangle):
            self._clipbox = TransformedBbox(
                Bbox.unit(), path.get_transform())
            self._clippath = None
        elif transform is None and isinstance(path, Patch):
            spec = self._patch_spec(path, spec)
            self._clippath = TransformedPatchPath(path)
        elif isinstance(path, TransformedPatchPath):
            spec = self._patch_spec(path._patch, spec)
            self._clippath = path
        elif isinstance(path, Path):
            self._clippath = TransformedPath(path, transform)
        else:
            self._clippath = path
        _artist.set_clip_path(self._native, spec)
        self.pchanged()

    def _patch_spec(self, patch, outline):
        """The clip path description of *patch*: followed when the patch is
        an artist of this artist's figure, its *outline* otherwise (also
        the description of the part a mirror stands for, an Axes'
        background say, which `clip_path_spec` asked)."""
        if (self._figure is not None and patch.figure is self._figure
                and not _mirrors.is_attached(patch)):
            return ("artist", patch._native)
        return outline

    def get_clip_path(self):
        """Return the clip path."""
        if "_clippath" in self.__dict__:
            return self._clippath
        mirrored = _mirrors.ask(self, "clip_path")
        if mirrored is not _mirrors.OWN:
            return mirrored
        axes = self._patch_clipping_axes()
        return None if axes is None else patch_clip_path(self, axes)

    def get_transformed_clip_path_and_affine(self):
        """
        Return the clip path with the non-affine part of its
        transformation applied, and the remaining affine part of its
        transformation.
        """
        clippath = self.get_clip_path()
        if clippath is not None:
            return clippath.get_transformed_path_and_affine()
        return None, None

    def _fully_clipped_to_axes(self):
        """
        Return a boolean flag, ``True`` if the artist is clipped to the Axes
        and can thus be skipped in layout calculations.
        """
        return (self._figure is not None
                and _artist.fully_clipped_to_axes(self._native))
