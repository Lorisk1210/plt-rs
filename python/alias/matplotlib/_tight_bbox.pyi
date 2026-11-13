# The type stub of matplotlib._tight_bbox: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from matplotlib.transforms import Bbox, TransformedBbox, Affine2D

def adjust_bbox(fig, bbox_inches, renderer, fixed_dpi=None):
    ...

def process_figure_for_rasterizing(fig, bbox_inches_restore, renderer, fixed_dpi=None):
    ...
__all__ = ['Bbox', 'TransformedBbox', 'Affine2D', 'adjust_bbox', 'process_figure_for_rasterizing']
