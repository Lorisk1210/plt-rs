# The type stub of matplotlib.sphinxext.plot_directive: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from collections import defaultdict
import contextlib
import doctest
from io import StringIO
import itertools
import os
from os.path import relpath
from pathlib import Path
import re
import shutil
import sys
import textwrap
import traceback
from docutils.parsers.rst import directives, Directive
from docutils.parsers.rst.directives.images import Image
import jinja2
from sphinx.environment.collectors import EnvironmentCollector
from sphinx.errors import ExtensionError
import matplotlib
from matplotlib.backend_bases import FigureManagerBase
import matplotlib.pyplot as plt
from matplotlib import _pylab_helpers, cbook
matplotlib.use('agg')
__version__ = 2

def _option_boolean(arg):
    ...

def _option_context(arg):
    ...

def _option_format(arg):
    ...

def mark_plot_labels(app, document):
    ...

class PlotDirective(Directive):
    has_content = True
    required_arguments = 0
    optional_arguments = 2
    final_argument_whitespace = False
    option_spec = {'alt': directives.unchanged, 'height': directives.length_or_unitless, 'width': directives.length_or_percentage_or_unitless, 'scale': directives.nonnegative_int, 'align': Image.align, 'class': directives.class_option, 'filename-prefix': directives.unchanged, 'include-source': _option_boolean, 'show-source-link': _option_boolean, 'format': _option_format, 'context': _option_context, 'nofigs': directives.flag, 'caption': directives.unchanged, 'code-caption': directives.unchanged}

    def run(self):
        ...

def _copy_css_file(app, exc):
    ...

def setup(app):
    ...

def init_filename_registry(app):
    ...

class _FilenameCollector(EnvironmentCollector):

    def process_doc(self, app, doctree):
        ...

    def clear_doc(self, app, env, docname):
        ...

    def merge_other(self, app, env, docnames, other):
        ...

def contains_doctest(text):
    ...

def _split_code_at_show(text, function_name):
    ...
_SOURCECODE = '\n{{ source_code }}\n\n.. only:: html\n\n   {% if src_name or (html_show_formats and not multi_image) %}\n   (\n   {%- if src_name -%}\n   :download:`Source code <{{ build_dir }}/{{ src_name }}>`\n   {%- endif -%}\n   {%- if html_show_formats and not multi_image -%}\n     {%- for img in images -%}\n       {%- for fmt in img.formats -%}\n         {%- if src_name or not loop.first -%}, {% endif -%}\n         :download:`{{ fmt }} <{{ build_dir }}/{{ img.basename }}.{{ fmt }}>`\n       {%- endfor -%}\n     {%- endfor -%}\n   {%- endif -%}\n   )\n   {% endif %}\n'
TEMPLATE_SRCSET = _SOURCECODE + '\n   {% for img in images %}\n   .. figure-mpl:: {{ build_dir }}/{{ img.basename }}.{{ default_fmt }}\n      {% for option in options -%}\n      {{ option }}\n      {% endfor %}\n      {%- if caption -%}\n      {{ caption }}  {# appropriate leading whitespace added beforehand #}\n      {% endif -%}\n      {%- if srcset -%}\n        :srcset: {{ build_dir }}/{{ img.basename }}.{{ default_fmt }}\n        {%- for sr in srcset -%}\n            , {{ build_dir }}/{{ img.basename }}.{{ sr }}.{{ default_fmt }} {{sr}}\n        {%- endfor -%}\n      {% endif %}\n\n   {% if html_show_formats and multi_image %}\n   (\n    {%- for fmt in img.formats -%}\n    {%- if not loop.first -%}, {% endif -%}\n    :download:`{{ fmt }} <{{ build_dir }}/{{ img.basename }}.{{ fmt }}>`\n    {%- endfor -%}\n   )\n   {% endif %}\n\n\n   {% endfor %}\n\n.. only:: not html\n\n   {% for img in images %}\n   .. figure-mpl:: {{ build_dir }}/{{ img.basename }}.*\n      {% for option in options -%}\n      {{ option }}\n      {% endfor -%}\n\n      {{ caption }}  {# appropriate leading whitespace added beforehand #}\n   {% endfor %}\n\n'
TEMPLATE = _SOURCECODE + '\n\n   {% for img in images %}\n   .. figure:: {{ build_dir }}/{{ img.basename }}.{{ default_fmt }}\n      {% for option in options -%}\n      {{ option }}\n      {% endfor %}\n\n      {% if html_show_formats and multi_image -%}\n        (\n        {%- for fmt in img.formats -%}\n        {%- if not loop.first -%}, {% endif -%}\n        :download:`{{ fmt }} <{{ build_dir }}/{{ img.basename }}.{{ fmt }}>`\n        {%- endfor -%}\n        )\n      {%- endif -%}\n\n      {{ caption }}  {# appropriate leading whitespace added beforehand #}\n   {% endfor %}\n\n.. only:: not html\n\n   {% for img in images %}\n   .. figure:: {{ build_dir }}/{{ img.basename }}.*\n      {% for option in options -%}\n      {{ option }}\n      {% endfor -%}\n\n      {{ caption }}  {# appropriate leading whitespace added beforehand #}\n   {% endfor %}\n\n'
exception_template = '\n.. only:: html\n\n   [`source code <%(linkdir)s/%(basename)s.py>`__]\n\nException occurred rendering plot.\n\n'
plot_context = dict()

class ImageFile:

    def __init__(self, basename, dirname):
        self.basename = basename
        self.dirname = dirname
        self.formats = []

    def filename(self, format):
        ...

    def filenames(self):
        ...

def out_of_date(original, derived, includes=None):
    ...

class PlotError(RuntimeError):
    pass

def _run_code(code, code_path, ns=None, function_name=None):
    ...

def clear_state(plot_rcparams, close=True):
    ...

def get_plot_formats(config):
    ...

def _parse_srcset(entries):
    ...

def check_output_base_name(env, output_base):
    ...

def render_figures(code, code_path, output_dir, output_base, context, function_name, config, context_reset=False, close_figs=False, code_includes=None):
    ...

def run(arguments, content, options, state_machine, state, lineno):
    ...
__all__ = ['defaultdict', 'contextlib', 'doctest', 'StringIO', 'itertools', 'os', 'relpath', 'Path', 're', 'shutil', 'sys', 'textwrap', 'traceback', 'directives', 'Directive', 'Image', 'jinja2', 'EnvironmentCollector', 'ExtensionError', 'matplotlib', 'FigureManagerBase', 'plt', 'cbook', 'mark_plot_labels', 'PlotDirective', 'setup', 'init_filename_registry', 'contains_doctest', 'TEMPLATE_SRCSET', 'TEMPLATE', 'exception_template', 'plot_context', 'ImageFile', 'out_of_date', 'PlotError', 'clear_state', 'get_plot_formats', 'check_output_base_name', 'render_figures', 'run']
