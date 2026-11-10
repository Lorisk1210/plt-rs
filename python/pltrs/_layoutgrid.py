"""
A layoutgrid is a nrows by ncols set of boxes, meant to be used by
constrained layout, each box is analogous to a subplotspec element of
a gridspec.

Each box is defined by left[ncols], right[ncols], bottom[nrows] and top[nrows],
and by two editable margins for each side.  The main margin gets its value
set by the size of ticklabels, titles, etc on each Axes that is in the figure.
The outer margin is the padding around the Axes, and space for any
colorbars.

pltrs (Rust) states the grids' constraints and solves them
(``pltrs::layout``); the `LayoutGrid` objects here mirror the grids it
solved, as plain values: ``ConstrainedLayoutEngine.execute(fig)`` returns
them as matplotlib's does (its ``layoutgrids``, keyed by the figures, the
grid specifications and ``(gridspec, 'top')`` for the cell a nested grid
specification sits in), for debugging and `plot_children`. They hold no
figure and no solver, so nothing stays alive through them.

The class's reading methods and `plot_children` are matplotlib 3.11.2's,
under the Matplotlib License.
"""

import itertools

import numpy as np

from .transforms import Bbox


class Variable:
    """
    A solved variable of a grid: its name and the value the solver gave it
    (the reading side of kiwisolver's ``Variable``).
    """

    def __init__(self, name, value=0.0):
        self._name = name
        self._value = float(value)

    def name(self):
        """The variable's name."""
        return self._name

    def value(self):
        """The value of the last solution (0 before the solver ran)."""
        return self._value

    def __repr__(self):
        return f'{self._name}: {self._value}'


