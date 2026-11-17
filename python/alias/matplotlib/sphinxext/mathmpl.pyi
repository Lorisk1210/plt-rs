# The type stub of matplotlib.sphinxext.mathmpl: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import hashlib
from pathlib import Path
from docutils import nodes
from docutils.parsers.rst import Directive, directives
import sphinx
from sphinx.errors import ConfigError, ExtensionError
import matplotlib as mpl
from matplotlib import _api, mathtext
from matplotlib.rcsetup import validate_float_or_None

class latex_math(nodes.General, nodes.Element):
    pass

def fontset_choice(arg):
    ...

def math_role(role, rawtext, text, lineno, inliner, options={}, content=[]):
    ...
math_role.options = {'fontset': fontset_choice, 'fontsize': validate_float_or_None}

class MathDirective(Directive):
    has_content = True
    required_arguments = 0
    optional_arguments = 0
    final_argument_whitespace = False
    option_spec = {'fontset': fontset_choice, 'fontsize': validate_float_or_None}

    def run(self):
        ...

def latex2png(latex, filename, fontset='cm', fontsize=10, dpi=100):
    ...

def latex2html(node, source):
    ...

def _config_inited(app, config):
    ...

def setup(app):
    ...
__all__ = ['hashlib', 'Path', 'nodes', 'Directive', 'directives', 'sphinx', 'ConfigError', 'ExtensionError', 'mpl', 'mathtext', 'validate_float_or_None', 'latex_math', 'fontset_choice', 'math_role', 'MathDirective', 'latex2png', 'latex2html', 'setup']
