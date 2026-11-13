# The type stub of matplotlib.backends.backend_pgf: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import codecs
import datetime
import functools
from io import BytesIO
import logging
import math
import os
import pathlib
import shutil
import subprocess
from tempfile import TemporaryDirectory
import weakref
from PIL import Image
import matplotlib as mpl
from matplotlib import cbook, font_manager as fm
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, RendererBase
from matplotlib.backends.backend_mixed import MixedModeRenderer
from matplotlib.backends.backend_pdf import _create_pdf_info_dict, _datetime_to_pdf
from matplotlib.path import Path
from matplotlib.figure import Figure
from matplotlib.font_manager import FontProperties
from matplotlib._pylab_helpers import Gcf
_log = logging.getLogger(__name__)
_DOCUMENTCLASS = '\\documentclass{article}'

def _get_preamble():
    ...
latex_pt_to_in = 1.0 / 72.27
latex_in_to_pt = 1.0 / latex_pt_to_in
mpl_pt_to_in = 1.0 / 72.0
mpl_in_to_pt = 1.0 / mpl_pt_to_in

def _tex_escape(text):
    ...

def _writeln(fh, line):
    ...

def _escape_and_apply_props(s, prop):
    ...

def _metadata_to_str(key, value):
    ...

def make_pdf_to_png_converter():
    ...

class LatexError(Exception):

    def __init__(self, message, latex_output=''):
        self.latex_output = latex_output

    def __str__(self):
        ...

class LatexManager:

    @staticmethod
    def _build_latex_header():
        ...

    @classmethod
    def _get_cached_or_new(cls):
        ...

    @classmethod
    @functools.lru_cache(1)
    def _get_cached_or_new_impl(cls, header):
        ...

    def _stdin_writeln(self, s):
        ...

    def _expect(self, s):
        self.latex = None

    def _expect_prompt(self):
        ...

    def __init__(self):
        self._tmpdir = TemporaryDirectory()
        self.tmpdir = self._tmpdir.name
        self._finalize_tmpdir = weakref.finalize(self, self._tmpdir.cleanup)
        self.latex = None
        self._get_box_metrics = functools.lru_cache(self._get_box_metrics)

    def _setup_latex_process(self, *, expect_reply=True):
        self._finalize_latex = weakref.finalize(self, finalize_latex, self.latex)
        self.latex = subprocess.Popen([mpl.rcParams['pgf.texsystem'], '-halt-on-error', '-no-shell-escape'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, encoding='utf-8', cwd=self.tmpdir)

    def get_width_height_descent(self, text, prop):
        ...

    def _get_box_metrics(self, tex):
        ...

@functools.lru_cache(1)
def _get_image_inclusion_command():
    ...

class RendererPgf(RendererBase):

    def __init__(self, figure, fh):
        self.dpi = figure.dpi
        self.fh = fh
        self.figure = figure
        self.image_counter = 0

    def draw_markers(self, gc, marker_path, marker_trans, path, trans, rgbFace=None):
        ...

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def _print_pgf_clip(self, gc):
        ...

    def _print_pgf_path_styles(self, gc, rgbFace):
        ...

    def _print_pgf_path(self, gc, path, transform, rgbFace=None):
        ...

    def _pgf_path_draw(self, stroke=True, fill=False):
        ...

    def option_scale_image(self):
        ...

    def option_image_nocomposite(self):
        ...

    def draw_image(self, gc, x, y, im, transform=None):
        ...

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

    def flipy(self):
        ...

    def get_canvas_width_height(self):
        ...

    def points_to_pixels(self, points):
        ...

class FigureCanvasPgf(FigureCanvasBase):
    filetypes = {'pgf': 'LaTeX PGF picture', 'pdf': 'LaTeX compiled PGF picture', 'png': 'Portable Network Graphics'}

    def get_default_filetype(self):
        ...

    def _print_pgf_to_fh(self, fh, *, bbox_inches_restore=None):
        ...

    def print_pgf(self, fname_or_fh, **kwargs):
        ...

    def print_pdf(self, fname_or_fh, *, metadata=None, **kwargs):
        ...

    def print_png(self, fname_or_fh, **kwargs):
        ...

    def get_renderer(self):
        ...

    def draw(self):
        ...
FigureManagerPgf = FigureManagerBase

@_Backend.export
class _BackendPgf(_Backend):
    FigureCanvas = FigureCanvasPgf

class PdfPages:

    def __init__(self, filename, *, metadata=None):
        self._output_name = filename
        self._n_figures = 0
        self._metadata = (metadata or {}).copy()
        self._info_dict = _create_pdf_info_dict('pgf', self._metadata)
        self._file = BytesIO()

    def _write_header(self, width_inches, height_inches):
        ...

    def __enter__(self):
        ...

    def __exit__(self, exc_type, exc_val, exc_tb):
        ...

    def close(self):
        ...

    def _run_latex(self):
        ...

    def savefig(self, figure=None, **kwargs):
        ...

    def get_pagecount(self):
        ...
__all__ = ['codecs', 'datetime', 'functools', 'BytesIO', 'logging', 'math', 'os', 'pathlib', 'shutil', 'subprocess', 'TemporaryDirectory', 'weakref', 'Image', 'mpl', 'cbook', 'fm', 'FigureCanvasBase', 'FigureManagerBase', 'RendererBase', 'MixedModeRenderer', 'Path', 'Figure', 'FontProperties', 'Gcf', 'latex_pt_to_in', 'latex_in_to_pt', 'mpl_pt_to_in', 'mpl_in_to_pt', 'make_pdf_to_png_converter', 'LatexError', 'LatexManager', 'RendererPgf', 'FigureCanvasPgf', 'FigureManagerPgf', 'PdfPages']
