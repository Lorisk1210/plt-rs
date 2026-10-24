"""matplotlib's non-image tests from ``tests/test_patches.py`` that only
draw: ``test_patch_linestyle_accents`` (cut stars edged in the eight
spellings of the named line styles) and
``test_patch_zero_linewidth_dashed_uses_solid_gc_dashes`` (an unfilled
dashed rectangle without width, in the ``mpl20`` style). The other tests
of the file are probes of the ``mpl_patches`` and ``patch_behaviour``
areas or cases of the ``mpl_collections`` group.
"""

import numpy as np
from matplotlib import patches as mpatches
from matplotlib import path as mpath

from figures._mpl_tests import group_case

GROUP = "mpl_tests_patches"
mpl_test = group_case(GROUP)


@mpl_test("patch_linestyle_accents")
def _(fig):
    star = mpath.Path.unit_regular_star(6)
    circle = mpath.Path.unit_circle()
    verts = np.concatenate([circle.vertices, star.vertices[::-1]])
    codes = np.concatenate([circle.codes, star.codes])
    linestyles = ["-", "--", "-.", ":", "solid", "dashed", "dashdot", "dotted"]
    ax = fig.subplots()
    for i, ls in enumerate(linestyles):
        patch = mpatches.PathPatch(mpath.Path(verts + i, codes), linewidth=3, linestyle=ls,
                                   facecolor=(1, 0, 0), edgecolor=(0, 0, 1))
        ax.add_patch(patch)
    ax.set_xlim([-1, i + 1])
    ax.set_ylim([-1, i + 1])


@mpl_test("patch_zero_linewidth_dashed", style="mpl20")
def _(fig):
    ax = fig.subplots()
    ax.add_patch(mpatches.Rectangle((0, 0), 1, 1, fill=False, linewidth=0, linestyle="--"))
