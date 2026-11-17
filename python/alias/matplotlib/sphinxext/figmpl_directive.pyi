# The type stub of matplotlib.sphinxext.figmpl_directive: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import os
from os.path import relpath
from pathlib import PurePath, Path
import shutil
from docutils import nodes
from docutils.parsers.rst import directives
from docutils.parsers.rst.directives.images import Figure, Image
from sphinx.errors import ExtensionError
import matplotlib

class figmplnode(nodes.General, nodes.Element):
    pass

class FigureMpl(Figure):
    has_content = False
    required_arguments = 1
    optional_arguments = 2
    final_argument_whitespace = False
    option_spec = {'alt': directives.unchanged, 'height': directives.length_or_unitless, 'width': directives.length_or_percentage_or_unitless, 'scale': directives.nonnegative_int, 'align': Image.align, 'class': directives.class_option, 'caption': directives.unchanged, 'srcset': directives.unchanged}

    def run(self):
        ...

def _parse_srcsetNodes(st):
    ...

def _copy_images_figmpl(self, node):
    ...

def visit_figmpl_html(self, node):
    ...

def visit_figmpl_latex(self, node):
    ...

def depart_figmpl_html(self, node):
    ...

def depart_figmpl_latex(self, node):
    ...

def figurempl_addnode(app):
    ...

def setup(app):
    ...
__all__ = ['os', 'relpath', 'PurePath', 'Path', 'shutil', 'nodes', 'directives', 'Figure', 'Image', 'ExtensionError', 'matplotlib', 'figmplnode', 'FigureMpl', 'visit_figmpl_html', 'visit_figmpl_latex', 'depart_figmpl_html', 'depart_figmpl_latex', 'figurempl_addnode', 'setup']
