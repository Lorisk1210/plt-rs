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

from . import _api, _live_transforms
from ._native import artist as _artist
from ._native import transforms as _transforms
from .path import Path
from .transforms import (Bbox, BboxBase, Transform, TransformedBbox,
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


def clip_path_spec(path, transform=None):
    """
    The native description of the clip path `.Artist.set_clip_path` takes
    as *path* and *transform* (None for none), a patch clipping with its
    outline as it is now: a `.Rectangle` with its bounding box, as
    matplotlib makes its clip box of it.
    """
    from .patches import Patch, Rectangle
    if transform is None:
        background_spec = getattr(path, "_clip_spec", None)
        if background_spec is not None:
            return background_spec()
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

    def set_clip_box(self, clipbox):
        """
        Set the artist's clip `.Bbox`.

        Parameters
        ----------
        clipbox : `~matplotlib.transforms.BboxBase` or None
            Will typically be created from a `.TransformedBbox`. For instance,
            ``TransformedBbox(Bbox([[0, 0], [1, 1]]), ax.transAxes)`` is the
            default clipping for an artist added to an Axes.
        """
        _api.check_isinstance((BboxBase, None), clipbox=clipbox)
        spanning = getattr(clipbox, "_spanning", None)
        if clipbox is None or spanning is not None:
            _artist.set_clip_box(
                self._native,
                None if spanning is None else _live_transforms.to_native(spanning))
        elif isinstance(clipbox, TransformedBbox):
            _artist.set_clip_path(self._native, (
                "bbox", _rectangle(clipbox._bbox),
                _live_transforms.to_native(clipbox._transform)))
        else:
            _artist.set_clip_path(self._native, (
                "bbox", _rectangle(clipbox), _live_transforms.to_native(None)))
        self._clipbox = clipbox
        self.pchanged()

    def get_clip_box(self):
        """Return the clipbox."""
        if "_clipbox" in self.__dict__:
            return self._clipbox
        if self._figure is None:
            return None
        spanning = _artist.get_clip_box(self._native)
        if spanning is None:
            return None
        return _live_transforms.LiveBbox.spanned(
            _live_transforms.from_native(self._figure, spanning))

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
        """
        from .patches import Patch, Rectangle

        spec = clip_path_spec(path, transform)
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
        an artist of this artist's figure, its *outline* otherwise."""
        if self._figure is not None and patch.figure is self._figure:
            return ("artist", patch._native)
        return outline

    def get_clip_path(self):
        """Return the clip path."""
        return self.__dict__.get("_clippath")

    def _fully_clipped_to_axes(self):
        """
        Return a boolean flag, ``True`` if the artist is clipped to the Axes
        and can thus be skipped in layout calculations.
        """
        return (self._figure is not None
                and _artist.fully_clipped_to_axes(self._native))
