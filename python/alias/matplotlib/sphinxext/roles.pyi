# The type stub of matplotlib.sphinxext.roles: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from urllib.parse import urlsplit, urlunsplit
from docutils import nodes
import matplotlib
from matplotlib import rcParamsDefault

class _QueryReference(nodes.Inline, nodes.TextElement):

    def to_query_string(self):
        ...

def _visit_query_reference_node(self, node):
    ...

def _depart_query_reference_node(self, node):
    ...
_RC_WILDCARD_LINK_MAPPING = {'animation.[name-of-encoder]_args': 'animation.ffmpeg_args', 'figure.subplot.*': 'figure.subplot.left', 'figure.subplot.[name]': 'figure.subplot.left', 'font.*': 'font.family', 'lines.*': 'lines.linewidth', 'patch.*': 'patch.edgecolor', 'grid.major.*': 'grid.major.color', 'grid.minor.*': 'grid.minor.color', 'grid.*': 'grid.color'}

def _rcparam_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    ...

def _mpltype_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    ...

def setup(app):
    ...
__all__ = ['urlsplit', 'urlunsplit', 'nodes', 'matplotlib', 'rcParamsDefault', 'setup']
