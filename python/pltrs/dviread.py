"""
A module for reading dvi files output by TeX, and the files of the TeX
installation they refer to (matplotlib's ``dviread``).

pltrs (Rust) reads them itself (pltrs-text's usetex), and finds TeX's files with
kpathsea, as matplotlib does.
"""

import os

from ._native import dviread as _dviread


def find_tex_file(filename):
    """
    Find a file in the texmf tree using kpathsea_.

    The kpathsea library, provided by most existing TeX distributions, both
    on Unix-like systems and on Windows (MikTeX), is invoked via a long-lived
    luatex process if luatex is installed, or via kpsewhich otherwise.

    .. _kpathsea: https://www.tug.org/kpathsea/

    Parameters
    ----------
    filename : str or path-like

    Raises
    ------
    FileNotFoundError
        If the file is not found.
    """
    # we expect these to always be ascii encoded, but use utf-8
    # out of caution
    if isinstance(filename, bytes):
        filename = filename.decode('utf-8', errors='replace')
    return _dviread.find_tex_file(os.fspath(filename))
