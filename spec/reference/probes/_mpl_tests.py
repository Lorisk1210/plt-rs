"""What the probe areas porting matplotlib's own tests (``mpl_*``) share:
the conditions matplotlib's test fixture runs a test under.

Every op of these areas runs in matplotlib's test style unless its
``style`` argument says otherwise: ``classic`` plus ``_classic_test_patch``
on top of the reference configuration, as the test fixture applies them to
every test without a style of its own. ``style: "mpl20"`` (tests marked
``mpl20`` or ``default``) runs in the reference configuration itself. An
``rc`` argument is applied on top, as ``rc_context(rc)``. pyplot's figures
are closed afterwards.
"""

import contextlib

import matplotlib as mpl
import matplotlib.pyplot as plt

#: The test suite's default style.
CLASSIC = ["classic", "_classic_test_patch"]


def in_test_style(fn):
    """``fn(**args)`` under the ``style`` and ``rc`` arguments."""

    def run(style=None, rc=None, **args):
        with contextlib.ExitStack() as stack:
            if style is None:
                stack.enter_context(mpl.style.context(CLASSIC))
            elif style != "mpl20":
                raise ValueError(f"unknown test style {style!r}")
            stack.enter_context(mpl.rc_context(rc or {}))
            try:
                return fn(**args)
            finally:
                plt.close("all")

    return run


def prober(probes):
    """A ``probe(op, fn, **args)`` function recording into ``probes`` in
    the test style."""

    def probe(op, fn, tol=None, **args):
        return probes.probe(op, in_test_style(fn), tol=tol, **args)

    return probe
