# The type stub of matplotlib._tight_layout: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import numpy as np
import matplotlib as mpl
from matplotlib import _api, artist as martist
from matplotlib.font_manager import FontProperties
from matplotlib.transforms import Bbox

def _auto_adjust_subplotpars(fig, renderer, shape, span_pairs, subplot_list, ax_bbox_list=None, pad=1.08, h_pad=None, w_pad=None, rect=None):
    ...

def get_subplotspec_list(axes_list, grid_spec=None):
    ...

def get_tight_layout_figure(fig, axes_list, subplotspec_list, renderer, pad=1.08, h_pad=None, w_pad=None, rect=None):
    ...
__all__ = ['np', 'mpl', 'martist', 'FontProperties', 'Bbox', 'get_subplotspec_list', 'get_tight_layout_figure']