class LayoutGrid:
    """
    Analogous to a gridspec, and contained in another LayoutGrid.

    A grid made here is unsolved, its variables zero, as matplotlib's are
    until its solver runs; `ConstrainedLayoutEngine.execute` makes the
    grids of a figure with the values pltrs (Rust) solved.
    """

    def __init__(self, parent=None, parent_pos=(0, 0),
                 parent_inner=False, name='', ncols=1, nrows=1,
                 h_pad=None, w_pad=None, width_ratios=None,
                 height_ratios=None):
        self.parent_pos = parent_pos
        self.parent_inner = parent_inner
        self.name = name + seq_id()
        if isinstance(parent, LayoutGrid):
            self.name = f'{parent.name}.{self.name}'
        self.nrows = nrows
        self.ncols = ncols
        self.height_ratios = np.atleast_1d(height_ratios)
        if height_ratios is None:
            self.height_ratios = np.ones(nrows)
        self.width_ratios = np.atleast_1d(width_ratios)
        if width_ratios is None:
            self.width_ratios = np.ones(ncols)

        sn = self.name + '_'
        if isinstance(parent, LayoutGrid):
            parent.add_child(self, *parent_pos)
        # keep track of artist associated w/ this layout.  Can be none
        self.artists = np.empty((nrows, ncols), dtype=object)
        self.children = np.empty((nrows, ncols), dtype=object)

        self.lefts = [Variable(f'{sn}lefts[{i}]') for i in range(ncols)]
        self.rights = [Variable(f'{sn}rights[{i}]') for i in range(ncols)]
        self.bottoms = [Variable(f'{sn}bottoms[{i}]') for i in range(nrows)]
        self.tops = [Variable(f'{sn}tops[{i}]') for i in range(nrows)]
        self.margins = {}
        self.margin_vals = {}
        for todo, count in (('left', ncols), ('right', ncols),
                            ('leftcb', ncols), ('rightcb', ncols),
                            ('bottom', nrows), ('top', nrows),
                            ('bottomcb', nrows), ('topcb', nrows)):
            self.margins[todo] = [Variable(f'{sn}margins[{todo}][{i}]')
                                  for i in range(count)]
            self.margin_vals[todo] = np.zeros(count)

        self.h_pad = h_pad
        self.w_pad = w_pad

    def __repr__(self):
        str = f'LayoutBox: {self.name:25s} {self.nrows}x{self.ncols},\n'
        for i in range(self.nrows):
            for j in range(self.ncols):
                str += f'{i}, {j}: '\
                       f'L{self.lefts[j].value():1.3f}, ' \
                       f'B{self.bottoms[i].value():1.3f}, ' \
                       f'R{self.rights[j].value():1.3f}, ' \
                       f'T{self.tops[i].value():1.3f}, ' \
                       f'ML{self.margins["left"][j].value():1.3f}, ' \
                       f'MR{self.margins["right"][j].value():1.3f}, ' \
                       f'MB{self.margins["bottom"][i].value():1.3f}, ' \
                       f'MT{self.margins["top"][i].value():1.3f}, \n'
        return str

    def _take_solution(self, solved):
        """Take the values pltrs (Rust) solved, the dict *solved*."""
        for variables, values in ((self.lefts, solved['lefts']),
                                  (self.rights, solved['rights']),
                                  (self.bottoms, solved['bottoms']),
                                  (self.tops, solved['tops'])):
            for variable, value in zip(variables, values):
                variable._value = float(value)
        for todo, values in solved['margins'].items():
            for variable, value in zip(self.margins[todo], values):
                variable._value = float(value)
        for todo, values in solved['margin_vals'].items():
            self.margin_vals[todo] = np.array(values, dtype=float)

    def add_child(self, child, i=0, j=0):
        # np.ix_ returns the cross product of i and j indices
        self.children[np.ix_(np.atleast_1d(i), np.atleast_1d(j))] = child

    def get_margins(self, todo, col):
        """Return the margin at this position"""
        return self.margin_vals[todo][col]

    def get_outer_bbox(self, rows=0, cols=0):
        """
        Return the outer bounding box of the subplot specs
        given by rows and cols.  rows and cols can be spans.
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            self.lefts[cols[0]].value(),
            self.bottoms[rows[-1]].value(),
            self.rights[cols[-1]].value(),
            self.tops[rows[0]].value())
        return bbox

    def get_inner_bbox(self, rows=0, cols=0):
        """
        Return the inner bounding box of the subplot specs
        given by rows and cols.  rows and cols can be spans.
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            (self.lefts[cols[0]].value() +
                self.margins['left'][cols[0]].value() +
                self.margins['leftcb'][cols[0]].value()),
            (self.bottoms[rows[-1]].value() +
                self.margins['bottom'][rows[-1]].value() +
                self.margins['bottomcb'][rows[-1]].value()),
            (self.rights[cols[-1]].value() -
                self.margins['right'][cols[-1]].value() -
                self.margins['rightcb'][cols[-1]].value()),
            (self.tops[rows[0]].value() -
                self.margins['top'][rows[0]].value() -
                self.margins['topcb'][rows[0]].value())
        )
        return bbox

    def get_bbox_for_cb(self, rows=0, cols=0):
        """
        Return the bounding box that includes the
        decorations but, *not* the colorbar...
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            (self.lefts[cols[0]].value() +
                self.margins['leftcb'][cols[0]].value()),
            (self.bottoms[rows[-1]].value() +
                self.margins['bottomcb'][rows[-1]].value()),
            (self.rights[cols[-1]].value() -
                self.margins['rightcb'][cols[-1]].value()),
            (self.tops[rows[0]].value() -
                self.margins['topcb'][rows[0]].value())
        )
        return bbox

    def get_left_margin_bbox(self, rows=0, cols=0):
        """
        Return the left margin bounding box of the subplot specs
        given by rows and cols.  rows and cols can be spans.
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            (self.lefts[cols[0]].value() +
                self.margins['leftcb'][cols[0]].value()),
            (self.bottoms[rows[-1]].value()),
            (self.lefts[cols[0]].value() +
                self.margins['leftcb'][cols[0]].value() +
                self.margins['left'][cols[0]].value()),
            (self.tops[rows[0]].value()))
        return bbox

    def get_bottom_margin_bbox(self, rows=0, cols=0):
        """
        Return the left margin bounding box of the subplot specs
        given by rows and cols.  rows and cols can be spans.
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            (self.lefts[cols[0]].value()),
            (self.bottoms[rows[-1]].value() +
             self.margins['bottomcb'][rows[-1]].value()),
            (self.rights[cols[-1]].value()),
            (self.bottoms[rows[-1]].value() +
                self.margins['bottom'][rows[-1]].value() +
             self.margins['bottomcb'][rows[-1]].value()
             ))
        return bbox

    def get_right_margin_bbox(self, rows=0, cols=0):
        """
        Return the left margin bounding box of the subplot specs
        given by rows and cols.  rows and cols can be spans.
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            (self.rights[cols[-1]].value() -
                self.margins['right'][cols[-1]].value() -
                self.margins['rightcb'][cols[-1]].value()),
            (self.bottoms[rows[-1]].value()),
            (self.rights[cols[-1]].value() -
                self.margins['rightcb'][cols[-1]].value()),
            (self.tops[rows[0]].value()))
        return bbox

    def get_top_margin_bbox(self, rows=0, cols=0):
        """
        Return the left margin bounding box of the subplot specs
        given by rows and cols.  rows and cols can be spans.
        """
        rows = np.atleast_1d(rows)
        cols = np.atleast_1d(cols)

        bbox = Bbox.from_extents(
            (self.lefts[cols[0]].value()),
            (self.tops[rows[0]].value() -
                self.margins['topcb'][rows[0]].value()),
            (self.rights[cols[-1]].value()),
            (self.tops[rows[0]].value() -
                self.margins['topcb'][rows[0]].value() -
                self.margins['top'][rows[0]].value()))
        return bbox


_layoutboxobjnum = itertools.count()


def seq_id():
    """Generate a short sequential id for layoutbox objects."""
    return '%06d' % next(_layoutboxobjnum)


