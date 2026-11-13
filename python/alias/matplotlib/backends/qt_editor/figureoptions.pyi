# The type stub of matplotlib.backends.qt_editor.figureoptions: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from itertools import chain
from matplotlib import cbook, cm, colors as mcolors, markers, image as mimage
from matplotlib.backends.qt_compat import QtGui
from matplotlib.backends.qt_editor import _formlayout
from matplotlib.dates import DateConverter, num2date
LINESTYLES = {'-': 'Solid', '--': 'Dashed', '-.': 'DashDot', ':': 'Dotted', 'None': 'None'}
DRAWSTYLES = {'default': 'Default', 'steps-pre': 'Steps (Pre)', 'steps': 'Steps (Pre)', 'steps-mid': 'Steps (Mid)', 'steps-post': 'Steps (Post)'}
MARKERS = markers.MarkerStyle.markers

def figure_edit(axes, parent=None):
    ...
__all__ = ['chain', 'cbook', 'cm', 'mcolors', 'markers', 'mimage', 'QtGui', 'DateConverter', 'num2date', 'LINESTYLES', 'DRAWSTYLES', 'MARKERS', 'figure_edit']
