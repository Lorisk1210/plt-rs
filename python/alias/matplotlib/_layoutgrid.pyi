# The type stub of matplotlib._layoutgrid: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import itertools
import kiwisolver as kiwi
import logging
import numpy as np
import matplotlib as mpl
import matplotlib.patches as mpatches
from matplotlib.transforms import Bbox
_log = logging.getLogger(__name__)

class LayoutGrid:

    def __init__(self, parent=None, parent_pos=(0, 0), parent_inner=False, name='', ncols=1, nrows=1, h_pad=None, w_pad=None, width_ratios=None, height_ratios=None):
        self.parent_pos = parent_pos
        self.parent_inner = parent_inner
        self.name = name + seq_id()
        self.nrows = nrows
        self.ncols = ncols
        self.height_ratios = np.atleast_1d(height_ratios)
        self.width_ratios = np.atleast_1d(width_ratios)
        self.artists = np.empty((nrows, ncols), dtype=object)
        self.children = np.empty((nrows, ncols), dtype=object)
        self.margins = {}
        self.margin_vals = {}
        self.lefts = [Variable(f'{sn}lefts[{i}]') for i in range(ncols)]
        self.rights = [Variable(f'{sn}rights[{i}]') for i in range(ncols)]
        self.bottoms = [Variable(f'{sn}bottoms[{i}]') for i in range(nrows)]
        self.tops = [Variable(f'{sn}tops[{i}]') for i in range(nrows)]
        self.h_pad = h_pad
        self.w_pad = w_pad
        self.name = f'{parent.name}.{self.name}'
        self.height_ratios = np.ones(nrows)
        self.width_ratios = np.ones(ncols)
        self.solver = kiwi.Solver()
        self.solver = parent.solver
        self.margin_vals[todo] = np.zeros(ncols)
        self.margins[todo] = [Variable(f'{sn}margins[{todo}][{i}]') for i in range(ncols)]
        self.margins[todo] = np.empty(nrows, dtype=object)
        self.margin_vals[todo] = np.zeros(nrows)
        self.margins[todo] = [Variable(f'{sn}margins[{todo}][{i}]') for i in range(nrows)]

    def __repr__(self):
        ...

    def reset_margins(self):
        ...

    def add_constraints(self, parent):
        ...

    def hard_constraints(self):
        ...

    def add_child(self, child, i=0, j=0):
        self.children[np.ix_(np.atleast_1d(i), np.atleast_1d(j))] = child

    def parent_constraints(self, parent):
        ...

    def grid_constraints(self):
        ...

    def edit_margin(self, todo, size, cell):
        self.margin_vals[todo][cell] = size

    def edit_margin_min(self, todo, size, cell=0):
        ...

    def edit_margins(self, todo, size):
        ...

    def edit_all_margins_min(self, todo, size):
        ...

    def edit_outer_margin_mins(self, margin, ss):
        ...

    def get_margins(self, todo, col):
        ...

    def get_outer_bbox(self, rows=0, cols=0):
        ...

    def get_inner_bbox(self, rows=0, cols=0):
        ...

    def get_bbox_for_cb(self, rows=0, cols=0):
        ...

    def get_left_margin_bbox(self, rows=0, cols=0):
        ...

    def get_bottom_margin_bbox(self, rows=0, cols=0):
        ...

    def get_right_margin_bbox(self, rows=0, cols=0):
        ...

    def get_top_margin_bbox(self, rows=0, cols=0):
        ...

    def update_variables(self):
        ...
_layoutboxobjnum = itertools.count()

def seq_id():
    ...

def plot_children(fig, lg=None, level=0):
    ...
__all__ = ['itertools', 'kiwi', 'logging', 'np', 'mpl', 'mpatches', 'Bbox', 'LayoutGrid', 'seq_id', 'plot_children']
