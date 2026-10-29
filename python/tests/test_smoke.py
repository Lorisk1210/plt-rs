"""
pltrs's own smoke tests (``cargo xtask python test``): the vertical slice
end to end, quickly. matplotlib's suite (``cargo xtask mpl-suite``) is
the real measure; these catch a broken build or binding in seconds.
"""

import io
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.testing.compare import compare_images
from pltrs.testing.decorators import check_figures_equal
from pltrs.testing.exceptions import ImageComparisonFailure


@pytest.fixture(autouse=True)
def close_figures():
    """Start and end every test without figures and with default rcParams."""
    with pltrs.rc_context():
        yield
    plt.close("all")


def test_pyplot_slice_draws_and_saves_every_format(tmp_path):
    fig, ax = plt.subplots()
    line, = plt.plot([1, 2, 3], [4, 5, 6], "r--", lw=3, label="line")
    plt.scatter([1, 2], [3, 4], s=20, c="g", label="points")
    bars = plt.bar([1, 2], [3, 4], label="bars")
    plt.title("Title")
    plt.xlabel("x")
    plt.ylabel("y")
    legend = plt.legend()
    assert plt.gcf() is fig and plt.gca() is ax
    assert ax.get_title() == "Title" and ax.get_xlabel() == "x"
    assert [t.get_text() for t in legend.get_texts()] == ["line", "points", "bars"]
    assert line.get_linewidth() == 3 and line.get_linestyle() == "--"
    assert bars.patches[1].get_height() == 4
    for fmt, magic in [("png", b"\x89PNG"), ("svg", b"<?xml"), ("pdf", b"%PDF")]:
        path = tmp_path / f"figure.{fmt}"
        plt.savefig(path)
        assert path.read_bytes().startswith(magic)
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png")
    assert buffer.getvalue().startswith(b"\x89PNG")


def test_artists_keep_their_identity_and_attributes():
    fig = plt.figure()
    ax = fig.add_subplot(111)
    line, = ax.plot(np.arange(5.0))
    line.note = "kept"
    assert ax.lines[0] is line and ax.lines[0].note == "kept"
    assert fig.axes == [ax] and line.figure is fig
    np.testing.assert_array_equal(line.get_xdata(), [0, 1, 2, 3, 4])
    line.set(color="tab:green", linewidth=2)
    assert line.get_color() == "tab:green" and line.get_lw() == 2


def test_figure_numbers_and_closing():
    first = plt.figure()
    second = plt.figure("labelled")
    assert plt.get_fignums() == [1, 2]
    assert plt.figure(1) is first and plt.gcf() is first
    plt.close("labelled")
    assert plt.get_fignums() == [1] and second.number == 2
    plt.close("all")
    assert plt.get_fignums() == []


def test_rcparams_validate_through_plt_rs():
    pltrs.rcParams["lines.linewidth"] = 4
    assert pltrs.rcParams["lines.linewidth"] == 4.0
    with pytest.raises(ValueError, match="lines.linewidth"):
        pltrs.rcParams["lines.linewidth"] = "wide"
    with pytest.raises(KeyError, match="not a valid rc parameter"):
        pltrs.rcParams["no.such.key"] = 1
    with pltrs.rc_context({"lines.linewidth": 7}):
        assert pltrs.rcParams["lines.linewidth"] == 7.0
    assert pltrs.rcParams["lines.linewidth"] == 4.0
    assert "plt.ink" not in pltrs.rcParams
    standalone = pltrs.RcParams({"figure.figsize": (3.5, 42)})
    assert dict(standalone) == {"figure.figsize": [3.5, 42.0]}


def test_python_formatters_run_inside_draws():
    seen = []

    def label(x, pos):
        seen.append(pos)
        return f"<{x:g}>"

    fig, ax = plt.subplots()
    ax.plot([0, 1])
    ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
    fig.savefig(io.BytesIO(), format="png")
    assert seen and all(isinstance(pos, int) for pos in seen)


def test_an_exception_in_a_formatter_reaches_the_caller():
    def broken(x, pos):
        raise ZeroDivisionError("from the formatter")

    fig, ax = plt.subplots()
    ax.yaxis.set_major_formatter(broken)
    with pytest.raises(ZeroDivisionError, match="from the formatter"):
        fig.savefig(io.BytesIO(), format="png")
    ax.yaxis.set_major_formatter(ticker.NullFormatter())
    fig.savefig(io.BytesIO(), format="png")


def test_native_errors_are_matplotlibs_exceptions():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="same first dimension"):
        ax.plot([1, 2], [1, 2, 3])
    with pytest.raises(AttributeError, match="unexpected keyword argument 'bogus'"):
        ax.plot([1, 2], bogus=1)
    with pytest.raises(TypeError, match="expected a native"):
        pltrs._native.lines.get_linewidth(ax._native)
    with pytest.raises(ValueError):
        fig.savefig(io.BytesIO(), format="nonsense")


@check_figures_equal()
def test_check_figures_equal_compares_through_plt_rs(fig_test, fig_ref):
    fig_test.subplots().plot([1, 3, 5])
    fig_ref.subplots().plot([0, 1, 2], [1, 3, 5])


def test_compare_images_reports_differences(tmp_path):
    for name, values in [("a", [1, 2, 3]), ("b", [3, 2, 1])]:
        fig, ax = plt.subplots()
        ax.plot(values)
        fig.savefig(tmp_path / f"{name}.png")
    assert compare_images(tmp_path / "a.png", tmp_path / "a.png", 0) is None
    result = compare_images(tmp_path / "a.png", tmp_path / "b.png", 0,
                            in_decorator=True)
    assert result["rms"] > 0 and result["diff"].endswith("-failed-diff.png")
    small = plt.figure(figsize=(2, 2))
    small.savefig(tmp_path / "small.png")
    with pytest.raises(ImageComparisonFailure, match="sizes do not match"):
        compare_images(tmp_path / "a.png", tmp_path / "small.png", 0)


def test_handles_belong_to_their_thread():
    import threading
    fig, ax = plt.subplots()
    errors = []
    thread = threading.Thread(target=lambda: errors.append(
        pytest.raises(RuntimeError, ax.get_title)))
    thread.start()
    thread.join()
    assert errors and "another thread" in str(errors[0].value)


def test_style_use_applies_bundled_sheets():
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        pltrs.style.use("classic")
    assert pltrs.rcParams["lines.linewidth"] == 1.0
