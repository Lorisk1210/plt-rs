# The type stub of matplotlib._constrained_layout: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import logging
import numpy as np
from matplotlib import _api, artist as martist
import matplotlib.transforms as mtransforms
import matplotlib._layoutgrid as mlayoutgrid
_log = logging.getLogger(__name__)

def do_constrained_layout(fig, h_pad, w_pad, hspace=None, wspace=None, rect=(0, 0, 1, 1), compress=False):
    ...

def make_layoutgrids(fig, layoutgrids, rect=(0, 0, 1, 1)):
    ...

def make_layoutgrids_gs(layoutgrids, gs):
    ...

def check_no_collapsed_axes(layoutgrids, fig):
    ...

def compress_fixed_aspect(layoutgrids, fig):
    ...

def get_margin_from_padding(obj, *, w_pad=0, h_pad=0, hspace=0, wspace=0):
    ...

def make_layout_margins(layoutgrids, fig, renderer, *, w_pad=0, h_pad=0, hspace=0, wspace=0):
    ...

def make_margin_suptitles(layoutgrids, fig, renderer, *, w_pad=0, h_pad=0):
    ...

def match_submerged_margins(layoutgrids, fig):
    ...

def get_cb_parent_spans(cbax):
    ...

def get_pos_and_bbox(ax, renderer):
    ...

def reposition_axes(layoutgrids, fig, renderer, *, w_pad=0, h_pad=0, hspace=0, wspace=0, compress=False):
    ...

def reposition_colorbar(layoutgrids, cbax, renderer, *, offset=None, compress=False):
    ...

def reset_margins(layoutgrids, fig):
    ...

def colorbar_get_pad(layoutgrids, cax):
    ...
__all__ = ['logging', 'np', 'martist', 'mtransforms', 'mlayoutgrid', 'do_constrained_layout', 'make_layoutgrids', 'make_layoutgrids_gs', 'check_no_collapsed_axes', 'compress_fixed_aspect', 'get_margin_from_padding', 'make_layout_margins', 'make_margin_suptitles', 'match_submerged_margins', 'get_cb_parent_spans', 'get_pos_and_bbox', 'reposition_axes', 'reposition_colorbar', 'reset_margins', 'colorbar_get_pad']
