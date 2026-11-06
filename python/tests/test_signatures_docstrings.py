"""matplotlib's signatures and docstrings in pltrs (Python): what
``cargo xtask diff --docs`` measures, with the calls whose meaning the
signatures carry. The expected values are matplotlib 3.11's."""

import inspect
import io
import warnings

import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs._api import MatplotlibDeprecationWarning
from pltrs.artist import kwdoc
from pltrs.axes import Axes
from pltrs.contour import ContourSet
from pltrs.lines import Line2D
from pltrs.text import Text


def _parameters(callable_):
    """The parameters of *callable_* as ``(name, kind, default)``."""
    return [(p.name, p.kind.name, p.default) for p in inspect.signature(callable_).parameters.values()]


def test_constructors_take_matplotlibs_keywords():
    assert str(inspect.signature(Text)) == (
        "(x=0, y=0, text='', *, color=None, verticalalignment='baseline', "
        "horizontalalignment='left', multialignment=None, fontproperties=None, "
        "rotation=None, linespacing=None, rotation_mode=None, usetex=None, "
        "wrap=False, transform_rotates_text=False, parse_math=None, "
        "antialiased=None, **kwargs)")
    assert [name for name, *_ in _parameters(ContourSet)][:8] == [
        "ax", "args", "levels", "filled", "linewidths", "linestyles", "hatches", "alpha"]
    # A keyword given by name and by alias: matplotlib applies the name,
    # then the alias.
    text = Text(0, 0, "a", color="r", c="b", rotation=45)
    assert (text.get_color(), text.get_rotation()) == ("b", 45.0)


def test_positional_parameters_matplotlib_deprecated_warn():
    fig, ax = plt.subplots()
    with pytest.warns(MatplotlibDeprecationWarning,
                      match="Passing the label parameter of hlines"):
        ax.hlines([1], 0, 1, "k", "solid", "lab")
    assert [(name, kind) for name, kind, _ in _parameters(plt.hist)][:3] == [
        ("x", "POSITIONAL_OR_KEYWORD"), ("bins", "POSITIONAL_OR_KEYWORD"),
        ("range", "KEYWORD_ONLY")]
    plt.close(fig)


def test_methods_have_matplotlibs_parameters():
    assert str(inspect.signature(Axes.stackplot)) == (
        "(axes, x, *args, labels=(), colors=None, baseline='zero', data=None, **kwargs)")
    assert str(inspect.signature(Axes.tricontour)) == "(ax, *args, **kwargs)"
    assert str(inspect.signature(Axes.set_xticklabels)) == (
        "(self, labels, *, minor=False, fontdict=None, **kwargs)")
    assert str(inspect.signature(plt.figtext)) == "(x, y, s, fontdict=None, **kwargs)"
    from pltrs.projections.polar import PolarAxes
    assert str(inspect.signature(PolarAxes.set_rticks)) == "(self, *args, **kwargs)"
    from pltrs.mpl_toolkits.mplot3d.art3d import Poly3DCollection
    assert str(inspect.signature(Poly3DCollection)) == (
        "(verts, *args, zsort='average', shade=False, lightsource=None, "
        "axlim_clip=False, **kwargs)")
    poly = Poly3DCollection([[(0, 0, 0), (1, 0, 0), (0, 1, 0)]], closed=False)
    assert poly._closed is False


def test_autoscale_none_keeps_the_setting():
    fig, ax = plt.subplots()
    ax.set_autoscalex_on(False)
    ax.set_autoscalex_on(None)
    assert ax.get_autoscalex_on() is False
    ax3 = fig.add_subplot(projection="3d")
    ax3.set_autoscalez_on(False)
    ax3.set_autoscalez_on(None)
    assert ax3.get_autoscalez_on() is False
    plt.close(fig)


def test_nested_subfigures_warn_for_get_figure():
    fig = plt.figure()
    nested = fig.subfigures(1, 1).subfigures(1, 1)
    with pytest.warns(MatplotlibDeprecationWarning, match="SubFigure.get_figure"):
        assert nested.get_figure() is fig
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert fig.get_figure() is fig
    plt.close(fig)


def test_svg_writer_names():
    from pltrs.backends import backend_svg
    with pytest.warns(MatplotlibDeprecationWarning, match="XMLWriter class was deprecated"):
        backend_svg.XMLWriter(io.StringIO())
    assert str(inspect.signature(backend_svg._XMLWriter.start)) == "(self, tag, attrib=None, **extra)"


def test_property_tables_are_matplotlibs():
    assert [line for line in kwdoc(Line2D).splitlines()
            if line.strip().startswith(("antialiased", "transform", "alpha"))] == [
        "    alpha: float or None", "    antialiased or aa: bool",
        "    transform: `~matplotlib.transforms.Transform`"]
    # matplotlib writes the docstring of ``set`` as the class is defined.
    assert [line for line in Line2D.set.__doc__.splitlines()
            if line.strip().startswith(("antialiased", "transform"))] == [
        "    antialiased: bool", "    transform: unknown"]


def test_pyplot_docstrings_are_the_methods_with_the_note():
    assert plt.plot.__doc__.startswith("Plot y versus x as lines and/or markers.")
    assert ("Notes\n-----\n\n.. note::\n\n    This is the :ref:`pyplot wrapper "
            "<pyplot_interface>` for `.axes.Axes.plot`.\n") in plt.plot.__doc__
    assert ("This is the :ref:`pyplot wrapper <pyplot_interface>` for "
            "`.axes.Axes.hist`.") in plt.hist.__doc__
    assert "pyplot wrapper" not in plt.sci.__doc__
    assert "figlegend(" in plt.figlegend.__doc__
    assert plt.fill_between.__doc__.startswith("Fill the area between two horizontal curves.")
    assert plt.figure().suptitle.__doc__.startswith(
        "Add a centered super title to the figure.\n\nParameters\n------")
    plt.close("all")


def test_tex_file_lookups_are_cached():
    from pltrs import dviread
    assert hasattr(dviread.find_tex_file, "cache_clear")


def test_patheffect_default_is_numpys():
    import numpy as np
    from pltrs.patheffects import TickedStroke
    default = inspect.signature(TickedStroke).parameters["length"].default
    assert type(default) is np.float64 and default == np.sqrt(2)


def test_package_functions_are_documented_as_matplotlibs():
    assert pltrs.artist.getp.__module__ == "pltrs.artist"
    assert plt.getp.__doc__.endswith("This is equivalent to `pltrs.artist.getp`.\n")
