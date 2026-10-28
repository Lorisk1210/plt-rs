"""Whether usetex can run: the spec's cases and probes that need TeX skip,
not fail, where it is missing.

``has_usetex`` is matplotlib's ``testing._markers._checkdep_usetex`` without
its Ghostscript check (Ghostscript only serves the PostScript backend, whose
usetex output the spec does not cover): ``tex`` on ``PATH`` and a ``dvipng``
matplotlib can run, plus ``latex``, which ``TexManager`` runs.
"""

from __future__ import annotations

import functools
import shutil

import pytest

#: Why cases needing TeX are skipped where it is missing.
SKIP_REASON = "usetex needs TeX (tex, latex and dvipng on PATH)"


@functools.cache
def has_usetex() -> bool:
    """matplotlib's ``_checkdep_usetex``, Ghostscript aside."""
    from matplotlib import ExecutableNotFoundError, _get_executable_info

    if not shutil.which("tex") or not shutil.which("latex"):
        return False
    try:
        _get_executable_info("dvipng")
    except ExecutableNotFoundError:
        return False
    return True


#: Marks a test needing TeX.
needs_usetex = pytest.mark.skipif(not has_usetex(), reason=SKIP_REASON)
