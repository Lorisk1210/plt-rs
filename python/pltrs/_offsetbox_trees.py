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

matplotlib keeps the settings and children of its boxes unchecked (a
``None`` child, a string padding) and fails only when it computes with
them, as it measures or draws the box. A description meeting a value pltrs
(Rust) cannot take raises `Unusable` (`number`, `box_tree`,
`artist_handle`); its owner then keeps the tree pltrs (Rust) had (a tree
joining a figure so joins as `EMPTY`) and, until the program mends it,
fails as matplotlib's does where matplotlib computes with the tree
(``_unusable``, `TreeOwner._fail_as_drawn`): the boxes of the tree then
measure themselves with matplotlib's algorithms, which raise matplotlib's
errors.
"""

import numbers

import numpy as np

from . import _handles, _python_drawing, _unusable
from ._native import artist as _artist, offsetbox as _offsetbox

#: The methods through which matplotlib's boxes size, place and draw each
#: other: a subclass of the package's boxes overriding one of them sizes
#: and draws itself (see ``OffsetBox._sizes_itself``).
PROGRAM_PROTOCOL = ("draw", "get_bbox", "get_offset", "_get_bbox_and_child_offsets")


#: The description a tree joins a figure with while pltrs (Rust) cannot
#: take it (see `Unusable`): an empty box, shown nowhere.
EMPTY = ("hpacker", False, 0.0, 0.0, None, None, "baseline", "fixed", [])

#: The alignments and packing modes of matplotlib's packers.
PACK_ALIGNS = ("baseline", "left", "top", "right", "bottom", "center")
PACK_MODES = ("fixed", "expand", "equal")


class Unusable(Exception):
    """A box's setting or child that pltrs (Rust) cannot take, met while
    describing the box (see the module documentation)."""


def number(value):
    """*value*, a box's setting matplotlib computes with as it is given, as
    the float pltrs (Rust) takes; `Unusable` for no real number."""
    if isinstance(value, (numbers.Real, np.bool_)) or (
            isinstance(value, np.ndarray) and value.ndim == 0
            and value.dtype.kind in "biuf"):
        return float(value)
    raise Unusable


def optional_number(value):
    """`number`, or None for None (a size matplotlib computes itself)."""
    return None if value is None else number(value)


def choice(value, choices):
    """*value*, one of *choices* (which matplotlib checks when it draws the
    box); `Unusable` otherwise."""
    if isinstance(value, str) and value in choices:
        return value
    raise Unusable


def box_tree(child):
    """The description of the box *child*; `Unusable` for a child that is
    no offset box."""
    describe = getattr(type(child), "_native_tree", None)
    if describe is None:
        raise Unusable
    return describe(child)


def artist_handle(child):
    """The native of the artist *child* of a leaf box; `Unusable` for a
    child that is no artist."""
    from .artist import Artist
    if not isinstance(child, Artist):
        raise Unusable
    return child._native


def children(box):
    """The children of *box* as a list; `Unusable` for children matplotlib
    cannot iterate."""
    try:
        return list(box.get_children())
    except TypeError:
        raise Unusable from None


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


def joinable(describe):
    """The description *describe()* gives a tree joining a figure: the tree
    as it is now, or `EMPTY` while pltrs (Rust) cannot take it."""
    try:
        return describe()
    except Unusable:
        return EMPTY


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

    #: The names under which the owner keeps what pltrs (Rust) cannot take
    #: (see ``_unusable``): the tree, and those of a subclass.
    _problems = ("tree",)

    def _tree_root(self):
        """The box whose tree this is."""
        return self

    def _comes_back(self, parent):
        """Whether the owner joins *parent*, an Axes or a (sub)figure, of
        the figure it was removed from with its tree: pltrs (Rust) kept
        both, and takes them back as they were."""
        return (bool(self.__dict__.get("_removed")) and "_tree_handle" in self.__dict__
                and self._figure is parent.get_figure(root=True))

    def _tree_description(self):
        """The tree's description as it is now (see ``_native.offsetbox``);
        `Unusable` while pltrs (Rust) cannot take it."""
        return box_tree(self._tree_root())

    def _tree_to_join(self):
        """The description the tree joins a figure with (see `joinable`)."""
        return joinable(self._tree_description)

    def _adopt_root(self, natives, figure):
        """Make the artists of the tree the objects of the handles
        *natives* (an iterator) reach in *figure*, in tree order."""
        self._tree_root()._adopt_tree(natives, figure)

    def _own_tree(self, natives, figure, handle, described):
        """Own the tree pltrs (Rust) just laid out in *figure* from the
        description *described* under the box of handle *handle*, *natives*
        being the handles of its artists in tree order: their objects follow
        them, and each box of the tree learns its place. A tree joined as
        `EMPTY` waits until pltrs (Rust) can take it."""
        self._tree_handle = handle
        if described is EMPTY:
            self._described = EMPTY
            self._keep_problem("tree")
            # Its boxes are in the owner's figure all the same, measured
            # with matplotlib's algorithms.
            joined = getattr(type(self._tree_root()), "_joined", None)
            if joined is not None:
                joined(self._tree_root(), self, None)
        else:
            self._adopt_root(iter(natives), figure)
            self._tree_root()._joined(self, handle)
            self._described = self._tree_description()
            _unusable.drop(self, "tree")
        follow(self)

    def _sync_tree(self):
        """Hand pltrs (Rust) the tree as it is now, if it changed since it
        was last handed (matplotlib reads it again whenever it draws it);
        not while pltrs (Rust) draws the figure, which cannot change it
        then, nor while it cannot take it, which it keeps the tree it had
        through."""
        if self._described is None or not _python_drawing.figure_changeable():
            return
        self._note_late_boxes()
        try:
            described = self._tree_description()
        except Unusable:
            self._keep_problem("tree")
            return
        if described == self._described:
            _unusable.drop(self, "tree")
            return
        handle = self._tree_handle
        natives = _offsetbox.rebuild_tree(handle, described)
        self._own_tree(natives, self.get_figure(root=True), handle, described)

    def _note_late_boxes(self):
        """Note the boxes that joined the tree after it joined a figure (a
        child set since, one put in a list of children): matplotlib's add
        gave them no figure (``_joined_late``, see ``TextArea.get_bbox``),
        unless the program set one since."""
        joined = getattr(type(self._tree_root()), "_boxes", None)
        for box in [] if joined is None else joined(self._tree_root()):
            if box.__dict__.get("_tree_owner") is not self and box._figure is None:
                box._joined_late = True

    def _keep_problem(self, name):
        """Keep the tree's *name* (one of `_problems`) as pltrs (Rust)
        cannot take it: drawing and measuring the owner fail as matplotlib's
        do (`_fail_as_drawn`)."""
        if not _unusable.kept(self, name, False):
            _unusable.keep(self, name, True, self._fail_as_drawn)

    def _tree_unusable(self):
        """Whether pltrs (Rust) cannot take the tree as it is now (see
        `_keep_problem`): its boxes measure themselves then."""
        return any(_unusable.kept(self, name, False) for name in self._problems)

    def _fail_as_drawn(self, _kept):
        """Fail as matplotlib's draw of the owner fails on its tree as it is
        now: measure the tree with matplotlib's algorithms, then reach the
        artists drawing it would reach (`OffsetBox._check_drawing`)."""
        renderer = _python_drawing.drawing_renderer(self)
        root = self._tree_root()
        root.get_window_extent(renderer)
        root._check_drawing(renderer)
