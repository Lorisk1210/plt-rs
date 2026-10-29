"""Whether usetex and the PostScript backend's external programs can run: the
spec's cases and probes that need them skip, not fail, where they are
missing.

``has_usetex`` is matplotlib's ``testing._markers._checkdep_usetex`` without
its Ghostscript check (Ghostscript only serves the PostScript backend):
``tex`` on ``PATH`` and a ``dvipng`` matplotlib can run, plus ``latex``,
which ``TexManager`` runs. usetex PostScript also needs ``dvips``, LaTeX's
``psfrag`` package and Ghostscript (``has_usetex_ps``), and the distillers
of ``ps.usedistiller`` Ghostscript and, for ``xpdf``, ``pdftops``.
"""

from __future__ import annotations

import functools
import shutil

import pytest

#: Why cases needing TeX are skipped where it is missing.
SKIP_REASON = "usetex needs TeX (tex, latex and dvipng on PATH)"

#: Why usetex PostScript is skipped where its programs are missing.
PS_SKIP_REASON = "usetex PostScript needs TeX, dvips, LaTeX's psfrag package and Ghostscript"


def _executable(name: str) -> bool:
    """Whether ``_get_executable_info(name)`` finds the program."""
    from matplotlib import ExecutableNotFoundError, _get_executable_info

    try:
        _get_executable_info(name)
    except ExecutableNotFoundError:
        return False
    return True


@functools.cache
def has_usetex() -> bool:
    """matplotlib's ``_checkdep_usetex``, Ghostscript aside."""
    if not shutil.which("tex") or not shutil.which("latex"):
        return False
    return _executable("dvipng")


@functools.cache
def has_ghostscript() -> bool:
    """Whether Ghostscript can run (the ``ghostscript`` distiller)."""
    return _executable("gs")


@functools.cache
def has_xpdf() -> bool:
    """Whether Ghostscript's ``ps2pdf`` and ``pdftops`` can run (the ``xpdf``
    distiller)."""
    return has_ghostscript() and _executable("pdftops")


@functools.cache
def has_usetex_ps() -> bool:
    """Whether usetex text can be saved as PostScript: TeX, ``dvips``,
    ``psfrag`` and Ghostscript."""
    from matplotlib.testing import _has_tex_package

    return (has_usetex() and shutil.which("dvips") is not None
            and _has_tex_package("psfrag") and has_ghostscript())


def document_skip_reason(case, fmt: str, rc: dict | None = None) -> str | None:
    """Why ``case`` cannot be saved as ``fmt`` with the rcParams ``rc`` on top
    of its own here, if it cannot: TeX is missing, or for PostScript what
    usetex or the distiller of ``ps.usedistiller`` runs."""
    if case.needs_tex and not has_usetex():
        return SKIP_REASON
    if fmt not in ("ps", "eps"):
        return None
    if case.needs_tex and not has_usetex_ps():
        return PS_SKIP_REASON
    distiller = {**case.rc, **(rc or {})}.get("ps.usedistiller")
    if distiller == "ghostscript" and not has_ghostscript():
        return "the ghostscript distiller needs Ghostscript"
    if distiller == "xpdf" and not has_xpdf():
        return "the xpdf distiller needs Ghostscript and pdftops"
    return None


#: Marks a test needing TeX.
needs_usetex = pytest.mark.skipif(not has_usetex(), reason=SKIP_REASON)
