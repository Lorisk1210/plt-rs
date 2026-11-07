"""A font database holding only matplotlib's bundled fonts.

Goldens that depend on font lookup must not depend on the fonts installed on
the host. :func:`bundled_fonts_only` swaps matplotlib's font list for one
built from ``mpl-data/fonts/{ttf,afm,pdfcorefonts}``, each file added with
``FontManager.addfont`` in ascending order of basename (so ``findfont``
tie-breaks are deterministic), and restores the host list afterwards.
:func:`figure_fonts_only` adds the fonts pltrs bundles beyond matplotlib's
that the figure cases install (:func:`add_spec_font`), which pltrs sees
there too.
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


#: Fonts pltrs bundles and matplotlib does not, as the figure cases that
#: use them add them (:func:`add_spec_font`).
_SPEC_FONTS: list[str] = []


def add_spec_font(path: str) -> None:
    """Install the font file ``path``, one pltrs bundles, for the figure
    cases: now, and inside every :func:`figure_fonts_only`."""
    font_manager.fontManager.addfont(path)
    _SPEC_FONTS.append(path)


def figure_fonts_only() -> contextlib.AbstractContextManager[None]:
    """Pretend only the fonts pltrs bundles are installed (matplotlib's and
    those added with :func:`add_spec_font`) while the context is active."""
    return bundled_fonts_only(extra=tuple(_SPEC_FONTS))


@contextlib.contextmanager
def bundled_fonts_only(extra: tuple[str, ...] = ()) -> Iterator[None]:
    """Pretend only mpl-data fonts, and the files in ``extra``, are
    installed while the context is active."""
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
        for path in extra:
            fm.addfont(path)
        clear_font_caches()
        data = os.path.realpath(matplotlib.get_data_path()) + os.sep
        extra_files = {os.path.realpath(path) for path in extra}
        assert all(os.path.realpath(f.fname).startswith(data) or os.path.realpath(f.fname) in extra_files
                   for f in fm.ttflist + fm.afmlist)
        yield
    finally:
        fm.ttflist, fm.afmlist = saved
        clear_font_caches()
        logger.setLevel(saved_level)
