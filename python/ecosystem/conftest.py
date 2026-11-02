"""
pltrs (Python) installed as ``matplotlib`` under the libraries that
import matplotlib by name (``cargo xtask ecosystem``): pandas' plotting
and seaborn, drawn to PNG.

Every test draws one figure with matplotlib's test settings
(``matplotlib.testing.setup()``), saves it into ``ECOSYSTEM_OUT`` (or
pytest's temporary directory) and compares a summary of what it drew
with the summary real matplotlib 3.11.2 gives for the same code,
recorded in ``expected.json``: run the tests under real matplotlib with
``ECOSYSTEM_RECORD=1`` to record them again. A module whose library is
not installed is skipped.
"""

import json
import os
import pathlib
import random

import numpy as np
import pytest

EXPECTED = pathlib.Path(__file__).with_name("expected.json")


def _rounded(values):
    """*values* to 6 significant digits, as a list."""
    return [float(f"{value:.6g}") for value in values]


def summary(fig):
    """
    What a figure holds that the libraries decide: its axes, and for each
    the number of its artists by kind, its labels, title and limits, the
    legend texts of the axes and the figure.

    The axes of a colorbar count no artists: pltrs keeps the parts of a
    colorbar (its solids, dividers and outline) as artists of other kinds
    than matplotlib's, which draw the same.
    """
    axes = []
    for ax in fig.axes:
        legend = ax.get_legend()
        if hasattr(ax, "_colorbar"):
            axes.append({"colorbar": True, "xlim": _rounded(ax.get_xlim()),
                         "ylim": _rounded(ax.get_ylim())})
            continue
        axes.append({
            "lines": len(ax.lines),
            "patches": len(ax.patches),
            "collections": len(ax.collections),
            "texts": len(ax.texts),
            "xlabel": ax.get_xlabel(),
            "ylabel": ax.get_ylabel(),
            "title": ax.get_title(),
            "xlim": _rounded(ax.get_xlim()),
            "ylim": _rounded(ax.get_ylim()),
            "legend": None if legend is None else [t.get_text() for t in legend.texts],
        })
    return {
        "size": _rounded(fig.get_size_inches()),
        "axes": axes,
        "legends": [[t.get_text() for t in legend.texts] for legend in fig.legends],
    }


@pytest.fixture
def check(request, tmp_path):
    """Saves the figure of a test and checks its summary against real
    matplotlib's (or records it, with ``ECOSYSTEM_RECORD=1``)."""
    import matplotlib

    def check(result):
        fig = getattr(result, "figure", None) or getattr(result, "fig", None)
        if fig is None:
            fig = getattr(result, "_figure", None) or matplotlib.pyplot.gcf()
        out = pathlib.Path(os.environ.get("ECOSYSTEM_OUT", tmp_path))
        out.mkdir(parents=True, exist_ok=True)
        fig.savefig(out / f"{request.node.name}.png")
        got = summary(fig)
        recorded = json.loads(EXPECTED.read_text()) if EXPECTED.exists() else {}
        if os.environ.get("ECOSYSTEM_RECORD"):
            recorded[request.node.nodeid.split("::", 1)[1]] = got
            EXPECTED.write_text(json.dumps(recorded, indent=1, sort_keys=True) + "\n")
            return
        assert got == recorded[request.node.nodeid.split("::", 1)[1]]

    return check


@pytest.fixture(autouse=True)
def matplotlib_test_settings():
    """matplotlib's test settings and seeded random numbers for every test,
    no figure left after it."""
    import matplotlib
    import matplotlib.pyplot as plt
    import matplotlib.testing
    matplotlib.testing.setup()
    np.random.seed(0)
    random.seed(0)
    yield
    plt.close("all")


@pytest.fixture
def frame():
    """A frame of deterministic data: curves of `x`, a category `cat`, a
    second one `hue` and a size."""
    pd = pytest.importorskip("pandas")
    n = 120
    i = np.arange(n)
    x = np.linspace(0, 10, n)
    return pd.DataFrame({
        "x": x,
        "y": np.sin(x) + 0.3 * np.cos(7.3 * x),
        "z": np.cos(x) * 2 + 0.2 * np.sin(5.1 * x),
        "w": (i % 7) * 0.5 + np.sin(i * 0.37),
        "cat": np.array(["a", "b", "c"])[i % 3],
        "hue": np.array(["u", "v"])[(i // 3) % 2],
        "size": (i % 5) + 1,
    })


@pytest.fixture
def small():
    """A frame of two short columns indexed by letters."""
    pd = pytest.importorskip("pandas")
    return pd.DataFrame({"p": [3, 5, 2, 7], "q": [1, 4, 6, 2]}, index=list("ABCD"))


@pytest.fixture
def series():
    """A daily time series."""
    pd = pytest.importorskip("pandas")
    n = 120
    return pd.Series(np.cumsum(np.sin(np.arange(n) * 0.3)),
                     index=pd.date_range("2024-01-01", periods=n, freq="D"))
