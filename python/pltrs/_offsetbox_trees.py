"""
The trees of offset boxes pltrs (Rust) lays out, as the program has them
now.

An offset box added to an axes or a figure (an `.AnchoredOffsetbox`, an
`.AnnotationBbox`, a plain box at an offset) hands pltrs (Rust) the tree of
boxes it holds (``OffsetBox._native_tree``, see ``_native.offsetbox``),
which lays it out and draws it. matplotlib reads the settings and the
children of a box again whenever it draws it, so the box that added a tree,
its *owner* (`TreeOwner`), hands it again whenever it changed
(`TreeOwner._sync_tree`): when a setting of one of its boxes is set
(`TreeSetting`), before every layout of a draw (the figure's layout hook,
`.axes._program_locators`), which sees what Python does not see being
changed (a list of children changed in place), and before Python measures a
box of the tree. pltrs (Rust) keeps the artists the tree held, takes the
loose ones it holds now and lets go of those it no longer holds
(``pltrs::offsetbox::rebuild_tree``).

Every box of a joined tree knows its place there, ``_node``: the native
handle of its owner and the box's index in tree order (a box before the
boxes inside it), through which it answers matplotlib's ``get_bbox`` and
``get_offset``.
"""

from . import _handles, _python_drawing
from ._native import artist as _artist, offsetbox as _offsetbox

#: The methods through which matplotlib's boxes size, place and draw each
#: other: a subclass of the package's boxes overriding one of them sizes
#: and draws itself (see ``OffsetBox._sizes_itself``).
PROGRAM_PROTOCOL = ("draw", "get_bbox", "get_offset", "_get_bbox_and_child_offsets")


class TreeSetting:
    """A setting of a box (a packer's ``sep``, a drawing area's ``width``)
    which the tree holding the box is handed again with once it is set,
    as matplotlib reads it again whenever it draws the box."""

    def __set_name__(self, owner, name):
        self._name = "_" + name

    def __get__(self, box, cls=None):
        return self if box is None else box.__dict__[self._name]

    def __set__(self, box, value):
        box.__dict__[self._name] = value
        box._changed()


def adopt_part(artist, native, figure):
    """Make *artist* the object of the artist the handle *native* reaches
    in *figure* (`._handles.adopt`), unless it is already; return whether
    it was not."""
    if (artist.__dict__.get("_figure") is figure._figure
            and _artist.id(artist._native) == _artist.id(native)):
        return False
    _handles.adopt(artist, native, figure)
    return True


def follow(owner):
    """Have the root figure of *owner* call its ``_locate_by_program``
    before every layout of a draw (see `.axes._program_locators`)."""
    from .axes import _program_locators
    _program_locators.follow(owner, owner.get_figure(root=True))


class TreeOwner:
    """
    What added a tree of boxes pltrs (Rust) lays out (see the module
    documentation): a subclass gives the box whose tree it is
    (`_tree_root`) and its description (`_tree_description`); the native
    handle of the box pltrs (Rust) laid out is ``_tree_handle`` once the
    tree joined a figure.
    """

    #: The tree's description as pltrs (Rust) was last handed it.
    _described = None

    def _tree_root(self):
        """The box whose tree this is."""
        return self

    def _tree_description(self):
        """The tree's description as it is now (see ``_native.offsetbox``)."""
        return self._tree_root()._native_tree()

    def _adopt_root(self, natives, figure):
        """Make the artists of the tree the objects of the handles
        *natives* (an iterator) reach in *figure*, in tree order."""
        self._tree_root()._adopt_tree(natives, figure)

    def _own_tree(self, natives, figure, handle):
        """Own the tree whose artists pltrs (Rust) just laid out in
        *figure* under the box of handle *handle*, *natives* being their
        handles in tree order: their objects follow them, and each box of
        the tree learns its place."""
        self._adopt_root(iter(natives), figure)
        self._tree_root()._joined(self, handle)
        self._tree_handle = handle
        self._described = self._tree_description()
        follow(self)

    def _sync_tree(self):
        """Hand pltrs (Rust) the tree as it is now, if it changed since it
        was last handed (matplotlib reads it again whenever it draws it);
        not while pltrs (Rust) draws the figure, which cannot change it
        then."""
        if self._described is None or not _python_drawing.figure_changeable():
            return
        described = self._tree_description()
        if described == self._described:
            return
        handle = self._tree_handle
        natives = _offsetbox.rebuild_tree(handle, described)
        self._own_tree(natives, self.get_figure(root=True), handle)
