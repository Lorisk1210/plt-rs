"""What the groups porting matplotlib's non-image tests (``mpl_tests_*``)
share: the conditions matplotlib's test fixture draws a test under.

A test without a style of its own draws under ``classic`` plus
``_classic_test_patch`` on classic's 8x6 in canvas; ``style="mpl20"``
(tests marked ``mpl20`` or ``default``) draws in the reference
configuration on its 6.4x4.8 in canvas. Cases are saved at 100 dpi.
"""

import matplotlib.style

from spec_support import case

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def group_case(group):
    """A decorator factory registering ``build`` as ``<group>/<name>``
    under the test fixture's conditions (see the module docstring)."""

    def register(name, *, style=None, figsize=None, rc=None):
        if style not in (None, "mpl20"):
            raise ValueError(f"unknown test style {style!r}")
        classic = style is None
        params = {**(CLASSIC if classic else {}), **(rc or {})}
        default_size = (8, 6) if classic else (6.4, 4.8)
        return case(f"{group}/{name}", figsize=figsize or default_size, rc=params)

    return register