def execute(fig, spec):
    """
    Have pltrs (Rust) lay *fig* out with the layout engine *spec* (as
    `.LayoutEngine._native_spec` describes one): constrained layout returns
    the grids it solved, as `LayoutGrid` objects (see `_from_native`), or
    None without grids.
    """
    from ._native import figure as _figure
    solved = _figure.execute_layout(fig._native, spec)
    if solved is not None:
        return _from_native(fig, solved)


def _from_native(fig, solved):
    """
    The layout grids of *fig* as pltrs (Rust) solved them, from the list
    *solved* of `_native.figure.execute_layout`: matplotlib's
    ``layoutgrids``, keyed by the (sub)figures, the grid specifications
    and ``(gridspec, 'top')``, with ``'hasgrids'``.
    """
    from . import gridspec
    from ._native import artist as _artist
    frames = {}
    pending = [fig]
    while pending:
        frame = pending.pop()
        frames[_artist.id(frame._native)] = frame
        pending.extend(frame.subfigs)
    names = {'frame': 'figlb', 'gridspec': 'gridspec', 'nested': 'top'}
    layoutgrids = {'hasgrids': True}
    grids = []
    for grid in solved:
        kind, owner = grid['owner']
        name = names[kind]
        parent = grid['parent']
        if parent[0] == 'rect':
            options = {'parent': parent[1]}
        else:
            _, index, rows, cols, inner = parent
            options = {'parent': grids[index], 'parent_inner': inner,
                       'parent_pos': (range(*rows), range(*cols))}
            if kind == 'frame':
                name = 'panellb'
        layoutgrid = LayoutGrid(
            name=name, nrows=len(grid['height_ratios']),
            ncols=len(grid['width_ratios']),
            width_ratios=grid['width_ratios'],
            height_ratios=grid['height_ratios'], **options)
        layoutgrid._take_solution(grid)
        grids.append(layoutgrid)
        if kind == 'frame':
            layoutgrids[frames[owner]] = layoutgrid
        elif kind == 'gridspec':
            layoutgrids[gridspec._wrap(owner, fig)] = layoutgrid
        else:
            layoutgrids[(gridspec._wrap(owner, fig), 'top')] = layoutgrid
    return layoutgrids


def plot_children(fig, lg=None, level=0):
    """Simple plotting to show where boxes are."""
    from . import patches as mpatches, rcParams
    if lg is None:
        _layoutgrids = fig.get_layout_engine().execute(fig)
        lg = _layoutgrids[fig]
    colors = rcParams["axes.prop_cycle"].by_key()["color"]
    col = colors[level]
    for i in range(lg.nrows):
        for j in range(lg.ncols):
            bb = lg.get_outer_bbox(rows=i, cols=j)
            fig.add_artist(
                mpatches.Rectangle(bb.p0, bb.width, bb.height, linewidth=1,
                                   edgecolor='0.7', facecolor='0.7',
                                   alpha=0.2, transform=fig.transFigure,
                                   zorder=-3))
            bbi = lg.get_inner_bbox(rows=i, cols=j)
            fig.add_artist(
                mpatches.Rectangle(bbi.p0, bbi.width, bbi.height, linewidth=2,
                                   edgecolor=col, facecolor='none',
                                   transform=fig.transFigure, zorder=-2))

            bbi = lg.get_left_margin_bbox(rows=i, cols=j)
            fig.add_artist(
                mpatches.Rectangle(bbi.p0, bbi.width, bbi.height, linewidth=0,
                                   edgecolor='none', alpha=0.2,
                                   facecolor=[0.5, 0.7, 0.5],
                                   transform=fig.transFigure, zorder=-2))
            bbi = lg.get_right_margin_bbox(rows=i, cols=j)
            fig.add_artist(
                mpatches.Rectangle(bbi.p0, bbi.width, bbi.height, linewidth=0,
                                   edgecolor='none', alpha=0.2,
                                   facecolor=[0.7, 0.5, 0.5],
                                   transform=fig.transFigure, zorder=-2))
            bbi = lg.get_bottom_margin_bbox(rows=i, cols=j)
            fig.add_artist(
                mpatches.Rectangle(bbi.p0, bbi.width, bbi.height, linewidth=0,
                                   edgecolor='none', alpha=0.2,
                                   facecolor=[0.5, 0.5, 0.7],
                                   transform=fig.transFigure, zorder=-2))
            bbi = lg.get_top_margin_bbox(rows=i, cols=j)
            fig.add_artist(
                mpatches.Rectangle(bbi.p0, bbi.width, bbi.height, linewidth=0,
                                   edgecolor='none', alpha=0.2,
                                   facecolor=[0.7, 0.2, 0.7],
                                   transform=fig.transFigure, zorder=-2))
    for ch in lg.children.flat:
        if ch is not None:
            plot_children(fig, ch, level=level+1)
