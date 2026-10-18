"""A font database holding only matplotlib's bundled fonts.

Goldens that depend on font lookup must not depend on the fonts installed on
the host. :func:`bundled_fonts_only` swaps matplotlib's font list for one
built from ``mpl-data/fonts/{ttf,afm,pdfcorefonts}``, each file added with
``FontManager.addfont`` in ascending order of basename (so ``findfont``
tie-breaks are deterministic), and restores the host list afterwards.
"""

from __future__ import annotations

import contextlib
import logging
import os
from collections.abc import Iterator

import matplotlib
from matplotlib import cbook, font_manager
from matplotlib.mathtext import MathTextParser


def bundled_font_files(fontext: str) -> list[str]:
    """The bundled font files of type ``fontext`` (``ttf`` or ``afm``), sorted by basename."""
    dirs = [cbook._get_data_path("fonts", d) for d in ("ttf", "afm", "pdfcorefonts")]
    return sorted(font_manager.findSystemFonts(dirs, fontext=fontext), key=os.path.basename)


def clear_font_caches() -> None:
    """Forget every font lookup and mathtext parse cached by matplotlib."""
    fm = font_manager.fontManager
    fm._findfont_cached.cache_clear()
    font_manager._get_font.cache_clear()
    MathTextParser._parse_cached.cache_clear()


@contextlib.contextmanager
def bundled_fonts_only() -> Iterator[None]:
    """Pretend only mpl-data fonts are installed while the context is active."""
    fm = font_manager.fontManager
    saved = fm.ttflist, fm.afmlist
    logger = logging.getLogger("matplotlib.font_manager")
    saved_level = logger.level
    logger.setLevel(logging.ERROR)
    fm.ttflist, fm.afmlist = [], []
    try:
        for fontext in ("afm", "ttf"):
            for path in bundled_font_files(fontext):
                fm.addfont(path)
        clear_font_caches()
        data = os.path.realpath(matplotlib.get_data_path()) + os.sep
        assert all(os.path.realpath(f.fname).startswith(data) for f in fm.ttflist + fm.afmlist)
        yield
    finally:
        fm.ttflist, fm.afmlist = saved
        clear_font_caches()
        logger.setLevel(saved_level)
