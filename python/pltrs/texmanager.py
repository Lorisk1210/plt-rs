r"""
Support for embedded TeX expressions in Matplotlib (matplotlib's
``texmanager``).

Requirements:

* LaTeX.
* \*Agg backends: dvipng>=1.6.
* PS backend: PSfrag, dvips, and Ghostscript>=9.0.

To enable TeX rendering of all text in your Matplotlib figure, set
:rc:`text.usetex` to True.

pltrs (Rust) writes the documents, runs LaTeX and dvipng and reads their
output (``pltrs_text::tex``), caching the results in the ``tex.cache``
directory of the cache directory as matplotlib does; this class is
matplotlib's interface to it, reading the rcParams at each call.

`TexManager.get_rgba` can also be used to directly obtain raster output as RGBA
NumPy arrays.
"""

import functools
import operator

import numpy as np

from ._native import texmanager as _texmanager


def _rc(name):
    """The rcParam *name*."""
    from . import rcParams
    return rcParams._get(name)


def _val_or_rc(val, name):
    """*val*, or the rcParam *name* when it is None."""
    return _rc(name) if val is None else val


def _fontsize(fontsize):
    """
    *fontsize* as pltrs (Rust) writes it into the document, as matplotlib
    writes it (an int as an int), after the check matplotlib's first use of
    it makes: its baselineskip, ``1.25 * fontsize``, refuses what is no
    number with Python's own error.
    """
    operator.mul(1.25, fontsize)
    return fontsize


class TexManager:
    """
    Convert strings to dvi files using TeX, caching the results to a directory.

    The cache directory is called ``tex.cache`` and is located in the directory
    returned by `.get_cachedir`.

    Repeated calls to this constructor always return the same instance.
    """

    _grey_arrayd = {}

    _font_families = ('serif', 'sans-serif', 'cursive', 'monospace')

    @functools.lru_cache  # Always return the same instance.
    def __new__(cls):
        return object.__new__(cls)

    @classmethod
    def _get_base_path(cls, tex, fontsize, dpi=None):
        """
        Return a file path based on a hash of the string, fontsize, and dpi.
        """
        from pathlib import Path
        return Path(_texmanager.base_path(tex, _fontsize(fontsize), dpi))

    @classmethod
    def get_basefile(cls, tex, fontsize, dpi=None):  # Kept for backcompat.
        """
        Return a filename based on a hash of the string, fontsize, and dpi.
        """
        return str(cls._get_base_path(tex, fontsize, dpi))

    @classmethod
    def get_font_preamble(cls):
        """
        Return a string containing font configuration for the tex preamble.
        """
        return _texmanager.font_preamble()

    @classmethod
    def get_custom_preamble(cls):
        """Return a string containing user additions to the tex preamble."""
        return _rc('text.latex.preamble')

    @classmethod
    def _get_tex_source(cls, tex, fontsize):
        """Return the complete TeX source for processing a TeX string."""
        return _texmanager.tex_source(tex, _fontsize(fontsize))

    @classmethod
    def make_tex(cls, tex, fontsize):
        """
        Generate a tex file to render the tex string at a specific font size.

        Return the file name.
        """
        return _texmanager.make_tex(tex, _fontsize(fontsize))

    @classmethod
    def make_dvi(cls, tex, fontsize):
        """
        Generate a dvi file containing latex's layout of tex string.

        Return the file name.
        """
        return _texmanager.make_dvi(tex, _fontsize(fontsize))

    @classmethod
    def make_png(cls, tex, fontsize, dpi):
        """
        Generate a png file containing latex's rendering of tex string.

        Return the file name.
        """
        return _texmanager.make_png(tex, _fontsize(fontsize), dpi)

    @classmethod
    def get_grey(cls, tex, fontsize=None, dpi=None):
        """Return the alpha channel."""
        fontsize = _val_or_rc(fontsize, 'font.size')
        dpi = _val_or_rc(dpi, 'savefig.dpi')
        key = cls._get_tex_source(tex, fontsize), dpi
        alpha = cls._grey_arrayd.get(key)
        if alpha is None:
            grey = _texmanager.grey(tex, fontsize, dpi)
            cls._grey_arrayd[key] = alpha = grey.astype(np.float32) / 255
        return alpha

    @classmethod
    def get_rgba(cls, tex, fontsize=None, dpi=None, rgb=(0, 0, 0)):
        r"""
        Return latex's rendering of the tex string as an RGBA array.

        Examples
        --------
        >>> texmanager = TexManager()
        >>> s = r"\TeX\ is $\displaystyle\sum_n\frac{-e^{i\pi}}{2^n}$!"
        >>> Z = texmanager.get_rgba(s, fontsize=12, dpi=80, rgb=(1, 0, 0))
        """
        from .colors import to_rgb
        alpha = cls.get_grey(tex, fontsize, dpi)
        rgba = np.empty((*alpha.shape, 4))
        rgba[..., :3] = to_rgb(rgb)
        rgba[..., -1] = alpha
        return rgba

    @classmethod
    def get_text_width_height_descent(cls, tex, fontsize, renderer=None):
        """Return width, height and descent of the text."""
        if tex.strip() == '':
            return 0, 0, 0
        dpi_fraction = renderer.points_to_pixels(1.) if renderer else 1
        # A total height (including the descent) is returned.
        return _texmanager.text_width_height_descent(
            tex, float(fontsize), float(dpi_fraction))


# What matplotlib's module also has from its imports, which programs
# import from it.
import hashlib  # noqa: E402, F401
import logging  # noqa: E402, F401
import subprocess  # noqa: E402, F401
from pathlib import Path  # noqa: E402, F401
from tempfile import TemporaryDirectory  # noqa: E402, F401
from . import cbook, dviread  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
